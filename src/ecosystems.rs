use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Rust,
    Node,
    Python,
    Gradle,
    Maven,
    CMake,
    Zig,
    Swift,
    Elixir,
    DotNet,
    Dart,
    Haskell,
    Terraform,
}

impl Ecosystem {
    pub const ALL: &'static [Ecosystem] = &[
        Ecosystem::Rust,
        Ecosystem::Node,
        Ecosystem::Python,
        Ecosystem::Gradle,
        Ecosystem::Maven,
        Ecosystem::CMake,
        Ecosystem::Zig,
        Ecosystem::Swift,
        Ecosystem::Elixir,
        Ecosystem::DotNet,
        Ecosystem::Dart,
        Ecosystem::Haskell,
        Ecosystem::Terraform,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ecosystem::Rust => "Rust",
            Ecosystem::Node => "Node",
            Ecosystem::Python => "Python",
            Ecosystem::Gradle => "Gradle",
            Ecosystem::Maven => "Maven",
            Ecosystem::CMake => "CMake",
            Ecosystem::Zig => "Zig",
            Ecosystem::Swift => "Swift",
            Ecosystem::Elixir => "Elixir",
            Ecosystem::DotNet => ".NET",
            Ecosystem::Dart => "Dart",
            Ecosystem::Haskell => "Haskell",
            Ecosystem::Terraform => "Terraform",
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
            Ecosystem::Zig => "zig",
            Ecosystem::Swift => "swift",
            Ecosystem::Elixir => "elixir",
            Ecosystem::DotNet => "dotnet",
            Ecosystem::Dart => "dart",
            Ecosystem::Haskell => "haskell",
            Ecosystem::Terraform => "terraform",
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
            "zig" => Some(Ecosystem::Zig),
            "swift" => Some(Ecosystem::Swift),
            "elixir" | "mix" => Some(Ecosystem::Elixir),
            "dotnet" | ".net" | "csharp" | "cs" => Some(Ecosystem::DotNet),
            "dart" | "flutter" => Some(Ecosystem::Dart),
            "haskell" | "hs" | "cabal" | "stack" => Some(Ecosystem::Haskell),
            "terraform" | "tf" => Some(Ecosystem::Terraform),
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
            Ecosystem::Zig => "\x1b[38;5;214m",       // amber
            Ecosystem::Swift => "\x1b[38;5;203m",     // salmon
            Ecosystem::Elixir => "\x1b[38;5;141m",    // purple
            Ecosystem::DotNet => "\x1b[38;5;105m",    // indigo
            Ecosystem::Dart => "\x1b[38;5;45m",       // turquoise
            Ecosystem::Haskell => "\x1b[38;5;177m",   // violet
            Ecosystem::Terraform => "\x1b[38;5;135m", // orchid
        }
    }
}

/// Rule defining a safely cleanable artifact directory inside a project root.
#[derive(Debug, Clone, Copy)]
pub struct ArtifactRule {
    pub ecosystem: Ecosystem,
    /// Exact directory name to match (e.g. `target`, `node_modules`).
    pub dir_name: &'static str,
    /// At least one of these marker files/extensions must exist in the parent directory.
    /// Entries starting with `.` match file extensions (e.g. `.csproj`, `.tf`).
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
    // Terraform
    ArtifactRule {
        ecosystem: Ecosystem::Terraform,
        dir_name: ".terraform",
        parent_markers: &[".tf"],
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
            if let Some(ext) = marker.strip_prefix('.') {
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
