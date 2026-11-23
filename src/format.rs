use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use crate::scanner::{Artifact, ScanReport};

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN: &str = "\x1b[36m";

pub fn should_use_color(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;

    let b = bytes as f64;
    if b >= TIB {
        format!("{:.2} TiB", b / TIB)
    } else if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3_600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3_600)
    } else if secs < 30 * 86_400 {
        format!("{}d ago", secs / 86_400)
    } else if secs < 365 * 86_400 {
        format!("{}mo ago", secs / (30 * 86_400))
    } else {
        format!("{}y ago", secs / (365 * 86_400))
    }
}

pub fn format_duration_short(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1_000 {
        format!("{ms}ms")
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

fn render_bar(size: u64, max_size: u64, width: usize) -> String {
    if max_size == 0 {
        return "░".repeat(width);
    }
    let ratio = (size as f64 / max_size as f64).clamp(0.0, 1.0);
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let filled = if size > 0 && filled == 0 { 1 } else { filled };
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

pub fn display_path(path: &Path, cwd: Option<&Path>) -> String {
    if let Some(base) = cwd
        && let Ok(rel) = path.strip_prefix(base)
    {
        let s = rel.to_string_lossy();
        if !s.is_empty() {
            return s.into_owned();
        }
    }
    if let Some(home) = std::env::var_os("HOME")
        && let Ok(rel) = path.strip_prefix(Path::new(&home))
    {
        return format!("~/{}", rel.to_string_lossy());
    }
    path.to_string_lossy().into_owned()
}

pub fn print_table(report: &ScanReport, color: bool) {
    let cwd = std::env::current_dir().ok();
    let max_size = report
        .artifacts
        .iter()
        .map(|a| a.size_bytes)
        .max()
        .unwrap_or(0);
    let idx_width = report.artifacts.len().to_string().len().max(2);

    if color {
        println!(
            "\n  {BOLD}{DIM}{:>idx_w$}  {:<10}  {:>10}  {:<10}  {:>8}  PATH{RESET}",
            "#",
            "TYPE",
            "SIZE",
            "SHARE",
            "AGE",
            idx_w = idx_width
        );
    } else {
        println!(
            "\n  {:>idx_w$}  {:<10}  {:>10}  {:<10}  {:>8}  PATH",
            "#",
            "TYPE",
            "SIZE",
            "SHARE",
            "AGE",
            idx_w = idx_width
        );
    }

    for (i, item) in report.artifacts.iter().enumerate() {
        let num = i + 1;
        let eco_name = item.ecosystem.name();
        let size_str = format_bytes(item.size_bytes);
        let bar = render_bar(item.size_bytes, max_size, 10);
        let age_str = format_age(item.project_age);
        let path_str = display_path(&item.path, cwd.as_deref());

        if color {
            let eco_color = item.ecosystem.ansi_color();
            let size_color = if item.size_bytes >= 1_073_741_824 {
                RED
            } else if item.size_bytes >= 104_857_600 {
                YELLOW
            } else {
                CYAN
            };
            let age_color = if item.project_age.as_secs() >= 30 * 86_400 {
                DIM
            } else {
                GREEN
            };
            println!(
                "  {DIM}{num:>idx_w$}{RESET}  {eco_color}{eco_name:<10}{RESET}  {size_color}{BOLD}{size_str:>10}{RESET}  {size_color}{bar}{RESET}  {age_color}{age_str:>8}{RESET}  {path_str}",
                idx_w = idx_width
            );
        } else {
            println!(
                "  {num:>idx_w$}  {eco_name:<10}  {size_str:>10}  {bar}  {age_str:>8}  {path_str}",
                idx_w = idx_width
            );
        }
    }

    let total_bytes: u64 = report.artifacts.iter().map(|a| a.size_bytes).sum();
    let count = report.artifacts.len();
    let noun = if count == 1 { "artifact" } else { "artifacts" };
    let elapsed = format_duration_short(report.elapsed);

    println!();
    if color {
        println!(
            "  {BOLD}Total:{RESET} {YELLOW}{BOLD}{}{RESET} across {BOLD}{count}{RESET} {noun} {DIM}(scanned {} dirs in {elapsed}){RESET}",
            format_bytes(total_bytes),
            report.dirs_scanned
        );
    } else {
        println!(
            "  Total: {} across {count} {noun} (scanned {} dirs in {elapsed})",
            format_bytes(total_bytes),
            report.dirs_scanned
        );
    }
}

pub fn render_json(report: &ScanReport) -> String {
    let total_bytes: u64 = report.artifacts.iter().map(|a| a.size_bytes).sum();
    let mut out = String::with_capacity(256 + report.artifacts.len() * 160);
    out.push_str("{\n");
    out.push_str(&format!("  \"total_bytes\": {total_bytes},\n"));
    out.push_str(&format!(
        "  \"total_human\": \"{}\",\n",
        json_escape(&format_bytes(total_bytes))
    ));
    out.push_str(&format!("  \"count\": {},\n", report.artifacts.len()));
    out.push_str(&format!("  \"dirs_scanned\": {},\n", report.dirs_scanned));
    out.push_str(&format!(
        "  \"elapsed_ms\": {},\n",
        report.elapsed.as_millis()
    ));
    out.push_str("  \"artifacts\": [\n");
    for (i, a) in report.artifacts.iter().enumerate() {
        push_artifact_json(&mut out, a);
        if i + 1 < report.artifacts.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

fn push_artifact_json(out: &mut String, a: &Artifact) {
    out.push_str("    {\n");
    out.push_str(&format!(
        "      \"path\": \"{}\",\n",
        json_escape(&a.path.to_string_lossy())
    ));
    out.push_str(&format!(
        "      \"project_root\": \"{}\",\n",
        json_escape(&a.project_root.to_string_lossy())
    ));
    out.push_str(&format!(
        "      \"ecosystem\": \"{}\",\n",
        json_escape(a.ecosystem.id())
    ));
    out.push_str(&format!("      \"size_bytes\": {},\n", a.size_bytes));
    out.push_str(&format!(
        "      \"size_human\": \"{}\",\n",
        json_escape(&format_bytes(a.size_bytes))
    ));
    out.push_str(&format!("      \"files\": {},\n", a.file_count));
    out.push_str(&format!(
        "      \"project_age_secs\": {},\n",
        a.project_age.as_secs()
    ));
    out.push_str(&format!(
        "      \"project_age_human\": \"{}\"\n",
        json_escape(&format_age(a.project_age))
    ));
    out.push_str("    }");
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
