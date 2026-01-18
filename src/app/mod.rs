//! Application state and logic

pub mod types;

pub use types::{AppMode, FocusedPane, InputMode, MergeQueue, MergeTask, Notification, ReviewState};

use crate::agent::{AgentManager, AgentStatus};
use crate::git_utils::{detect_github_repo, get_commit_log, get_worker_commits};
use anyhow::{Context, Result};
use cctakt::{
    available_themes, create_theme, current_theme_id, debug, render_task, set_theme,
    Config, DiffView, GitHubClient, Issue, IssuePicker, MergeManager, Plan, PlanManager,
    suggest_branch_name, TaskAction, TaskResult, TaskStatus, WorktreeManager,
};
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Application state
pub struct App {
    pub agent_manager: AgentManager,
    pub should_quit: bool,
    pub content_rows: u16,
    pub content_cols: u16,
    /// Current application mode
    pub mode: AppMode,
    /// Focused pane in split view
    pub focused_pane: FocusedPane,
    /// Input mode (Navigation or Input)
    pub input_mode: InputMode,
    /// Configuration
    pub config: Config,
    /// Worktree manager
    pub worktree_manager: Option<WorktreeManager>,
    /// GitHub client
    pub github_client: Option<GitHubClient>,
    /// Issue picker UI
    pub issue_picker: IssuePicker,
    /// Current issue being worked on (per agent)
    pub agent_issues: Vec<Option<Issue>>,
    /// Worktree paths per agent
    pub agent_worktrees: Vec<Option<PathBuf>>,
    /// Review state for merge review mode
    pub review_state: Option<ReviewState>,
    /// Plan manager for orchestrator communication
    pub plan_manager: PlanManager,
    /// Current plan being executed
    pub current_plan: Option<Plan>,
    /// Task ID to agent index mapping
    pub task_agents: std::collections::HashMap<String, usize>,
    /// Notifications to display
    pub notifications: Vec<Notification>,
    /// Pending prompt to send to agent after it initializes (unused in non-interactive mode)
    pub pending_agent_prompt: Option<String>,
    /// Frame counter for delayed prompt sending (unused in non-interactive mode)
    pub prompt_delay_frames: u32,
    /// Pending review task ID to mark as completed after merge
    pub pending_review_task_id: Option<String>,
    /// Merge queue for sequential merge processing
    pub merge_queue: MergeQueue,
    /// Theme picker: show picker modal
    pub show_theme_picker: bool,
    /// Theme picker: currently selected index
    pub theme_picker_index: usize,
    /// BuildWorker agent index (None if not spawned)
    pub build_worker_index: Option<usize>,
    /// Branch name associated with the current build worker
    pub build_worker_branch: Option<String>,
}

impl App {
    pub fn new(rows: u16, cols: u16, config: Config) -> Self {
        // Initialize worktree manager
        let worktree_manager = WorktreeManager::from_current_dir().ok();

        // Initialize GitHub client if repository is configured
        let github_client = config
            .github
            .repository
            .as_ref()
            .and_then(|repo| GitHubClient::new(repo).ok());

        Self {
            agent_manager: AgentManager::new(),
            should_quit: false,
            content_rows: rows,
            content_cols: cols,
            mode: AppMode::Normal,
            focused_pane: FocusedPane::Right, // Default to worker pane
            input_mode: InputMode::Input,     // Default to input mode
            config,
            worktree_manager,
            github_client,
            issue_picker: IssuePicker::new(),
            agent_issues: Vec::new(),
            agent_worktrees: Vec::new(),
            review_state: None,
            plan_manager: PlanManager::current_dir(),
            current_plan: None,
            task_agents: std::collections::HashMap::new(),
            notifications: Vec::new(),
            pending_agent_prompt: None,
            prompt_delay_frames: 0,
            pending_review_task_id: None,
            merge_queue: MergeQueue::new(),
            show_theme_picker: false,
            theme_picker_index: 0,
            build_worker_index: None,
            build_worker_branch: None,
        }
    }

    /// Open issue picker and fetch issues
    pub fn open_issue_picker(&mut self) {
        if self.github_client.is_none() {
            // Try to detect repository from git remote
            if let Some(repo) = detect_github_repo() {
                self.github_client = GitHubClient::new(&repo).ok();
            }
        }

        if self.github_client.is_some() {
            self.mode = AppMode::IssuePicker;
            self.add_notification(
                "Opening issue picker...".to_string(),
                cctakt::plan::NotifyLevel::Info,
            );
            self.fetch_issues();
        } else {
            self.add_notification(
                "GitHub repository not configured. Set 'repository' in cctakt.toml or add a git remote.".to_string(),
                cctakt::plan::NotifyLevel::Warning,
            );
        }
    }

