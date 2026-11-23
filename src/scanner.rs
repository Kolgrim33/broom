use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::cli::{Config, SortBy};
use crate::ecosystems::{self, Ecosystem};

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

struct WorkItem {
    dir: PathBuf,
    depth: usize,
}

struct WorkQueue {
    items: VecDeque<WorkItem>,
    active_workers: usize,
}

pub fn scan(cfg: &Config) -> ScanReport {
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
                    let (found, next_dirs) = process_directory(&work.dir, work.depth, cfg, now);

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
        if !cfg.ecosystems.is_empty() && !cfg.ecosystems.contains(&a.ecosystem) {
            return false;
        }
        true
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

            let (size_bytes, file_count, artifact_mtime) = measure_dir(path);
            let effective_mtime = newest_project_mtime
                .or(artifact_mtime)
                .or_else(|| fs::metadata(dir).and_then(|m| m.modified()).ok());
            let project_age = effective_mtime
                .and_then(|m| now.duration_since(m).ok())
                .unwrap_or(Duration::ZERO);

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

/// Computes actual disk usage (with hardlink deduplication on Unix) and file count
/// without following symlinks.
pub fn measure_dir(root: &Path) -> (u64, u64, Option<SystemTime>) {
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
                    // st_blocks is always in 512-byte units on POSIX/Linux/macOS
                    let blocks_bytes = meta.blocks().saturating_mul(512);
                    total_bytes = total_bytes.saturating_add(if blocks_bytes > 0 {
                        blocks_bytes
                    } else {
                        meta.len()
                    });
                }
                #[cfg(not(unix))]
                {
                    total_bytes = total_bytes.saturating_add(meta.len());
                }
            }
        }
    }

    (total_bytes, total_files, newest_mtime)
}
