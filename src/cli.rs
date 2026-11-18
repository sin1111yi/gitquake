use std::path::PathBuf;

use clap::Parser;

/// ghg — a git history waveform for your terminal.
///
/// Every commit becomes a bar whose height and colour map to how much it
/// changed. Time flows left to right. Zoom, pan, search and themes are all
/// configurable through a btop-style config file.
#[derive(Parser, Debug)]
#[command(
    name = "ghg",
    version,
    about = "Git history as a heat-coloured waveform",
    after_help = "Keys: ↑↓/jk select · ←→/hl pan · +/- zoom · ↵ detail · / search\n\
                  t sort · s scale · a all branches · T theme · L list · R reload · v commit\n\
                  ? help · q quit"
)]
pub struct Cli {
    /// Repository path (defaults to the current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Maximum number of commits to load (0 = all)
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// Only commits newer than this date (passed to git --since)
    #[arg(long)]
    pub since: Option<String>,

    /// Only commits by this author (passed to git --author)
    #[arg(long)]
    pub author: Option<String>,

    /// Include commits from all branches
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Skip merge commits
    #[arg(long)]
    pub no_merges: bool,

    /// Heat mapping: log or linear
    #[arg(long, value_name = "log|linear")]
    pub scale: Option<String>,

    /// Theme name (built-in or a file in the themes directory)
    #[arg(long)]
    pub theme: Option<String>,

    /// Print a static, pipe-friendly rendering instead of the TUI
    #[arg(long)]
    pub plain: bool,

    /// Force ANSI colours even when stdout is not a terminal
    #[arg(long)]
    pub color: bool,

    /// Disable ANSI colours
    #[arg(long)]
    pub no_color: bool,

    /// Use this config file instead of the default
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Do not read or create a config file
    #[arg(long)]
    pub no_config: bool,

    /// Print the effective configuration and exit
    #[arg(long)]
    pub dump_config: bool,

    /// List available themes and exit
    #[arg(long)]
    pub list_themes: bool,
}