    /// Open theme picker
    pub fn open_theme_picker(&mut self) {
        // Set index to current theme
        let current = current_theme_id().id();
        let themes = available_themes();
        self.theme_picker_index = themes
            .iter()
            .position(|(id, _, _)| *id == current)
            .unwrap_or(0);
        self.show_theme_picker = true;
        self.mode = AppMode::ThemePicker;
    }

    /// Apply selected theme and save to config
    pub fn apply_theme(&mut self, theme_id: &str) {
        // Set the theme
        set_theme(create_theme(theme_id));

        // Update config
        self.config.theme = theme_id.to_string();

        // Save config to file
        if let Err(e) = self.config.save() {
            self.add_notification(
                format!("Failed to save theme: {e}"),
                cctakt::plan::NotifyLevel::Warning,
            );
        } else {
            let themes = available_themes();
            let name = themes
                .iter()
                .find(|(id, _, _)| *id == theme_id)
                .map(|(_, name, _)| *name)
                .unwrap_or(theme_id);
            self.add_notification(
                format!("Theme changed to {name}"),
                cctakt::plan::NotifyLevel::Success,
            );
        }
    }

    /// Fetch issues from GitHub
    pub fn fetch_issues(&mut self) {
        self.issue_picker.set_loading(true);

        if let Some(ref client) = self.github_client {
            let labels: Vec<&str> = self
                .config
                .github
                .labels
                .iter()
                .map(|s| s.as_str())
                .collect();

            match client.fetch_issues(&labels, "open") {
                Ok(issues) => {
                    let count = issues.len();
                    self.issue_picker.set_issues(issues);
                    self.issue_picker.set_loading(false);
                    if count == 0 {
                        self.add_notification(
                            "No open issues found in repository.".to_string(),
                            cctakt::plan::NotifyLevel::Info,
                        );
                    }
                }
                Err(e) => {
                    self.issue_picker.set_error(Some(e.to_string()));
                    self.add_notification(
                        format!("Failed to fetch issues: {e}"),
                        cctakt::plan::NotifyLevel::Error,
                    );
                }
            }
        }
    }

    /// Add a new agent from a selected issue
    pub fn add_agent_from_issue(&mut self, issue: Issue) -> Result<()> {
        let branch_name = suggest_branch_name(&issue, &self.config.branch_prefix);

        // Create worktree if available
        let (working_dir, worktree_path) = if let Some(ref wt_manager) = self.worktree_manager {
            match wt_manager.create(&branch_name, &self.config.worktree_dir) {
                Ok(path) => (path.clone(), Some(path)),
                Err(_) => (
                    env::current_dir().context("Failed to get current directory")?,
                    None,
                ),
            }
        } else {
            (
                env::current_dir().context("Failed to get current directory")?,
                None,
            )
        };

        // Generate task prompt from issue
        let task_prompt = render_task(&issue);

        let name = format!("#{}", issue.number);
        self.agent_manager
            .add_non_interactive(name, working_dir, &task_prompt, None)?;

        self.agent_issues.push(Some(issue));
        self.agent_worktrees.push(worktree_path);

        // Update PTY sizes for pane split
        self.update_agent_sizes();

        Ok(())
    }

    /// Add a new agent with the current directory (interactive mode for orchestrator)
    pub fn add_agent(&mut self) -> Result<()> {
        let working_dir = env::current_dir().context("Failed to get current directory")?;
        let name = working_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let agent_count = self.agent_manager.list().len();
        let display_name = if agent_count == 0 {
            name
        } else {
            format!("{}-{}", name, agent_count + 1)
        };

        // Use interactive mode (PTY) for manual agent creation
        self.agent_manager
            .add(display_name, working_dir, self.content_rows, self.content_cols)?;
        self.agent_issues.push(None);
        self.agent_worktrees.push(None);
        Ok(())
    }

    /// Close the active agent
    pub fn close_active_agent(&mut self) {
        let index = self.agent_manager.active_index();
        self.agent_manager.close(index);
        if index < self.agent_issues.len() {
            self.agent_issues.remove(index);
        }
        if index < self.agent_worktrees.len() {
            self.agent_worktrees.remove(index);
        }
        // Update PTY sizes after closing (e.g., restore full width)
        self.update_agent_sizes();
    }

