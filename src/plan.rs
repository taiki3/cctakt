//! Execution plan management for cctakt
//!
//! Provides structured communication between the orchestrator (Claude Code in main repo)
//! and cctakt. The orchestrator writes plans to `.cctakt/plan.json`, and cctakt
//! watches and executes them.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Default plan directory
const PLAN_DIR: &str = ".cctakt";

/// Plan file name
const PLAN_FILE: &str = "plan.json";

/// Current plan schema version
const PLAN_VERSION: u32 = 1;

/// Execution plan created by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Schema version
    pub version: u32,

    /// Plan creation timestamp (Unix epoch)
    #[serde(default)]
    pub created_at: u64,

    /// Plan description
    #[serde(default)]
    pub description: Option<String>,

    /// Tasks in the plan
    pub tasks: Vec<Task>,
}

/// A task in the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,

    /// Task action type
    pub action: TaskAction,

    /// Task status
    #[serde(default)]
    pub status: TaskStatus,

    /// Error message if failed
    #[serde(default)]
    pub error: Option<String>,

    /// Timestamp when status was last updated
    #[serde(default)]
    pub updated_at: Option<u64>,

    /// Task result (populated on completion)
    #[serde(default)]
    pub result: Option<TaskResult>,
}

/// Result of a completed task
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskResult {
    /// Commits made during the task (format: "hash message")
    #[serde(default)]
    pub commits: Vec<String>,

    /// PR number if a PR was created
    #[serde(default)]
    pub pr_number: Option<u64>,

    /// PR URL if a PR was created
    #[serde(default)]
    pub pr_url: Option<String>,
}

/// Task action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskAction {
    /// Create a worktree and spawn a worker agent
    CreateWorker {
        /// Branch name for the worktree
        branch: String,
        /// Task description for the worker
        task_description: String,
        /// Base branch to create from (default: current branch)
        #[serde(default)]
        base_branch: Option<String>,
    },

    /// Create a pull request
    CreatePr {
        /// Branch to create PR from
        branch: String,
        /// PR title
        title: String,
        /// PR body
        #[serde(default)]
        body: Option<String>,
        /// Target branch (default: main)
        #[serde(default)]
        base: Option<String>,
        /// Create as draft
        #[serde(default)]
        draft: bool,
    },

    /// Merge a branch
    MergeBranch {
        /// Branch to merge
        branch: String,
        /// Target branch (default: main)
        #[serde(default)]
        target: Option<String>,
    },

    /// Clean up a worktree
    CleanupWorktree {
        /// Worktree path or branch name
        worktree: String,
    },

    /// Run a command in a worktree
    RunCommand {
        /// Worktree path or branch name
        worktree: String,
        /// Command to run
        command: String,
    },

    /// Notify/message (no action, just for logging)
    Notify {
        /// Message to display
        message: String,
        /// Message level
        #[serde(default)]
        level: NotifyLevel,
    },
}

/// Notification level
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifyLevel {
    #[default]
    Info,
    Warning,
    Error,
    Success,
}

/// Task execution status
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is waiting to be executed
    #[default]
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was skipped
    Skipped,
}

impl Plan {
    /// Create a new empty plan
    pub fn new() -> Self {
        Self {
            version: PLAN_VERSION,
            created_at: current_timestamp(),
            description: None,
            tasks: Vec::new(),
        }
    }

    /// Create a plan with description
    pub fn with_description(description: impl Into<String>) -> Self {
        Self {
            version: PLAN_VERSION,
            created_at: current_timestamp(),
            description: Some(description.into()),
            tasks: Vec::new(),
        }
    }

    /// Add a task to the plan
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Get next pending task
    pub fn next_pending(&self) -> Option<&Task> {
        self.tasks.iter().find(|t| t.status == TaskStatus::Pending)
    }

