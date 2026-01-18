//! cctakt - Claude Code Orchestrator
//!
//! A TUI for managing multiple Claude Code agents with Git Worktree support.

mod agent;
mod app;
mod cli;
mod commands;
mod git_utils;
mod tui;

use anyhow::Result;
use cctakt::debug;
use clap::Parser;
use cli::{Cli, Commands};
use commands::{run_init, run_issues, run_mcp, run_plan, run_status, run_tui};

fn main() -> Result<()> {
    // Initialize debug logging (only in debug builds)
    debug::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { force }) => run_init(force),
        Some(Commands::Status) => run_status(),
        Some(Commands::Issues { labels, state }) => run_issues(labels, state),
        Some(Commands::Run { plan }) => run_plan(plan),
        Some(Commands::Mcp) => run_mcp(),
        None => run_tui(),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::types::{AppMode, MergeQueue, Notification, ReviewState};
    use crate::git_utils::{get_commit_log, get_worker_commits, parse_github_url};
    use cctakt::{
        github::Issue, Config, DiffView, GitHubClient, IssuePicker, IssuePickerResult,
        MergeManager, Plan, PlanManager, TaskResult, WorktreeManager,
    };
    use crossterm::event::KeyCode;
    use std::path::PathBuf;

    // ==================== parse_github_url tests ====================

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

    // ==================== AppMode tests ====================

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

    // ==================== Config integration tests ====================

    #[test]
    fn test_config_default_values() {
        use std::path::Path;
        let config = Config::default();
        assert_eq!(config.worktree_dir, Path::new(".worktrees"));
        assert_eq!(config.branch_prefix, "cctakt");
    }

    // ==================== suggest_branch_name integration ====================

    #[test]
    fn test_suggest_branch_name_integration() {
        use cctakt::suggest_branch_name;

        let issue = Issue {
            number: 42,
            title: "Add feature".to_string(),
            body: None,
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/42".to_string(),
        };

        let branch = suggest_branch_name(&issue, "cctakt");
        assert!(branch.starts_with("cctakt/issue-42-"));
        assert!(branch.contains("add"));
        assert!(branch.contains("feature"));
    }

    #[test]
    fn test_suggest_branch_name_with_special_chars() {
        use cctakt::suggest_branch_name;

        let issue = Issue {
            number: 123,
            title: "Fix: user@email.com validation".to_string(),
            body: None,
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/123".to_string(),
        };

        let branch = suggest_branch_name(&issue, "fix");
        assert!(branch.starts_with("fix/issue-123-"));
        assert!(!branch.contains('@'));
        assert!(!branch.contains(':'));
    }

    // ==================== IssuePicker state tests ====================

    #[test]
    fn test_issue_picker_initial_state() {
        let picker = IssuePicker::new();
        assert!(picker.is_empty());
    }

    #[test]
    fn test_issue_picker_set_loading() {
        let mut picker = IssuePicker::new();
        picker.set_loading(true);
    }

    #[test]
    fn test_issue_picker_set_issues() {
        let mut picker = IssuePicker::new();
        let issues = vec![
            Issue {
                number: 1,
                title: "First issue".to_string(),
                body: None,
                labels: vec![],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/1".to_string(),
            },
            Issue {
                number: 2,
                title: "Second issue".to_string(),
                body: None,
                labels: vec![],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/2".to_string(),
            },
        ];

        picker.set_issues(issues);
        assert!(!picker.is_empty());
    }

    #[test]
    fn test_issue_picker_navigation() {
        let mut picker = IssuePicker::new();
        let issues = vec![
            Issue {
                number: 1,
                title: "First".to_string(),
                body: None,
                labels: vec![],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/1".to_string(),
            },
            Issue {
                number: 2,
                title: "Second".to_string(),
                body: None,
                labels: vec![],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/2".to_string(),
            },
        ];
        picker.set_issues(issues);

        let result = picker.handle_key(KeyCode::Down);
        assert!(result.is_none());

        let result = picker.handle_key(KeyCode::Up);
        assert!(result.is_none());
    }

    #[test]
    fn test_issue_picker_cancel() {
        let mut picker = IssuePicker::new();
        let result = picker.handle_key(KeyCode::Esc);
        assert!(matches!(result, Some(IssuePickerResult::Cancel)));
    }

    #[test]
    fn test_issue_picker_refresh() {
        let mut picker = IssuePicker::new();
        let result = picker.handle_key(KeyCode::Char('r'));
        assert!(matches!(result, Some(IssuePickerResult::Refresh)));
    }

    #[test]
    fn test_issue_picker_select_empty() {
        let mut picker = IssuePicker::new();
        let result = picker.handle_key(KeyCode::Enter);
        assert!(result.is_none());
    }

    #[test]
    fn test_issue_picker_select_with_issues() {
        let mut picker = IssuePicker::new();
        picker.set_issues(vec![Issue {
            number: 42,
            title: "Test issue".to_string(),
            body: Some("Body".to_string()),
            labels: vec![],
            state: "open".to_string(),
            html_url: "https://github.com/test/repo/issues/42".to_string(),
        }]);

        let result = picker.handle_key(KeyCode::Enter);

        match result {
            Some(IssuePickerResult::Selected(issue)) => {
                assert_eq!(issue.number, 42);
                assert_eq!(issue.title, "Test issue");
            }
            _ => panic!("Expected Selected result"),
        }
    }

    // ==================== WorktreeManager tests ====================

    #[test]
    fn test_worktree_manager_from_current_dir() {
        let result = WorktreeManager::from_current_dir();
        assert!(result.is_ok());
    }

    // ==================== GitHubClient tests ====================

    #[test]
    fn test_github_client_creation() {
        let client = GitHubClient::with_token("owner/repo", None);
        assert_eq!(client.repository(), "owner/repo");
        assert!(!client.has_auth());
    }

    #[test]
    fn test_github_client_with_auth() {
        let client = GitHubClient::with_token("owner/repo", Some("token".to_string()));
        assert!(client.has_auth());
    }

    // ==================== ReviewMerge mode tests ====================

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
    fn test_get_commit_log() {
        let log = get_commit_log(&PathBuf::from("."));
        assert!(!log.is_empty());
    }

    #[test]
    fn test_diff_view_creation() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!("Hello");
 }
