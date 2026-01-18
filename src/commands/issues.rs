//! Issues command implementation

use crate::git_utils::detect_github_repo;
use anyhow::Result;
use cctakt::{Config, GitHubClient};

/// List GitHub issues
pub fn run_issues(labels: Option<String>, state: String) -> Result<()> {
    let config = Config::load()?;

    // Get repository from config or detect from git
    let repo = config
        .github
        .repository
        .clone()
        .or_else(detect_github_repo)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No repository configured. Set 'repository' in cctakt.toml or add a git remote."
            )
        })?;

    let client = GitHubClient::new(&repo)?;

    let label_vec: Vec<&str> = labels
        .as_ref()
        .map(|l| l.split(',').map(|s| s.trim()).collect())
        .unwrap_or_default();

    println!("Fetching issues from {repo}...\n");

    let issues = client.fetch_issues(&label_vec, &state)?;

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    for issue in &issues {
        let labels_str = if issue.labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", issue.label_names())
        };
        println!("#{:<5} {}{}", issue.number, issue.title, labels_str);
    }

    println!("\nTotal: {} issues", issues.len());
    Ok(())
}
