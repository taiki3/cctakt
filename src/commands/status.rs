//! Status command implementation

use crate::commands::init::{check_claude_cli, check_github_token};
use crate::git_utils::detect_github_repo;
use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

/// Run the status command
pub fn run_status() -> Result<()> {
    println!("cctakt Environment Status\n");

    // Git repository
    print!("📂 Git repository: ");
    io::stdout().flush().ok();
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if is_git_repo {
        println!("✅ Yes");

        // Get repo info
        if let Some(repo) = detect_github_repo() {
            println!("   Repository: {repo}");
        }
    } else {
        println!("❌ No");
    }

    // Check directories
    print!("📁 .claude/ directory: ");
    io::stdout().flush().ok();
    if PathBuf::from(".claude").exists() {
        println!("✅ Exists");
    } else {
        println!("❌ Missing");
    }

    print!("📁 .cctakt/ directory: ");
    io::stdout().flush().ok();
    if PathBuf::from(".cctakt").exists() {
        println!("✅ Exists");
    } else {
        println!("❌ Missing");
    }

    // Check orchestrator skill
    print!("📄 Orchestrator skill: ");
    io::stdout().flush().ok();
    if PathBuf::from(".claude/commands/orchestrator.md").exists() {
        println!("✅ Installed");
    } else {
        println!("❌ Not installed");
    }

    // Check config
    print!("⚙️  Configuration: ");
    io::stdout().flush().ok();
    if PathBuf::from("cctakt.toml").exists() {
        println!("✅ Found");
    } else {
        println!("⚠️  Using defaults");
    }

    println!();

    // Check GitHub token
    check_github_token();

    // Check claude CLI
    check_claude_cli();

    println!();
    println!("Run 'cctakt init' to set up missing components.");

    Ok(())
}
