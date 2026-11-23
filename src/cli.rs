use std::path::PathBuf;
use std::time::Duration;

use crate::ecosystems::Ecosystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Size,
    Age,
    Path,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub paths: Vec<PathBuf>,
    pub dry_run: bool,
    pub yes: bool,
    pub interactive: bool,
    pub json: bool,
    pub no_color: bool,
    pub include_hidden: bool,
    pub min_size: u64,
    pub older_than: Option<Duration>,
    pub max_depth: Option<usize>,
    pub ecosystems: Vec<Ecosystem>,
    pub excludes: Vec<String>,
    pub sort_by: SortBy,
    pub reverse: bool,
    pub limit: Option<usize>,
    pub threads: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            dry_run: false,
            yes: false,
            interactive: false,
            json: false,
            no_color: false,
            include_hidden: false,
            min_size: 0,
            older_than: None,
            max_depth: None,
            ecosystems: Vec::new(),
            excludes: Vec::new(),
            sort_by: SortBy::Size,
            reverse: false,
            limit: None,
            threads: None,
        }
    }
}

pub enum ParseOutcome {
    Run(Config),
    PrintHelp,
    PrintVersion,
}

pub fn parse_args<I>(args: I) -> Result<ParseOutcome, String>
where
    I: IntoIterator<Item = String>,
{
    let mut cfg = Config::default();
    let mut iter = args.into_iter().skip(1).peekable();
    let mut positional_only = false;

    while let Some(arg) = iter.next() {
        if positional_only {
            cfg.paths.push(PathBuf::from(arg));
            continue;
        }

        if arg == "--" {
            positional_only = true;
            continue;
        }

        if let Some((flag, inline_val)) = split_long_flag(&arg) {
            match flag {
                "--help" => return Ok(ParseOutcome::PrintHelp),
                "--version" => return Ok(ParseOutcome::PrintVersion),
                "--dry-run" => cfg.dry_run = true,
                "--yes" => cfg.yes = true,
                "--interactive" => cfg.interactive = true,
                "--json" => cfg.json = true,
                "--no-color" => cfg.no_color = true,
                "--hidden" => cfg.include_hidden = true,
                "--reverse" => cfg.reverse = true,
                "--min-size" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    cfg.min_size = parse_size(&val)?;
                }
                "--older-than" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    cfg.older_than = Some(parse_duration(&val)?);
                }
                "--max-depth" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    cfg.max_depth = Some(
                        val.parse::<usize>()
                            .map_err(|_| format!("invalid --max-depth value: '{val}'"))?,
                    );
                }
                "--type" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    parse_ecosystems(&val, &mut cfg.ecosystems)?;
                }
                "--exclude" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    cfg.excludes.push(val);
                }
                "--sort" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    cfg.sort_by = parse_sort(&val)?;
                }
                "--limit" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    cfg.limit = Some(
                        val.parse::<usize>()
                            .map_err(|_| format!("invalid --limit value: '{val}'"))?,
                    );
                }
                "--threads" => {
                    let val = take_value(flag, inline_val, &mut iter)?;
                    let n = val
                        .parse::<usize>()
                        .map_err(|_| format!("invalid --threads value: '{val}'"))?;
                    if n == 0 {
                        return Err("--threads must be at least 1".to_string());
                    }
                    cfg.threads = Some(n);
                }
                _ => return Err(format!("unknown option: '{flag}' (try --help)")),
            }
            continue;
        }

        if arg.starts_with('-') && arg.len() > 1 {
            match arg.as_str() {
                "-h" => return Ok(ParseOutcome::PrintHelp),
                "-V" => return Ok(ParseOutcome::PrintVersion),
                "-n" => cfg.dry_run = true,
                "-y" => cfg.yes = true,
                "-i" => cfg.interactive = true,
                "-r" => cfg.reverse = true,
                "-s" => {
                    let val = take_value("-s", None, &mut iter)?;
                    cfg.min_size = parse_size(&val)?;
                }
                "-a" => {
                    let val = take_value("-a", None, &mut iter)?;
                    cfg.older_than = Some(parse_duration(&val)?);
                }
                "-d" => {
                    let val = take_value("-d", None, &mut iter)?;
                    cfg.max_depth = Some(
                        val.parse::<usize>()
                            .map_err(|_| format!("invalid -d value: '{val}'"))?,
                    );
                }
                "-t" => {
                    let val = take_value("-t", None, &mut iter)?;
                    parse_ecosystems(&val, &mut cfg.ecosystems)?;
                }
                "-e" => {
                    let val = take_value("-e", None, &mut iter)?;
                    cfg.excludes.push(val);
                }
                "-j" => {
                    let val = take_value("-j", None, &mut iter)?;
                    let n = val
                        .parse::<usize>()
                        .map_err(|_| format!("invalid -j value: '{val}'"))?;
                    if n == 0 {
                        return Err("-j must be at least 1".to_string());
                    }
                    cfg.threads = Some(n);
                }
                _ => return Err(format!("unknown option: '{arg}' (try --help)")),
            }
            continue;
        }

        cfg.paths.push(PathBuf::from(arg));
    }

    if cfg.paths.is_empty() {
        cfg.paths.push(PathBuf::from("."));
    }

    if cfg.dry_run && cfg.yes {
        return Err("cannot combine --dry-run and --yes".to_string());
    }

    Ok(ParseOutcome::Run(cfg))
}

