use std::cmp::Ordering as CmpOrdering;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::cli::{Config, SortBy};
use crate::ecosystems::{self, Ecosystem};
use crate::progress::ScanProgress;

pub const DEFAULT_FILES_LIMIT: usize = 25;

#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: PathBuf,
    pub project_root: PathBuf,
    pub ecosystem: Ecosystem,
    pub size_bytes: u64,
    pub file_count: u64,
    pub project_age: Duration,
}

#[derive(Debug)]
pub struct ScanReport {
    pub artifacts: Vec<Artifact>,
    pub dirs_scanned: usize,
    pub elapsed: Duration,
}

#[derive(Debug, Clone)]
pub struct LargeFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub age: Duration,
    pub extension: String,
}

#[derive(Debug)]
pub struct FileScanReport {
    pub files: Vec<LargeFile>,
    pub total_matched: usize,
    pub files_scanned: usize,
    pub dirs_scanned: usize,
    pub elapsed: Duration,
}

struct WorkItem {
    dir: PathBuf,
    depth: usize,
}

struct WorkQueue {
    items: VecDeque<WorkItem>,
    active_workers: usize,
}

pub fn scan(cfg: &Config, progress: Option<&ScanProgress>) -> ScanReport {
    let start = Instant::now();
    let now = SystemTime::now();

    let num_threads = cfg.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 16)
    });

    let queue = Mutex::new(WorkQueue {
        items: VecDeque::new(),
        active_workers: 0,
    });
    let condvar = Condvar::new();
    let results: Mutex<Vec<Artifact>> = Mutex::new(Vec::new());
    let visited: Mutex<HashSet<PathBuf>> = Mutex::new(HashSet::new());
    let dirs_scanned = AtomicUsize::new(0);

    {
        let mut q = queue.lock().unwrap();
        let mut seen = visited.lock().unwrap();
        for p in &cfg.paths {
            let resolved = fs::canonicalize(p).unwrap_or_else(|_| p.clone());
            if resolved.is_dir() && seen.insert(resolved.clone()) {
                q.items.push_back(WorkItem {
                    dir: resolved,
                    depth: 0,
                });
            }
        }
    }

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            scope.spawn(|| {
                loop {
                    let work = {
                        let mut q = queue.lock().unwrap();
                        loop {
                            if let Some(item) = q.items.pop_front() {
                                q.active_workers += 1;
                                break Some(item);
                            }
                            if q.active_workers == 0 {
                                break None;
                            }
                            q = condvar.wait(q).unwrap();
                        }
                    };

                    let Some(work) = work else {
                        break;
                    };

                    dirs_scanned.fetch_add(1, Ordering::Relaxed);
                    if let Some(p) = progress {
                        p.add_dir();
                    }
                    let (found, next_dirs) =
                        process_directory(&work.dir, work.depth, cfg, now, progress);

                    if !found.is_empty() {
                        results.lock().unwrap().extend(found);
                    }

                    {
                        let mut q = queue.lock().unwrap();
                        if !next_dirs.is_empty() {
                            let mut seen = visited.lock().unwrap();
                            for d in next_dirs {
                                if seen.insert(d.clone()) {
                                    q.items.push_back(WorkItem {
                                        dir: d,
                                        depth: work.depth + 1,
                                    });
                                }
                            }
                        }
                        q.active_workers -= 1;
                        if !q.items.is_empty() || q.active_workers == 0 {
                            condvar.notify_all();
                        }
                    }
                }
            });
        }
    });

    let mut artifacts = results.into_inner().unwrap();

    // Apply filters
    artifacts.retain(|a| {
        if a.size_bytes < cfg.min_size {
            return false;
        }
        if let Some(min_age) = cfg.older_than
            && a.project_age < min_age
        {
            return false;
        }
        cfg.ecosystems.is_empty() || cfg.ecosystems.contains(&a.ecosystem)
    });

    // Sort results
    artifacts.sort_by(|a, b| {
        let ord = match cfg.sort_by {
            SortBy::Size => b
                .size_bytes
                .cmp(&a.size_bytes)
                .then_with(|| a.path.cmp(&b.path)),
            SortBy::Age => b
                .project_age
                .cmp(&a.project_age)
                .then_with(|| b.size_bytes.cmp(&a.size_bytes)),
            SortBy::Path => a.path.cmp(&b.path),
        };
        if cfg.reverse { ord.reverse() } else { ord }
    });

    if let Some(limit) = cfg.limit {
        artifacts.truncate(limit);
    }

    ScanReport {
        artifacts,
        dirs_scanned: dirs_scanned.load(Ordering::Relaxed),
        elapsed: start.elapsed(),
    }
}

