# broom

**Blazing-fast, zero-dependency CLI to find and sweep stale build artifacts and dev caches.**

Reclaim tens of gigabytes of `target/`, `node_modules/`, `.venv/`, `build/`, `.next/`, and more across all your repositories in milliseconds.

```text
  #  TYPE        SIZE        SHARE       AGE       PATH
  1  Rust        2.84 GiB    ██████████  4mo ago   ~/Projects/paru/target
  2  Gradle      1.91 GiB    ███████░░░  2mo ago   ~/Projects/besu/build
  3  Node        642.0 MiB   ██░░░░░░░░  14d ago   ~/Projects/web-app/node_modules
  4  Python      184.3 MiB   █░░░░░░░░░  6mo ago   ~/Projects/ml-exp/.venv

  Total: 5.54 GiB across 4 artifacts (scanned 412 dirs in 29ms)

  Sweep? [y]es all / [i]nteractive / [1,3-5] select / [N]o:
```

## Why `broom`?

- **Zero dependencies:** Pure Rust `std` — compiles in <1 second and ships as a tiny single static binary.
- **Fast parallel scanner:** Multi-threaded directory walker with POSIX block-size & hardlink deduplication (matches `du`).
- **Marker-verified safety:** Never blindly deletes a folder named `target` or `build`. Verifies parent project markers (e.g., `Cargo.toml`, `package.json`, `build.gradle`) and inner markers (`CMakeCache.txt`, `pyvenv.cfg`), ignores symlinks, and defaults to **No** at the prompt.
- **Smart project age:** Tracks when the *project* was last touched (source files + `.git/index`) so you can clean stale repos with `-a 30d` while keeping active builds warm.
- **Pure CLI ergonomics:** Pick all (`y`), step one-by-one (`i`), select specific rows right at the prompt (`1,3-5`), dry-run (`-n`), or emit `--json` for scripts.

## Supported Ecosystems

| Ecosystem | Project Marker | Cleaned Artifacts |
| --- | --- | --- |
| **Rust** | `Cargo.toml` | `target` |
| **Node / JS / TS** | `package.json` | `node_modules`, `.next`, `.nuxt`, `.svelte-kit`, `.astro`, `.turbo`, `.parcel-cache` |
| **Python** | `pyproject.toml`, `requirements.txt`, `setup.py`, `*.py` | `.venv` / `venv` *(requires `pyvenv.cfg`)*, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox` |
| **Gradle** | `build.gradle(.kts)`, `settings.gradle(.kts)` | `build`, `.gradle` |
| **Maven** | `pom.xml` | `target` |
| **CMake** | `CMakeLists.txt` | `build`, `cmake-build-debug`, `cmake-build-release` *(requires `CMakeCache.txt`)* |
| **Zig** | `build.zig` | `.zig-cache`, `zig-cache`, `zig-out` |
| **Swift** | `Package.swift` | `.build` |
| **Elixir** | `mix.exs` | `_build`, `deps` |
| **.NET** | `*.csproj`, `*.fsproj`, `*.vbproj` | `bin`, `obj` |
| **Dart / Flutter** | `pubspec.yaml` | `.dart_tool`, `build` |
| **Haskell** | `cabal.project`, `*.cabal`, `stack.yaml` | `dist-newstyle`, `.stack-work` |
| **Terraform** | `*.tf` | `.terraform` |

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Scan current directory and interactively decide what to sweep
broom

# Preview reclaimable space in ~/Projects without prompting
broom -n ~/Projects

# Find artifacts larger than 100MB in projects untouched for 30+ days
broom -a 30d -s 100M ~/Projects

# Sweep only Rust and Node artifacts without prompting
broom -t rust,node -y ~/Projects

# Sort oldest projects first, excluding a path pattern
broom --sort age -e work-critical ~/Projects

# Machine-readable JSON output
broom --json ~/Projects | jq '.artifacts[] | {path, size_human}'
```

## License

MIT
