use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::targets::{match_target, Hit};

pub fn validate_root(root: &Path, force: bool) -> anyhow::Result<PathBuf> {
    let canonical = root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve path '{}': {}", root.display(), e))?;

    if !canonical.is_dir() {
        anyhow::bail!("Path '{}' is not a directory", root.display());
    }

    if !force {
        // Check if filesystem root (e.g. / on Unix or C:\ on Windows)
        let lossy = canonical.to_string_lossy();
        let stripped = lossy.trim_start_matches(r"\\?\");
        let is_root = canonical.parent().is_none()
            || stripped == "/"
            || stripped == "\\"
            || (stripped.len() <= 3 && stripped.contains(':'));

        if is_root {
            anyhow::bail!(
                "Refusing to scan filesystem root '{}' without --force flag.",
                canonical.display()
            );
        }

        // Check if user home directory
        let home_dir = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from);

        if let Some(home) = home_dir {
            if let Ok(canon_home) = home.canonicalize() {
                if canonical == canon_home {
                    anyhow::bail!(
                        "Refusing to scan user home directory '{}' without --force flag.",
                        canonical.display()
                    );
                }
            }
        }
    }

    Ok(canonical)
}

pub fn scan(root: &Path, depth: usize, custom_targets: &[String]) -> Vec<Hit> {
    let mut hits = Vec::new();
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let mut it = WalkDir::new(root)
        .max_depth(depth)
        .follow_links(false)
        .into_iter();

    while let Some(entry) = it.next() {
        let Ok(entry) = entry else { continue };

        // Do not match the scan root itself
        if entry.depth() == 0 {
            continue;
        }

        if !entry.file_type().is_dir() {
            continue;
        }

        // Safety check for symlinks: refuse symlinks pointing outside scan root
        if entry.path_is_symlink() {
            if let Ok(target) = std::fs::read_link(entry.path()) {
                let resolved = if target.is_relative() {
                    entry.path().parent().unwrap_or(root).join(&target)
                } else {
                    target
                };
                if let Ok(canon_resolved) = resolved.canonicalize() {
                    if !canon_resolved.starts_with(&canonical_root) {
                        // Symlink points outside scan root, skip
                        it.skip_current_dir();
                        continue;
                    }
                } else {
                    // Unresolvable link
                    it.skip_current_dir();
                    continue;
                }
            }
        }

        // Safety: refuse to target a directory that contains the currently running executable
        if let Ok(current_exe) = std::env::current_exe() {
            if let Ok(canon_exe) = current_exe.canonicalize() {
                if let Ok(canon_dir) = entry.path().canonicalize() {
                    if canon_exe.starts_with(&canon_dir) {
                        it.skip_current_dir();
                        continue;
                    }
                }
            }
        }

        let name = entry.file_name().to_string_lossy();
        let parent = entry.path().parent().unwrap_or(root);

        if let Some(kind) = match_target(&name, parent, entry.path(), custom_targets) {
            hits.push(Hit {
                path: entry.path().to_path_buf(),
                kind,
            });
            it.skip_current_dir(); // Never descend into junk directories
        }
    }

    hits
}