    /// Check all agents for completion and auto-transition to review mode
    pub fn check_agent_completion(&mut self) {
        use std::time::Duration;

        // Don't check if already in review mode
        if self.mode == AppMode::ReviewMerge {
            return;
        }

        let idle_threshold = Duration::from_secs(5); // 5 seconds idle = potentially done

        // First pass: find agent that just completed
        let mut completed_agent: Option<(usize, String)> = None;
        for i in 0..self.agent_manager.list().len() {
            if let Some(agent) = self.agent_manager.get_mut(i) {
                if agent.update_work_state(idle_threshold) {
                    completed_agent = Some((i, agent.name.clone()));
                    break;
                }
            }
        }

        // Second pass: handle completion (separate borrow)
        if let Some((index, name)) = completed_agent {
            self.add_notification(
                format!("Agent '{name}' completed work. Starting review..."),
                cctakt::plan::NotifyLevel::Success,
            );

            // Auto-start review for this agent
            self.agent_manager.switch_to(index);
            self.start_review(index);
        }
    }

    /// Start review mode for the agent at given index
    pub fn start_review(&mut self, agent_index: usize) {
        // Get worktree path for this agent
        let worktree_path = if agent_index < self.agent_worktrees.len() {
            self.agent_worktrees[agent_index].clone()
        } else {
            None
        };

        let Some(worktree_path) = worktree_path else {
            // No worktree, can't review
            return;
        };

        // Get actual branch name from git (not directory name which has / replaced with -)
        let branch = Command::new("git")
            .current_dir(&worktree_path)
            .args(["branch", "--show-current"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Get main repo path
        let repo_path = env::current_dir().unwrap_or_default();
        let merger = MergeManager::new(&repo_path);

        // Get diff
        let diff = merger.diff(&branch).unwrap_or_default();

        // Get commit log
        let commit_log = get_commit_log(&worktree_path);

        // Get merge preview
        let preview = merger.preview(&branch).ok();
        let (files_changed, insertions, deletions, conflicts) = match preview {
            Some(p) => (p.files_changed, p.insertions, p.deletions, p.conflicts),
            None => (0, 0, 0, vec![]),
        };

        // Create diff view
        let diff_view = DiffView::new(diff).with_title(format!("{branch} → main"));

        self.review_state = Some(ReviewState {
            agent_index,
            branch,
            worktree_path,
            diff_view,
            commit_log,
            files_changed,
            insertions,
            deletions,
            conflicts,
        });

        self.mode = AppMode::ReviewMerge;
    }

    /// Enqueue merge task and start MergeWorker if needed
    pub fn enqueue_merge(&mut self) {
        let review = self.review_state.take();
        let Some(review) = review else {
            self.mode = AppMode::Normal;
            return;
        };

        // Close the worker agent (the implementation tab disappears)
        if review.agent_index != usize::MAX {
            self.agent_manager.close(review.agent_index);
            if review.agent_index < self.agent_issues.len() {
                self.agent_issues.remove(review.agent_index);
            }
            if review.agent_index < self.agent_worktrees.len() {
                self.agent_worktrees.remove(review.agent_index);
            }
            // Update PTY sizes after closing worker
            self.update_agent_sizes();
        }

        let task = MergeTask {
            branch: review.branch.clone(),
            worktree_path: review.worktree_path.clone(),
            task_id: self.pending_review_task_id.take(),
        };

        let pending_count = self.merge_queue.pending_count();
        self.merge_queue.enqueue(task);

        self.add_notification(
            format!(
                "Merge queued: {} (pending: {})",
                review.branch,
                pending_count + 1
            ),
            cctakt::plan::NotifyLevel::Info,
        );

        self.mode = AppMode::Normal;

        // Start processing if not already busy
        self.process_merge_queue();
    }

    /// Process the next merge task in queue
    pub fn process_merge_queue(&mut self) {
        // Skip if already processing
        if self.merge_queue.is_busy() {
            return;
        }

        // Get next task and clone branch name to avoid borrow issues
        let branch = match self.merge_queue.start_next() {
            Some(task) => task.branch.clone(),
            None => return,
        };

        // Spawn MergeWorker
        self.spawn_merge_worker(&branch);
    }

    /// Spawn MergeWorker to execute merge
    fn spawn_merge_worker(&mut self, branch: &str) {
        let repo_path = match env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                self.add_notification(
                    format!("Failed to get current directory: {e}"),
                    cctakt::plan::NotifyLevel::Error,
                );
                self.merge_queue.complete_current();
                return;
            }
        };

        let task_description = format!(
            "mainブランチに {} をマージしてください。\n\n\
             手順:\n\
             1. git checkout main\n\
             2. git pull origin main (最新を取得)\n\
             3. git merge --no-ff {}\n\
             4. コンフリクトがあれば解決してコミット\n\n\
             重要: マージコミットを必ず作成してください。",
            branch, branch
        );

        match self.agent_manager.add_non_interactive(
            "merge-worker".to_string(),
            repo_path,
            &task_description,
            Some(10), // max_turns: enough for conflict resolution
        ) {
            Ok(agent_id) => {
                // Find the agent index (it's the last one added)
                let agent_index = self.agent_manager.len() - 1;
                self.merge_queue.worker_agent_index = Some(agent_index);
                // Update PTY sizes for pane split
                self.update_agent_sizes();
                self.add_notification(
                    format!("MergeWorker started (agent {})", agent_id),
                    cctakt::plan::NotifyLevel::Info,
                );
            }
            Err(e) => {
                self.add_notification(
                    format!("Failed to start MergeWorker: {e}"),
                    cctakt::plan::NotifyLevel::Error,
                );
                self.merge_queue.complete_current();
            }
        }
    }

