use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Rust,
    Node,
    Python,
    Gradle,
    Maven,
    CMake,
    Meson,
    Zig,
    Swift,
    CocoaPods,
    Elixir,
    Erlang,
    Gleam,
    DotNet,
    Dart,
    Scala,
    Clojure,
    Haskell,
    OCaml,
    Php,
    Ruby,
    Godot,
    Unity,
    Unreal,
    Terraform,
    Bazel,
}

impl Ecosystem {
    pub const ALL: &'static [Ecosystem] = &[
        Ecosystem::Rust,
        Ecosystem::Node,
        Ecosystem::Python,
        Ecosystem::Gradle,
        Ecosystem::Maven,
        Ecosystem::CMake,
        Ecosystem::Meson,
        Ecosystem::Zig,
        Ecosystem::Swift,
        Ecosystem::CocoaPods,
        Ecosystem::Elixir,
        Ecosystem::Erlang,
        Ecosystem::Gleam,
        Ecosystem::DotNet,
        Ecosystem::Dart,
        Ecosystem::Scala,
        Ecosystem::Clojure,
        Ecosystem::Haskell,
        Ecosystem::OCaml,
        Ecosystem::Php,
        Ecosystem::Ruby,
        Ecosystem::Godot,
        Ecosystem::Unity,
        Ecosystem::Unreal,
        Ecosystem::Terraform,
        Ecosystem::Bazel,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ecosystem::Rust => "Rust",
            Ecosystem::Node => "Node",
            Ecosystem::Python => "Python",
            Ecosystem::Gradle => "Gradle",
            Ecosystem::Maven => "Maven",
            Ecosystem::CMake => "CMake",
            Ecosystem::Meson => "Meson",
            Ecosystem::Zig => "Zig",
            Ecosystem::Swift => "Swift",
            Ecosystem::CocoaPods => "CocoaPods",
            Ecosystem::Elixir => "Elixir",
            Ecosystem::Erlang => "Erlang",
            Ecosystem::Gleam => "Gleam",
            Ecosystem::DotNet => ".NET",
            Ecosystem::Dart => "Dart",
            Ecosystem::Scala => "Scala",
            Ecosystem::Clojure => "Clojure",
            Ecosystem::Haskell => "Haskell",
            Ecosystem::OCaml => "OCaml",
            Ecosystem::Php => "PHP",
            Ecosystem::Ruby => "Ruby",
            Ecosystem::Godot => "Godot",
            Ecosystem::Unity => "Unity",
            Ecosystem::Unreal => "Unreal",
            Ecosystem::Terraform => "Terraform",
            Ecosystem::Bazel => "Bazel",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Ecosystem::Rust => "rust",
            Ecosystem::Node => "node",
            Ecosystem::Python => "python",
            Ecosystem::Gradle => "gradle",
            Ecosystem::Maven => "maven",
            Ecosystem::CMake => "cmake",
            Ecosystem::Meson => "meson",
            Ecosystem::Zig => "zig",
            Ecosystem::Swift => "swift",
            Ecosystem::CocoaPods => "cocoapods",
            Ecosystem::Elixir => "elixir",
            Ecosystem::Erlang => "erlang",
            Ecosystem::Gleam => "gleam",
            Ecosystem::DotNet => "dotnet",
            Ecosystem::Dart => "dart",
            Ecosystem::Scala => "scala",
            Ecosystem::Clojure => "clojure",
            Ecosystem::Haskell => "haskell",
            Ecosystem::OCaml => "ocaml",
            Ecosystem::Php => "php",
            Ecosystem::Ruby => "ruby",
            Ecosystem::Godot => "godot",
            Ecosystem::Unity => "unity",
            Ecosystem::Unreal => "unreal",
            Ecosystem::Terraform => "terraform",
            Ecosystem::Bazel => "bazel",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Ecosystem> {
        match s.trim().to_ascii_lowercase().as_str() {
            "rust" | "cargo" | "rs" => Some(Ecosystem::Rust),
            "node" | "nodejs" | "js" | "ts" | "npm" | "pnpm" | "yarn" => Some(Ecosystem::Node),
            "python" | "py" | "venv" => Some(Ecosystem::Python),
            "gradle" | "android" => Some(Ecosystem::Gradle),
            "maven" | "mvn" => Some(Ecosystem::Maven),
            "cmake" | "cpp" | "c++" | "c" => Some(Ecosystem::CMake),
            "meson" => Some(Ecosystem::Meson),
            "zig" => Some(Ecosystem::Zig),
            "swift" => Some(Ecosystem::Swift),
            "cocoapods" | "pods" | "ios" => Some(Ecosystem::CocoaPods),
            "elixir" | "mix" => Some(Ecosystem::Elixir),
            "erlang" | "erl" | "rebar" | "rebar3" => Some(Ecosystem::Erlang),
            "gleam" => Some(Ecosystem::Gleam),
            "dotnet" | ".net" | "csharp" | "cs" => Some(Ecosystem::DotNet),
            "dart" | "flutter" => Some(Ecosystem::Dart),
            "scala" | "sbt" => Some(Ecosystem::Scala),
            "clojure" | "clj" | "lein" | "leiningen" => Some(Ecosystem::Clojure),
            "haskell" | "hs" | "cabal" | "stack" => Some(Ecosystem::Haskell),
            "ocaml" | "dune" | "opam" => Some(Ecosystem::OCaml),
            "php" | "composer" => Some(Ecosystem::Php),
            "ruby" | "rb" | "bundler" | "rails" => Some(Ecosystem::Ruby),
            "godot" | "gd" | "gdscript" => Some(Ecosystem::Godot),
            "unity" | "unity3d" => Some(Ecosystem::Unity),
            "unreal" | "ue" | "ue4" | "ue5" => Some(Ecosystem::Unreal),
            "terraform" | "tf" | "tofu" | "opentofu" => Some(Ecosystem::Terraform),
            "bazel" | "bzl" => Some(Ecosystem::Bazel),
            _ => None,
        }
    }