    /// Get task by ID
    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Update task status
    pub fn update_status(&mut self, id: &str, status: TaskStatus) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            task.status = status;
            task.updated_at = Some(current_timestamp());
            true
        } else {
            false
        }
    }

    /// Mark task as failed with error message
    pub fn mark_failed(&mut self, id: &str, error: impl Into<String>) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            task.status = TaskStatus::Failed;
            task.error = Some(error.into());
            task.updated_at = Some(current_timestamp());
            true
        } else {
            false
        }
    }

    /// Mark task as completed with result
    pub fn mark_completed(&mut self, id: &str, result: TaskResult) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result);
            task.updated_at = Some(current_timestamp());
            true
        } else {
            false
        }
    }

    /// Check if all tasks are completed (or failed/skipped)
    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| {
            matches!(
                t.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Skipped
            )
        })
    }

    /// Count tasks by status
    pub fn count_by_status(&self) -> (usize, usize, usize, usize) {
        let pending = self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        let running = self.tasks.iter().filter(|t| t.status == TaskStatus::Running).count();
        let completed = self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = self.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count();
        (pending, running, completed, failed)
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

impl Task {
    /// Create a new task with action
    pub fn new(id: impl Into<String>, action: TaskAction) -> Self {
        Self {
            id: id.into(),
            action,
            status: TaskStatus::Pending,
            error: None,
            updated_at: None,
            result: None,
        }
    }

    /// Create a worker creation task
    pub fn create_worker(
        id: impl Into<String>,
        branch: impl Into<String>,
        task_description: impl Into<String>,
    ) -> Self {
        Self::new(
            id,
            TaskAction::CreateWorker {
                branch: branch.into(),
                task_description: task_description.into(),
                base_branch: None,
            },
        )
    }

    /// Create a PR creation task
    pub fn create_pr(
        id: impl Into<String>,
        branch: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self::new(
            id,
            TaskAction::CreatePr {
                branch: branch.into(),
                title: title.into(),
                body: None,
                base: None,
                draft: false,
            },
        )
    }

    /// Create a notification task
    pub fn notify(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            id,
            TaskAction::Notify {
                message: message.into(),
                level: NotifyLevel::Info,
            },
        )
    }
}

/// Plan file manager
pub struct PlanManager {
    /// Plan directory path
    plan_dir: PathBuf,

    /// Last known modification time
    last_modified: Option<SystemTime>,
}

