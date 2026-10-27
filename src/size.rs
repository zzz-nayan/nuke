use rayon::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

use crate::targets::Hit;

/// Calculates total size of files inside a directory tree without following symlinks.
pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Measures the disk footprint of all discovered hits in parallel.
pub fn measure(hits: Vec<Hit>) -> Vec<(Hit, u64)> {
    hits.into_par_iter()
        .map(|h| {
            let size = dir_size(&h.path);
            (h, size)
        })
        .collect()
}
