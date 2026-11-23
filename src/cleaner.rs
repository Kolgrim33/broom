use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::scanner::Artifact;

#[derive(Debug)]
pub struct CleanFailure {
    pub path: String,
    pub error: String,
}

#[derive(Debug)]
pub struct CleanSummary {
    pub deleted_count: usize,
    pub freed_bytes: u64,
    pub failures: Vec<CleanFailure>,
    pub elapsed: Duration,
}

/// Validates that a path is safe to delete (not filesystem root, not `$HOME`, not a symlink).
fn assert_safe_target(path: &Path) -> Result<(), String> {
    if path.parent().is_none() {
        return Err("refusing to delete filesystem root".to_string());
    }
    if let Some(home) = std::env::var_os("HOME")
        && let (Ok(canon_path), Ok(canon_home)) =
            (fs::canonicalize(path), fs::canonicalize(Path::new(&home)))
        && canon_path == canon_home
    {
        return Err("refusing to delete home directory".to_string());
    }
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("refusing to delete symlink".to_string());
    }
    if !meta.file_type().is_dir() {
        return Err("target is not a directory".to_string());
    }
    Ok(())
}

pub fn delete_artifact(artifact: &Artifact) -> Result<u64, String> {
    assert_safe_target(&artifact.path)?;
    fs::remove_dir_all(&artifact.path).map_err(|e| e.to_string())?;
    Ok(artifact.size_bytes)
}

pub fn clean_artifacts(targets: &[&Artifact]) -> CleanSummary {
    let start = Instant::now();
    let mut deleted_count = 0usize;
    let mut freed_bytes = 0u64;
    let mut failures = Vec::new();

    for artifact in targets {
        match delete_artifact(artifact) {
            Ok(bytes) => {
                deleted_count += 1;
                freed_bytes = freed_bytes.saturating_add(bytes);
            }
            Err(err) => {
                failures.push(CleanFailure {
                    path: artifact.path.to_string_lossy().into_owned(),
                    error: err,
                });
            }
        }
    }

    CleanSummary {
        deleted_count,
        freed_bytes,
        failures,
        elapsed: start.elapsed(),
    }
}

/// Parses an interactive selection string like `"1,3-5,8"` into sorted, deduplicated 0-based indices.
pub fn parse_selection(input: &str, max_len: usize) -> Result<Vec<usize>, String> {
    let mut selected = Vec::new();
    for token in input.split(',') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        if let Some((start_s, end_s)) = t.split_once('-') {
            let start: usize = start_s
                .trim()
                .parse()
                .map_err(|_| format!("invalid range '{t}'"))?;
            let end: usize = end_s
                .trim()
                .parse()
                .map_err(|_| format!("invalid range '{t}'"))?;
            if start == 0 || end == 0 || start > end || end > max_len {
                return Err(format!("range '{t}' out of bounds (valid: 1..={max_len})"));
            }
            for idx in start..=end {
                selected.push(idx - 1);
            }
        } else {
            let idx: usize = t
                .parse()
                .map_err(|_| format!("invalid item number '{t}'"))?;
            if idx == 0 || idx > max_len {
                return Err(format!("item '{idx}' out of bounds (valid: 1..={max_len})"));
            }
            selected.push(idx - 1);
        }
    }
    selected.sort_unstable();
    selected.dedup();
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_index_selections() {
        assert_eq!(parse_selection("1,3-5,2", 6).unwrap(), vec![0, 1, 2, 3, 4]);
        assert_eq!(parse_selection("4", 4).unwrap(), vec![3]);
        assert!(parse_selection("0", 5).is_err());
        assert!(parse_selection("2-6", 5).is_err());
        assert!(parse_selection("4-2", 5).is_err());
    }
}
