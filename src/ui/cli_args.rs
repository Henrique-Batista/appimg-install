use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install an AppImage
    Install {
        /// Path or URL to the AppImage
        path: String,

        /// Install globally in /opt/appimages (requires sudo)
        #[arg(long)]
        global: bool,

        /// Dry-run mode: validates and shows what would be done without modifying the system
        #[arg(long)]
        dry_run: bool,

        /// Custom target directory
        #[arg(long)]
        target_dir: Option<PathBuf>,

        /// Skip creating a .desktop launcher entry in ~/.local/share/applications/
        #[arg(long)]
        no_desktop: bool,
    },
    /// Remove an installed AppImage
    Remove {
        /// Name of the AppImage to remove
        name: String,
    },
    /// List installed AppImages
    List,
    /// Open the Interactive TUI (default when no args provided)
    Tui,
}
