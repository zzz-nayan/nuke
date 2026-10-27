use std::path::Path;

#[derive(Debug, Clone)]
pub struct TargetRule {
    pub name: &'static str,
    pub kind: &'static str,
    /// Optional validator to reduce false positives for generic names.
    /// Returns true if the directory matches the expected project environment.
    pub validator: Option<fn(parent: &Path, dir: &Path) -> bool>,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub path: std::path::PathBuf,
    pub kind: String,
}

fn has_extension_sibling(parent: &Path, extensions: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return false;
    };
    for entry in entries.flatten() {
        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
            if extensions.iter().any(|&candidate| candidate.eq_ignore_ascii_case(ext)) {
                return true;
            }
        }
    }
    false
}

fn has_file_sibling(parent: &Path, filenames: &[&str]) -> bool {
    filenames.iter().any(|f| parent.join(f).exists())
}

/// Sibling check for .NET bin/obj: look for .csproj, .fsproj, .vbproj, .sln, etc.
fn is_dotnet_target(parent: &Path, _dir: &Path) -> bool {
    has_extension_sibling(parent, &["csproj", "fsproj", "vbproj", "sln", "props", "targets"])
        || has_file_sibling(parent, &["Directory.Build.props", "Directory.Build.targets", "project.json"])
}

/// Check for .vs folder: parent has a solution file, git, or .csproj
fn is_vs_cache(parent: &Path, _dir: &Path) -> bool {
    has_extension_sibling(parent, &["sln", "csproj", "fsproj"])
        || parent.join(".git").exists()
}

/// Sibling check for Unity: Unity projects have Assets and/or ProjectSettings folders
fn is_unity_target(parent: &Path, _dir: &Path) -> bool {
    parent.join("Assets").is_dir() || parent.join("ProjectSettings").is_dir()
}

/// Sibling check for Rust: target directory with Cargo.toml in parent
fn is_rust_target(parent: &Path, _dir: &Path) -> bool {
    parent.join("Cargo.toml").exists()
}

/// Sibling check for Python virtual environments
fn is_python_env(parent: &Path, dir: &Path) -> bool {
    // pyvenv.cfg inside the env dir is standard
    if dir.join("pyvenv.cfg").exists() {
        return true;
    }
    // Check common Python project indicator files
    has_file_sibling(
        parent,
        &[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
            "Pipfile",
            "environment.yml",
            "poetry.lock",
        ],
    ) || has_extension_sibling(parent, &["py"])
}

/// Check for web/node build artifacts (dist, build)
fn is_web_or_build_artifact(parent: &Path, _dir: &Path) -> bool {
    has_file_sibling(
        parent,
        &[
            "package.json",
            "tsconfig.json",
            "vite.config.js",
            "vite.config.ts",
            "vite.config.mjs",
            "webpack.config.js",
            "webpack.config.ts",
            "rollup.config.js",
            "nuxt.config.ts",
            "nuxt.config.js",
            "next.config.js",
            "next.config.mjs",
            "next.config.ts",
            "astro.config.mjs",
            "svelte.config.js",
            "CMakeLists.txt",
            "Makefile",
            "pom.xml",
            "build.gradle",
        ],
    )
}

/// Check for C/C++ or build system out folder
fn is_build_out_artifact(parent: &Path, _dir: &Path) -> bool {
    has_file_sibling(
        parent,
        &[
            "CMakeLists.txt",
            "Makefile",
            "meson.build",
            "build.ninja",
            "Android.mk",
            "Cargo.toml",
            "package.json",
        ],
    )
}

pub const BUILTIN_TARGETS: &[TargetRule] = &[
    // Node / Web
    TargetRule { name: "node_modules", kind: "Node", validator: None },
    TargetRule { name: ".next", kind: "Next.js", validator: None },
    TargetRule { name: ".nuxt", kind: "Nuxt", validator: None },
    TargetRule { name: ".turbo", kind: "Turbo", validator: None },
    TargetRule { name: "dist", kind: "Build/Dist", validator: Some(is_web_or_build_artifact) },
    TargetRule { name: "build", kind: "Build", validator: Some(is_web_or_build_artifact) },

    // C# / .NET
    TargetRule { name: "bin", kind: ".NET", validator: Some(is_dotnet_target) },
    TargetRule { name: "obj", kind: ".NET", validator: Some(is_dotnet_target) },
    TargetRule { name: ".vs", kind: "Visual Studio", validator: Some(is_vs_cache) },

    // Unity
    TargetRule { name: "Library", kind: "Unity", validator: Some(is_unity_target) },
    TargetRule { name: "Temp", kind: "Unity", validator: Some(is_unity_target) },
    TargetRule { name: "Obj", kind: "Unity", validator: Some(is_unity_target) },
    TargetRule { name: "Build", kind: "Unity", validator: Some(is_unity_target) },
    TargetRule { name: "Builds", kind: "Unity", validator: Some(is_unity_target) },
    TargetRule { name: "Logs", kind: "Unity", validator: Some(is_unity_target) },
    TargetRule { name: "MemoryCaptures", kind: "Unity", validator: Some(is_unity_target) },

    // Python
    TargetRule { name: "__pycache__", kind: "Python", validator: None },
    TargetRule { name: ".pytest_cache", kind: "Python", validator: None },
    TargetRule { name: ".venv", kind: "Python venv", validator: Some(is_python_env) },
    TargetRule { name: "env", kind: "Python venv", validator: Some(is_python_env) },

    // Rust / C++
    TargetRule { name: "target", kind: "Rust", validator: Some(is_rust_target) },
    TargetRule { name: "out", kind: "Build Out", validator: Some(is_build_out_artifact) },
];

/// Match a directory name against built-in targets and custom targets.
pub fn match_target(
    name: &str,
    parent: &Path,
    dir: &Path,
    custom_targets: &[String],
) -> Option<String> {
    // Check custom targets first (case-insensitive)
    for custom in custom_targets {
        if custom.eq_ignore_ascii_case(name) {
            return Some("Custom".to_string());
        }
    }

    // Check built-in targets
    for rule in BUILTIN_TARGETS {
        // Exact name match or case-insensitive if appropriate
        let matches_name = if cfg!(windows) {
            rule.name.eq_ignore_ascii_case(name)
        } else {
            rule.name == name
        };

        if matches_name {
            if let Some(validator) = rule.validator {
                if validator(parent, dir) {
                    return Some(rule.kind.to_string());
                }
            } else {
                return Some(rule.kind.to_string());
            }
        }
    }

    None
}
