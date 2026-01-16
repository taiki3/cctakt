//! Task template module for cctakt
//!
//! Generates task instructions from GitHub issues using templates.

use crate::github::Issue;

/// Default task template
const DEFAULT_TEMPLATE: &str = r#"
Please work on the following GitHub issue:

## Issue #{{number}}: {{title}}

{{body}}

## Instructions
1. Read the issue carefully
2. Implement the required changes
3. Write tests if applicable
4. Commit with message referencing the issue (e.g., "Fix #{{number}}: ...")
"#;

/// Task template for generating agent instructions from issues
#[derive(Debug, Clone)]
pub struct TaskTemplate {
    /// Template string with placeholders
    template: String,
}

impl TaskTemplate {
    /// Create a new template with custom template string
    ///
    /// Available placeholders:
    /// - `{{number}}` - Issue number
    /// - `{{title}}` - Issue title
    /// - `{{body}}` - Issue body/description
    /// - `{{url}}` - Issue URL
    /// - `{{labels}}` - Comma-separated label names
    /// - `{{state}}` - Issue state (open/closed)
    pub fn new(template: &str) -> Self {
        Self {
            template: template.to_string(),
        }
    }

    /// Create template with default content
    pub fn default_template() -> Self {
        Self::new(DEFAULT_TEMPLATE)
    }

    /// Render the template with issue data
    pub fn render(&self, issue: &Issue) -> String {
        let body = issue.body.clone().unwrap_or_else(|| "(No description provided)".to_string());
        let labels = issue.label_names();

        self.template
            .replace("{{number}}", &issue.number.to_string())
            .replace("{{title}}", &issue.title)
            .replace("{{body}}", &body)
            .replace("{{url}}", &issue.html_url)
            .replace("{{labels}}", &labels)
            .replace("{{state}}", &issue.state)
    }

    /// Get the raw template string
    pub fn template_string(&self) -> &str {
        &self.template
    }
}

impl Default for TaskTemplate {
    fn default() -> Self {
        Self::default_template()
    }
}

/// Quick template for simple task generation
pub fn render_task(issue: &Issue) -> String {
    TaskTemplate::default().render(issue)
}

/// Template for commit message suggestion
pub fn suggest_commit_message(issue: &Issue) -> String {
    format!("Fix #{}: {}", issue.number, issue.title)
}

/// Template for branch name suggestion
pub fn suggest_branch_name(issue: &Issue, prefix: &str) -> String {
    let sanitized_title = issue
        .title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect::<String>();

    // Limit title length in branch name
    let max_title_len = 40;
    let truncated_title = if sanitized_title.len() > max_title_len {
        &sanitized_title[..max_title_len]
    } else {
        &sanitized_title
    };

    format!("{}/issue-{}-{}", prefix, issue.number, truncated_title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::Label;

    fn create_test_issue() -> Issue {
        Issue {
            number: 42,
            title: "Add user authentication".to_string(),
            body: Some("We need to add JWT-based authentication.\n\n- Login endpoint\n- Token refresh".to_string()),
            labels: vec![
                Label {
                    name: "feature".to_string(),
                    color: "a2eeef".to_string(),
                },
                Label {
                    name: "security".to_string(),
                    color: "d73a4a".to_string(),
                },
            ],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/42".to_string(),
        }
    }

    #[test]
    fn test_default_template() {
        let template = TaskTemplate::default();
        let issue = create_test_issue();

        let result = template.render(&issue);

        assert!(result.contains("Issue #42: Add user authentication"));
        assert!(result.contains("JWT-based authentication"));
        assert!(result.contains("Fix #42"));
    }

    #[test]
    fn test_custom_template() {
        let template = TaskTemplate::new("Task: {{title}} (Issue #{{number}})\nLabels: {{labels}}");
        let issue = create_test_issue();

        let result = template.render(&issue);

        assert_eq!(
            result,
            "Task: Add user authentication (Issue #42)\nLabels: feature, security"
        );
    }

    #[test]
    fn test_render_with_no_body() {
        let template = TaskTemplate::default();
        let issue = Issue {
            number: 1,
            title: "Test".to_string(),
            body: None,
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/1".to_string(),
        };

        let result = template.render(&issue);

        assert!(result.contains("(No description provided)"));
    }

    #[test]
    fn test_url_placeholder() {
        let template = TaskTemplate::new("See: {{url}}");
        let issue = create_test_issue();

        let result = template.render(&issue);

        assert_eq!(result, "See: https://github.com/test/repo/issues/42");
    }

    #[test]
    fn test_state_placeholder() {
        let template = TaskTemplate::new("State: {{state}}");
        let issue = create_test_issue();

        let result = template.render(&issue);

        assert_eq!(result, "State: open");
    }

    #[test]
    fn test_render_task_function() {
        let issue = create_test_issue();
        let result = render_task(&issue);

        assert!(result.contains("#42"));
        assert!(result.contains("Add user authentication"));
    }

    #[test]
    fn test_suggest_commit_message() {
        let issue = create_test_issue();
        let message = suggest_commit_message(&issue);

        assert_eq!(message, "Fix #42: Add user authentication");
    }

    #[test]
    fn test_suggest_branch_name() {
        let issue = create_test_issue();
        let branch = suggest_branch_name(&issue, "cctakt");

        assert_eq!(branch, "cctakt/issue-42-add-user-authentication");
    }

    #[test]
    fn test_branch_name_with_special_chars() {
        let issue = Issue {
            number: 123,
            title: "Fix: user@email.com validation (v2)".to_string(),
            body: None,
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/123".to_string(),
        };

        let branch = suggest_branch_name(&issue, "feature");

        // Special characters should be replaced
        assert!(branch.starts_with("feature/issue-123-"));
        assert!(!branch.contains('@'));
        assert!(!branch.contains(':'));
        assert!(!branch.contains('('));
    }

    #[test]
    fn test_branch_name_truncation() {
        let issue = Issue {
            number: 1,
            title: "This is a very long title that should be truncated to avoid extremely long branch names".to_string(),
            body: None,
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/1".to_string(),
        };

        let branch = suggest_branch_name(&issue, "fix");

        // Branch name should be reasonably short
        assert!(branch.len() < 80);
    }

    #[test]
    fn test_template_string() {
        let template = TaskTemplate::new("Hello {{title}}");
        assert_eq!(template.template_string(), "Hello {{title}}");
    }
}
