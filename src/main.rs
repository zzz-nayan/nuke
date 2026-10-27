use clap::{CommandFactory, Parser};
use std::path::PathBuf;

use nuke::{cli, delete, scan, size, ui};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    if let Some(ref p) = args.path {
        if p.to_string_lossy().eq_ignore_ascii_case("help") && !p.is_dir() {
            cli::Args::command().print_help()?;
            println!();
            return Ok(());
        }
    }

    let scan_path = args.path.unwrap_or_else(|| PathBuf::from("."));
    let canonical_root = scan::validate_root(&scan_path, args.force)?;

    ui::print_scan_start(&canonical_root);

    let hits = scan::scan(&canonical_root, args.depth, &args.target);

    if hits.is_empty() {
        ui::print_scan_empty(args.depth);
        return Ok(());
    }

    let mut measured = size::measure(hits);
    // Sort largest first for visibility
    measured.sort_by(|a, b| b.1.cmp(&a.1));

    ui::print_scan_found(measured.len(), args.depth);
    ui::render_table(&canonical_root, &measured);

    if args.dry_run {
        println!(" ℹ️  Dry run mode: no files or directories were deleted.\n");
        return Ok(());
    }

    let selected = if args.yes {
        measured
    } else {
        if !console::Term::stdout().is_term() {
            anyhow::bail!(
                "Non-interactive terminal detected. Run with -y / --yes to delete discovered directories."
            );
        }
        ui::prompt_selection(&measured, &canonical_root)?
    };

    if selected.is_empty() {
        println!(" ✨ Clean cancelled. No directories were deleted.\n");
        return Ok(());
    }

    let total_to_delete: u64 = selected.iter().map(|(_, size)| *size).sum();
    let dirs_word = if selected.len() == 1 { "directory" } else { "directories" };
    println!(" 💥 Nuking {} {}...", selected.len(), dirs_word);

    let pb = ui::create_progress_bar(total_to_delete, total_to_delete == 0);
    let result = delete::delete_targets(&selected, Some(&pb));
    if result.deleted_bytes > 0 {
        pb.set_position(result.deleted_bytes);
        pb.finish();
    } else {
        pb.abandon();
    }
    println!();

    println!(" ✨ Clean complete. {} recovered.\n", ui::format_size(result.deleted_bytes));

    if !result.failures.is_empty() {
        let fail_word = if result.failures.len() == 1 { "directory" } else { "directories" };
        eprintln!(" ⚠️  Failed to delete {} {}:", result.failures.len(), fail_word);
        for (path, err) in &result.failures {
            eprintln!("   • {}: {}", ui::format_relative_path(&canonical_root, path), err);
        }
        eprintln!();
    }

    Ok(())
}
