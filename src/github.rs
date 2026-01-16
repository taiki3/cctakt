//! GitHub Issues integration for cctakt
//!
//! Provides functionality to fetch issues from GitHub and use them as agent tasks.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// GitHub Issue representation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    /// Issue number
    pub number: u64,

    /// Issue title
    pub title: String,

    /// Issue body (description)
    pub body: Option<String>,

    /// Labels attached to the issue
    pub labels: Vec<Label>,

    /// Issue state ("open" or "closed")
    pub state: String,

    /// URL to the issue on GitHub
    pub html_url: String,
}

/// GitHub Label representation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    /// Label name
    pub name: String,

    /// Label color (hex without #)
    pub color: String,
}

/// GitHub API client
pub struct GitHubClient {
    /// Repository in "owner/repo" format
    repository: String,

    /// Authentication token (optional for public repos)
    token: Option<String>,
}

impl GitHubClient {
    /// Create a new GitHub client
    ///
    /// Authentication is obtained from:
    /// 1. Environment variable `GITHUB_TOKEN`
    /// 2. `gh auth token` command output (GitHub CLI)
    pub fn new(repository: &str) -> Result<Self> {
        let token = Self::get_token();

        Ok(Self {
            repository: repository.to_string(),
            token,
        })
    }

    /// Create a new GitHub client with explicit token
    pub fn with_token(repository: &str, token: Option<String>) -> Self {
        Self {
            repository: repository.to_string(),
            token,
        }
    }

    /// Get authentication token from environment or gh CLI
    fn get_token() -> Option<String> {
        // First try environment variable
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                return Some(token);
            }
        }

        // Fall back to gh CLI
        Self::get_token_from_gh_cli()
    }

    /// Get token from GitHub CLI
    fn get_token_from_gh_cli() -> Option<String> {
        let output = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;

        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }

        None
    }

    /// Build HTTP request with common headers
    fn build_request(&self, url: &str) -> ureq::Request {
        let mut request = ureq::get(url)
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "cctakt");

        if let Some(ref token) = self.token {
            request = request.set("Authorization", &format!("Bearer {}", token));
        }

        request
    }

    /// Fetch issues with optional label and state filters
    ///
    /// # Arguments
    /// * `labels` - Labels to filter by (issues must have at least one of these labels)
    /// * `state` - Issue state: "open", "closed", or "all"
    ///
    /// # Returns
    /// List of issues matching the criteria
    pub fn fetch_issues(&self, labels: &[&str], state: &str) -> Result<Vec<Issue>> {
        let labels_param = labels.join(",");

        let url = if labels.is_empty() {
            format!(
                "https://api.github.com/repos/{}/issues?state={}",
                self.repository, state
            )
        } else {
            format!(
                "https://api.github.com/repos/{}/issues?labels={}&state={}",
                self.repository, labels_param, state
            )
        };

        let request = self.build_request(&url);
        let response = request
            .call()
            .with_context(|| format!("Failed to fetch issues from {}", self.repository))?;

        let issues: Vec<Issue> = response
            .into_json()
            .context("Failed to parse issues response")?;

        Ok(issues)
    }

    /// Get a single issue by number
    pub fn get_issue(&self, number: u64) -> Result<Issue> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            self.repository, number
        );

        let request = self.build_request(&url);
        let response = request
            .call()
            .with_context(|| format!("Failed to fetch issue #{}", number))?;

        let issue: Issue = response
            .into_json()
            .context("Failed to parse issue response")?;

        Ok(issue)
    }

    /// Add a comment to an issue
    pub fn add_comment(&self, number: u64, body: &str) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}/comments",
            self.repository, number
        );

        let token = self.token.as_ref()
            .ok_or_else(|| anyhow!("Authentication required to add comments"))?;

        let response = ureq::post(&url)
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "cctakt")
            .set("Authorization", &format!("Bearer {}", token))
            .send_json(ureq::json!({ "body": body }))
            .with_context(|| format!("Failed to add comment to issue #{}", number))?;

        if response.status() != 201 {
            return Err(anyhow!(
                "Failed to add comment: HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Close an issue
    pub fn close_issue(&self, number: u64) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            self.repository, number
        );

        let token = self.token.as_ref()
            .ok_or_else(|| anyhow!("Authentication required to close issues"))?;

        let response = ureq::patch(&url)
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "cctakt")
            .set("Authorization", &format!("Bearer {}", token))
            .send_json(ureq::json!({ "state": "closed" }))
            .with_context(|| format!("Failed to close issue #{}", number))?;

        if response.status() != 200 {
            return Err(anyhow!(
                "Failed to close issue: HTTP {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Check if client has authentication
    pub fn has_auth(&self) -> bool {
        self.token.is_some()
    }

    /// Get the repository name
    pub fn repository(&self) -> &str {
        &self.repository
    }
}

impl Issue {
    /// Get a short description of the issue
    pub fn short_description(&self) -> String {
        format!("#{}: {}", self.number, self.title)
    }

    /// Get label names as a comma-separated string
    pub fn label_names(&self) -> String {
        self.labels
            .iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Check if issue has a specific label
    pub fn has_label(&self, name: &str) -> bool {
        self.labels.iter().any(|l| l.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_short_description() {
        let issue = Issue {
            number: 123,
            title: "Test issue".to_string(),
            body: Some("Body text".to_string()),
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/123".to_string(),
        };

        assert_eq!(issue.short_description(), "#123: Test issue");
    }

    #[test]
    fn test_issue_label_names() {
        let issue = Issue {
            number: 1,
            title: "Test".to_string(),
            body: None,
            labels: vec![
                Label {
                    name: "bug".to_string(),
                    color: "d73a4a".to_string(),
                },
                Label {
                    name: "enhancement".to_string(),
                    color: "a2eeef".to_string(),
                },
            ],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/1".to_string(),
        };

        assert_eq!(issue.label_names(), "bug, enhancement");
    }

    #[test]
    fn test_issue_has_label() {
        let issue = Issue {
            number: 1,
            title: "Test".to_string(),
            body: None,
            labels: vec![Label {
                name: "bug".to_string(),
                color: "d73a4a".to_string(),
            }],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/1".to_string(),
        };

        assert!(issue.has_label("bug"));
        assert!(!issue.has_label("enhancement"));
    }

    #[test]
    fn test_github_client_with_token() {
        let client = GitHubClient::with_token("owner/repo", Some("test-token".to_string()));

        assert_eq!(client.repository(), "owner/repo");
        assert!(client.has_auth());
    }

    #[test]
    fn test_github_client_without_token() {
        let client = GitHubClient::with_token("owner/repo", None);

        assert_eq!(client.repository(), "owner/repo");
        assert!(!client.has_auth());
    }
}

// Integration tests that require actual GitHub API access
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore] // Run with: cargo test github_integration -- --ignored
    fn test_fetch_issues_from_public_repo() {
        // Test against a known public repository
        let client = GitHubClient::new("rust-lang/rust").unwrap();
        let issues = client.fetch_issues(&[], "open").unwrap();

        // Should be able to fetch at least some issues
        assert!(!issues.is_empty());
    }

    #[test]
    #[ignore]
    fn test_get_single_issue() {
        let client = GitHubClient::new("rust-lang/rust").unwrap();
        // Issue #1 exists in rust-lang/rust
        let issue = client.get_issue(1).unwrap();

        assert_eq!(issue.number, 1);
    }
}