    /// ANSI 256 / standard color code for terminal badges.
    pub fn ansi_color(self) -> &'static str {
        match self {
            Ecosystem::Rust => "\x1b[38;5;208m",      // orange
            Ecosystem::Node => "\x1b[38;5;77m",       // green
            Ecosystem::Python => "\x1b[38;5;220m",    // yellow
            Ecosystem::Gradle => "\x1b[38;5;39m",     // sky blue
            Ecosystem::Maven => "\x1b[38;5;167m",     // coral red
            Ecosystem::CMake => "\x1b[38;5;81m",      // cyan
            Ecosystem::Meson => "\x1b[38;5;37m",      // teal
            Ecosystem::Zig => "\x1b[38;5;214m",       // amber
            Ecosystem::Swift => "\x1b[38;5;203m",     // salmon
            Ecosystem::CocoaPods => "\x1b[38;5;204m", // rose pink
            Ecosystem::Elixir => "\x1b[38;5;141m",    // purple
            Ecosystem::Erlang => "\x1b[38;5;125m",    // maroon
            Ecosystem::Gleam => "\x1b[38;5;213m",     // gleam pink
            Ecosystem::DotNet => "\x1b[38;5;105m",    // indigo
            Ecosystem::Dart => "\x1b[38;5;45m",       // turquoise
            Ecosystem::Scala => "\x1b[38;5;160m",     // crimson
            Ecosystem::Clojure => "\x1b[38;5;114m",   // leaf green
            Ecosystem::Haskell => "\x1b[38;5;177m",   // violet
            Ecosystem::OCaml => "\x1b[38;5;209m",     // warm amber
            Ecosystem::Php => "\x1b[38;5;99m",        // slate purple
            Ecosystem::Ruby => "\x1b[38;5;196m",      // ruby red
            Ecosystem::Godot => "\x1b[38;5;75m",      // godot blue
            Ecosystem::Unity => "\x1b[38;5;250m",     // silver
            Ecosystem::Unreal => "\x1b[38;5;63m",     // royal blue
            Ecosystem::Terraform => "\x1b[38;5;135m", // orchid
            Ecosystem::Bazel => "\x1b[38;5;40m",      // green
        }
    }
}

/// Rule defining a safely cleanable artifact directory inside a project root.
#[derive(Debug, Clone, Copy)]
pub struct ArtifactRule {
    pub ecosystem: Ecosystem,
    /// Exact directory name to match (e.g. `target`, `node_modules`).
    pub dir_name: &'static str,
    /// At least one of these markers must exist in the parent directory.
    /// - Entries starting with `.` match file extensions (e.g. `.csproj`, `.tf`).
    /// - Entries containing `/` check a relative path from the parent directory
    ///   (e.g. `ProjectSettings/ProjectVersion.txt`).
    /// - Other entries match exact sibling filenames.
    pub parent_markers: &'static [&'static str],
    /// Optional marker that must exist *inside* the candidate artifact directory itself
    /// (prevents false positives on generic directory names like `build`).
    pub inner_marker: Option<&'static str>,
}