fn split_long_flag(arg: &str) -> Option<(&str, Option<&str>)> {
    if !arg.starts_with("--") {
        return None;
    }
    if let Some((k, v)) = arg.split_once('=') {
        Some((k, Some(v)))
    } else {
        Some((arg, None))
    }
}

fn take_value<I>(flag: &str, inline: Option<&str>, iter: &mut I) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    if let Some(v) = inline {
        return Ok(v.to_string());
    }
    iter.next()
        .ok_or_else(|| format!("option '{flag}' requires a value"))
}

fn parse_ecosystems(raw: &str, out: &mut Vec<Ecosystem>) -> Result<(), String> {
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        match Ecosystem::from_str_loose(trimmed) {
            Some(eco) => {
                if !out.contains(&eco) {
                    out.push(eco);
                }
            }
            None => {
                let valid: Vec<&str> = Ecosystem::ALL.iter().map(|e| e.id()).collect();
                return Err(format!(
                    "unknown ecosystem '{trimmed}'. Valid values: {}",
                    valid.join(", ")
                ));
            }
        }
    }
    Ok(())
}

fn parse_sort(raw: &str) -> Result<SortBy, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "size" | "s" => Ok(SortBy::Size),
        "age" | "a" | "time" => Ok(SortBy::Age),
        "path" | "p" | "name" => Ok(SortBy::Path),
        _ => Err(format!(
            "invalid --sort value '{raw}'. Expected: size, age, or path"
        )),
    }
}

/// Parses human-friendly byte sizes like `500`, `10K`, `50MB`, `1.5GiB`, `2G`.
pub fn parse_size(raw: &str) -> Result<u64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("size cannot be empty".to_string());
    }
    let idx = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let (num_str, unit_str) = s.split_at(idx);
    let value: f64 = num_str
        .parse()
        .map_err(|_| format!("invalid size '{raw}'"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("invalid size '{raw}'"));
    }

    let multiplier: f64 = match unit_str.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" => 1_000.0,
        "ki" | "kib" => 1024.0,
        "m" | "mb" => 1_000_000.0,
        "mi" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" => 1_000_000_000.0,
        "gi" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1_000_000_000_000.0,
        "ti" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("unknown size unit in '{raw}'")),
    };

    Ok((value * multiplier) as u64)
}

/// Parses human-friendly durations like `12h`, `7d`, `2w`, `3mo` / `3m`, `1y`.
pub fn parse_duration(raw: &str) -> Result<Duration, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("duration cannot be empty".to_string());
    }
    let idx = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num_str, unit_str) = s.split_at(idx);
    let value: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration '{raw}' (examples: 7d, 2w, 3mo, 1y)"))?;

    let secs_per_unit: u64 = match unit_str.trim().to_ascii_lowercase().as_str() {
        "" | "d" | "day" | "days" => 86_400,
        "h" | "hr" | "hour" | "hours" => 3_600,
        "w" | "wk" | "week" | "weeks" => 7 * 86_400,
        "m" | "mo" | "month" | "months" => 30 * 86_400,
        "y" | "yr" | "year" | "years" => 365 * 86_400,
        _ => {
            return Err(format!(
                "unknown duration unit in '{raw}' (use h, d, w, mo, or y)"
            ));
        }
    };

    Ok(Duration::from_secs(value.saturating_mul(secs_per_unit)))
}

