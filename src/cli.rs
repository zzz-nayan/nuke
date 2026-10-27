use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "nuke",
    version,
    about = "A fast, interactive directory-cleaner CLI for developers"
)]
pub struct Args {
    /// Directory to scan (defaults to the current directory)
    pub path: Option<PathBuf>,

    /// Skip confirmation and delete everything discovered
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Maximum directory traversal depth
    #[arg(short = 'd', long, default_value_t = 4)]
    pub depth: usize,

    /// Scan and report only; delete nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Extra folder names to target, comma separated
    #[arg(short = 't', long, value_delimiter = ',')]
    pub target: Vec<String>,

    /// Force scanning filesystem root or home directory
    #[arg(short = 'f', long)]
    pub force: bool,
}
