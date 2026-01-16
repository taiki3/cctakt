//! GitHub Issues integration for cctakt
//!
//! Provides functionality to fetch issues from GitHub and use them as agent tasks.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(test)]
use mockall::automock;

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

/// GitHub Pull Request representation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequest {
    /// PR number
    pub number: u64,

    /// PR title
    pub title: String,

    /// PR body (description)
    pub body: Option<String>,

    /// PR state ("open", "closed", "merged")
    pub state: String,

    /// URL to the PR on GitHub
    pub html_url: String,

    /// Head branch name
    pub head: PullRequestRef,

    /// Base branch name
    pub base: PullRequestRef,

    /// Whether the PR is a draft
    #[serde(default)]
    pub draft: bool,
}

/// Pull Request branch reference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequestRef {
    /// Branch name
    #[serde(rename = "ref")]
    pub branch: String,

    /// SHA of the commit
    pub sha: String,
}

/// Parameters for creating a pull request
#[derive(Debug, Clone)]
pub struct CreatePullRequest {
    /// PR title
    pub title: String,

    /// PR body/description
    pub body: Option<String>,

    /// Head branch (the branch with changes)
    pub head: String,

    /// Base branch (the branch to merge into)
    pub base: String,

    /// Whether to create as draft
    pub draft: bool,
}

/// HTTP response abstraction for testing
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// HTTP headers type
pub type Headers = Vec<(String, String)>;

/// Trait for HTTP operations (allows mocking)
#[cfg_attr(test, automock)]
pub trait HttpClient: Send + Sync {
    /// Send a GET request
    fn get(&self, url: &str, headers: Headers) -> Result<HttpResponse>;

    /// Send a POST request with JSON body
    fn post(&self, url: &str, headers: Headers, body: String) -> Result<HttpResponse>;

    /// Send a PATCH request with JSON body
    fn patch(&self, url: &str, headers: Headers, body: String) -> Result<HttpResponse>;
}

/// Real HTTP client using ureq
#[derive(Default)]
pub struct UreqHttpClient;

impl HttpClient for UreqHttpClient {
    fn get(&self, url: &str, headers: Headers) -> Result<HttpResponse> {
        let mut request = ureq::get(url);
        for (key, value) in &headers {
            request = request.set(key, value);
        }
        let response = request.call().context("HTTP GET failed")?;
        let status = response.status();
        let body = response.into_string().context("Failed to read response body")?;
        Ok(HttpResponse { status, body })
    }

    fn post(&self, url: &str, headers: Headers, body: String) -> Result<HttpResponse> {
        let mut request = ureq::post(url);
        for (key, value) in &headers {
            request = request.set(key, value);
        }
        let response = request.send_string(&body).context("HTTP POST failed")?;
        let status = response.status();
        let body = response.into_string().context("Failed to read response body")?;
        Ok(HttpResponse { status, body })
    }

    fn patch(&self, url: &str, headers: Headers, body: String) -> Result<HttpResponse> {
        let mut request = ureq::patch(url);
        for (key, value) in &headers {
            request = request.set(key, value);
        }
        let response = request.send_string(&body).context("HTTP PATCH failed")?;
        let status = response.status();
        let body = response.into_string().context("Failed to read response body")?;
        Ok(HttpResponse { status, body })
    }
}

/// GitHub API client
pub struct GitHubClient<H: HttpClient = UreqHttpClient> {
    /// Repository in "owner/repo" format
    repository: String,

    /// Authentication token (optional for public repos)
    token: Option<String>,

    /// HTTP client
    http: H,
}

