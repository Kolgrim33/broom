use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::format::{CYAN, DIM, GREEN, RESET, YELLOW, format_bytes};

const INITIAL_DELAY: Duration = Duration::from_millis(120);
const TICK_INTERVAL: Duration = Duration::from_millis(65);
const BAR_WIDTH: usize = 16;
const SLUG_WIDTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Artifacts,
    Files,
}

/// Live atomic counters updated by scanner workers.
pub struct ScanProgress {
    pub dirs: AtomicUsize,
    pub files: AtomicUsize,
    pub found: AtomicUsize,
    pub bytes: AtomicU64,
}

impl ScanProgress {
    fn new() -> Self {
        Self {
            dirs: AtomicUsize::new(0),
            files: AtomicUsize::new(0),
            found: AtomicUsize::new(0),
            bytes: AtomicU64::new(0),
        }
    }

    pub fn add_dir(&self) {
        self.dirs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_files(&self, n: usize) {
        if n > 0 {
            self.files.fetch_add(n, Ordering::Relaxed);
        }
    }

    pub fn add_found(&self, bytes: u64) {
        self.found.fetch_add(1, Ordering::Relaxed);
        if bytes > 0 {
            self.bytes.fetch_add(bytes, Ordering::Relaxed);
        }
    }

    pub fn add_bytes(&self, bytes: u64) {
        if bytes > 0 {
            self.bytes.fetch_add(bytes, Ordering::Relaxed);
        }
    }
}

/// Live state updated by the cleaner as targets are deleted.
pub struct CleanProgress {
    pub total_count: usize,
    pub total_bytes: u64,
    pub done_count: AtomicUsize,
    pub done_bytes: AtomicU64,
    current_label: Mutex<String>,
}

impl CleanProgress {
    fn new(total_count: usize, total_bytes: u64) -> Self {
        Self {
            total_count,
            total_bytes,
            done_count: AtomicUsize::new(0),
            done_bytes: AtomicU64::new(0),
            current_label: Mutex::new(String::new()),
        }
    }

    pub fn set_current(&self, label: String) {
        if let Ok(mut guard) = self.current_label.lock() {
            *guard = label;
        }
    }

    pub fn complete_item(&self, bytes: u64) {
        self.done_count.fetch_add(1, Ordering::Relaxed);
        if bytes > 0 {
            self.done_bytes.fetch_add(bytes, Ordering::Relaxed);
        }
    }

    fn current_label(&self) -> String {
        self.current_label
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }
}

enum ProgressState {
    Scan {
        mode: ScanMode,
        progress: Arc<ScanProgress>,
    },
    Clean {
        progress: Arc<CleanProgress>,
    },
}

struct StopSignal {
    stopped: AtomicBool,
    mutex: Mutex<bool>,
    condvar: Condvar,
}

impl StopSignal {
    fn new() -> Self {
        Self {
            stopped: AtomicBool::new(false),
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        if let Ok(mut done) = self.mutex.lock() {
            *done = true;
            self.condvar.notify_all();
        }
    }

    /// Waits up to `dur` or returns early when stopped. Returns `true` if stopped.
    fn wait_timeout(&self, dur: Duration) -> bool {
        if self.is_stopped() {
            return true;
        }
        let Ok(guard) = self.mutex.lock() else {
            return true;
        };
        if *guard {
            return true;
        }
        let (guard, _) = match self.condvar.wait_timeout(guard, dur) {
            Ok(res) => res,
            Err(_) => return true,
        };
        *guard
    }
}

/// RAII progress bar rendered on `stderr` when attached to a TTY.
/// Spawns no thread when disabled, delays initial render to avoid flicker on fast runs,
/// and always clears its line cleanly on drop.
pub struct ProgressBar {
    stop: Option<Arc<StopSignal>>,
    handle: Option<JoinHandle<bool>>,
}

impl ProgressBar {
    pub fn start_scan(
        mode: ScanMode,
        enabled: bool,
        color: bool,
    ) -> (Self, Option<Arc<ScanProgress>>) {
        if !enabled {
            return (Self::disabled(), None);
        }
        let progress = Arc::new(ScanProgress::new());
        let state = ProgressState::Scan {
            mode,
            progress: Arc::clone(&progress),
        };
        (Self::spawn(state, color), Some(progress))
    }

    pub fn start_clean(
        total_count: usize,
        total_bytes: u64,
        enabled: bool,
        color: bool,
    ) -> (Self, Option<Arc<CleanProgress>>) {
        if !enabled || total_count == 0 {
            return (Self::disabled(), None);
        }
        let progress = Arc::new(CleanProgress::new(total_count, total_bytes));
        let state = ProgressState::Clean {
            progress: Arc::clone(&progress),
        };
        (Self::spawn(state, color), Some(progress))
    }

    fn disabled() -> Self {
        Self {
            stop: None,
            handle: None,
        }
    }

