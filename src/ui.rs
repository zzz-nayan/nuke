use std::path::Path;

use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, Table, TableComponent};
use indicatif::{ProgressBar, ProgressStyle};

use crate::targets::Hit;

/// Formats a byte count into decimal human-readable units (e.g. 412.5 MB, 9.16 GB).
pub fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string()
}

/// Formats a path relative to the scan root, with forward slashes for clean terminal display.
pub fn format_relative_path(root: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(root) {
        let s = rel.to_string_lossy().replace('\\', "/");
        if s.is_empty() {
            ".".to_string()
        } else {
            format!("./{}", s)
        }
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Prints the initial scanning message.
pub fn print_scan_start(root: &Path) {
    let display_str = root.to_string_lossy();
    let cleaned = display_str.strip_prefix(r"\\?\").unwrap_or(&display_str);
    println!("\n 🔍 Scanning for junk directories in {}...", cleaned);
}

/// Prints the number of found directories.
pub fn print_scan_found(count: usize, depth: usize) {
    let word = if count == 1 { "directory" } else { "directories" };
    println!(" Found {} purgeable {} (traversed {} levels):\n", count, word, depth);
}

/// Prints clean message when nothing was found.
pub fn print_scan_empty(depth: usize) {
    println!(" ✨ Clean! No purgeable directories found (traversed {} levels).\n", depth);
}

/// Renders the scan results in a formatted UTF-8 table with summary footer.
pub fn render_table(root: &Path, hits_with_sizes: &[(Hit, u64)]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_style(TableComponent::LeftHeaderIntersection, '├')
        .set_style(TableComponent::HeaderLines, '─')
        .set_style(TableComponent::MiddleHeaderIntersections, '┼')
        .set_style(TableComponent::RightHeaderIntersection, '┤')
        .set_style(TableComponent::VerticalLines, '│');

    table.remove_style(TableComponent::HorizontalLines);
    table.remove_style(TableComponent::MiddleIntersections);
    table.remove_style(TableComponent::LeftBorderIntersections);
    table.remove_style(TableComponent::RightBorderIntersections);

    table.set_header(vec![
        Cell::new("Directory Path").set_alignment(CellAlignment::Left),
        Cell::new("Type").set_alignment(CellAlignment::Left),
        Cell::new("Size").set_alignment(CellAlignment::Right),
    ]);

    let mut total_bytes = 0u64;

    for (hit, size) in hits_with_sizes {
        total_bytes += *size;
        let rel_path = format_relative_path(root, &hit.path);
        let size_str = format_size(*size);

        table.add_row(vec![
            Cell::new(rel_path).set_alignment(CellAlignment::Left),
            Cell::new(&hit.kind).set_alignment(CellAlignment::Left),
            Cell::new(size_str).set_alignment(CellAlignment::Right),
        ]);
    }

    let table_str = table.to_string();
    let mut lines: Vec<&str> = table_str.lines().collect();

    if let Some(last_line) = lines.pop() {
        // Print table header and rows
        for line in &lines {
            println!(" {line}");
        }

        // Replace bottom border with middle separator: ├───────────┴─────────┴──────────┤
        let mid_border = last_line
            .replacen('└', "├", 1)
            .replace('┘', "┤");
        println!(" {mid_border}");

        // Print footer line with total
        let char_count = last_line.chars().count();
        let footer_text = format!("Total reclaimable space: {}", format_size(total_bytes));
        let inner_width = char_count.saturating_sub(4);
        println!(" │ {:<inner_width$} │", footer_text, inner_width = inner_width);

        // Print final closing border: └───────────────────────────┘
        println!(" └{}┘\n", "─".repeat(char_count.saturating_sub(2)));
    }
}

/// Interactive selection prompt using dialoguer.
pub fn prompt_selection(
    hits_with_sizes: &[(Hit, u64)],
    root: &Path,
) -> anyhow::Result<Vec<(Hit, u64)>> {
    let mut items = Vec::with_capacity(hits_with_sizes.len() + 1);
    items.push("Select All".to_string());

    for (hit, size) in hits_with_sizes {
        let rel = format_relative_path(root, &hit.path);
        let sz = format_size(*size);
        items.push(format!("{} ({})", rel, sz));
    }

    let defaults = vec![true; items.len()];

    let chosen = dialoguer::MultiSelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Select directories to destroy")
        .items(&items)
        .defaults(&defaults)
        .interact_opt()?;

    let Some(chosen) = chosen else {
        return Ok(Vec::new());
    };

    if chosen.is_empty() {
        return Ok(Vec::new());
    }

    let selected: Vec<(Hit, u64)> = if chosen.len() == 1 && chosen[0] == 0 {
        // Only "Select All" was checked
        hits_with_sizes.to_vec()
    } else {
        // Collect directories whose corresponding index (1..=N) was selected
        chosen
            .into_iter()
            .filter(|&idx| idx >= 1)
            .filter_map(|idx| hits_with_sizes.get(idx - 1).cloned())
            .collect()
    };

    Ok(selected)
}

/// Creates a progress bar configured according to specification.
pub fn create_progress_bar(total_bytes: u64, is_count_based: bool) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    if is_count_based {
        pb.set_style(
            ProgressStyle::default_bar()
                .template(" [{bar:40}] {percent}% ({pos}/{len} directories)")
                .expect("valid template")
                .progress_chars("== "),
        );
    } else {
        pb.set_style(
            ProgressStyle::default_bar()
                .template(" [{bar:40}] {percent}% (Deleted {bytes})")
                .expect("valid template")
                .progress_chars("== "),
        );
    }
    pb
}
