use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempWorkspace {
    root: PathBuf,
}

impl TempWorkspace {
    fn new(label: &str) -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "broom_test_{}_{}_{}",
            label,
            std::process::id(),
            id
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write_file(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let p = self.root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, bytes).unwrap();
        p
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn broom_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_broom"))
}

#[test]
fn detects_supported_ecosystems_and_avoids_false_positives() {
    let ws = TempWorkspace::new("detect");

    // Valid Rust project
    ws.write_file("rust-app/Cargo.toml", b"[package]\nname = \"a\"\n");
    ws.write_file("rust-app/src/main.rs", b"fn main() {}\n");
    ws.write_file("rust-app/target/debug/app", &vec![0u8; 8192]);

    // Valid Node project with node_modules + .next
    ws.write_file("web-app/package.json", b"{}\n");
    ws.write_file("web-app/node_modules/pkg/index.js", &vec![1u8; 4096]);
    ws.write_file("web-app/.next/cache/chunk.js", &vec![2u8; 2048]);

    // Valid Python project with .venv (has pyvenv.cfg)
    ws.write_file("py-app/pyproject.toml", b"[project]\nname = \"py\"\n");
    ws.write_file("py-app/.venv/pyvenv.cfg", b"home = /usr/bin\n");
    ws.write_file("py-app/.venv/lib/mod.py", &vec![3u8; 4096]);

    // Valid CMake project (build/ has CMakeCache.txt)
    ws.write_file("cpp-app/CMakeLists.txt", b"project(demo)\n");
    ws.write_file(
        "cpp-app/build/CMakeCache.txt",
        b"CMAKE_BUILD_TYPE:STRING=Debug\n",
    );
    ws.write_file("cpp-app/build/demo.o", &vec![4u8; 4096]);

    // Valid Terraform dir
    ws.write_file("infra/main.tf", b"terraform {}\n");
    ws.write_file("infra/.terraform/providers/bin", &vec![5u8; 4096]);

    // FALSE POSITIVE 1: `target/` with NO Cargo.toml or pom.xml
    ws.write_file("random-notes/README.md", b"hello\n");
    ws.write_file("random-notes/target/important.txt", b"do not delete");

    // FALSE POSITIVE 2: CMake `build/` WITHOUT CMakeCache.txt
    ws.write_file("cpp-handwritten/CMakeLists.txt", b"project(x)\n");
    ws.write_file("cpp-handwritten/build/handwritten.sh", b"echo keep me");

    // FALSE POSITIVE 3: `.venv` WITHOUT pyvenv.cfg
    ws.write_file("py-fake/pyproject.toml", b"[project]\n");
    ws.write_file("py-fake/.venv/not-a-venv.txt", b"keep me");

    let output = broom_bin()
        .arg("--json")
        .arg(ws.path())
        .output()
        .expect("failed to execute broom");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("rust-app/target"),
        "missing rust target:\n{stdout}"
    );
    assert!(
        stdout.contains("web-app/node_modules"),
        "missing node_modules:\n{stdout}"
    );
    assert!(stdout.contains("web-app/.next"), "missing .next:\n{stdout}");
    assert!(stdout.contains("py-app/.venv"), "missing .venv:\n{stdout}");
    assert!(
        stdout.contains("cpp-app/build"),
        "missing cmake build:\n{stdout}"
    );
    assert!(
        stdout.contains("infra/.terraform"),
        "missing .terraform:\n{stdout}"
    );
    assert!(
        stdout.contains("\"count\": 6"),
        "expected exactly 6 artifacts:\n{stdout}"
    );

    assert!(
        !stdout.contains("random-notes/target"),
        "false positive target:\n{stdout}"
    );
    assert!(
        !stdout.contains("cpp-handwritten/build"),
        "false positive cmake build:\n{stdout}"
    );
    assert!(
        !stdout.contains("py-fake/.venv"),
        "false positive venv:\n{stdout}"
    );
}

#[test]
fn filters_by_ecosystem_and_exclude() {
    let ws = TempWorkspace::new("filters");
    ws.write_file("a/Cargo.toml", b"[package]\n");
    ws.write_file("a/target/bin", &vec![0u8; 4096]);
    ws.write_file("b/Cargo.toml", b"[package]\n");
    ws.write_file("b/target/bin", &vec![0u8; 4096]);
    ws.write_file("c/package.json", b"{}\n");
    ws.write_file("c/node_modules/x.js", &vec![0u8; 4096]);

    let output = broom_bin()
        .args(["--json", "-t", "rust", "-e", "/b/"])
        .arg(ws.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("\"count\": 1"),
        "unexpected output:\n{stdout}"
    );
    assert!(stdout.contains("a/target"));
    assert!(!stdout.contains("b/target"));
    assert!(!stdout.contains("node_modules"));
}

#[test]
fn dry_run_preserves_and_yes_deletes_only_artifacts() {
    let ws = TempWorkspace::new("clean");
    ws.write_file("mycrate/Cargo.toml", b"[package]\nname = \"demo\"\n");
    ws.write_file("mycrate/src/main.rs", b"fn main() {}\n");
    ws.write_file("mycrate/target/debug/demo", &vec![7u8; 16384]);

    // Dry run must not delete anything
    let dry = broom_bin()
        .args(["--dry-run", "--no-color"])
        .arg(ws.path())
        .output()
        .unwrap();
    assert!(dry.status.success());
    assert!(ws.path().join("mycrate/target/debug/demo").exists());

    // Actual sweep with -y deletes target/ while keeping source intact
    let clean = broom_bin()
        .args(["-y", "--no-color"])
        .arg(ws.path())
        .output()
        .unwrap();
    assert!(
        clean.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let stdout = String::from_utf8(clean.stdout).unwrap();
    assert!(stdout.contains("Swept "), "unexpected output:\n{stdout}");

    assert!(!ws.path().join("mycrate/target").exists());
    assert!(ws.path().join("mycrate/Cargo.toml").exists());
    assert!(ws.path().join("mycrate/src/main.rs").exists());
}

#[cfg(unix)]
#[test]
fn ignores_symlinked_artifact_directories() {
    use std::os::unix::fs::symlink;

    let ws = TempWorkspace::new("symlink");
    ws.write_file("precious/data.txt", b"never delete this");
    ws.write_file("fake-rust/Cargo.toml", b"[package]\n");
    symlink(
        ws.path().join("precious"),
        ws.path().join("fake-rust/target"),
    )
    .unwrap();

    let output = broom_bin()
        .args(["-y", "--no-color"])
        .arg(ws.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(ws.path().join("precious/data.txt").exists());
    assert!(
        ws.path()
            .join("fake-rust/target")
            .symlink_metadata()
            .is_ok()
    );
}
