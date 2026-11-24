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
fn detects_bazel_artifacts_and_avoids_false_positives() {
    let ws = TempWorkspace::new("bazel");

    ws.write_file("bazel-ws/WORKSPACE", b"workspace(name = \"demo\")\n");
    ws.write_file("bazel-ws/bazel-bin/app.bin", &vec![1u8; 4096]);
    ws.write_file("bazel-ws/bazel-out/app.out", &vec![1u8; 4096]);

    ws.write_file("bazel-module/MODULE.bazel", b"module(name = \"demo\")\n");
    ws.write_file("bazel-module/bazel-out/gen.out", &vec![2u8; 2048]);

    ws.write_file(
        "bazel-ws-bzl/WORKSPACE.bazel",
        b"workspace(name = \"other\")\n",
    );
    ws.write_file("bazel-ws-bzl/bazel-bin/other.bin", &vec![2u8; 2048]);

    // False positives: Bazel output names without any workspace marker.
    ws.write_file("plain/bazel-bin/keep.bin", b"keep me");
    ws.write_file("plain/bazel-out/keep.out", b"keep me");

    let output = broom_bin()
        .args(["--json", "-t", "bazel"])
        .arg(ws.path())
        .output()
        .expect("failed to execute broom");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\\\\", "/");

    let expected = [
        "bazel-ws/bazel-bin",
        "bazel-ws/bazel-out",
        "bazel-module/bazel-out",
        "bazel-ws-bzl/bazel-bin",
    ];
    for path in expected {
        assert!(
            stdout.contains(path),
            "expected artifact '{path}' not found:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("\"count\": 4"),
        "expected exactly 4 artifacts:\n{stdout}"
    );
    assert!(
        !stdout.contains("plain/bazel-bin"),
        "false positive bazel-bin:\n{stdout}"
    );
    assert!(
        !stdout.contains("plain/bazel-out"),
        "false positive bazel-out:\n{stdout}"
    );
}
