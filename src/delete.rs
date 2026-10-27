use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::targets::Hit;

fn clear_readonly(path: &Path) {
    for entry in WalkDir::new(path).follow_links(false).into_iter().filter_map(Result::ok) {
        if let Ok(meta) = entry.metadata() {
            let mut perms = meta.permissions();
            if perms.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = fs::set_permissions(entry.path(), perms);
            }
        }
    }
    if let Ok(meta) = fs::symlink_metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
    }
}

pub fn force_remove(path: &Path) -> io::Result<()> {
    if path.is_symlink() {
        if path.is_dir() {
            return fs::remove_dir(path);
        } else {
            return fs::remove_file(path);
        }
    }

    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(5) => {
            clear_readonly(path);
            fs::remove_dir_all(path)
        }
        Err(e) => Err(e),
    }
}

#[allow(dead_code)]
pub struct DeleteResult {
    pub deleted_bytes: u64,
    pub deleted_count: usize,
    pub failures: Vec<(PathBuf, String)>,
}

pub fn delete_targets(targets: &[(Hit, u64)], pb: Option<&ProgressBar>) -> DeleteResult {
    let results: Vec<Result<(u64, PathBuf), (PathBuf, String)>> = targets
        .into_par_iter()
        .map(|(hit, size)| {
            match force_remove(&hit.path) {
                Ok(()) => {
                    if let Some(bar) = pb {
                        bar.inc(*size);
                    }
                    Ok((*size, hit.path.clone()))
                }
                Err(e) => Err((hit.path.clone(), e.to_string())),
            }
        })
        .collect();

    let mut deleted_bytes = 0u64;
    let mut deleted_count = 0usize;
    let mut failures = Vec::new();

    for res in results {
        match res {
            Ok((size, _)) => {
                deleted_bytes += size;
                deleted_count += 1;
            }
            Err((path, err)) => {
                failures.push((path, err));
            }
        }
    }

    DeleteResult {
        deleted_bytes,
        deleted_count,
        failures,
    }
}
