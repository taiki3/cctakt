//! cctakt - Claude Code Terminal Agile Kit for Tasks
//!
//! Advanced features module containing GitHub integration, configuration,
//! and task management utilities.

pub mod config;
pub mod github;
pub mod issue_picker;
pub mod template;

// Re-export commonly used types
pub use config::{Config, GitHubConfig, KeyBindings};
pub use github::{GitHubClient, Issue, Label};
pub use issue_picker::{IssuePicker, IssuePickerResult};
pub use template::{TaskTemplate, render_task, suggest_branch_name, suggest_commit_message};
