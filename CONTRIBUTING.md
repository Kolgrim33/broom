# Contributing to `broom`

Thank you for considering contributing to `broom`! We love small, clean, focused PRs — especially new ecosystem rules, edge-case tests, and UX improvements.

## Design Principles

1. **Zero external runtime dependencies.** Keep `Cargo.toml` free of external crates so `broom` compiles in under a second and ships as a tiny single binary.
2. **Safety first.** Never match generic directory names (like `build` or `target`) without verifying a parent project marker — and an inner marker (like `CMakeCache.txt` or `pyvenv.cfg`) whenever ambiguity exists.
3. **Never follow symlinks** when walking, sizing, or deleting.

## Adding a New Ecosystem (Great First Issue!)

Adding support for a new language or build tool usually takes ~15 lines in [`src/ecosystems.rs`](src/ecosystems.rs):

1. Add a variant to `Ecosystem` (and update `ALL`, `name`, `id`, `from_str_loose`, and `ansi_color`).
2. Add one or more `ArtifactRule` entries to `RULES`:
   ```rust
   ArtifactRule {
       ecosystem: Ecosystem::Zig,
       dir_name: ".zig-cache",
       parent_markers: &["build.zig"],
       inner_marker: None,
   },
   ```
3. Add a positive *and* negative (false-positive safety) test case in [`tests/integration.rs`](tests/integration.rs).
4. Add the ecosystem to the table in [`README.md`](README.md).

## Local Development

```bash
# Format, lint, and run all unit + integration tests
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets

# Build optimized release binary
cargo build --release
```

## Pull Request Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --all-targets` passes
- [ ] New behavior includes tests in `tests/integration.rs` or unit tests
- [ ] No external crates added to `Cargo.toml` without prior discussion in an issue
