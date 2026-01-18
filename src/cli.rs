//! CLI argument parsing

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cctakt")]
#[command(author, version, about = "Claude Code Orchestrator - TUI for managing multiple Claude Code agents")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize cctakt in the current repository
    Init {
        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },
    /// Check environment setup status
    Status,
    /// List GitHub issues
    Issues {
        /// Filter by labels (comma-separated)
        #[arg(short, long)]
        labels: Option<String>,
        /// Issue state: open, closed, all
        #[arg(short, long, default_value = "open")]
        state: String,
    },
    /// Run workers from a plan file (CLI mode, no TUI)
    Run {
        /// Path to plan.json file (default: .cctakt/plan.json)
        #[arg(default_value = ".cctakt/plan.json")]
        plan: PathBuf,
    },
}
