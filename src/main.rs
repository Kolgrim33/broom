mod cleaner;
mod cli;
mod ecosystems;
mod format;
mod scanner;

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::cleaner::{CleanSummary, clean_artifacts, clean_files, parse_selection};
use crate::cli::{Config, ParseOutcome, help_text, parse_args};
use crate::format::{
    BOLD, CYAN, DIM, GREEN, RED, RESET, YELLOW, display_path, ext_ansi_color, ext_display,
    format_age, format_bytes, format_duration_short, print_files_table, print_table,
    render_files_json, render_json, should_use_color,
};
use crate::scanner::{Artifact, LargeFile};

trait SweepItem {
    fn path(&self) -> &Path;
    fn size_bytes(&self) -> u64;
    fn age(&self) -> Duration;
    fn label(&self) -> String;
    fn label_color(&self) -> &'static str;
}

impl SweepItem for Artifact {
    fn path(&self) -> &Path {
        &self.path
    }
    fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
    fn age(&self) -> Duration {
        self.project_age
    }
    fn label(&self) -> String {
        self.ecosystem.name().to_string()
    }
    fn label_color(&self) -> &'static str {
        self.ecosystem.ansi_color()
    }
}

impl SweepItem for LargeFile {
    fn path(&self) -> &Path {
        &self.path
    }
    fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
    fn age(&self) -> Duration {
        self.age
    }
    fn label(&self) -> String {
        ext_display(&self.extension)
    }
    fn label_color(&self) -> &'static str {
        ext_ansi_color(&self.extension)
    }
}

fn main() -> ExitCode {
    let outcome = match parse_args(std::env::args()) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("broom: error: {msg}");
            return ExitCode::from(2);
        }
    };

    let cfg = match outcome {
        ParseOutcome::PrintHelp => {
            print!("{}", help_text());
            return ExitCode::SUCCESS;
        }
        ParseOutcome::PrintVersion => {
            println!("broom {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        ParseOutcome::PrintCompletions(script) => {
            print!("{script}");
            return ExitCode::SUCCESS;
        }
        ParseOutcome::Run(cfg) => cfg,
    };

    // Validate paths exist before scanning
    for path in &cfg.paths {
        if !path.exists() {
            eprintln!("broom: path does not exist: {}", path.display());
            return ExitCode::from(2);
        }
    }

    if cfg.files {
        run_files_mode(&cfg)
    } else {
        run_artifacts_mode(&cfg)
    }
}

fn run_artifacts_mode(cfg: &Config) -> ExitCode {
    let report = scanner::scan(cfg);

    if cfg.json {
        print!("{}", render_json(&report));
        return ExitCode::SUCCESS;
    }

    let color = should_use_color(cfg.no_color);

    if report.artifacts.is_empty() {
        let elapsed = format_duration_short(report.elapsed);
        if color {
            println!(
                "{GREEN}Nothing to sweep!{RESET} {DIM}(scanned {} dirs in {elapsed}){RESET}",
                report.dirs_scanned
            );
        } else {
            println!(
                "Nothing to sweep! (scanned {} dirs in {elapsed})",
                report.dirs_scanned
            );
        }
        return ExitCode::SUCCESS;
    }

    print_table(&report, color);

    let is_tty = io::stdin().is_terminal() && io::stdout().is_terminal();
    if cfg.dry_run || (!cfg.yes && !is_tty) {
        return ExitCode::SUCCESS;
    }

    let targets = select_targets(&report.artifacts, cfg, color);
    if targets.is_empty() {
        print_aborted(color);
        return ExitCode::SUCCESS;
    }

    let summary = clean_artifacts(&targets);
    finish_clean_summary(&summary, "artifact", "artifacts", color)
}

fn run_files_mode(cfg: &Config) -> ExitCode {
    let report = scanner::scan_files(cfg);

    if cfg.json {
        print!("{}", render_files_json(&report));
        return ExitCode::SUCCESS;
    }

    let color = should_use_color(cfg.no_color);

    if report.files.is_empty() {
        let elapsed = format_duration_short(report.elapsed);
        if color {
            println!(
                "{GREEN}No matching files found!{RESET} {DIM}(scanned {} files in {} dirs in {elapsed}){RESET}",
                report.files_scanned, report.dirs_scanned
            );
        } else {
            println!(
                "No matching files found! (scanned {} files in {} dirs in {elapsed})",
                report.files_scanned, report.dirs_scanned
            );
        }
        return ExitCode::SUCCESS;
    }

    print_files_table(&report, color);

    let is_tty = io::stdin().is_terminal() && io::stdout().is_terminal();
    if cfg.dry_run || (!cfg.yes && !is_tty) {
        return ExitCode::SUCCESS;
    }

    let targets = select_targets(&report.files, cfg, color);
    if targets.is_empty() {
        print_aborted(color);
        return ExitCode::SUCCESS;
    }

    let summary = clean_files(&targets);
    finish_clean_summary(&summary, "file", "files", color)
}

