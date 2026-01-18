//! Application types and state structures

use cctakt::DiffView;
use std::path::PathBuf;

/// Application mode
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Normal mode - agent PTY view
    Normal,
    /// Issue picker mode
    IssuePicker,
    /// Review and merge mode - show diff and commit log
    ReviewMerge,
    /// Theme picker mode
    ThemePicker,
    /// Build confirmation after merge
    ConfirmBuild,
    /// Task complete - shows completion summary
    TaskComplete,
}

/// Focused pane in split view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Left pane (orchestrator/interactive agent)
    Left,
    /// Right pane (worker/non-interactive agent)
    Right,
}

/// Input mode (vim-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Navigation mode - hjkl moves between panes
    Navigation,
    /// Input mode - keys are sent to the focused agent
    Input,
}

/// Review state for a completed agent
pub struct ReviewState {
    /// Agent index being reviewed
    pub agent_index: usize,
    /// Branch name
    pub branch: String,
    /// Working directory (worktree path)
    pub worktree_path: PathBuf,
    /// Diff view
    pub diff_view: DiffView,
    /// Commit log
    pub commit_log: String,
    /// Merge preview info
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    /// Potential conflicts
    pub conflicts: Vec<String>,
}

/// Merge task for the queue
pub struct MergeTask {
    /// Branch name to merge
    pub branch: String,
    /// Worktree path (for cleanup after merge)
    pub worktree_path: PathBuf,
    /// Task ID (for plan update)
    pub task_id: Option<String>,
}

/// Merge queue for sequential merge processing
pub struct MergeQueue {
    /// Pending merge tasks
    pub queue: std::collections::VecDeque<MergeTask>,
    /// Currently processing task
    pub current: Option<MergeTask>,
    /// MergeWorker agent index (None if not spawned)
    pub worker_agent_index: Option<usize>,
}

impl MergeQueue {
    pub fn new() -> Self {
        Self {
            queue: std::collections::VecDeque::new(),
            current: None,
            worker_agent_index: None,
        }
    }

    pub fn enqueue(&mut self, task: MergeTask) {
        self.queue.push_back(task);
    }

    pub fn start_next(&mut self) -> Option<&MergeTask> {
        if self.current.is_none() {
            self.current = self.queue.pop_front();
        }
        self.current.as_ref()
    }

    pub fn complete_current(&mut self) {
        self.current = None;
    }

    pub fn is_busy(&self) -> bool {
        self.current.is_some()
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len() + if self.current.is_some() { 1 } else { 0 }
    }
}

impl Default for MergeQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification message
pub struct Notification {
    pub message: String,
    pub level: cctakt::plan::NotifyLevel,
    pub created_at: std::time::Instant,
}

/// Task completion state for the TaskComplete mode
pub struct TaskCompleteState {
    /// Branch name that was merged
    pub branch: String,
    /// Whether the build was run
    pub build_run: bool,
    /// Whether the build succeeded (None if not run)
    pub build_success: Option<bool>,
    /// Summary message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_mode_equality() {
        assert_eq!(AppMode::Normal, AppMode::Normal);
        assert_eq!(AppMode::IssuePicker, AppMode::IssuePicker);
        assert_ne!(AppMode::Normal, AppMode::IssuePicker);
    }

    #[test]
    fn test_app_mode_clone() {
        let mode = AppMode::IssuePicker;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_app_mode_review_merge() {
        assert_eq!(AppMode::ReviewMerge, AppMode::ReviewMerge);
        assert_ne!(AppMode::ReviewMerge, AppMode::Normal);
        assert_ne!(AppMode::ReviewMerge, AppMode::IssuePicker);
    }

    #[test]
    fn test_review_state_creation() {
        let state = ReviewState {
            agent_index: 0,
            branch: "feature/test".to_string(),
            worktree_path: PathBuf::from("/tmp/worktree"),
            diff_view: DiffView::new("+ added line\n- removed line".to_string()),
            commit_log: "abc1234 Initial commit".to_string(),
            files_changed: 5,
            insertions: 100,
            deletions: 20,
            conflicts: vec!["src/main.rs".to_string()],
        };

        assert_eq!(state.agent_index, 0);
        assert_eq!(state.branch, "feature/test");
        assert_eq!(state.files_changed, 5);
        assert_eq!(state.insertions, 100);
        assert_eq!(state.deletions, 20);
        assert_eq!(state.conflicts.len(), 1);
    }

    #[test]
    fn test_review_state_empty_conflicts() {
        let state = ReviewState {
            agent_index: 0,
            branch: "test".to_string(),
            worktree_path: PathBuf::from("/tmp"),
            diff_view: DiffView::new(String::new()),
            commit_log: String::new(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            conflicts: vec![],
        };

        assert!(state.conflicts.is_empty());
        assert_eq!(state.files_changed, 0);
    }

    #[test]
    fn test_review_state_multiple_conflicts() {
        let state = ReviewState {
            agent_index: 1,
            branch: "feature".to_string(),
            worktree_path: PathBuf::from("/worktree"),
            diff_view: DiffView::new("diff".to_string()),
            commit_log: "log".to_string(),
            files_changed: 10,
            insertions: 500,
            deletions: 100,
            conflicts: vec![
                "file1.rs".to_string(),
                "file2.rs".to_string(),
                "file3.rs".to_string(),
            ],
        };

        assert_eq!(state.conflicts.len(), 3);
        assert_eq!(state.insertions, 500);
        assert_eq!(state.deletions, 100);
    }

    #[test]
    fn test_notification_creation() {
        let notification = Notification {
            message: "Test message".to_string(),
            level: cctakt::plan::NotifyLevel::Info,
            created_at: std::time::Instant::now(),
        };
        assert_eq!(notification.message, "Test message");
    }

    #[test]
    fn test_notification_levels() {
        let levels = [
            cctakt::plan::NotifyLevel::Info,
            cctakt::plan::NotifyLevel::Warning,
            cctakt::plan::NotifyLevel::Error,
            cctakt::plan::NotifyLevel::Success,
        ];

        for level in levels {
            let notification = Notification {
                message: "Test".to_string(),
                level,
                created_at: std::time::Instant::now(),
            };
            let _ = notification.message;
        }
    }
}
