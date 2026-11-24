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
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\\\\", "/");

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
    ws.write_file("keep-me/Cargo.toml", b"[package]\n");
    ws.write_file("keep-me/target/bin", &vec![0u8; 4096]);
    ws.write_file("skip-me/Cargo.toml", b"[package]\n");
    ws.write_file("skip-me/target/bin", &vec![0u8; 4096]);
    ws.write_file("web/package.json", b"{}\n");
    ws.write_file("web/node_modules/x.js", &vec![0u8; 4096]);

    let output = broom_bin()
        .args(["--json", "-t", "rust", "-e", "skip-me"])
        .arg(ws.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\\\\", "/");
    assert!(
        stdout.contains("\"count\": 1"),
        "unexpected output:\n{stdout}"
    );
    assert!(stdout.contains("keep-me/target"));
    assert!(!stdout.contains("skip-me/target"));
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

#[test]
fn detects_new_ecosystems_and_prevents_false_positives() {
    let ws = TempWorkspace::new("new_ecosystems");

    // PHP / Composer (requires vendor/autoload.php)
    ws.write_file("php-app/composer.json", b"{}\n");
    ws.write_file("php-app/vendor/autoload.php", b"<?php\n");
    ws.write_file("php-app/vendor/pkg/lib.php", &vec![1u8; 4096]);
    ws.write_file("php-app/.phpunit.cache/results", &vec![1u8; 1024]);

    // PHP false positives: vendor without autoload.php, or without composer.json
    ws.write_file("php-fake/composer.json", b"{}\n");
    ws.write_file("php-fake/vendor/custom.php", b"<?php // keep me\n");
    ws.write_file("go-vendored/go.mod", b"module example.com/m\n");
    ws.write_file("go-vendored/vendor/modules.txt", b"# committed vendor\n");

    // Ruby
    ws.write_file("rails-app/Gemfile", b"source 'https://rubygems.org'\n");
    ws.write_file("rails-app/.ruby-lsp/bundle", &vec![2u8; 2048]);
    ws.write_file("rails-app/.yardoc/objects", &vec![2u8; 1024]);

    // Godot 4 & 3
    ws.write_file("godot-game/project.godot", b"[application]\n");
    ws.write_file("godot-game/.godot/imported/tex.ctex", &vec![3u8; 4096]);
    ws.write_file("godot-game/.import/old.stex", &vec![3u8; 2048]);
    // Godot false positive: .godot without project.godot
    ws.write_file("not-godot/.godot/keep.txt", b"keep");

    // Unity (requires ProjectSettings/ProjectVersion.txt)
    ws.write_file(
        "unity-game/ProjectSettings/ProjectVersion.txt",
        b"m_EditorVersion: 6000.0.1f1\n",
    );
    ws.write_file("unity-game/Library/ArtifactDB", &vec![4u8; 8192]);
    ws.write_file("unity-game/Temp/worker.tmp", &vec![4u8; 1024]);
    ws.write_file("unity-game/Obj/Assembly.dll", &vec![4u8; 1024]);
    ws.write_file("unity-game/Logs/Editor.log", &vec![4u8; 1024]);
    // Unity false positive: Library/ without ProjectSettings/ProjectVersion.txt
    ws.write_file("not-unity/Library/Preferences/keep.plist", b"keep");

    // Unreal Engine
    ws.write_file("ue-game/MyGame.uproject", b"{}\n");
    ws.write_file("ue-game/Intermediate/Build/game.obj", &vec![5u8; 8192]);
    ws.write_file("ue-game/DerivedDataCache/Boot.ddc", &vec![5u8; 4096]);
    // Unreal false positive: Intermediate/ without .uproject
    ws.write_file("not-ue/Intermediate/notes.txt", b"keep");

    // Scala / sbt
    ws.write_file("scala-app/build.sbt", b"name := \"demo\"\n");
    ws.write_file("scala-app/target/scala-3/app.jar", &vec![6u8; 4096]);
    ws.write_file("scala-app/.bloop/demo.json", &vec![6u8; 1024]);
    ws.write_file("scala-app/.metals/metals.db", &vec![6u8; 1024]);

    // Clojure
    ws.write_file("clj-lein/project.clj", b"(defproject demo \"0.1\")\n");
    ws.write_file("clj-lein/target/demo.jar", &vec![7u8; 2048]);
    ws.write_file("clj-deps/deps.edn", b"{}\n");
    ws.write_file("clj-deps/.cpcache/1234.cp", &vec![7u8; 1024]);
    ws.write_file("cljs-app/shadow-cljs.edn", b"{}\n");
    ws.write_file("cljs-app/.shadow-cljs/builds/app", &vec![7u8; 1024]);

    // OCaml / Dune
    ws.write_file("ocaml-app/dune-project", b"(lang dune 3.16)\n");
    ws.write_file("ocaml-app/_build/default/main.exe", &vec![8u8; 4096]);
    ws.write_file("ocaml-app/_opam/.opam-switch/switch-state", b"state");
    ws.write_file("ocaml-app/_opam/bin/ocaml", &vec![8u8; 2048]);
    // OCaml false positive: _opam without .opam-switch
    ws.write_file("ocaml-fake/dune-project", b"(lang dune 3.16)\n");
    ws.write_file("ocaml-fake/_opam/keep.txt", b"keep");

    // Gleam
    ws.write_file("gleam-app/gleam.toml", b"name = \"demo\"\n");
    ws.write_file("gleam-app/build/dev/erlang/app.beam", &vec![9u8; 2048]);

    // Erlang / Rebar3
    ws.write_file("erl-app/rebar.config", b"{deps, []}.\n");
    ws.write_file("erl-app/_build/default/lib/app.beam", &vec![10u8; 2048]);

    // Meson (requires inner meson-private)
    ws.write_file("meson-app/meson.build", b"project('demo', 'c')\n");
    ws.write_file("meson-app/builddir/meson-private/coredata.dat", b"data");
    ws.write_file("meson-app/builddir/demo.o", &vec![11u8; 4096]);
    // Meson false positive: build/ without meson-private
    ws.write_file("meson-fake/meson.build", b"project('demo', 'c')\n");
    ws.write_file("meson-fake/build/handwritten.sh", b"keep");

    // CocoaPods (requires inner Manifest.lock)
    ws.write_file("ios-app/Podfile", b"platform :ios, '17.0'\n");
    ws.write_file("ios-app/Pods/Manifest.lock", b"PODS:\n");
    ws.write_file("ios-app/Pods/Alamofire/file.swift", &vec![12u8; 4096]);
    // CocoaPods false positive: Pods/ without Manifest.lock
    ws.write_file("ios-fake/Podfile", b"platform :ios, '17.0'\n");
    ws.write_file("ios-fake/Pods/custom.swift", b"keep");

    // Enriched existing ecosystems: OpenTofu (.tofu), Node (.vite, .expo), Python (.nox), CMake
    ws.write_file("tofu-infra/main.tofu", b"terraform {}\n");
    ws.write_file("tofu-infra/.terraform/providers/bin", &vec![13u8; 2048]);
    ws.write_file("expo-app/package.json", b"{}\n");
    ws.write_file("expo-app/.expo/web/cache", &vec![14u8; 2048]);
    ws.write_file("expo-app/.vite/deps/chunk.js", &vec![14u8; 2048]);
    ws.write_file("nox-py/noxfile.py", b"import nox\n");
    ws.write_file("nox-py/.nox/tests/bin", &vec![15u8; 2048]);
    ws.write_file("cmake-rwdi/CMakeLists.txt", b"project(x)\n");
    ws.write_file(
        "cmake-rwdi/cmake-build-relwithdebinfo/CMakeCache.txt",
        b"ok",
    );
    ws.write_file(
        "cmake-rwdi/cmake-build-relwithdebinfo/app.o",
        &vec![16u8; 2048],
    );

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
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\\\\", "/");

    let expected = [
        "php-app/vendor",
        "php-app/.phpunit.cache",
        "rails-app/.ruby-lsp",
        "rails-app/.yardoc",
        "godot-game/.godot",
        "godot-game/.import",
        "unity-game/Library",
        "unity-game/Temp",
        "unity-game/Obj",
        "unity-game/Logs",
        "ue-game/Intermediate",
        "ue-game/DerivedDataCache",
        "scala-app/target",
        "scala-app/.bloop",
        "scala-app/.metals",
        "clj-lein/target",
        "clj-deps/.cpcache",
        "cljs-app/.shadow-cljs",
        "ocaml-app/_build",
        "ocaml-app/_opam",
        "gleam-app/build",
        "erl-app/_build",
        "meson-app/builddir",
        "ios-app/Pods",
        "tofu-infra/.terraform",
        "expo-app/.expo",
        "expo-app/.vite",
        "nox-py/.nox",
        "cmake-rwdi/cmake-build-relwithdebinfo",
    ];
    for path in expected {
        assert!(
            stdout.contains(path),
            "expected artifact '{path}' not found:\n{stdout}"
        );
    }
    assert!(
        stdout.contains(&format!("\"count\": {}", expected.len())),
        "expected exactly {} artifacts:\n{stdout}",
        expected.len()
    );

    let forbidden = [
        "php-fake/vendor",
        "go-vendored/vendor",
        "not-godot/.godot",
        "not-unity/Library",
        "not-ue/Intermediate",
        "ocaml-fake/_opam",
        "meson-fake/build",
        "ios-fake/Pods",
    ];
    for path in forbidden {
        assert!(
            !stdout.contains(path),
            "false positive '{path}' should not be matched:\n{stdout}"
        );
    }
}

#[test]
fn files_mode_finds_largest_files_and_respects_filters() {
    let ws = TempWorkspace::new("large_files");
    ws.write_file("videos/movie.mp4", &vec![1u8; 65_536]);
    ws.write_file("isos/distro.iso", &vec![2u8; 131_072]);
    ws.write_file("notes/readme.txt", &vec![3u8; 4_096]);
    ws.write_file("skip-dir/giant.bin", &vec![4u8; 262_144]);
    ws.write_file(".hidden_giant.dat", &vec![5u8; 262_144]);
    ws.write_file(".hidden_dir/secret.dat", &vec![6u8; 262_144]);
    ws.write_file("noext_blob", &vec![7u8; 32_768]);

    // Default --files skips hidden files/dirs, respects -e and --limit, sorts largest first
    let output = broom_bin()
        .args(["-f", "--json", "-e", "skip-dir", "--limit", "2"])
        .arg(ws.path())
        .output()
        .expect("failed to execute broom -f");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\\\\", "/");

    assert!(stdout.contains("\"mode\": \"files\""), "{stdout}");
    assert!(stdout.contains("\"count\": 2"), "{stdout}");
    assert!(stdout.contains("\"total_matched\": 4"), "{stdout}");

    let iso_pos = stdout
        .find("isos/distro.iso")
        .expect("missing distro.iso in output");
    let mp4_pos = stdout
        .find("videos/movie.mp4")
        .expect("missing movie.mp4 in output");
    assert!(
        iso_pos < mp4_pos,
        "expected distro.iso before movie.mp4:\n{stdout}"
    );

    assert!(!stdout.contains("skip-dir/giant.bin"), "{stdout}");
    assert!(!stdout.contains(".hidden_giant.dat"), "{stdout}");
    assert!(!stdout.contains(".hidden_dir/secret.dat"), "{stdout}");
    assert!(!stdout.contains("notes/readme.txt"), "{stdout}");

    // With --hidden and --min-size, hidden large files are included and small ones excluded
    let hidden_out = broom_bin()
        .args(["--files", "--hidden", "--min-size", "200K", "--json"])
        .arg(ws.path())
        .output()
        .expect("failed to execute broom --files --hidden");

    assert!(hidden_out.status.success());
    let hidden_stdout = String::from_utf8(hidden_out.stdout)
        .unwrap()
        .replace("\\\\", "/");
    assert!(
        hidden_stdout.contains("skip-dir/giant.bin"),
        "{hidden_stdout}"
    );
    assert!(
        hidden_stdout.contains(".hidden_giant.dat"),
        "{hidden_stdout}"
    );
    assert!(
        hidden_stdout.contains(".hidden_dir/secret.dat"),
        "{hidden_stdout}"
    );
    assert!(
        !hidden_stdout.contains("isos/distro.iso"),
        "{hidden_stdout}"
    );
    assert!(hidden_stdout.contains("\"count\": 3"), "{hidden_stdout}");

    // Table output renders cleanly without color
    let table_out = broom_bin()
        .args(["-f", "-n", "--no-color", "-e", "skip-dir"])
        .arg(ws.path())
        .output()
        .unwrap();
    assert!(table_out.status.success());
    let table_stdout = String::from_utf8(table_out.stdout).unwrap();
    assert!(table_stdout.contains("EXT"), "{table_stdout}");
    assert!(table_stdout.contains(".iso"), "{table_stdout}");
    assert!(table_stdout.contains("noext_blob"), "{table_stdout}");
}

#[test]
fn files_mode_dry_run_preserves_and_yes_deletes_only_matched_files() {
    let ws = TempWorkspace::new("files_clean");
    ws.write_file("big.bin", &vec![9u8; 65_536]);
    ws.write_file("small.txt", b"keep me");

    // Dry run must preserve everything
    let dry = broom_bin()
        .args(["-f", "-n", "--min-size", "32KiB", "--no-color"])
        .arg(ws.path())
        .output()
        .unwrap();
    assert!(dry.status.success());
    assert!(ws.path().join("big.bin").exists());
    assert!(ws.path().join("small.txt").exists());

    // Sweep with -y deletes only matched large file
    let clean = broom_bin()
        .args(["-f", "-y", "--min-size", "32KiB", "--no-color"])
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
    assert!(stdout.contains("(1 file)"), "unexpected output:\n{stdout}");

    assert!(!ws.path().join("big.bin").exists());
    assert!(ws.path().join("small.txt").exists());
}

#[cfg(unix)]
#[test]
fn files_mode_ignores_symlinks() {
    use std::os::unix::fs::symlink;

    let ws = TempWorkspace::new("files_symlink");
    ws.write_file("real/precious.bin", &vec![1u8; 65_536]);
    symlink(
        ws.path().join("real/precious.bin"),
        ws.path().join("link.bin"),
    )
    .unwrap();

    let output = broom_bin()
        .args(["-f", "--json", "-e", "real"])
        .arg(ws.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"count\": 0"), "{stdout}");
    assert!(!stdout.contains("link.bin"), "{stdout}");
    assert!(ws.path().join("real/precious.bin").exists());
}

#[test]
fn files_mode_rejects_ecosystem_type_flag() {
    let output = broom_bin().args(["-f", "-t", "rust"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("cannot combine --files and --type"),
        "unexpected stderr: {stderr}"
    );
}
