//! cctakt - Claude Code Orchestrator
//!
//! This crate provides UI components and utilities for managing
//! multiple Claude Code agent sessions.
//!
//! # Modules
//!
//! ## Core
//! - [`worktree`] - Git worktree lifecycle management
//!
//! ## UI Components
//! - [`dialog`] - Input dialog widget for user input
//! - [`statusbar`] - Status bar for displaying agent statuses
//! - [`diffview`] - Diff viewer for reviewing changes
//! - [`issue_picker`] - GitHub issue selection UI
//!
//! ## Git Operations
//! - [`merge`] - Git merge operations manager
//!
//! ## GitHub Integration
//! - [`github`] - GitHub API client
//! - [`config`] - Configuration file support
//! - [`template`] - Task template generation

// Core
pub mod worktree;

// UI Components
pub mod dialog;
pub mod statusbar;
pub mod diffview;
pub mod issue_picker;

// Git Operations
pub mod merge;

// GitHub Integration
pub mod config;
pub mod github;
pub mod template;

// Re-export commonly used types
pub use worktree::{WorktreeInfo, WorktreeManager};
pub use dialog::{DialogResult, InputDialog};
pub use diffview::DiffView;
pub use merge::{MergeManager, MergePreview};
pub use statusbar::{AgentStatusInfo, AgentStatusKind, StatusBar};
pub use config::{Config, GitHubConfig, KeyBindings};
pub use github::{GitHubClient, Issue, Label};
pub use issue_picker::{IssuePicker, IssuePickerResult};
pub use template::{TaskTemplate, render_task, suggest_branch_name, suggest_commit_message};