impl GitHubClient<UreqHttpClient> {
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
            http: UreqHttpClient,
        })
    }

    /// Create a new GitHub client with explicit token
    pub fn with_token(repository: &str, token: Option<String>) -> Self {
        Self {
            repository: repository.to_string(),
            token,
            http: UreqHttpClient,
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
}

impl<H: HttpClient> GitHubClient<H> {
    /// Create client with custom HTTP client (for testing)
    pub fn with_http_client(repository: &str, token: Option<String>, http: H) -> Self {
        Self {
            repository: repository.to_string(),
            token,
            http,
        }
    }

    /// Build common headers for requests
    fn build_headers(&self) -> Headers {
        let mut headers = vec![
            ("Accept".to_string(), "application/vnd.github.v3+json".to_string()),
            ("User-Agent".to_string(), "cctakt".to_string()),
        ];

        if let Some(ref token) = self.token {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }

        headers
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

        let headers = self.build_headers();
        let response = self.http.get(&url, headers)
            .with_context(|| format!("Failed to fetch issues from {}", self.repository))?;

        let issues: Vec<Issue> = serde_json::from_str(&response.body)
            .context("Failed to parse issues response")?;

        Ok(issues)
    }

    /// Get a single issue by number
    pub fn get_issue(&self, number: u64) -> Result<Issue> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}",
            self.repository, number
        );

        let headers = self.build_headers();
        let response = self.http.get(&url, headers)
            .with_context(|| format!("Failed to fetch issue #{number}"))?;

        let issue: Issue = serde_json::from_str(&response.body)
            .context("Failed to parse issue response")?;

        Ok(issue)
    }

    /// Add a comment to an issue
    pub fn add_comment(&self, number: u64, body: &str) -> Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/issues/{}/comments",
            self.repository, number
        );

        self.token.as_ref()
            .ok_or_else(|| anyhow!("Authentication required to add comments"))?;

        let mut headers = self.build_headers();
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
        let json_body = serde_json::json!({ "body": body }).to_string();

        let response = self.http.post(&url, headers, json_body)
            .with_context(|| format!("Failed to add comment to issue #{number}"))?;

        if response.status != 201 {
            return Err(anyhow!(
                "Failed to add comment: HTTP {}",
                response.status
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

        self.token.as_ref()
            .ok_or_else(|| anyhow!("Authentication required to close issues"))?;

        let mut headers = self.build_headers();
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
        let json_body = serde_json::json!({ "state": "closed" }).to_string();

        let response = self.http.patch(&url, headers, json_body)
            .with_context(|| format!("Failed to close issue #{number}"))?;

        if response.status != 200 {
            return Err(anyhow!(
                "Failed to close issue: HTTP {}",
                response.status
            ));
        }

        Ok(())
    }

    /// Create a pull request
    ///
    /// # Arguments
    /// * `params` - Pull request parameters
    ///
    /// # Returns
    /// The created pull request
    pub fn create_pull_request(&self, params: &CreatePullRequest) -> Result<PullRequest> {
        let url = format!(
            "https://api.github.com/repos/{}/pulls",
            self.repository
        );

        self.token.as_ref()
            .ok_or_else(|| anyhow!("Authentication required to create pull requests"))?;

        let mut json_body = serde_json::json!({
            "title": params.title,
            "head": params.head,
            "base": params.base,
            "draft": params.draft,
        });

        if let Some(ref body) = params.body {
            json_body["body"] = serde_json::Value::String(body.clone());
        }

        let mut headers = self.build_headers();
        headers.push(("Content-Type".to_string(), "application/json".to_string()));

        let response = self.http.post(&url, headers, json_body.to_string())
            .context("Failed to create pull request")?;

        if response.status != 201 {
            return Err(anyhow!(
                "Failed to create pull request: HTTP {}",
                response.status
            ));
        }

        let pr: PullRequest = serde_json::from_str(&response.body)
            .context("Failed to parse pull request response")?;

        Ok(pr)
    }

    /// Get a pull request by number
    pub fn get_pull_request(&self, number: u64) -> Result<PullRequest> {
        let url = format!(
            "https://api.github.com/repos/{}/pulls/{}",
            self.repository, number
        );

        let headers = self.build_headers();
        let response = self.http.get(&url, headers)
            .with_context(|| format!("Failed to fetch pull request #{number}"))?;

        let pr: PullRequest = serde_json::from_str(&response.body)
            .context("Failed to parse pull request response")?;

        Ok(pr)
    }

    /// List pull requests
    ///
    /// # Arguments
    /// * `state` - PR state: "open", "closed", or "all"
    /// * `head` - Filter by head branch (format: "owner:branch" or just "branch")
    /// * `base` - Filter by base branch
    pub fn list_pull_requests(
        &self,
        state: &str,
        head: Option<&str>,
        base: Option<&str>,
    ) -> Result<Vec<PullRequest>> {
        let mut url = format!(
            "https://api.github.com/repos/{}/pulls?state={}",
            self.repository, state
        );

        if let Some(head_branch) = head {
            url.push_str(&format!("&head={head_branch}"));
        }

        if let Some(base_branch) = base {
            url.push_str(&format!("&base={base_branch}"));
        }

        let headers = self.build_headers();
        let response = self.http.get(&url, headers)
            .context("Failed to fetch pull requests")?;

        let prs: Vec<PullRequest> = serde_json::from_str(&response.body)
            .context("Failed to parse pull requests response")?;

        Ok(prs)
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

    #[test]
    fn test_create_pull_request_params() {
        let params = CreatePullRequest {
            title: "Add new feature".to_string(),
            body: Some("This PR adds a new feature".to_string()),
            head: "feature-branch".to_string(),
            base: "main".to_string(),
            draft: false,
        };

        assert_eq!(params.title, "Add new feature");
        assert_eq!(params.head, "feature-branch");
        assert_eq!(params.base, "main");
        assert!(!params.draft);
    }

    #[test]
    fn test_create_pull_request_params_without_body() {
        let params = CreatePullRequest {
            title: "Quick fix".to_string(),
            body: None,
            head: "fix-branch".to_string(),
            base: "main".to_string(),
            draft: true,
        };

        assert!(params.body.is_none());
        assert!(params.draft);
    }

    #[test]
    fn test_pull_request_ref() {
        let json = r#"{
            "ref": "feature-branch",
            "sha": "abc123def456"
        }"#;

        let pr_ref: PullRequestRef = serde_json::from_str(json).unwrap();
        assert_eq!(pr_ref.branch, "feature-branch");
        assert_eq!(pr_ref.sha, "abc123def456");
    }

    #[test]
    fn test_pull_request_deserialize() {
        let json = r#"{
            "number": 42,
            "title": "Add authentication",
            "body": "This PR adds JWT authentication",
            "state": "open",
            "html_url": "https://github.com/test/repo/pull/42",
            "head": {
                "ref": "feature/auth",
                "sha": "abc123"
            },
            "base": {
                "ref": "main",
                "sha": "def456"
            },
            "draft": false
        }"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Add authentication");
        assert_eq!(pr.body, Some("This PR adds JWT authentication".to_string()));
        assert_eq!(pr.state, "open");
        assert_eq!(pr.head.branch, "feature/auth");
        assert_eq!(pr.base.branch, "main");
        assert!(!pr.draft);
    }

    #[test]
    fn test_pull_request_deserialize_draft_default() {
        let json = r#"{
            "number": 1,
            "title": "Test",
            "body": null,
            "state": "open",
            "html_url": "https://github.com/test/repo/pull/1",
            "head": {
                "ref": "test",
                "sha": "abc"
            },
            "base": {
                "ref": "main",
                "sha": "def"
            }
        }"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();
        // draft should default to false when not present
        assert!(!pr.draft);
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