    /// Check MergeWorker completion and handle result
    pub fn check_merge_worker_completion(&mut self) {
        let Some(worker_idx) = self.merge_queue.worker_agent_index else {
            return;
        };

        let Some(agent) = self.agent_manager.get(worker_idx) else {
            return;
        };

        if agent.status != AgentStatus::Ended {
            return;
        }

        // Get current task info before processing
        let task = match self.merge_queue.current.take() {
            Some(t) => t,
            None => return,
        };

        // Check merge result by looking at git log
        let repo_path = match env::current_dir() {
            Ok(p) => p,
            Err(_) => {
                self.handle_merge_failure(&task);
                self.merge_queue.worker_agent_index = None;
                self.process_merge_queue();
                return;
            }
        };

        // Check if branch was merged by looking for the branch in git log
        let merged = std::process::Command::new("git")
            .args([
                "log",
                "--oneline",
                "-1",
                "--grep",
                &format!("Merge branch '{}'", task.branch),
            ])
            .current_dir(&repo_path)
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

        if merged {
            self.handle_merge_success(&task);
        } else {
            self.handle_merge_failure(&task);
        }

        // Close MergeWorker agent
        self.agent_manager.close(worker_idx);
        self.merge_queue.worker_agent_index = None;
        // Update PTY sizes after closing worker
        self.update_agent_sizes();

        // Process next task
        self.process_merge_queue();
    }

    /// Handle successful merge
    fn handle_merge_success(&mut self, task: &MergeTask) {
        self.add_notification(
            format!("Merged: {} → main", task.branch),
            cctakt::plan::NotifyLevel::Success,
        );

        // Remove worktree
        if let Some(ref wt_manager) = self.worktree_manager {
            let _ = wt_manager.remove(&task.worktree_path);
        }

        // Note: Worker agent is already closed in enqueue_merge()

        // Mark task as completed
        if let Some(ref task_id) = task.task_id {
            if let Some(ref mut plan) = self.current_plan {
                plan.update_status(task_id, TaskStatus::Completed);
                let _ = self.plan_manager.save(plan);
            }
        }

        // Automatically run build (no confirmation dialog)
        self.spawn_build_worker(task.branch.clone());
    }

    /// Handle failed merge
    fn handle_merge_failure(&mut self, task: &MergeTask) {
        self.add_notification(
            format!(
                "Merge failed: {} (MergeWorker could not complete)",
                task.branch
            ),
            cctakt::plan::NotifyLevel::Error,
        );

        // Mark task as failed
        if let Some(ref task_id) = task.task_id {
            if let Some(ref mut plan) = self.current_plan {
                plan.mark_failed(task_id, "MergeWorker could not complete merge");
                let _ = self.plan_manager.save(plan);
            }
        }
    }