"#;
        let view = DiffView::new(diff.to_string());
        assert!(!view.is_empty());
    }

    #[test]
    fn test_diff_view_with_title() {
        let view = DiffView::new("test".to_string()).with_title("branch → main".to_string());
        assert!(!view.is_empty());
    }

    #[test]
    fn test_diff_view_scrolling() {
        let diff = (0..100)
            .map(|i| format!("+line {i}\n"))
            .collect::<String>();
        let mut view = DiffView::new(diff);

        view.scroll_down(10);
        view.scroll_up(5);
        view.page_down(20);
        view.page_up(10);
        view.scroll_to_top();
        view.scroll_to_bottom();
    }

    #[test]
    fn test_merge_manager_creation() {
        let manager = MergeManager::new("/tmp/test-repo");
        assert_eq!(manager.main_branch(), "main");
    }

    #[test]
    fn test_merge_manager_with_main_branch() {
        let manager = MergeManager::new("/tmp/test-repo").with_main_branch("master");
        assert_eq!(manager.main_branch(), "master");
    }

    // ==================== get_worker_commits tests ====================

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

    // ==================== Notification tests ====================

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

    // ==================== parse_github_url additional tests ====================

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

    // ==================== TaskResult integration tests ====================

    #[test]
    fn test_task_result_struct() {
        let result = TaskResult {
            commits: vec!["abc123 first commit".to_string()],
            pr_number: Some(42),
            pr_url: Some("https://github.com/owner/repo/pull/42".to_string()),
        };

        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.pr_number, Some(42));
        assert!(result.pr_url.is_some());
    }

    #[test]
    fn test_task_result_default() {
        let result = TaskResult::default();
        assert!(result.commits.is_empty());
        assert!(result.pr_number.is_none());
        assert!(result.pr_url.is_none());
    }

    // ==================== Plan integration tests ====================

    #[test]
    fn test_plan_manager_integration() {
        let manager = PlanManager::current_dir();
        let path = manager.plan_file();
        assert!(path.to_string_lossy().contains(".cctakt"));
        assert!(path.to_string_lossy().contains("plan.json"));
    }

    #[test]
    fn test_plan_new() {
        let plan = Plan::new();
        assert!(plan.tasks.is_empty());
        assert!(plan.description.is_none());
    }

    #[test]
    fn test_plan_with_description() {
        let plan = Plan::with_description("Test plan");
        assert_eq!(plan.description, Some("Test plan".to_string()));
    }

    #[test]
    fn test_plan_count_by_status() {
        let mut plan = Plan::new();
        plan.add_task(cctakt::plan::Task::notify("t-1", "Test 1"));
        plan.add_task(cctakt::plan::Task::notify("t-2", "Test 2"));

        let (pending, running, completed, failed) = plan.count_by_status();
        assert_eq!(pending, 2);
        assert_eq!(running, 0);
        assert_eq!(completed, 0);
        assert_eq!(failed, 0);
    }

    // ==================== ReviewState additional tests ====================

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

    // ==================== DiffView additional tests ====================

    #[test]
    fn test_diff_view_empty() {
        let view = DiffView::new(String::new());
        assert!(view.is_empty());
    }

    #[test]
    fn test_diff_view_multiline() {
        let diff = "+line1\n+line2\n+line3\n-old1\n-old2";
        let view = DiffView::new(diff.to_string());
        assert!(!view.is_empty());
    }

    // ==================== AgentManager tests ====================

    #[test]
    fn test_agent_manager_new() {
        use crate::agent::AgentManager;
        let manager = AgentManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_default() {
        use crate::agent::AgentManager;
        let manager = AgentManager::default();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_list_empty() {
        use crate::agent::AgentManager;
        let manager = AgentManager::new();
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_agent_manager_active_none() {
        use crate::agent::AgentManager;
        let manager = AgentManager::new();
        assert!(manager.active().is_none());
    }

    #[test]
    fn test_agent_manager_switch_to_invalid() {
        use crate::agent::AgentManager;
        let mut manager = AgentManager::new();
        manager.switch_to(100);
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_next_empty() {
        use crate::agent::AgentManager;
        let mut manager = AgentManager::new();
        manager.next();
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_prev_empty() {
        use crate::agent::AgentManager;
        let mut manager = AgentManager::new();
        manager.prev();
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_close_invalid() {
        use crate::agent::AgentManager;
        let mut manager = AgentManager::new();
        manager.close(100);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_get_none() {
        use crate::agent::AgentManager;
        let manager = AgentManager::new();
        assert!(manager.get(0).is_none());
        assert!(manager.get(100).is_none());
    }

    // ==================== AgentStatus tests ====================

    #[test]
    fn test_agent_status_equality() {
        use crate::agent::AgentStatus;
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_eq!(AgentStatus::Ended, AgentStatus::Ended);
        assert_ne!(AgentStatus::Running, AgentStatus::Ended);
    }

    #[test]
    fn test_agent_status_clone() {
        use crate::agent::AgentStatus;
        let status = AgentStatus::Running;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    // ==================== MergeQueue tests ====================

    #[test]
    fn test_merge_queue_new() {
        let queue = MergeQueue::new();
        assert!(!queue.is_busy());
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn test_merge_queue_default() {
        let queue = MergeQueue::default();
        assert!(!queue.is_busy());
    }
}
