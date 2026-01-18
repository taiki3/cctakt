//! Init command implementation

use anyhow::Result;
use cctakt::Config;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

/// Run the init command
pub fn run_init(force: bool) -> Result<()> {
    println!("🚀 Initializing cctakt...\n");

    // Check if we're in a git repository
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        return Err(anyhow::anyhow!(
            "Not a git repository. Please run 'cctakt init' from within a git repository."
        ));
    }

    // 1. Check/create .claude directory
    let claude_dir = PathBuf::from(".claude");
    let commands_dir = claude_dir.join("commands");

    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)?;
        println!("✅ Created .claude/ directory");
    } else {
        println!("📁 .claude/ directory already exists");
    }

    if !commands_dir.exists() {
        fs::create_dir_all(&commands_dir)?;
        println!("✅ Created .claude/commands/ directory");
    }

    // 2. Create orchestrator skill
    let orchestrator_skill_path = commands_dir.join("orchestrator.md");
    if !orchestrator_skill_path.exists() || force {
        let skill_content = include_str!("../../templates/orchestrator_skill.md");
        fs::write(&orchestrator_skill_path, skill_content)?;
        println!("✅ Created orchestrator skill: .claude/commands/orchestrator.md");
    } else {
        println!("📄 Orchestrator skill already exists (use --force to overwrite)");
    }

    // 3. Create orchestrator.md reference
    let orchestrator_md_path = claude_dir.join("orchestrator.md");
    if !orchestrator_md_path.exists() || force {
        let orchestrator_content = include_str!("../../templates/orchestrator.md");
        fs::write(&orchestrator_md_path, orchestrator_content)?;
        println!("✅ Created orchestrator reference: .claude/orchestrator.md");
    } else {
        println!("📄 Orchestrator reference already exists (use --force to overwrite)");
    }

    // 4. Create .cctakt directory
    let cctakt_dir = PathBuf::from(".cctakt");
    if !cctakt_dir.exists() {
        fs::create_dir_all(&cctakt_dir)?;
        println!("✅ Created .cctakt/ directory");
    } else {
        println!("📁 .cctakt/ directory already exists");
    }

    // 5. Create cctakt.toml config if not exists
    let config_path = PathBuf::from("cctakt.toml");
    if !config_path.exists() || force {
        Config::generate_default(&config_path)?;
        println!("✅ Created configuration: cctakt.toml");
    } else {
        println!("📄 Configuration file already exists (use --force to overwrite)");
    }

    // 6. Update .gitignore
    let gitignore_path = PathBuf::from(".gitignore");
    let gitignore_entries = [".cctakt/plan_*.json"];

    let existing_gitignore = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let mut added_entries = Vec::new();

    for entry in gitignore_entries {
        if !existing_gitignore.contains(entry) {
            added_entries.push(entry);
        }
    }

    if !added_entries.is_empty() {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)?;

        if !existing_gitignore.is_empty() && !existing_gitignore.ends_with('\n') {
            writeln!(file)?;
        }
        writeln!(file, "\n# cctakt")?;
        for entry in &added_entries {
            writeln!(file, "{entry}")?;
        }
        println!("✅ Updated .gitignore with cctakt entries");
    }

    println!("\n---\n");

    // 7. Check GitHub token
    check_github_token();

    // 8. Check claude CLI
    check_claude_cli();

    println!("\n🎉 cctakt initialization complete!");
    println!("\nNext steps:");
    println!("  1. Run 'cctakt' to start the TUI");
    println!("  2. Press 'i' to select an issue");
    println!("  3. The orchestrator Claude Code can use /orchestrator skill");

    Ok(())
}

/// Check GitHub token availability
pub fn check_github_token() {
    print!("🔑 GitHub token: ");
    io::stdout().flush().ok();

    // Check environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            println!("✅ Found (GITHUB_TOKEN environment variable)");
            return;
        }
    }

    // Check gh CLI
    let gh_token = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|t| !t.is_empty());

    if gh_token.is_some() {
        println!("✅ Found (gh CLI)");
        return;
    }

    println!("⚠️  Not found");
    println!("   To enable GitHub integration:");
    println!("   - Set GITHUB_TOKEN environment variable, or");
    println!("   - Run 'gh auth login' to authenticate with GitHub CLI");
}

/// Check claude CLI availability
pub fn check_claude_cli() {
    print!("🤖 Claude CLI: ");
    io::stdout().flush().ok();

    let claude_available = Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if claude_available {
        println!("✅ Available");
    } else {
        println!("❌ Not found");
        println!("   Install Claude Code CLI: npm install -g @anthropic-ai/claude-code");
    }
}