    /// Spawn BuildWorker to run cargo build after merge
    pub fn spawn_build_worker(&mut self, branch: String) {
        let repo_path = match env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                self.add_notification(
                    format!("Failed to get current directory: {e}"),
                    cctakt::plan::NotifyLevel::Error,
                );
                return;
            }
        };

        let task_description = "マージ後のビルドチェックを実行してください。\n\n\
             手順:\n\
             1. cargo build を実行\n\
             2. エラーがあれば修正してコミット\n\
             3. cargo test を実行（オプション）\n\n\
             ビルドが成功したら完了です。"
            .to_string();

        match self.agent_manager.add_non_interactive(
            "build-worker".to_string(),
            repo_path,
            &task_description,
            Some(15), // max_turns: enough for build fixes
        ) {
            Ok(agent_id) => {
                let agent_index = self.agent_manager.len() - 1;
                self.build_worker_index = Some(agent_index);
                self.build_worker_branch = Some(branch);
                // Update PTY sizes for pane split
                self.update_agent_sizes();
                self.add_notification(
                    format!("BuildWorker started (agent {})", agent_id),
                    cctakt::plan::NotifyLevel::Info,
                );
            }
            Err(e) => {
                self.add_notification(
                    format!("Failed to start BuildWorker: {e}"),
                    cctakt::plan::NotifyLevel::Error,
                );
            }
        }
    }

    /// Check BuildWorker completion and show notification (no popup)
    pub fn check_build_worker_completion(&mut self) {
        let Some(worker_idx) = self.build_worker_index else {
            return;
        };

        let Some(agent) = self.agent_manager.get(worker_idx) else {
            return;
        };

        if agent.status != AgentStatus::Ended {
            return;
        }

        // Check if build succeeded (by error presence)
        let build_success = agent.error.is_none();

        // Get the branch name
        let branch = self.build_worker_branch.take().unwrap_or_else(|| "unknown".to_string());

        // Close BuildWorker agent
        self.agent_manager.close(worker_idx);
        self.build_worker_index = None;
        // Update PTY sizes after closing worker
        self.update_agent_sizes();

        // Show notification instead of popup
        if build_success {
            self.add_notification(
                format!("Build succeeded: {}", branch),
                cctakt::plan::NotifyLevel::Success,
            );
        } else {
            self.add_notification(
                format!("Build failed: {}", branch),
                cctakt::plan::NotifyLevel::Error,
            );
        }
    }

    /// Cancel review and return to normal mode
    pub fn cancel_review(&mut self) {
        self.review_state = None;
        self.mode = AppMode::Normal;
    }

    /// Check for plan file changes and load
    pub fn check_plan(&mut self) {
        if self.plan_manager.has_changes() {
            match self.plan_manager.load() {
                Ok(Some(plan)) => {
                    // 完了済みプランはクリア（通知なし）
                    if plan.is_complete() {
                        self.current_plan = None;
                    } else {
                        if let Some(desc) = &plan.description {
                            self.add_notification(
                                format!("Plan loaded: {desc}"),
                                cctakt::plan::NotifyLevel::Info,
                            );
                        }
                        self.current_plan = Some(plan);
                    }
                }
                Ok(None) => {
                    self.current_plan = None;
                }
                Err(e) => {
                    self.add_notification(
                        format!("Failed to load plan: {e}"),
                        cctakt::plan::NotifyLevel::Error,
                    );
                }
            }
        }
    }

    /// Process pending tasks in the current plan
    pub fn process_plan(&mut self) {
        // First, recover orphaned running tasks (no corresponding agent)
        self.recover_orphaned_tasks();

        // Get next pending task (clone to avoid borrow issues)
        let next_task = self
            .current_plan
            .as_ref()
            .and_then(|p| p.next_pending())
            .cloned();

        if let Some(task) = next_task {
            self.execute_task(&task.id.clone());
        }

        // Save plan if we have changes
        if let Some(ref plan) = self.current_plan {
            let _ = self.plan_manager.save(plan);
        }
    }

    /// Recover orphaned running tasks (tasks marked running but no agent exists)
    fn recover_orphaned_tasks(&mut self) {
        // Find running tasks without corresponding agents
        let orphaned: Vec<String> = self
            .current_plan
            .as_ref()
            .map(|plan| {
                plan.tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Running)
                    .filter(|t| !self.task_agents.contains_key(&t.id))
                    .map(|t| t.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Reset orphaned tasks to pending and collect notifications
        let notifications: Vec<String> = orphaned
            .iter()
            .filter_map(|task_id| {
                if let Some(ref mut plan) = self.current_plan {
                    plan.update_status(task_id, TaskStatus::Pending);
                    Some(format!("Recovered orphaned task: {task_id}"))
                } else {
                    None
                }
            })
            .collect();

        // Persist changes if any orphaned tasks were recovered
        if !notifications.is_empty() {
            self.save_plan();
        }

        // Add notifications after releasing plan borrow
        for msg in notifications {
            self.add_notification(msg, cctakt::plan::NotifyLevel::Warning);
        }
    }

    /// Execute a task by ID
    fn execute_task(&mut self, task_id: &str) {
        // Mark task as running and persist
        if let Some(ref mut plan) = self.current_plan {
            plan.update_status(task_id, TaskStatus::Running);
        }
        self.save_plan();

        // Get task action (clone to avoid borrow issues)
        let task_action = self
            .current_plan
            .as_ref()
            .and_then(|p| p.get_task(task_id))
            .map(|t| t.action.clone());

        let Some(action) = task_action else {
            return;
        };

        match action {
            TaskAction::CreateWorker {
                branch,
                task_description,
                base_branch,
            } => {
                self.execute_create_worker(task_id, &branch, &task_description, base_branch.as_deref());
            }
            TaskAction::CreatePr {
                branch,
                title,
                body,
                base,
                draft,
            } => {
                self.execute_create_pr(
                    task_id,
                    &branch,
                    &title,
                    body.as_deref(),
                    base.as_deref(),
                    draft,
                );
            }
            TaskAction::MergeBranch { branch, target } => {
                self.execute_merge_branch(task_id, &branch, target.as_deref());
            }
            TaskAction::CleanupWorktree { worktree } => {
                self.execute_cleanup_worktree(task_id, &worktree);
            }
            TaskAction::RunCommand { worktree, command } => {
                self.execute_run_command(task_id, &worktree, &command);
            }
            TaskAction::Notify { message, level } => {
                self.add_notification(message, level);
                if let Some(ref mut plan) = self.current_plan {
                    plan.update_status(task_id, TaskStatus::Completed);
                }
                self.save_plan();
            }
            TaskAction::RequestReview { branch, after_task } => {
                self.execute_request_review(task_id, &branch, after_task.as_deref());
            }
        }
    }

    /// Execute RequestReview task
    fn execute_request_review(&mut self, task_id: &str, branch: &str, after_task: Option<&str>) {
        // Check if after_task is completed (if specified)
        if let Some(after_task_id) = after_task {
            let after_completed = self
                .current_plan
                .as_ref()
                .and_then(|p| p.get_task(after_task_id))
                .map(|t| t.status == TaskStatus::Completed)
                .unwrap_or(false);

            if !after_completed {
                // after_task not yet completed, skip for now
                // Reset status to pending so it can be retried
                if let Some(ref mut plan) = self.current_plan {
                    plan.update_status(task_id, TaskStatus::Pending);
                }
                self.save_plan();
                return;
            }
        }

        // Find the agent index for this branch
        let agent_index = self.agent_worktrees.iter().position(|wt| {
            wt.as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|n| n == branch)
                .unwrap_or(false)
        });

        if let Some(index) = agent_index {
            // Store the task_id in review state for later completion marking
            self.pending_review_task_id = Some(task_id.to_string());
            self.start_review(index);
        } else {
            // Try to start review directly from branch name (worktree might be in worktree_dir)
            let worktree_path = self.config.worktree_dir.join(branch);
            if worktree_path.exists() {
                self.pending_review_task_id = Some(task_id.to_string());
                self.start_review_for_branch(branch, &worktree_path);
            } else {
                self.mark_task_failed(task_id, &format!("Branch '{branch}' not found"));
            }
        }
    }

    /// Start review mode for a specific branch and worktree path
    pub fn start_review_for_branch(&mut self, branch: &str, worktree_path: &PathBuf) {
        // Get main repo path
        let repo_path = env::current_dir().unwrap_or_default();
        let merger = MergeManager::new(&repo_path);

        // Get diff
        let diff = merger.diff(branch).unwrap_or_default();

        // Get commit log
        let commit_log = get_commit_log(worktree_path);

        // Get merge preview
        let preview = merger.preview(branch).ok();
        let (files_changed, insertions, deletions, conflicts) = match preview {
            Some(p) => (p.files_changed, p.insertions, p.deletions, p.conflicts),
            None => (0, 0, 0, vec![]),
        };

        // Create diff view
        let diff_view = DiffView::new(diff).with_title(format!("{branch} → main"));

        self.review_state = Some(ReviewState {
            agent_index: usize::MAX, // No agent associated
            branch: branch.to_string(),
            worktree_path: worktree_path.clone(),
            diff_view,
            commit_log,
            files_changed,
            insertions,
            deletions,
            conflicts,
        });

        self.mode = AppMode::ReviewMerge;
    }

    /// Execute CreateWorker task
    fn execute_create_worker(
        &mut self,
        task_id: &str,
        branch: &str,
        task_description: &str,
        _base_branch: Option<&str>,
    ) {
        // Create worktree
        let (working_dir, worktree_path) = if let Some(ref wt_manager) = self.worktree_manager {
            match wt_manager.create(branch, &self.config.worktree_dir) {
                Ok(path) => {
                    debug::log_worktree("created", &path);
                    (path.clone(), Some(path))
                }
                Err(e) => {
                    self.mark_task_failed(task_id, &format!("Failed to create worktree: {e}"));
                    return;
                }
            }
        } else {
            match env::current_dir() {
                Ok(dir) => (dir, None),
                Err(e) => {
                    self.mark_task_failed(task_id, &format!("Failed to get current directory: {e}"));
                    return;
                }
            }
        };

        // Create agent in non-interactive mode
        let name = branch.to_string();
        let full_prompt = format!(
            "{}\n\n\
            重要: 作業完了後は必ず git add と git commit を実行してコミットしてください。\n\
            コミットせずに終了すると変更が失われます。",
            task_description
        );
        match self.agent_manager.add_non_interactive(
            name.clone(),
            working_dir,
            &full_prompt,
            None, // No turn limit for plan-based workers
        ) {
            Ok(_) => {
                let agent_index = self.agent_manager.list().len() - 1;
                self.agent_issues.push(None);
                self.agent_worktrees.push(worktree_path);
                self.task_agents.insert(task_id.to_string(), agent_index);

                // Update PTY sizes for pane split
                self.update_agent_sizes();

                debug::log_task(task_id, "pending", "running");
                self.add_notification(
                    format!("Worker started: {name}"),
                    cctakt::plan::NotifyLevel::Success,
                );
            }
            Err(e) => {
                self.mark_task_failed(task_id, &format!("Failed to create agent: {e}"));
            }
        }
    }

    /// Execute CreatePr task
    fn execute_create_pr(
        &mut self,
        task_id: &str,
        branch: &str,
        title: &str,
        body: Option<&str>,
        base: Option<&str>,
        draft: bool,
    ) {
        let Some(ref client) = self.github_client else {
            self.mark_task_failed(task_id, "GitHub client not configured");
            return;
        };

        let create_req = cctakt::github::CreatePullRequest {
            title: title.to_string(),
            body: body.map(String::from),
            head: branch.to_string(),
            base: base.unwrap_or("main").to_string(),
            draft,
        };

        match client.create_pull_request(&create_req) {
            Ok(pr) => {
                self.add_notification(
                    format!("PR created: #{} - {}", pr.number, pr.title),
                    cctakt::plan::NotifyLevel::Success,
                );
                let result = TaskResult {
                    commits: Vec::new(),
                    pr_number: Some(pr.number),
                    pr_url: Some(pr.html_url),
                };
                if let Some(ref mut plan) = self.current_plan {
                    plan.mark_completed(task_id, result);
                    if let Err(e) = self.plan_manager.save(plan) {
                        debug::log(&format!("Failed to save plan: {e}"));
                    }
                }
            }
            Err(e) => {
                self.mark_task_failed(task_id, &format!("Failed to create PR: {e}"));
            }
        }
    }

    /// Execute MergeBranch task
    fn execute_merge_branch(&mut self, task_id: &str, branch: &str, target: Option<&str>) {
        let repo_path = match env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                self.mark_task_failed(task_id, &format!("Failed to get current directory: {e}"));
                return;
            }
        };

        let merger = MergeManager::new(&repo_path);
        let merger = if let Some(t) = target {
            merger.with_main_branch(t)
        } else {
            merger
        };

        match merger.merge_no_ff(branch, None) {
            Ok(()) => {
                self.add_notification(
                    format!("Merged: {} → {}", branch, target.unwrap_or("main")),
                    cctakt::plan::NotifyLevel::Success,
                );
                if let Some(ref mut plan) = self.current_plan {
                    plan.update_status(task_id, TaskStatus::Completed);
                }
                self.save_plan();
            }
            Err(e) => {
                self.mark_task_failed(task_id, &format!("Failed to merge: {e}"));
            }
        }
    }

    /// Execute CleanupWorktree task
    fn execute_cleanup_worktree(&mut self, task_id: &str, worktree: &str) {
        if let Some(ref wt_manager) = self.worktree_manager {
            let worktree_path = self.config.worktree_dir.join(worktree);
            match wt_manager.remove(&worktree_path) {
                Ok(()) => {
                    self.add_notification(
                        format!("Worktree cleaned up: {worktree}"),
                        cctakt::plan::NotifyLevel::Info,
                    );
                    if let Some(ref mut plan) = self.current_plan {
                        plan.update_status(task_id, TaskStatus::Completed);
                    }
                    self.save_plan();
                }
                Err(e) => {
                    self.mark_task_failed(task_id, &format!("Failed to cleanup worktree: {e}"));
                }
            }
        } else {
            self.mark_task_failed(task_id, "Worktree manager not available");
        }
    }

    /// Execute RunCommand task (not implemented yet - just marks complete)
    fn execute_run_command(&mut self, task_id: &str, worktree: &str, command: &str) {
        self.add_notification(
            format!("RunCommand not implemented: {command} in {worktree}"),
            cctakt::plan::NotifyLevel::Warning,
        );
        if let Some(ref mut plan) = self.current_plan {
            plan.update_status(task_id, TaskStatus::Skipped);
        }
        self.save_plan();
    }

    /// Mark a task as failed
    pub fn mark_task_failed(&mut self, task_id: &str, error: &str) {
        self.add_notification(
            format!("Task failed: {error}"),
            cctakt::plan::NotifyLevel::Error,
        );
        if let Some(ref mut plan) = self.current_plan {
            plan.mark_failed(task_id, error);
            if let Err(e) = self.plan_manager.save(plan) {
                debug::log(&format!("Failed to save plan: {e}"));
            }
        }
    }

    /// Add a notification
    pub fn add_notification(&mut self, message: String, level: cctakt::plan::NotifyLevel) {
        self.notifications.push(Notification {
            message,
            level,
            created_at: std::time::Instant::now(),
        });
    }

    /// Save current plan to file (persist status changes across restarts)
    pub fn save_plan(&mut self) {
        if let Some(ref plan) = self.current_plan {
            if let Err(e) = self.plan_manager.save(plan) {
                debug::log(&format!("Failed to save plan: {e}"));
            }
        }
    }

    /// Clean up old notifications (older than 5 seconds)
    pub fn cleanup_notifications(&mut self) {
        let now = std::time::Instant::now();
        self.notifications
            .retain(|n| now.duration_since(n.created_at).as_secs() < 5);
    }

    /// Check if any agent completed its task and update plan
    pub fn check_agent_task_completions(&mut self) {
        // Collect ended agents with their task info
        let ended: Vec<(String, usize, Option<String>)> = self
            .task_agents
            .iter()
            .filter_map(|(task_id, &agent_index)| {
                self.agent_manager
                    .get(agent_index)
                    .filter(|a| a.status == AgentStatus::Ended)
                    .map(|a| (task_id.clone(), agent_index, a.error.clone()))
            })
            .collect();

        // Process ended agents
        for (task_id, agent_index, error) in ended {
            if let Some(error_msg) = error {
                // Agent ended with error - mark task as failed
                debug::log_task(&task_id, "running", "failed");
                self.add_notification(
                    format!("Worker failed: {error_msg}"),
                    cctakt::plan::NotifyLevel::Error,
                );
                if let Some(ref mut plan) = self.current_plan {
                    plan.mark_failed(&task_id, &error_msg);
                    // Persist plan to file so status survives restart
                    if let Err(e) = self.plan_manager.save(plan) {
                        debug::log(&format!("Failed to save plan: {e}"));
                    }
                }
            } else {
                // Agent ended successfully - get commits and mark completed
                let commits = if agent_index < self.agent_worktrees.len() {
                    if let Some(ref worktree_path) = self.agent_worktrees[agent_index] {
                        get_worker_commits(worktree_path)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Warn if no commits
                if commits.is_empty() {
                    self.add_notification(
                        format!("Worker {task_id} completed with no commits"),
                        cctakt::plan::NotifyLevel::Warning,
                    );
                }

                let result = TaskResult {
                    commits,
                    pr_number: None,
                    pr_url: None,
                };

                if let Some(ref mut plan) = self.current_plan {
                    plan.mark_completed(&task_id, result);
                    // Persist plan to file so status survives restart
                    if let Err(e) = self.plan_manager.save(plan) {
                        debug::log(&format!("Failed to save plan: {e}"));
                    }
                }
                debug::log_task(&task_id, "running", "completed");
            }
            self.task_agents.remove(&task_id);
        }
    }

    /// Resize all agents
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.content_cols = cols;
        self.content_rows = rows;
        self.update_agent_sizes();
    }

    /// Update PTY sizes based on current pane layout
    pub fn update_agent_sizes(&mut self) {
        let has_workers = self.agent_manager.has_non_interactive();

        if has_workers {
            // Split view: 50% each (minus separator)
            let pane_width = (self.content_cols.saturating_sub(1)) / 2;

            // Resize interactive agent (left pane)
            if let Some(agent) = self.agent_manager.get_interactive_mut() {
                agent.resize(pane_width, self.content_rows);
            }

            // Resize non-interactive agents (right pane)
            for agent in self.agent_manager.get_all_non_interactive_mut() {
                agent.resize(pane_width, self.content_rows);
            }
        } else {
            // Full width for single agent
            self.agent_manager.resize_all(self.content_cols, self.content_rows);
        }
    }
}