fn select_targets<'a, T: SweepItem>(items: &'a [T], cfg: &Config, color: bool) -> Vec<&'a T> {
    if cfg.yes {
        items.iter().collect()
    } else if cfg.interactive {
        prompt_interactive(items, color)
    } else {
        prompt_batch(items, color)
    }
}

fn print_aborted(color: bool) {
    if color {
        println!("  {DIM}Aborted — nothing deleted.{RESET}");
    } else {
        println!("  Aborted — nothing deleted.");
    }
}

fn finish_clean_summary(
    summary: &CleanSummary,
    singular: &str,
    plural: &str,
    color: bool,
) -> ExitCode {
    let elapsed = format_duration_short(summary.elapsed);
    let freed = format_bytes(summary.freed_bytes);
    let noun = if summary.deleted_count == 1 {
        singular
    } else {
        plural
    };

    println!();
    if color {
        println!(
            "  {GREEN}{BOLD}Swept {freed}{RESET} ({} {noun}) in {DIM}{elapsed}{RESET}.",
            summary.deleted_count
        );
    } else {
        println!(
            "  Swept {freed} ({} {noun}) in {elapsed}.",
            summary.deleted_count
        );
    }

    if !summary.failures.is_empty() {
        for f in &summary.failures {
            if color {
                eprintln!("  {RED}Failed{RESET} {}: {}", f.path, f.error);
            } else {
                eprintln!("  Failed {}: {}", f.path, f.error);
            }
        }
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn prompt_batch<T: SweepItem>(items: &[T], color: bool) -> Vec<&T> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        if color {
            print!(
                "\n  {BOLD}Sweep?{RESET} [{GREEN}y{RESET}]es all / [{CYAN}i{RESET}]nteractive / [{YELLOW}1,3-5{RESET}] select / [{DIM}N{RESET}]o: "
            );
        } else {
            print!("\n  Sweep? [y]es all / [i]nteractive / [1,3-5] select / [N]o: ");
        }
        let _ = stdout.flush();

        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            return Vec::new();
        }
        let input = line.trim();
        if input.is_empty() || input.eq_ignore_ascii_case("n") || input.eq_ignore_ascii_case("no") {
            return Vec::new();
        }
        if input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes") {
            return items.iter().collect();
        }
        if input.eq_ignore_ascii_case("i") || input.eq_ignore_ascii_case("interactive") {
            return prompt_interactive(items, color);
        }

        match parse_selection(input, items.len()) {
            Ok(indices) if !indices.is_empty() => {
                let chosen: Vec<&T> = indices.into_iter().map(|i| &items[i]).collect();
                let bytes: u64 = chosen.iter().map(|a| a.size_bytes()).sum();
                if color {
                    print!(
                        "  Delete {} selected ({YELLOW}{}{RESET})? [y/N]: ",
                        chosen.len(),
                        format_bytes(bytes)
                    );
                } else {
                    print!(
                        "  Delete {} selected ({})? [y/N]: ",
                        chosen.len(),
                        format_bytes(bytes)
                    );
                }
                let _ = stdout.flush();
                let mut confirm = String::new();
                if stdin.read_line(&mut confirm).unwrap_or(0) == 0 {
                    return Vec::new();
                }
                if confirm.trim().eq_ignore_ascii_case("y")
                    || confirm.trim().eq_ignore_ascii_case("yes")
                {
                    return chosen;
                }
                return Vec::new();
            }
            Ok(_) => return Vec::new(),
            Err(err) => {
                if color {
                    println!("  {RED}{err}{RESET}");
                } else {
                    println!("  {err}");
                }
            }
        }
    }
}

fn prompt_interactive<T: SweepItem>(items: &[T], color: bool) -> Vec<&T> {
    let cwd = std::env::current_dir().ok();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut chosen = Vec::new();

    println!();
    for (i, item) in items.iter().enumerate() {
        let path_str = display_path(item.path(), cwd.as_deref());
        let size_str = format_bytes(item.size_bytes());
        let age_str = format_age(item.age());
        let label = item.label();

        loop {
            if color {
                print!(
                    "  [{}/{}] {}{label}{RESET} {YELLOW}{BOLD}{size_str}{RESET} {DIM}({age_str}){RESET} {path_str} — delete? [y/N/a/q]: ",
                    i + 1,
                    items.len(),
                    item.label_color()
                );
            } else {
                print!(
                    "  [{}/{}] {label} {size_str} ({age_str}) {path_str} — delete? [y/N/a/q]: ",
                    i + 1,
                    items.len()
                );
            }
            let _ = stdout.flush();

            let mut line = String::new();
            if stdin.read_line(&mut line).unwrap_or(0) == 0 {
                return chosen;
            }
            match line.trim().to_ascii_lowercase().as_str() {
                "" | "n" | "no" => break,
                "y" | "yes" => {
                    chosen.push(item);
                    break;
                }
                "a" | "all" => {
                    chosen.extend(&items[i..]);
                    return chosen;
                }
                "q" | "quit" => return chosen,
                _ => println!("  Enter y (yes), n (no), a (all remaining), or q (quit)."),
            }
        }
    }

    chosen
}