impl PlanManager {
    /// Create a new plan manager
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            plan_dir: base_dir.as_ref().join(PLAN_DIR),
            last_modified: None,
        }
    }

    /// Create plan manager for current directory
    pub fn current_dir() -> Self {
        Self::new(".")
    }

    /// Get the plan file path
    pub fn plan_file(&self) -> PathBuf {
        self.plan_dir.join(PLAN_FILE)
    }

    /// Ensure plan directory exists
    pub fn ensure_dir(&self) -> Result<()> {
        if !self.plan_dir.exists() {
            fs::create_dir_all(&self.plan_dir)
                .with_context(|| format!("Failed to create plan directory: {:?}", self.plan_dir))?;
        }
        Ok(())
    }

    /// Load plan from file
    pub fn load(&mut self) -> Result<Option<Plan>> {
        let path = self.plan_file();
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read plan file: {:?}", path))?;

        let plan: Plan = serde_json::from_str(&content)
            .with_context(|| "Failed to parse plan file")?;

        // Update last modified time
        if let Ok(metadata) = fs::metadata(&path) {
            self.last_modified = metadata.modified().ok();
        }

        Ok(Some(plan))
    }

    /// Save plan to file
    pub fn save(&mut self, plan: &Plan) -> Result<()> {
        self.ensure_dir()?;

        let path = self.plan_file();
        let content = serde_json::to_string_pretty(plan)
            .context("Failed to serialize plan")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write plan file: {:?}", path))?;

        // Update last modified time
        if let Ok(metadata) = fs::metadata(&path) {
            self.last_modified = metadata.modified().ok();
        }

        Ok(())
    }

    /// Check if plan file has been modified since last load
    pub fn has_changes(&self) -> bool {
        let path = self.plan_file();
        if !path.exists() {
            return false;
        }

        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                return self.last_modified.map_or(true, |last| modified > last);
            }
        }

        false
    }

    /// Delete plan file
    pub fn clear(&mut self) -> Result<()> {
        let path = self.plan_file();
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove plan file: {:?}", path))?;
        }
        self.last_modified = None;
        Ok(())
    }

    /// Archive current plan (move to timestamped file)
    pub fn archive(&mut self) -> Result<Option<PathBuf>> {
        let path = self.plan_file();
        if !path.exists() {
            return Ok(None);
        }

        let timestamp = current_timestamp();
        let archive_name = format!("plan_{timestamp}.json");
        let archive_path = self.plan_dir.join(archive_name);

        fs::rename(&path, &archive_path)
            .with_context(|| "Failed to archive plan file")?;

        self.last_modified = None;
        Ok(Some(archive_path))
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_plan_new() {
        let plan = Plan::new();
        assert_eq!(plan.version, PLAN_VERSION);
        assert!(plan.tasks.is_empty());
        assert!(plan.description.is_none());
    }

    #[test]
    fn test_plan_with_description() {
        let plan = Plan::with_description("Test plan");
        assert_eq!(plan.description, Some("Test plan".to_string()));
    }

    #[test]
    fn test_plan_add_task() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("task-1", "Hello"));
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, "task-1");
    }

    #[test]
    fn test_task_create_worker() {
        let task = Task::create_worker("w-1", "feat/auth", "Implement authentication");
        assert_eq!(task.id, "w-1");
        assert_eq!(task.status, TaskStatus::Pending);
        match task.action {
            TaskAction::CreateWorker { branch, task_description, .. } => {
                assert_eq!(branch, "feat/auth");
                assert_eq!(task_description, "Implement authentication");
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_task_create_pr() {
        let task = Task::create_pr("pr-1", "feat/auth", "Add authentication");
        match task.action {
            TaskAction::CreatePr { branch, title, draft, .. } => {
                assert_eq!(branch, "feat/auth");
                assert_eq!(title, "Add authentication");
                assert!(!draft);
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_plan_next_pending() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "First"));
        plan.add_task(Task::notify("t-2", "Second"));

        let next = plan.next_pending();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, "t-1");

        plan.update_status("t-1", TaskStatus::Completed);
        let next = plan.next_pending();
        assert_eq!(next.unwrap().id, "t-2");
    }

    #[test]
    fn test_plan_update_status() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));

        assert!(plan.update_status("t-1", TaskStatus::Running));
        assert_eq!(plan.get_task("t-1").unwrap().status, TaskStatus::Running);

        assert!(!plan.update_status("nonexistent", TaskStatus::Completed));
    }

    #[test]
    fn test_plan_mark_failed() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));

        plan.mark_failed("t-1", "Something went wrong");
        let task = plan.get_task("t-1").unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_plan_is_complete() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));
        plan.add_task(Task::notify("t-2", "Test"));

        assert!(!plan.is_complete());

        plan.update_status("t-1", TaskStatus::Completed);
        assert!(!plan.is_complete());

        plan.update_status("t-2", TaskStatus::Failed);
        assert!(plan.is_complete());
    }

    #[test]
    fn test_plan_count_by_status() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));
        plan.add_task(Task::notify("t-2", "Test"));
        plan.add_task(Task::notify("t-3", "Test"));

        let (pending, running, completed, failed) = plan.count_by_status();
        assert_eq!((pending, running, completed, failed), (3, 0, 0, 0));

        plan.update_status("t-1", TaskStatus::Running);
        plan.update_status("t-2", TaskStatus::Completed);

        let (pending, running, completed, failed) = plan.count_by_status();
        assert_eq!((pending, running, completed, failed), (1, 1, 1, 0));
    }

    #[test]
    fn test_plan_manager_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        let mut plan = Plan::with_description("Test");
        plan.add_task(Task::create_worker("w-1", "feat/test", "Test task"));

        manager.save(&plan).unwrap();

        let loaded = manager.load().unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.description, Some("Test".to_string()));
        assert_eq!(loaded.tasks.len(), 1);
    }

    #[test]
    fn test_plan_manager_has_changes() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        assert!(!manager.has_changes());

        let plan = Plan::new();
        manager.save(&plan).unwrap();

        // After save, should not detect changes
        assert!(!manager.has_changes());
    }

    #[test]
    fn test_plan_manager_clear() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        let plan = Plan::new();
        manager.save(&plan).unwrap();
        assert!(manager.plan_file().exists());

        manager.clear().unwrap();
        assert!(!manager.plan_file().exists());
    }

    #[test]
    fn test_task_action_serialize() {
        let action = TaskAction::CreateWorker {
            branch: "feat/test".to_string(),
            task_description: "Test".to_string(),
            base_branch: None,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"create_worker\""));
        assert!(json.contains("\"branch\":\"feat/test\""));
    }

    #[test]
    fn test_task_action_deserialize() {
        let json = r#"{
            "type": "create_pr",
            "branch": "feat/auth",
            "title": "Add auth",
            "draft": true
        }"#;

        let action: TaskAction = serde_json::from_str(json).unwrap();
        match action {
            TaskAction::CreatePr { branch, title, draft, .. } => {
                assert_eq!(branch, "feat/auth");
                assert_eq!(title, "Add auth");
                assert!(draft);
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_task_status_default() {
        let task = Task::notify("t-1", "Test");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_notify_level_default() {
        let action = TaskAction::Notify {
            message: "Test".to_string(),
            level: NotifyLevel::default(),
        };

        match action {
            TaskAction::Notify { level, .. } => {
                assert!(matches!(level, NotifyLevel::Info));
            }
            _ => panic!("Wrong action type"),
        }
    }

    // ==================== TaskResult tests ====================

    #[test]
    fn test_task_result_default() {
        let result = TaskResult::default();
        assert!(result.commits.is_empty());
        assert!(result.pr_number.is_none());
        assert!(result.pr_url.is_none());
    }

    #[test]
    fn test_task_result_with_commits() {
        let result = TaskResult {
            commits: vec![
                "abc1234 feat: add feature".to_string(),
                "def5678 fix: bug fix".to_string(),
            ],
            pr_number: None,
            pr_url: None,
        };
        assert_eq!(result.commits.len(), 2);
        assert!(result.commits[0].contains("abc1234"));
    }

    #[test]
    fn test_task_result_with_pr() {
        let result = TaskResult {
            commits: Vec::new(),
            pr_number: Some(42),
            pr_url: Some("https://github.com/owner/repo/pull/42".to_string()),
        };
        assert_eq!(result.pr_number, Some(42));
        assert!(result.pr_url.as_ref().unwrap().contains("pull/42"));
    }

    #[test]
    fn test_task_result_serialize() {
        let result = TaskResult {
            commits: vec!["abc1234 test commit".to_string()],
            pr_number: Some(123),
            pr_url: Some("https://example.com/pr/123".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"commits\""));
        assert!(json.contains("abc1234"));
        assert!(json.contains("\"pr_number\":123"));
    }

    #[test]
    fn test_task_result_deserialize() {
        let json = r#"{
            "commits": ["abc1234 first", "def5678 second"],
            "pr_number": 99,
            "pr_url": "https://github.com/test/repo/pull/99"
        }"#;
        let result: TaskResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.commits.len(), 2);
        assert_eq!(result.pr_number, Some(99));
    }

    #[test]
    fn test_task_result_deserialize_partial() {
        // Test that missing fields use defaults
        let json = r#"{"commits": ["abc123 test"]}"#;
        let result: TaskResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.commits.len(), 1);
        assert!(result.pr_number.is_none());
        assert!(result.pr_url.is_none());
    }

    #[test]
    fn test_task_result_deserialize_empty() {
        let json = "{}";
        let result: TaskResult = serde_json::from_str(json).unwrap();
        assert!(result.commits.is_empty());
    }

    // ==================== mark_completed tests ====================

    #[test]
    fn test_plan_mark_completed() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));

        let result = TaskResult {
            commits: vec!["abc123 test".to_string()],
            pr_number: None,
            pr_url: None,
        };

        assert!(plan.mark_completed("t-1", result));
        let task = plan.get_task("t-1").unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.result.is_some());
        assert_eq!(task.result.as_ref().unwrap().commits.len(), 1);
    }

    #[test]
    fn test_plan_mark_completed_nonexistent() {
        let mut plan = Plan::new();
        let result = TaskResult::default();
        assert!(!plan.mark_completed("nonexistent", result));
    }

    #[test]
    fn test_plan_mark_completed_sets_timestamp() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));

        let result = TaskResult::default();
        plan.mark_completed("t-1", result);

        let task = plan.get_task("t-1").unwrap();
        assert!(task.updated_at.is_some());
        assert!(task.updated_at.unwrap() > 0);
    }

    // ==================== Task with result serialization ====================

    #[test]
    fn test_task_with_result_serialize() {
        let mut task = Task::notify("t-1", "Test");
        task.status = TaskStatus::Completed;
        task.result = Some(TaskResult {
            commits: vec!["abc123 done".to_string()],
            pr_number: None,
            pr_url: None,
        });

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"result\""));
        assert!(json.contains("abc123"));
    }

    #[test]
    fn test_task_with_result_deserialize() {
        let json = r#"{
            "id": "t-1",
            "action": {"type": "notify", "message": "Test"},
            "status": "completed",
            "result": {
                "commits": ["abc123 test"],
                "pr_number": 42,
                "pr_url": "https://example.com/pr/42"
            }
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.id, "t-1");
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.result.is_some());
        let result = task.result.unwrap();
        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.pr_number, Some(42));
    }

    #[test]
    fn test_task_without_result_deserialize() {
        let json = r#"{
            "id": "t-1",
            "action": {"type": "notify", "message": "Test"},
            "status": "pending"
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();
        assert!(task.result.is_none());
    }

    // ==================== Archive tests ====================

    #[test]
    fn test_plan_manager_archive() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        let plan = Plan::with_description("Test plan");
        manager.save(&plan).unwrap();
        assert!(manager.plan_file().exists());

        let archive_path = manager.archive().unwrap();
        assert!(archive_path.is_some());
        assert!(!manager.plan_file().exists());

        let archive = archive_path.unwrap();
        assert!(archive.exists());
        assert!(archive.to_string_lossy().contains("plan_"));
    }

    #[test]
    fn test_plan_manager_archive_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        let result = manager.archive().unwrap();
        assert!(result.is_none());
    }

    // ==================== Additional TaskAction tests ====================

    #[test]
    fn test_task_action_merge_branch_serialize() {
        let action = TaskAction::MergeBranch {
            branch: "feat/test".to_string(),
            target: Some("develop".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"merge_branch\""));
        assert!(json.contains("\"target\":\"develop\""));
    }

    #[test]
    fn test_task_action_cleanup_worktree_serialize() {
        let action = TaskAction::CleanupWorktree {
            worktree: "feat/auth".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"cleanup_worktree\""));
        assert!(json.contains("\"worktree\":\"feat/auth\""));
    }

    #[test]
    fn test_task_action_run_command_serialize() {
        let action = TaskAction::RunCommand {
            worktree: "feat/test".to_string(),
            command: "cargo test".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"run_command\""));
        assert!(json.contains("cargo test"));
    }

    #[test]
    fn test_task_action_notify_levels() {
        let levels = [
            (NotifyLevel::Info, "info"),
            (NotifyLevel::Warning, "warning"),
            (NotifyLevel::Error, "error"),
            (NotifyLevel::Success, "success"),
        ];

        for (level, expected) in levels {
            let action = TaskAction::Notify {
                message: "Test".to_string(),
                level,
            };
            let json = serde_json::to_string(&action).unwrap();
            assert!(json.contains(expected), "Expected {} in {}", expected, json);
        }
    }

    // ==================== Plan edge cases ====================

    #[test]
    fn test_plan_empty_is_complete() {
        let plan = Plan::new();
        assert!(plan.is_complete()); // Empty plan is considered complete
    }

    #[test]
    fn test_plan_all_skipped_is_complete() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));
        plan.update_status("t-1", TaskStatus::Skipped);
        assert!(plan.is_complete());
    }

    #[test]
    fn test_plan_get_task_mut() {
        let mut plan = Plan::new();
        plan.add_task(Task::notify("t-1", "Test"));

        {
            let task = plan.get_task_mut("t-1").unwrap();
            task.error = Some("Modified".to_string());
        }

        assert_eq!(plan.get_task("t-1").unwrap().error, Some("Modified".to_string()));
    }

    #[test]
    fn test_plan_multiple_tasks_completion() {
        let mut plan = Plan::new();
        plan.add_task(Task::create_worker("w-1", "feat/a", "Task A"));
        plan.add_task(Task::create_worker("w-2", "feat/b", "Task B"));
        plan.add_task(Task::create_pr("pr-1", "feat/a", "PR A"));

        // Complete first worker
        plan.mark_completed("w-1", TaskResult {
            commits: vec!["abc123 done".to_string()],
            ..Default::default()
        });

        let (pending, running, completed, failed) = plan.count_by_status();
        assert_eq!(pending, 2);
        assert_eq!(completed, 1);

        // Complete second worker
        plan.mark_completed("w-2", TaskResult::default());

        // Fail PR creation
        plan.mark_failed("pr-1", "API error");

        assert!(plan.is_complete());
        let (_, _, completed, failed) = plan.count_by_status();
        assert_eq!(completed, 2);
        assert_eq!(failed, 1);
    }

    // ==================== PlanManager edge cases ====================

    #[test]
    fn test_plan_manager_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        let result = manager.load().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_plan_manager_plan_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PlanManager::new(temp_dir.path());

        let path = manager.plan_file();
        assert!(path.ends_with("plan.json"));
        assert!(path.to_string_lossy().contains(".cctakt"));
    }

    #[test]
    fn test_plan_manager_current_dir() {
        let manager = PlanManager::current_dir();
        let path = manager.plan_file();
        assert!(path.ends_with("plan.json"));
    }

    #[test]
    fn test_plan_manager_save_creates_dir() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        // Directory doesn't exist yet
        let plan_dir = temp_dir.path().join(".cctakt");
        assert!(!plan_dir.exists());

        // Save should create it
        let plan = Plan::new();
        manager.save(&plan).unwrap();
        assert!(plan_dir.exists());
    }

    #[test]
    fn test_plan_roundtrip_with_all_fields() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PlanManager::new(temp_dir.path());

        let mut plan = Plan::with_description("Full test");

        // Add various task types
        plan.add_task(Task::create_worker("w-1", "feat/test", "Test task"));
        plan.add_task(Task::create_pr("pr-1", "feat/test", "Test PR"));
        plan.add_task(Task::notify("n-1", "Notification"));

        // Set various statuses and results
        plan.update_status("w-1", TaskStatus::Running);
        plan.mark_completed("pr-1", TaskResult {
            commits: vec!["commit1".to_string(), "commit2".to_string()],
            pr_number: Some(100),
            pr_url: Some("https://example.com/pr/100".to_string()),
        });
        plan.mark_failed("n-1", "Test error");

        manager.save(&plan).unwrap();

        // Load and verify
        let loaded = manager.load().unwrap().unwrap();
        assert_eq!(loaded.description, Some("Full test".to_string()));
        assert_eq!(loaded.tasks.len(), 3);

        let w1 = loaded.get_task("w-1").unwrap();
        assert_eq!(w1.status, TaskStatus::Running);

        let pr1 = loaded.get_task("pr-1").unwrap();
        assert_eq!(pr1.status, TaskStatus::Completed);
        assert!(pr1.result.is_some());
        assert_eq!(pr1.result.as_ref().unwrap().pr_number, Some(100));

        let n1 = loaded.get_task("n-1").unwrap();
        assert_eq!(n1.status, TaskStatus::Failed);
        assert_eq!(n1.error, Some("Test error".to_string()));
    }
}