pub const RULES: &[ArtifactRule] = &[
    // Rust
    ArtifactRule {
        ecosystem: Ecosystem::Rust,
        dir_name: "target",
        parent_markers: &["Cargo.toml"],
        inner_marker: None,
    },
    // Node / JS / TS
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: "node_modules",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".next",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".nuxt",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".svelte-kit",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".astro",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".turbo",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".parcel-cache",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".angular",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".expo",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".nx",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".output",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Node,
        dir_name: ".vite",
        parent_markers: &["package.json"],
        inner_marker: None,
    },
    // Python
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".venv",
        parent_markers: &[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
            "Pipfile",
            "poetry.lock",
            "uv.lock",
        ],
        inner_marker: Some("pyvenv.cfg"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: "venv",
        parent_markers: &[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
            "Pipfile",
            "poetry.lock",
            "uv.lock",
        ],
        inner_marker: Some("pyvenv.cfg"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: "__pycache__",
        parent_markers: &[".py"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".pytest_cache",
        parent_markers: &[
            "pyproject.toml",
            "pytest.ini",
            "setup.cfg",
            "requirements.txt",
            ".py",
        ],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".mypy_cache",
        parent_markers: &["pyproject.toml", "mypy.ini", "setup.cfg", ".py"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".ruff_cache",
        parent_markers: &["pyproject.toml", "ruff.toml", ".ruff.toml", ".py"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".tox",
        parent_markers: &["tox.ini", "pyproject.toml"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".nox",
        parent_markers: &["noxfile.py", "pyproject.toml"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Python,
        dir_name: ".hypothesis",
        parent_markers: &["pyproject.toml", "pytest.ini", "setup.cfg", ".py"],
        inner_marker: None,
    },
    // Gradle
    ArtifactRule {
        ecosystem: Ecosystem::Gradle,
        dir_name: ".gradle",
        parent_markers: &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
        ],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Gradle,
        dir_name: "build",
        parent_markers: &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
        ],
        inner_marker: None,
    },
    // Maven
    ArtifactRule {
        ecosystem: Ecosystem::Maven,
        dir_name: "target",
        parent_markers: &["pom.xml"],
        inner_marker: None,
    },
    // CMake (verified with inner CMakeCache.txt for safety)
    ArtifactRule {
        ecosystem: Ecosystem::CMake,
        dir_name: "build",
        parent_markers: &["CMakeLists.txt"],
        inner_marker: Some("CMakeCache.txt"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::CMake,
        dir_name: "_build",
        parent_markers: &["CMakeLists.txt"],
        inner_marker: Some("CMakeCache.txt"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::CMake,
        dir_name: "cmake-build-debug",
        parent_markers: &["CMakeLists.txt"],
        inner_marker: Some("CMakeCache.txt"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::CMake,
        dir_name: "cmake-build-release",
        parent_markers: &["CMakeLists.txt"],
        inner_marker: Some("CMakeCache.txt"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::CMake,
        dir_name: "cmake-build-relwithdebinfo",
        parent_markers: &["CMakeLists.txt"],
        inner_marker: Some("CMakeCache.txt"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::CMake,
        dir_name: "cmake-build-minsizerel",
        parent_markers: &["CMakeLists.txt"],
        inner_marker: Some("CMakeCache.txt"),
    },
    // Meson (verified with inner meson-private for safety)
    ArtifactRule {
        ecosystem: Ecosystem::Meson,
        dir_name: "build",
        parent_markers: &["meson.build"],
        inner_marker: Some("meson-private"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::Meson,
        dir_name: "builddir",
        parent_markers: &["meson.build"],
        inner_marker: Some("meson-private"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::Meson,
        dir_name: "_build",
        parent_markers: &["meson.build"],
        inner_marker: Some("meson-private"),
    },
    // Zig
    ArtifactRule {
        ecosystem: Ecosystem::Zig,
        dir_name: ".zig-cache",
        parent_markers: &["build.zig"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Zig,
        dir_name: "zig-cache",
        parent_markers: &["build.zig"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Zig,
        dir_name: "zig-out",
        parent_markers: &["build.zig"],
        inner_marker: None,
    },
    // Swift
    ArtifactRule {
        ecosystem: Ecosystem::Swift,
        dir_name: ".build",
        parent_markers: &["Package.swift"],
        inner_marker: None,
    },
    // CocoaPods (verified with inner Manifest.lock for safety)
    ArtifactRule {
        ecosystem: Ecosystem::CocoaPods,
        dir_name: "Pods",
        parent_markers: &["Podfile"],
        inner_marker: Some("Manifest.lock"),
    },
    // Elixir
    ArtifactRule {
        ecosystem: Ecosystem::Elixir,
        dir_name: "_build",
        parent_markers: &["mix.exs"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Elixir,
        dir_name: "deps",
        parent_markers: &["mix.exs"],
        inner_marker: None,
    },
    // Erlang / Rebar3
    ArtifactRule {
        ecosystem: Ecosystem::Erlang,
        dir_name: "_build",
        parent_markers: &["rebar.config"],
        inner_marker: None,
    },
    // Gleam
    ArtifactRule {
        ecosystem: Ecosystem::Gleam,
        dir_name: "build",
        parent_markers: &["gleam.toml"],
        inner_marker: None,
    },
    // .NET
    ArtifactRule {
        ecosystem: Ecosystem::DotNet,
        dir_name: "bin",
        parent_markers: &[".csproj", ".fsproj", ".vbproj"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::DotNet,
        dir_name: "obj",
        parent_markers: &[".csproj", ".fsproj", ".vbproj"],
        inner_marker: None,
    },
    // Dart / Flutter
    ArtifactRule {
        ecosystem: Ecosystem::Dart,
        dir_name: ".dart_tool",
        parent_markers: &["pubspec.yaml"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Dart,
        dir_name: "build",
        parent_markers: &["pubspec.yaml"],
        inner_marker: None,
    },
    // Scala / sbt
    ArtifactRule {
        ecosystem: Ecosystem::Scala,
        dir_name: "target",
        parent_markers: &["build.sbt", ".sbt"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Scala,
        dir_name: ".bloop",
        parent_markers: &["build.sbt", ".sbt"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Scala,
        dir_name: ".metals",
        parent_markers: &["build.sbt", ".sbt"],
        inner_marker: None,
    },
    // Clojure
    ArtifactRule {
        ecosystem: Ecosystem::Clojure,
        dir_name: "target",
        parent_markers: &["project.clj"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Clojure,
        dir_name: ".cpcache",
        parent_markers: &["deps.edn"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Clojure,
        dir_name: ".shadow-cljs",
        parent_markers: &["shadow-cljs.edn"],
        inner_marker: None,
    },
    // Haskell
    ArtifactRule {
        ecosystem: Ecosystem::Haskell,
        dir_name: "dist-newstyle",
        parent_markers: &["cabal.project", ".cabal"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Haskell,
        dir_name: ".stack-work",
        parent_markers: &["stack.yaml"],
        inner_marker: None,
    },
    // OCaml / Dune
    ArtifactRule {
        ecosystem: Ecosystem::OCaml,
        dir_name: "_build",
        parent_markers: &["dune-project", "dune-workspace", ".opam"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::OCaml,
        dir_name: "_opam",
        parent_markers: &["dune-project", "dune-workspace", ".opam"],
        inner_marker: Some(".opam-switch"),
    },
    // PHP / Composer (vendor verified with inner autoload.php for safety)
    ArtifactRule {
        ecosystem: Ecosystem::Php,
        dir_name: "vendor",
        parent_markers: &["composer.json"],
        inner_marker: Some("autoload.php"),
    },
    ArtifactRule {
        ecosystem: Ecosystem::Php,
        dir_name: ".phpunit.cache",
        parent_markers: &["composer.json", "phpunit.xml", "phpunit.xml.dist"],
        inner_marker: None,
    },
    // Ruby
    ArtifactRule {
        ecosystem: Ecosystem::Ruby,
        dir_name: ".ruby-lsp",
        parent_markers: &["Gemfile", ".gemspec"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Ruby,
        dir_name: ".yardoc",
        parent_markers: &["Gemfile", ".gemspec"],
        inner_marker: None,
    },
    // Godot
    ArtifactRule {
        ecosystem: Ecosystem::Godot,
        dir_name: ".godot",
        parent_markers: &["project.godot"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Godot,
        dir_name: ".import",
        parent_markers: &["project.godot"],
        inner_marker: None,
    },
    // Unity (verified via ProjectSettings/ProjectVersion.txt)
    ArtifactRule {
        ecosystem: Ecosystem::Unity,
        dir_name: "Library",
        parent_markers: &["ProjectSettings/ProjectVersion.txt"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Unity,
        dir_name: "Temp",
        parent_markers: &["ProjectSettings/ProjectVersion.txt"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Unity,
        dir_name: "Obj",
        parent_markers: &["ProjectSettings/ProjectVersion.txt"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Unity,
        dir_name: "Logs",
        parent_markers: &["ProjectSettings/ProjectVersion.txt"],
        inner_marker: None,
    },
    // Unreal Engine
    ArtifactRule {
        ecosystem: Ecosystem::Unreal,
        dir_name: "Intermediate",
        parent_markers: &[".uproject"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Unreal,
        dir_name: "DerivedDataCache",
        parent_markers: &[".uproject"],
        inner_marker: None,
    },
    // Terraform / OpenTofu
    ArtifactRule {
        ecosystem: Ecosystem::Terraform,
        dir_name: ".terraform",
        parent_markers: &[".tf", ".tofu"],
        inner_marker: None,
    },
    // Bazel output trees are named specifically, so no inner marker is needed.
    ArtifactRule {
        ecosystem: Ecosystem::Bazel,
        dir_name: "bazel-bin",
        parent_markers: &["WORKSPACE", "WORKSPACE.bazel", "MODULE.bazel"],
        inner_marker: None,
    },
    ArtifactRule {
        ecosystem: Ecosystem::Bazel,
        dir_name: "bazel-out",
        parent_markers: &["WORKSPACE", "WORKSPACE.bazel", "MODULE.bazel"],
        inner_marker: None,
    },
];

/// Fast check: could a directory with this exact name match *any* rule?
pub fn is_candidate_dir_name(name: &str) -> bool {
    RULES.iter().any(|r| r.dir_name == name)
}

/// Checks whether `candidate_dir` (with basename `dir_name`) inside `parent_dir`
/// matches a known artifact rule, given the list of sibling filenames in `parent_dir`.
pub fn match_artifact(
    dir_name: &str,
    candidate_dir: &Path,
    parent_file_names: &[String],
) -> Option<Ecosystem> {
    for rule in RULES {
        if rule.dir_name != dir_name {
            continue;
        }
        let parent_ok = rule.parent_markers.iter().any(|marker| {
            if marker.contains('/') {
                candidate_dir.parent().is_some_and(|parent| {
                    let mut p = parent.to_path_buf();
                    for seg in marker.split('/') {
                        p.push(seg);
                    }
                    p.exists()
                })
            } else if let Some(ext) = marker.strip_prefix('.') {
                parent_file_names.iter().any(|f| {
                    Path::new(f)
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
                })
            } else {
                parent_file_names.iter().any(|f| f == marker)
            }
        });
        if !parent_ok {
            continue;
        }
        if let Some(inner) = rule.inner_marker
            && !candidate_dir.join(inner).exists()
        {
            continue;
        }
        return Some(rule.ecosystem);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn all_ecosystems_have_unique_ids_roundtrip_and_rules() {
        let mut ids = HashSet::new();
        for &eco in Ecosystem::ALL {
            assert!(ids.insert(eco.id()), "duplicate id: {}", eco.id());
            assert_eq!(
                Ecosystem::from_str_loose(eco.id()),
                Some(eco),
                "id {} did not round-trip",
                eco.id()
            );
            assert!(
                eco.name().len() <= 10,
                "ecosystem name '{}' exceeds 10-char table column",
                eco.name()
            );
            assert!(
                RULES.iter().any(|r| r.ecosystem == eco),
                "ecosystem {:?} has no rules",
                eco
            );
        }
    }

    #[test]
    fn loose_aliases_parse() {
        assert_eq!(Ecosystem::from_str_loose("composer"), Some(Ecosystem::Php));
        assert_eq!(Ecosystem::from_str_loose("sbt"), Some(Ecosystem::Scala));
        assert_eq!(Ecosystem::from_str_loose("dune"), Some(Ecosystem::OCaml));
        assert_eq!(Ecosystem::from_str_loose("clj"), Some(Ecosystem::Clojure));
        assert_eq!(Ecosystem::from_str_loose("rebar3"), Some(Ecosystem::Erlang));
        assert_eq!(Ecosystem::from_str_loose("gleam"), Some(Ecosystem::Gleam));
        assert_eq!(Ecosystem::from_str_loose("meson"), Some(Ecosystem::Meson));
        assert_eq!(
            Ecosystem::from_str_loose("pods"),
            Some(Ecosystem::CocoaPods)
        );
        assert_eq!(Ecosystem::from_str_loose("rails"), Some(Ecosystem::Ruby));
        assert_eq!(
            Ecosystem::from_str_loose("gdscript"),
            Some(Ecosystem::Godot)
        );
        assert_eq!(Ecosystem::from_str_loose("unity3d"), Some(Ecosystem::Unity));
        assert_eq!(Ecosystem::from_str_loose("ue5"), Some(Ecosystem::Unreal));
        assert_eq!(
            Ecosystem::from_str_loose("opentofu"),
            Some(Ecosystem::Terraform)
        );
    }
}
