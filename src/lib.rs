//! cctakt - Claude Code Orchestrator
//!
//! This crate provides UI components and utilities for managing
//! multiple Claude Code agent sessions.
//!
//! # Modules
//!
//! - [`worktree`] - Git worktree lifecycle management
//! - [`dialog`] - Input dialog widget for user input
//! - [`statusbar`] - Status bar for displaying agent statuses
//! - [`merge`] - Git merge operations manager
//! - [`diffview`] - Diff viewer for reviewing changes

pub mod worktree;
pub mod dialog;
pub mod diffview;
pub mod merge;
pub mod statusbar;

// Re-export commonly used types
pub use worktree::{WorktreeInfo, WorktreeManager};
pub use dialog::{DialogResult, InputDialog};
pub use diffview::DiffView;
pub use merge::{MergeManager, MergePreview};
pub use statusbar::{AgentStatusInfo, AgentStatusKind, StatusBar};