fn process_directory(
    dir: &Path,
    depth: usize,
    cfg: &Config,
    now: SystemTime,
    progress: Option<&ScanProgress>,
) -> (Vec<Artifact>, Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };

    let mut file_names: Vec<String> = Vec::new();
    let mut subdirs: Vec<(String, PathBuf)> = Vec::new();
    let mut newest_project_mtime: Option<SystemTime> = None;

    for entry in read_dir.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        // Never follow symlinks
        if ft.is_symlink() {
            continue;
        }
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy().into_owned();
        let path = entry.path();

        if ft.is_dir() {
            if name == ".git" {
                if let Some(git_mtime) = git_activity_mtime(&path) {
                    update_newest(&mut newest_project_mtime, git_mtime);
                }
                continue;
            }
            subdirs.push((name, path));
        } else {
            if let Ok(meta) = entry.metadata()
                && let Ok(m) = meta.modified()
            {
                update_newest(&mut newest_project_mtime, m);
            }
            file_names.push(name);
        }
    }

    let mut found = Vec::new();
    let mut next_dirs = Vec::new();
    let mut matched_dirs = HashSet::new();

    // First pass: check if any subdir is a cleanable artifact of `dir`
    for (name, path) in &subdirs {
        if is_excluded(path, &cfg.excludes) {
            continue;
        }
        if !ecosystems::is_candidate_dir_name(name) {
            continue;
        }
        if let Some(ecosystem) = ecosystems::match_artifact(name, path, &file_names) {
            matched_dirs.insert(path.clone());

            // Pre-filter ecosystem early to avoid sizing directories the user doesn't care about
            if !cfg.ecosystems.is_empty() && !cfg.ecosystems.contains(&ecosystem) {
                continue;
            }

            let stream_bytes = cfg.min_size == 0 && cfg.older_than.is_none();
            let (size_bytes, file_count, artifact_mtime) =
                measure_dir_inner(path, if stream_bytes { progress } else { None });
            let effective_mtime = newest_project_mtime
                .or(artifact_mtime)
                .or_else(|| fs::metadata(dir).and_then(|m| m.modified()).ok());
            let project_age = effective_mtime
                .and_then(|m| now.duration_since(m).ok())
                .unwrap_or(Duration::ZERO);

            if let Some(p) = progress {
                if stream_bytes {
                    p.add_found(0);
                } else if size_bytes >= cfg.min_size
                    && cfg.older_than.is_none_or(|min_age| project_age >= min_age)
                {
                    p.add_found(size_bytes);
                }
            }

            found.push(Artifact {
                path: path.clone(),
                project_root: dir.to_path_buf(),
                ecosystem,
                size_bytes,
                file_count,
                project_age,
            });
        }
    }

    // Second pass: enqueue remaining non-artifact subdirectories for traversal
    let can_descend = cfg.max_depth.is_none_or(|max| depth < max);
    if can_descend {
        for (name, path) in subdirs {
            if matched_dirs.contains(&path) {
                continue;
            }
            if is_excluded(&path, &cfg.excludes) {
                continue;
            }
            // Skip hidden directories unless explicitly requested
            if !cfg.include_hidden && name.starts_with('.') {
                continue;
            }
            // Never recurse into known huge non-project OS/VCS directories
            if matches!(name.as_str(), ".git" | ".hg" | ".svn" | "proc" | "sys") {
                continue;
            }
            next_dirs.push(path);
        }
    }

    (found, next_dirs)
}

fn is_excluded(path: &Path, excludes: &[String]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let s = path.to_string_lossy();
    excludes.iter().any(|pat| s.contains(pat.as_str()))
}

fn git_activity_mtime(git_dir: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    for rel in ["index", "HEAD", "FETCH_HEAD", "COMMIT_EDITMSG"] {
        if let Ok(meta) = fs::metadata(git_dir.join(rel))
            && let Ok(m) = meta.modified()
        {
            update_newest(&mut newest, m);
        }
    }
    newest
}

fn update_newest(slot: &mut Option<SystemTime>, candidate: SystemTime) {
    match slot {
        Some(existing) if *existing >= candidate => {}
        _ => *slot = Some(candidate),
    }
}

/// Computes actual disk usage of a single file (POSIX blocks on Unix, apparent length otherwise).
fn file_disk_bytes(meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // st_blocks is always in 512-byte units on POSIX/Linux/macOS
        let blocks_bytes = meta.blocks().saturating_mul(512);
        if blocks_bytes > 0 {
            blocks_bytes
        } else {
            meta.len()
        }
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}

