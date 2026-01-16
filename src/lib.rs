//! cctakt - Claude Code Orchestrator
//!
//! This crate provides UI components and utilities for managing
//! multiple Claude Code agent sessions.
//!
//! # Modules
//!
//! - [`dialog`] - Input dialog widget for user input
//! - [`statusbar`] - Status bar for displaying agent statuses
//! - [`merge`] - Git merge operations manager
//! - [`diffview`] - Diff viewer for reviewing changes

pub mod dialog;
pub mod diffview;
pub mod merge;
pub mod statusbar;

// Re-export commonly used types
pub use dialog::{DialogResult, InputDialog};
pub use diffview::DiffView;
pub use merge::{MergeManager, MergePreview};
pub use statusbar::{AgentStatusInfo, AgentStatusKind, StatusBar};