pub fn help_text() -> String {
    let ecosystems: Vec<&str> = Ecosystem::ALL.iter().map(|e| e.id()).collect();
    format!(
        "\
broom {version} — fast, zero-dependency dev build-artifact cleaner

USAGE:
    broom [OPTIONS] [PATH...]

ARGS:
    [PATH...]                 Directories to scan (defaults to current directory)

OPTIONS:
    -n, --dry-run             Scan and print report only; do not prompt or delete
    -y, --yes                 Delete all matched artifacts without prompting
    -i, --interactive         Prompt individually for each artifact
    -a, --older-than <AGE>    Only match projects untouched for at least AGE (e.g. 7d, 2w, 3mo, 1y)
    -s, --min-size <SIZE>     Only match artifacts at least SIZE (e.g. 50M, 1GiB)
    -t, --type <LIST>         Filter by ecosystem (comma-separated, repeatable)
    -e, --exclude <PATTERN>   Exclude paths containing PATTERN (repeatable)
    -d, --max-depth <N>       Maximum directory depth to scan
        --sort <FIELD>        Sort by: size (default), age, or path
    -r, --reverse             Reverse sort order
        --limit <N>           Show/operate on at most N results
        --hidden              Descend into hidden directories (other than artifact dirs)
        --json                Output results as JSON (implies --dry-run)
        --no-color            Disable colored output
    -j, --threads <N>         Number of worker threads (default: CPU count)
    -h, --help                Print help
    -V, --version             Print version

SUPPORTED ECOSYSTEMS:
    {ecosystems}

EXAMPLES:
    broom ~/Projects                      Scan and interactively clean
    broom -n ~/Projects                   Preview reclaimable space without deleting
    broom -a 30d -s 100M ~/Projects       Find artifacts >100MB untouched for 30+ days
    broom -t rust,node -y ~/Projects      Clean all Rust & Node artifacts immediately
    broom --json ~/Projects               Machine-readable report for scripts
",
        version = env!("CARGO_PKG_VERSION"),
        ecosystems = ecosystems.join(", "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("512").unwrap(), 512);
        assert_eq!(parse_size("10K").unwrap(), 10_000);
        assert_eq!(parse_size("1KiB").unwrap(), 1024);
        assert_eq!(parse_size("50M").unwrap(), 50_000_000);
        assert_eq!(parse_size("1.5GiB").unwrap(), 1_610_612_736);
        assert!(parse_size("abc").is_err());
    }

    #[test]
    fn parses_durations() {
        assert_eq!(
            parse_duration("7d").unwrap(),
            Duration::from_secs(7 * 86_400)
        );
        assert_eq!(
            parse_duration("2w").unwrap(),
            Duration::from_secs(14 * 86_400)
        );
        assert_eq!(
            parse_duration("3mo").unwrap(),
            Duration::from_secs(90 * 86_400)
        );
        assert_eq!(
            parse_duration("12h").unwrap(),
            Duration::from_secs(12 * 3_600)
        );
        assert_eq!(
            parse_duration("30").unwrap(),
            Duration::from_secs(30 * 86_400)
        );
        assert!(parse_duration("5x").is_err());
    }

    #[test]
    fn parses_cli_flags() {
        let args = [
            "broom",
            "-n",
            "-a",
            "14d",
            "--min-size=50M",
            "-t",
            "rust,node",
            "--sort",
            "age",
            "/tmp/code",
        ]
        .into_iter()
        .map(String::from);

        let ParseOutcome::Run(cfg) = parse_args(args).unwrap() else {
            panic!("expected Run");
        };
        assert!(cfg.dry_run);
        assert_eq!(cfg.older_than, Some(Duration::from_secs(14 * 86_400)));
        assert_eq!(cfg.min_size, 50_000_000);
        assert_eq!(cfg.ecosystems, vec![Ecosystem::Rust, Ecosystem::Node]);
        assert_eq!(cfg.sort_by, SortBy::Age);
        assert_eq!(cfg.paths, vec![PathBuf::from("/tmp/code")]);
    }
}