    fn spawn(state: ProgressState, color: bool) -> Self {
        let stop = Arc::new(StopSignal::new());
        let stop_clone = Arc::clone(&stop);
        let start = Instant::now();

        let handle = thread::spawn(move || {
            // Avoid flashing a progress bar for instant operations.
            if stop_clone.wait_timeout(INITIAL_DELAY) {
                return false;
            }

            let mut stderr = io::stderr();
            let mut tick: usize = 0;
            let mut rendered = false;

            loop {
                let line = render_frame(&state, tick, start.elapsed(), color);
                if write!(stderr, "\r\x1b[2K{line}").is_ok() {
                    let _ = stderr.flush();
                    rendered = true;
                }
                tick = tick.wrapping_add(1);

                if stop_clone.wait_timeout(TICK_INTERVAL) {
                    break;
                }
            }

            rendered
        });

        Self {
            stop: Some(stop),
            handle: Some(handle),
        }
    }

    pub fn finish(mut self) {
        self.stop_and_clear();
    }

    fn stop_and_clear(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.stop();
        }
        if let Some(handle) = self.handle.take()
            && let Ok(rendered) = handle.join()
            && rendered
        {
            let mut stderr = io::stderr();
            let _ = write!(stderr, "\r\x1b[2K");
            let _ = stderr.flush();
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.stop_and_clear();
    }
}

pub fn should_show_progress(json_mode: bool) -> bool {
    if json_mode {
        return false;
    }
    if std::env::var("TERM").ok().as_deref() == Some("dumb") {
        return false;
    }
    io::stderr().is_terminal()
}

pub fn should_use_stderr_color(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    io::stderr().is_terminal()
}

fn render_frame(state: &ProgressState, tick: usize, elapsed: Duration, color: bool) -> String {
    match state {
        ProgressState::Scan { mode, progress } => render_scan_line(
            *mode,
            progress.dirs.load(Ordering::Relaxed),
            progress.files.load(Ordering::Relaxed),
            progress.found.load(Ordering::Relaxed),
            progress.bytes.load(Ordering::Relaxed),
            tick,
            elapsed,
            color,
        ),
        ProgressState::Clean { progress } => render_clean_line(
            progress.done_count.load(Ordering::Relaxed),
            progress.total_count,
            progress.done_bytes.load(Ordering::Relaxed),
            progress.total_bytes,
            &progress.current_label(),
            tick,
            elapsed,
            color,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_scan_line(
    mode: ScanMode,
    dirs: usize,
    files: usize,
    found: usize,
    bytes: u64,
    tick: usize,
    elapsed: Duration,
    color: bool,
) -> String {
    let bar = render_indeterminate_bar(tick, BAR_WIDTH, SLUG_WIDTH, color);
    let elapsed_s = format_elapsed_progress(elapsed);

    match mode {
        ScanMode::Artifacts => {
            let dirs_s = format_count(dirs);
            if found > 0 || bytes > 0 {
                let bytes_s = format_bytes(bytes);
                if color {
                    format!(
                        "  {bar}  Scanning  {dirs_s} dirs · {YELLOW}{found} found ({bytes_s}){RESET} · {DIM}{elapsed_s}{RESET}"
                    )
                } else {
                    format!(
                        "  {bar}  Scanning  {dirs_s} dirs · {found} found ({bytes_s}) · {elapsed_s}"
                    )
                }
            } else if color {
                format!("  {bar}  Scanning  {dirs_s} dirs · {DIM}{elapsed_s}{RESET}")
            } else {
                format!("  {bar}  Scanning  {dirs_s} dirs · {elapsed_s}")
            }
        }
        ScanMode::Files => {
            let files_s = format_count(files);
            let dirs_s = format_count(dirs);
            if color {
                format!(
                    "  {bar}  Scanning  {files_s} files in {dirs_s} dirs · {DIM}{elapsed_s}{RESET}"
                )
            } else {
                format!("  {bar}  Scanning  {files_s} files in {dirs_s} dirs · {elapsed_s}")
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_clean_line(
    done_count: usize,
    total_count: usize,
    done_bytes: u64,
    total_bytes: u64,
    current_label: &str,
    tick: usize,
    elapsed: Duration,
    color: bool,
) -> String {
    let label = truncate_tail(current_label, 28);
    let elapsed_s = format_elapsed_progress(elapsed);

    if total_count <= 1 {
        let bar = render_indeterminate_bar(tick, BAR_WIDTH, SLUG_WIDTH, color);
        let size_s = format_bytes(total_bytes);
        if color {
            format!("  {bar}  Sweeping  {label} {DIM}({size_s} · {elapsed_s}){RESET}")
        } else {
            format!("  {bar}  Sweeping  {label} ({size_s} · {elapsed_s})")
        }
    } else {
        let (done_ratio, total_ratio) = if total_bytes > 0 {
            (done_bytes.min(total_bytes), total_bytes)
        } else {
            (done_count.min(total_count) as u64, total_count as u64)
        };
        let (bar, pct) = render_determinate_bar(done_ratio, total_ratio, BAR_WIDTH, color);
        let freed_s = format_bytes(done_bytes);
        let total_s = format_bytes(total_bytes);
        if label.is_empty() {
            if color {
                format!(
                    "  {bar} {pct:>3}%  Sweeping {done_count}/{total_count} {DIM}({freed_s} / {total_s}){RESET}"
                )
            } else {
                format!(
                    "  {bar} {pct:>3}%  Sweeping {done_count}/{total_count} ({freed_s} / {total_s})"
                )
            }
        } else if color {
            format!(
                "  {bar} {pct:>3}%  Sweeping {done_count}/{total_count} {DIM}({freed_s} / {total_s}){RESET}  {label}"
            )
        } else {
            format!(
                "  {bar} {pct:>3}%  Sweeping {done_count}/{total_count} ({freed_s} / {total_s})  {label}"
            )
        }
    }
}

fn render_indeterminate_bar(tick: usize, width: usize, slug: usize, color: bool) -> String {
    let slug = slug.min(width);
    let travel = width.saturating_sub(slug);
    let pos = if travel == 0 {
        0
    } else {
        let period = travel * 2;
        let phase = tick % period;
        if phase <= travel {
            phase
        } else {
            period - phase
        }
    };
    let left = "░".repeat(pos);
    let bar = "█".repeat(slug);
    let right = "░".repeat(width - pos - slug);

    if color {
        format!("{DIM}{left}{RESET}{CYAN}{bar}{RESET}{DIM}{right}{RESET}")
    } else {
        format!("{left}{bar}{right}")
    }
}

fn render_determinate_bar(done: u64, total: u64, width: usize, color: bool) -> (String, u8) {
    let ratio = if total == 0 {
        1.0
    } else {
        (done as f64 / total as f64).clamp(0.0, 1.0)
    };
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let pct = (ratio * 100.0).round() as u8;
    let left = "█".repeat(filled);
    let right = "░".repeat(width - filled);

    let bar = if color {
        format!("{GREEN}{left}{RESET}{DIM}{right}{RESET}")
    } else {
        format!("{left}{right}")
    };
    (bar, pct)
}

fn format_count(n: usize) -> String {
    let s = n.to_string();
    let len = s.len();
    if len <= 3 {
        return s;
    }
    let mut out = String::with_capacity(len + len / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn format_elapsed_progress(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{:.1}s", d.as_secs_f64())
    } else {
        format!("{}m {:02}s", secs / 60, secs % 60)
    }
}

fn truncate_tail(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars || max_chars == 0 {
        return s.to_string();
    }
    let skip = count - (max_chars - 1);
    let tail: String = s.chars().skip(skip).collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_counts_with_commas() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(42), "42");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(42_810), "42,810");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn indeterminate_bar_bounces_smoothly() {
        assert_eq!(render_indeterminate_bar(0, 8, 2, false), "██░░░░░░");
        assert_eq!(render_indeterminate_bar(1, 8, 2, false), "░██░░░░░");
        assert_eq!(render_indeterminate_bar(6, 8, 2, false), "░░░░░░██");
        assert_eq!(render_indeterminate_bar(7, 8, 2, false), "░░░░░██░");
        assert_eq!(render_indeterminate_bar(12, 8, 2, false), "██░░░░░░");
    }

    #[test]
    fn determinate_bar_tracks_percentage() {
        let (bar0, pct0) = render_determinate_bar(0, 100, 10, false);
        assert_eq!((bar0.as_str(), pct0), ("░░░░░░░░░░", 0));

        let (bar50, pct50) = render_determinate_bar(50, 100, 10, false);
        assert_eq!((bar50.as_str(), pct50), ("█████░░░░░", 50));

        let (bar100, pct100) = render_determinate_bar(100, 100, 10, false);
        assert_eq!((bar100.as_str(), pct100), ("██████████", 100));
    }

    #[test]
    fn truncates_long_paths_from_left() {
        assert_eq!(truncate_tail("short/path", 20), "short/path");
        assert_eq!(
            truncate_tail("very/long/project/node_modules", 15),
            "…t/node_modules"
        );
    }

    #[test]
    fn formats_scan_and_clean_lines() {
        let scan_line = render_scan_line(
            ScanMode::Artifacts,
            1_250,
            0,
            3,
            1_073_741_824,
            2,
            Duration::from_millis(1_400),
            false,
        );
        assert!(scan_line.contains("1,250 dirs"), "{scan_line}");
        assert!(scan_line.contains("3 found (1.00 GiB)"), "{scan_line}");
        assert!(scan_line.contains("1.4s"), "{scan_line}");

        let clean_line = render_clean_line(
            2,
            4,
            512 * 1024 * 1024,
            1024 * 1024 * 1024,
            "mycrate/target",
            0,
            Duration::from_secs(2),
            false,
        );
        assert!(clean_line.contains(" 50%"), "{clean_line}");
        assert!(clean_line.contains("Sweeping 2/4"), "{clean_line}");
        assert!(clean_line.contains("mycrate/target"), "{clean_line}");
    }
}
