use std::process::Command;

fn broom_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_broom"))
}

#[test]
fn completion_scripts_cover_flags_and_values() {
    for shell in ["bash", "zsh", "fish"] {
        let output = broom_bin()
            .arg("--completions")
            .arg(shell)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{shell} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let script = String::from_utf8(output.stdout).unwrap();
        assert!(
            !script.trim().is_empty(),
            "{shell} produced no completion script"
        );
        for flag in [
            "dry-run",
            "yes",
            "interactive",
            "sort",
            "type",
            "threads",
            "completions",
        ] {
            assert!(
                script.contains(flag),
                "{shell} completion is missing {flag}"
            );
        }
        for value in ["size", "age", "path"] {
            assert!(
                script.contains(value),
                "{shell} completion is missing sort value {value}"
            );
        }
        for eco in ["rust", "node", "python", "terraform"] {
            assert!(
                script.contains(eco),
                "{shell} completion is missing ecosystem {eco}"
            );
        }
    }
}

#[test]
fn completions_accept_inline_value_and_reject_unknown_shell() {
    let ok = broom_bin().arg("--completions=zsh").output().unwrap();
    assert!(ok.status.success());
    assert!(!ok.stdout.is_empty());

    let bad = broom_bin()
        .args(["--completions", "tcsh"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let err = String::from_utf8(bad.stderr).unwrap();
    assert!(err.contains("unknown shell 'tcsh'"));
    assert!(err.contains("bash, zsh, fish"));
}