/// Computes actual disk usage (with hardlink deduplication on Unix) and file count
/// without following symlinks.
fn measure_dir_inner(
    root: &Path,
    progress: Option<&ScanProgress>,
) -> (u64, u64, Option<SystemTime>) {
    let mut stack = vec![root.to_path_buf()];
    let mut total_bytes: u64 = 0;
    let mut total_files: u64 = 0;
    let mut newest_mtime: Option<SystemTime> = None;
    #[cfg(unix)]
    let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut dir_bytes: u64 = 0;
        for entry in entries.flatten() {
            let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            let ft = meta.file_type();
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(entry.path());
            } else {
                total_files += 1;
                if let Ok(m) = meta.modified() {
                    update_newest(&mut newest_mtime, m);
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let nlink = meta.nlink();
                    if nlink > 1 && !seen_inodes.insert((meta.dev(), meta.ino())) {
                        continue;
                    }
                }
                dir_bytes = dir_bytes.saturating_add(file_disk_bytes(&meta));
            }
        }
        total_bytes = total_bytes.saturating_add(dir_bytes);
        if let Some(p) = progress {
            p.add_bytes(dir_bytes);
        }
    }

    (total_bytes, total_files, newest_mtime)
}

struct WorstFirst(LargeFile);

impl PartialEq for WorstFirst {
    fn eq(&self, other: &Self) -> bool {
        self.0.size_bytes == other.0.size_bytes && self.0.path == other.0.path
    }
}

impl Eq for WorstFirst {}

