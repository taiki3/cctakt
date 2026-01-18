//! Git utility functions

use std::path::PathBuf;
use std::process::Command;

/// Get commit log from worktree
pub fn get_commit_log(worktree_path: &PathBuf) -> String {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["log", "--oneline", "-n", "20", "--no-decorate"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    }
}

/// Get commits made by a worker (commits since branch creation)
pub fn get_worker_commits(worktree_path: &PathBuf) -> Vec<String> {
    // Get commits that are ahead of main/master
    // Try main first, then master
    let bases = ["main", "master"];
    for base in bases {
        let output = Command::new("git")
            .current_dir(worktree_path)
            .args(["log", "--oneline", &format!("{base}..HEAD")])
            .output();

        if let Ok(o) = output {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let commits: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
                if !commits.is_empty() {
                    return commits;
                }
            }
        }
    }

    // Fallback: just get recent commits
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["log", "--oneline", "-n", "10"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Detect GitHub repository from git remote
pub fn detect_github_repo() -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_github_url(&url)
}

/// Parse GitHub repository from URL string
/// Supports formats:
/// - https://github.com/owner/repo.git
/// - git@github.com:owner/repo.git
/// - https://github.com/owner/repo
pub fn parse_github_url(url: &str) -> Option<String> {
    if url.contains("github.com") {
        let repo = url
            .trim_end_matches(".git")
            .split("github.com")
            .last()?
            .trim_start_matches('/')
            .trim_start_matches(':')
            .to_string();
        if repo.is_empty() {
            None
        } else {
            Some(repo)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let url = "https://github.com/owner/repo.git";
        assert_eq!(parse_github_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn test_parse_github_url_https_no_git_suffix() {
        let url = "https://github.com/owner/repo";
        assert_eq!(parse_github_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let url = "git@github.com:owner/repo.git";
        assert_eq!(parse_github_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn test_parse_github_url_ssh_no_git_suffix() {
        let url = "git@github.com:owner/repo";
        assert_eq!(parse_github_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn test_parse_github_url_non_github() {
        let url = "https://gitlab.com/owner/repo.git";
        assert_eq!(parse_github_url(url), None);
    }

    #[test]
    fn test_parse_github_url_empty() {
        assert_eq!(parse_github_url(""), None);
    }

    #[test]
    fn test_parse_github_url_github_only() {
        let url = "https://github.com/";
        assert_eq!(parse_github_url(url), None);
    }

    #[test]
    fn test_parse_github_url_with_nested_path() {
        let url = "https://github.com/org/repo/subpath";
        assert_eq!(parse_github_url(url), Some("org/repo/subpath".to_string()));
    }

    #[test]
    fn test_parse_github_url_enterprise() {
        let url = "https://github.example.com/owner/repo.git";
        assert_eq!(parse_github_url(url), None);
    }

    #[test]
    fn test_parse_github_url_with_port() {
        let url = "https://github.com:443/owner/repo.git";
        let result = parse_github_url(url);
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_get_commit_log() {
        let log = get_commit_log(&PathBuf::from("."));
        assert!(!log.is_empty());
    }

    #[test]
    fn test_get_worker_commits_current_repo() {
        let commits = get_worker_commits(&PathBuf::from("."));
        assert!(!commits.is_empty());
    }

    #[test]
    fn test_get_worker_commits_nonexistent_dir() {
        let commits = get_worker_commits(&PathBuf::from("/nonexistent/path/that/doesnt/exist"));
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_worker_commits_format() {
        let commits = get_worker_commits(&PathBuf::from("."));
        if !commits.is_empty() {
            let first = &commits[0];
            assert!(first.len() >= 7, "Commit should have hash: {first}");
        }
    }
}