// Mock-based tests for GitHub API
#[cfg(test)]
mod mock_tests {
    use super::*;

    fn mock_issue_json() -> String {
        r#"{
            "number": 42,
            "title": "Test issue",
            "body": "Issue body",
            "labels": [{"name": "bug", "color": "d73a4a"}],
            "state": "open",
            "html_url": "https://github.com/test/repo/issues/42"
        }"#.to_string()
    }

    fn mock_issues_json() -> String {
        format!("[{}]", mock_issue_json())
    }

    fn mock_pr_json() -> String {
        r#"{
            "number": 123,
            "title": "Test PR",
            "body": "PR body",
            "state": "open",
            "html_url": "https://github.com/test/repo/pull/123",
            "head": {"ref": "feature", "sha": "abc123"},
            "base": {"ref": "main", "sha": "def456"},
            "draft": false
        }"#.to_string()
    }

    fn mock_prs_json() -> String {
        format!("[{}]", mock_pr_json())
    }

    #[test]
    fn test_fetch_issues_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url: &str, _: &Headers| url.contains("/issues"))
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: mock_issues_json(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let issues = client.fetch_issues(&[], "open").unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 42);
        assert_eq!(issues[0].title, "Test issue");
    }

    #[test]
    fn test_fetch_issues_with_labels() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url: &str, _: &Headers| url.contains("labels=bug,enhancement"))
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: mock_issues_json(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let issues = client.fetch_issues(&["bug", "enhancement"], "open").unwrap();

        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_get_issue_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url: &str, _: &Headers| url.contains("/issues/42"))
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: mock_issue_json(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let issue = client.get_issue(42).unwrap();

        assert_eq!(issue.number, 42);
        assert_eq!(issue.title, "Test issue");
        assert!(issue.has_label("bug"));
    }

    #[test]
    fn test_add_comment_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .withf(|url: &str, _: &Headers, body: &String| {
                url.contains("/issues/42/comments") && body.contains("Test comment")
            })
            .returning(|_, _, _| Ok(HttpResponse {
                status: 201,
                body: "{}".to_string(),
            }));

        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let result = client.add_comment(42, "Test comment");

        assert!(result.is_ok());
    }

    #[test]
    fn test_add_comment_requires_auth() {
        let mock = MockHttpClient::new();
        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let result = client.add_comment(42, "Test comment");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Authentication required"));
    }

    #[test]
    fn test_add_comment_http_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .returning(|_, _, _| Ok(HttpResponse {
                status: 403,
                body: "Forbidden".to_string(),
            }));

        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let result = client.add_comment(42, "Test comment");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 403"));
    }

    #[test]
    fn test_close_issue_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_patch()
            .withf(|url: &str, _: &Headers, body: &String| {
                url.contains("/issues/42") && body.contains("closed")
            })
            .returning(|_, _, _| Ok(HttpResponse {
                status: 200,
                body: "{}".to_string(),
            }));

        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let result = client.close_issue(42);

        assert!(result.is_ok());
    }

    #[test]
    fn test_close_issue_requires_auth() {
        let mock = MockHttpClient::new();
        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let result = client.close_issue(42);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Authentication required"));
    }

    #[test]
    fn test_close_issue_http_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_patch()
            .returning(|_, _, _| Ok(HttpResponse {
                status: 404,
                body: "Not found".to_string(),
            }));

        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let result = client.close_issue(42);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 404"));
    }

    #[test]
    fn test_create_pull_request_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .withf(|url: &str, _: &Headers, body: &String| {
                url.contains("/pulls")
                    && body.contains("Test PR")
                    && body.contains("feature")
                    && body.contains("main")
            })
            .returning(|_, _, _| Ok(HttpResponse {
                status: 201,
                body: mock_pr_json(),
            }));

        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let params = CreatePullRequest {
            title: "Test PR".to_string(),
            body: Some("PR description".to_string()),
            head: "feature".to_string(),
            base: "main".to_string(),
            draft: false,
        };
        let pr = client.create_pull_request(&params).unwrap();

        assert_eq!(pr.number, 123);
        assert_eq!(pr.title, "Test PR");
    }

    #[test]
    fn test_create_pull_request_requires_auth() {
        let mock = MockHttpClient::new();
        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let params = CreatePullRequest {
            title: "Test PR".to_string(),
            body: None,
            head: "feature".to_string(),
            base: "main".to_string(),
            draft: false,
        };
        let result = client.create_pull_request(&params);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Authentication required"));
    }

    #[test]
    fn test_create_pull_request_http_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .returning(|_, _, _| Ok(HttpResponse {
                status: 422,
                body: "Validation failed".to_string(),
            }));

        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let params = CreatePullRequest {
            title: "Test PR".to_string(),
            body: None,
            head: "feature".to_string(),
            base: "main".to_string(),
            draft: false,
        };
        let result = client.create_pull_request(&params);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 422"));
    }

    #[test]
    fn test_get_pull_request_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url: &str, _: &Headers| url.contains("/pulls/123"))
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: mock_pr_json(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let pr = client.get_pull_request(123).unwrap();

        assert_eq!(pr.number, 123);
        assert_eq!(pr.head.branch, "feature");
        assert_eq!(pr.base.branch, "main");
    }

    #[test]
    fn test_list_pull_requests_with_mock() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url: &str, _: &Headers| url.contains("/pulls?state=open"))
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: mock_prs_json(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let prs = client.list_pull_requests("open", None, None).unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 123);
    }

    #[test]
    fn test_list_pull_requests_with_filters() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url: &str, _: &Headers| {
                url.contains("state=open")
                    && url.contains("head=feature")
                    && url.contains("base=main")
            })
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: mock_prs_json(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let prs = client.list_pull_requests("open", Some("feature"), Some("main")).unwrap();

        assert_eq!(prs.len(), 1);
    }

    #[test]
    fn test_http_get_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .returning(|_, _| Err(anyhow!("Network error")));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let result = client.fetch_issues(&[], "open");

        assert!(result.is_err());
    }

    #[test]
    fn test_json_parse_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .returning(|_, _| Ok(HttpResponse {
                status: 200,
                body: "invalid json".to_string(),
            }));

        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let result = client.fetch_issues(&[], "open");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parse"));
    }

    #[test]
    fn test_build_headers_with_token() {
        let mock = MockHttpClient::new();
        let client = GitHubClient::with_http_client(
            "test/repo",
            Some("test-token".to_string()),
            mock,
        );
        let headers = client.build_headers();

        assert!(headers.iter().any(|(k, v)| k == "Authorization" && v.contains("Bearer")));
        assert!(headers.iter().any(|(k, _)| k == "Accept"));
        assert!(headers.iter().any(|(k, _)| k == "User-Agent"));
    }

    #[test]
    fn test_build_headers_without_token() {
        let mock = MockHttpClient::new();
        let client = GitHubClient::with_http_client("test/repo", None, mock);
        let headers = client.build_headers();

        assert!(!headers.iter().any(|(k, _)| k == "Authorization"));
        assert!(headers.iter().any(|(k, _)| k == "Accept"));
    }

    #[test]
    fn test_http_response_struct() {
        let response = HttpResponse {
            status: 200,
            body: "test body".to_string(),
        };
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "test body");
    }
}