impl PartialOrd for WorstFirst {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for WorstFirst {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        // Max-heap root is the worst item among the top-K: smallest size, or largest path on tie.
        other
            .0
            .size_bytes
            .cmp(&self.0.size_bytes)
            .then_with(|| self.0.path.cmp(&other.0.path))
    }
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Parallel walker that finds the largest individual files under `cfg.paths`.
pub fn scan_files(cfg: &Config, progress: Option<&ScanProgress>) -> FileScanReport {
    let start = Instant::now();
    let now = SystemTime::now();
    let effective_limit = cfg.limit.unwrap_or(DEFAULT_FILES_LIMIT);
    let use_top_heap = cfg.sort_by == SortBy::Size && !cfg.reverse && effective_limit > 0;

    let num_threads = cfg.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 16)
    });

    let queue = Mutex::new(WorkQueue {
        items: VecDeque::new(),
        active_workers: 0,
    });
    let condvar = Condvar::new();
    let results: Mutex<Vec<LargeFile>> = Mutex::new(Vec::new());
    let visited: Mutex<HashSet<PathBuf>> = Mutex::new(HashSet::new());
    let dirs_scanned = AtomicUsize::new(0);
    let files_scanned = AtomicUsize::new(0);
    let total_matched = AtomicUsize::new(0);

    {
        let mut q = queue.lock().unwrap();
        let mut seen = visited.lock().unwrap();
        let mut initial_files = Vec::new();
        for p in &cfg.paths {
            let resolved = fs::canonicalize(p).unwrap_or_else(|_| p.clone());
            if !seen.insert(resolved.clone()) {
                continue;
            }
            if resolved.is_dir() {
                q.items.push_back(WorkItem {
                    dir: resolved,
                    depth: 0,
                });
            } else if resolved.is_file() {
                files_scanned.fetch_add(1, Ordering::Relaxed);
                if !is_excluded(&resolved, &cfg.excludes)
                    && let Ok(meta) = fs::metadata(&resolved)
                {
                    let size_bytes = file_disk_bytes(&meta);
                    let age = meta
                        .modified()
                        .ok()
                        .and_then(|m| now.duration_since(m).ok())
                        .unwrap_or(Duration::ZERO);
                    if size_bytes >= cfg.min_size
                        && cfg.older_than.is_none_or(|min_age| age >= min_age)
                    {
                        total_matched.fetch_add(1, Ordering::Relaxed);
                        let extension = file_extension(&resolved);
                        initial_files.push(LargeFile {
                            path: resolved,
                            size_bytes,
                            age,
                            extension,
                        });
                    }
                }
            }
        }
        if !initial_files.is_empty() {
            results.lock().unwrap().extend(initial_files);
        }
    }

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            scope.spawn(|| {
                let mut local_heap: BinaryHeap<WorstFirst> = if use_top_heap {
                    BinaryHeap::with_capacity(effective_limit)
                } else {
                    BinaryHeap::new()
                };
                let mut local_vec: Vec<LargeFile> = Vec::new();
                let mut local_files_scanned: usize = 0;
                let mut local_matched: usize = 0;

                loop {
                    let work = {
                        let mut q = queue.lock().unwrap();
                        loop {
                            if let Some(item) = q.items.pop_front() {
                                q.active_workers += 1;
                                break Some(item);
                            }
                            if q.active_workers == 0 {
                                break None;
                            }
                            q = condvar.wait(q).unwrap();
                        }
                    };

                    let Some(work) = work else {
                        break;
                    };

                    dirs_scanned.fetch_add(1, Ordering::Relaxed);
                    if let Some(p) = progress {
                        p.add_dir();
                    }
                    let mut next_dirs = Vec::new();
                    let can_descend = cfg.max_depth.is_none_or(|max| work.depth < max);
                    let mut dir_files_scanned: usize = 0;

                    if let Ok(read_dir) = fs::read_dir(&work.dir) {
                        for entry in read_dir.flatten() {
                            let Ok(ft) = entry.file_type() else {
                                continue;
                            };
                            if ft.is_symlink() {
                                continue;
                            }
                            let name_os = entry.file_name();
                            let name = name_os.to_string_lossy();
                            let path = entry.path();

                            if is_excluded(&path, &cfg.excludes) {
                                continue;
                            }

                            if ft.is_dir() {
                                if !can_descend {
                                    continue;
                                }
                                if !cfg.include_hidden && name.starts_with('.') {
                                    continue;
                                }
                                if matches!(name.as_ref(), ".git" | ".hg" | ".svn" | "proc" | "sys")
                                {
                                    continue;
                                }
                                next_dirs.push(path);
                            } else if ft.is_file() {
                                if !cfg.include_hidden && name.starts_with('.') {
                                    continue;
                                }
                                dir_files_scanned += 1;

                                let Ok(meta) = entry.metadata() else {
                                    continue;
                                };
                                let size_bytes = file_disk_bytes(&meta);
                                if size_bytes < cfg.min_size {
                                    continue;
                                }
                                let age = meta
                                    .modified()
                                    .ok()
                                    .and_then(|m| now.duration_since(m).ok())
                                    .unwrap_or(Duration::ZERO);
                                if let Some(min_age) = cfg.older_than
                                    && age < min_age
                                {
                                    continue;
                                }

                                local_matched += 1;

                                if effective_limit == 0 {
                                    continue;
                                }

                                if use_top_heap {
                                    if local_heap.len() == effective_limit
                                        && let Some(worst) = local_heap.peek()
                                        && (size_bytes < worst.0.size_bytes
                                            || (size_bytes == worst.0.size_bytes
                                                && path >= worst.0.path))
                                    {
                                        continue;
                                    }
                                    let extension = file_extension(&path);
                                    let item = WorstFirst(LargeFile {
                                        path,
                                        size_bytes,
                                        age,
                                        extension,
                                    });
                                    if local_heap.len() == effective_limit {
                                        local_heap.pop();
                                    }
                                    local_heap.push(item);
                                } else {
                                    let extension = file_extension(&path);
                                    local_vec.push(LargeFile {
                                        path,
                                        size_bytes,
                                        age,
                                        extension,
                                    });
                                }
                            }
                        }
                    }

                    if dir_files_scanned > 0 {
                        local_files_scanned += dir_files_scanned;
                        if let Some(p) = progress {
                            p.add_files(dir_files_scanned);
                        }
                    }

                    {
                        let mut q = queue.lock().unwrap();
                        if !next_dirs.is_empty() {
                            let mut seen = visited.lock().unwrap();
                            for d in next_dirs {
                                if seen.insert(d.clone()) {
                                    q.items.push_back(WorkItem {
                                        dir: d,
                                        depth: work.depth + 1,
                                    });
                                }
                            }
                        }
                        q.active_workers -= 1;
                        if !q.items.is_empty() || q.active_workers == 0 {
                            condvar.notify_all();
                        }
                    }
                }

                files_scanned.fetch_add(local_files_scanned, Ordering::Relaxed);
                total_matched.fetch_add(local_matched, Ordering::Relaxed);

                if use_top_heap {
                    if !local_heap.is_empty() {
                        results
                            .lock()
                            .unwrap()
                            .extend(local_heap.into_iter().map(|w| w.0));
                    }
                } else if !local_vec.is_empty() {
                    results.lock().unwrap().extend(local_vec);
                }
            });
        }
    });

    let mut files = results.into_inner().unwrap();

    files.sort_by(|a, b| {
        let ord = match cfg.sort_by {
            SortBy::Size => b
                .size_bytes
                .cmp(&a.size_bytes)
                .then_with(|| a.path.cmp(&b.path)),
            SortBy::Age => b
                .age
                .cmp(&a.age)
                .then_with(|| b.size_bytes.cmp(&a.size_bytes))
                .then_with(|| a.path.cmp(&b.path)),
            SortBy::Path => a.path.cmp(&b.path),
        };
        if cfg.reverse { ord.reverse() } else { ord }
    });

    files.truncate(effective_limit);

    FileScanReport {
        files,
        total_matched: total_matched.load(Ordering::Relaxed),
        files_scanned: files_scanned.load(Ordering::Relaxed),
        dirs_scanned: dirs_scanned.load(Ordering::Relaxed),
        elapsed: start.elapsed(),
    }
}
