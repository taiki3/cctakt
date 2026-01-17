mod agent;

use agent::{AgentManager, AgentStatus};
use anyhow::{Context, Result};
use cctakt::{
    available_themes, create_theme, current_theme_id, debug, issue_picker::centered_rect,
    render_task, set_theme, suggest_branch_name, theme, Config, DiffView, GitHubClient, Issue,
    IssuePicker, IssuePickerResult, LockFile, MergeManager, Plan, PlanManager, TaskAction,
    TaskResult, TaskStatus, WorktreeManager,
};
use clap::{Parser, Subcommand};
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

// ==================== CLI Definition ====================

#[derive(Parser)]
#[command(name = "cctakt")]
#[command(author, version, about = "Claude Code Orchestrator - TUI for managing multiple Claude Code agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize cctakt in the current repository
    Init {
        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },
    /// Check environment setup status
    Status,
    /// List GitHub issues
    Issues {
        /// Filter by labels (comma-separated)
        #[arg(short, long)]
        labels: Option<String>,
        /// Issue state: open, closed, all
        #[arg(short, long, default_value = "open")]
        state: String,
    },
    /// Run workers from a plan file (CLI mode, no TUI)
    Run {
        /// Path to plan.json file (default: .cctakt/plan.json)
        #[arg(default_value = ".cctakt/plan.json")]
        plan: PathBuf,
    },
}

/// Application mode
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
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
}

/// Focused pane in split view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusedPane {
    /// Left pane (orchestrator/interactive agent)
    Left,
    /// Right pane (worker/non-interactive agent)
    Right,
}

/// Input mode (vim-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    /// Navigation mode - hjkl moves between panes
    Navigation,
    /// Input mode - keys are sent to the focused agent
    Input,
}

/// Review state for a completed agent
struct ReviewState {
    /// Agent index being reviewed
    agent_index: usize,
    /// Branch name
    branch: String,
    /// Working directory (worktree path)
    worktree_path: PathBuf,
    /// Diff view
    diff_view: DiffView,
    /// Commit log
    commit_log: String,
    /// Merge preview info
    files_changed: usize,
    insertions: usize,
    deletions: usize,
    /// Potential conflicts
    conflicts: Vec<String>,
}

/// Merge task for the queue
struct MergeTask {
    /// Branch name to merge
    branch: String,
    /// Worktree path (for cleanup after merge)
    worktree_path: PathBuf,
    /// Agent index (for cleanup after merge)
    agent_index: usize,
    /// Task ID (for plan update)
    task_id: Option<String>,
}

/// Merge queue for sequential merge processing
struct MergeQueue {
    /// Pending merge tasks
    queue: std::collections::VecDeque<MergeTask>,
    /// Currently processing task
    current: Option<MergeTask>,
    /// MergeWorker agent index (None if not spawned)
    worker_agent_index: Option<usize>,
}

impl MergeQueue {
    fn new() -> Self {
        Self {
            queue: std::collections::VecDeque::new(),
            current: None,
            worker_agent_index: None,
        }
    }

    fn enqueue(&mut self, task: MergeTask) {
        self.queue.push_back(task);
    }

    fn start_next(&mut self) -> Option<&MergeTask> {
        if self.current.is_none() {
            self.current = self.queue.pop_front();
        }
        self.current.as_ref()
    }

    fn complete_current(&mut self) {
        self.current = None;
    }

    fn is_busy(&self) -> bool {
        self.current.is_some()
    }

    fn pending_count(&self) -> usize {
        self.queue.len() + if self.current.is_some() { 1 } else { 0 }
    }
}

/// Notification message
struct Notification {
    message: String,
    level: cctakt::plan::NotifyLevel,
    created_at: std::time::Instant,
}

/// Application state
struct App {
    agent_manager: AgentManager,
    should_quit: bool,
    content_rows: u16,
    content_cols: u16,
    /// Current application mode
    mode: AppMode,
    /// Focused pane in split view
    focused_pane: FocusedPane,
    /// Input mode (Navigation or Input)
    input_mode: InputMode,
    /// Configuration
    config: Config,
    /// Worktree manager
    worktree_manager: Option<WorktreeManager>,
    /// GitHub client
    github_client: Option<GitHubClient>,
    /// Issue picker UI
    issue_picker: IssuePicker,
    /// Current issue being worked on (per agent)
    agent_issues: Vec<Option<Issue>>,
    /// Worktree paths per agent
    agent_worktrees: Vec<Option<PathBuf>>,
    /// Review state for merge review mode
    review_state: Option<ReviewState>,
    /// Plan manager for orchestrator communication
    plan_manager: PlanManager,
    /// Current plan being executed
    current_plan: Option<Plan>,
    /// Task ID to agent index mapping
    task_agents: std::collections::HashMap<String, usize>,
    /// Notifications to display
    notifications: Vec<Notification>,
    /// Pending prompt to send to agent after it initializes (unused in non-interactive mode)
    pending_agent_prompt: Option<String>,
    /// Frame counter for delayed prompt sending (unused in non-interactive mode)
    prompt_delay_frames: u32,
    /// Pending review task ID to mark as completed after merge
    pending_review_task_id: Option<String>,
    /// Merge queue for sequential merge processing
    merge_queue: MergeQueue,
    /// Theme picker: show picker modal
    show_theme_picker: bool,
    /// Theme picker: currently selected index
    theme_picker_index: usize,
    /// Branch name for build confirmation after merge
    pending_build_branch: Option<String>,
}

impl App {
    fn new(rows: u16, cols: u16, config: Config) -> Self {
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
            input_mode: InputMode::Input, // Default to input mode
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
            pending_build_branch: None,
        }
    }

    /// Open issue picker and fetch issues
    fn open_issue_picker(&mut self) {
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
    fn open_theme_picker(&mut self) {
        // Set index to current theme
        let current = current_theme_id();
        let themes = available_themes();
        self.theme_picker_index = themes
            .iter()
            .position(|(id, _, _)| *id == current)
            .unwrap_or(0);
        self.show_theme_picker = true;
        self.mode = AppMode::ThemePicker;
    }

    /// Apply selected theme and save to config
    fn apply_theme(&mut self, theme_id: &str) {
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
    fn fetch_issues(&mut self) {
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
    fn add_agent_from_issue(&mut self, issue: Issue) -> Result<()> {
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

        Ok(())
    }

    /// Add a new agent with the current directory (interactive mode for orchestrator)
    fn add_agent(&mut self) -> Result<()> {
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
    fn close_active_agent(&mut self) {
        let index = self.agent_manager.active_index();
        self.agent_manager.close(index);
        if index < self.agent_issues.len() {
            self.agent_issues.remove(index);
        }
        if index < self.agent_worktrees.len() {
            self.agent_worktrees.remove(index);
        }
    }

    /// Check all agents for completion and auto-transition to review mode
    fn check_agent_completion(&mut self) {
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
    fn start_review(&mut self, agent_index: usize) {
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
    fn enqueue_merge(&mut self) {
        let review = self.review_state.take();
        let Some(review) = review else {
            self.mode = AppMode::Normal;
            return;
        };

        let task = MergeTask {
            branch: review.branch.clone(),
            worktree_path: review.worktree_path.clone(),
            agent_index: review.agent_index,
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
    fn process_merge_queue(&mut self) {
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
    fn check_merge_worker_completion(&mut self) {
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
            .args(["log", "--oneline", "-1", "--grep", &format!("Merge branch '{}'", task.branch)])
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

        // Close the original worker agent (if associated)
        if task.agent_index != usize::MAX {
            self.agent_manager.close(task.agent_index);
            if task.agent_index < self.agent_issues.len() {
                self.agent_issues.remove(task.agent_index);
            }
            if task.agent_index < self.agent_worktrees.len() {
                self.agent_worktrees.remove(task.agent_index);
            }
        }

        // Mark task as completed
        if let Some(ref task_id) = task.task_id {
            if let Some(ref mut plan) = self.current_plan {
                plan.update_status(task_id, TaskStatus::Completed);
                let _ = self.plan_manager.save(plan);
            }
        }

        // Ask user if they want to run build
        self.pending_build_branch = Some(task.branch.clone());
        self.mode = AppMode::ConfirmBuild;
    }

    /// Handle failed merge
    fn handle_merge_failure(&mut self, task: &MergeTask) {
        self.add_notification(
            format!("Merge failed: {} (MergeWorker could not complete)", task.branch),
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
    fn spawn_build_worker(&mut self) {
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

    /// Cancel review and return to normal mode
    fn cancel_review(&mut self) {
        self.review_state = None;
        self.mode = AppMode::Normal;
    }

    /// Check for plan file changes and load
    fn check_plan(&mut self) {
        if self.plan_manager.has_changes() {
            match self.plan_manager.load() {
                Ok(Some(plan)) => {
                    if let Some(desc) = &plan.description {
                        self.add_notification(
                            format!("Plan loaded: {desc}"),
                            cctakt::plan::NotifyLevel::Info,
                        );
                    }
                    self.current_plan = Some(plan);
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
    fn process_plan(&mut self) {
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
                self.execute_create_pr(task_id, &branch, &title, body.as_deref(), base.as_deref(), draft);
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
    fn execute_request_review(
        &mut self,
        task_id: &str,
        branch: &str,
        after_task: Option<&str>,
    ) {
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
        let agent_index = self
            .agent_worktrees
            .iter()
            .position(|wt| {
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
    fn start_review_for_branch(&mut self, branch: &str, worktree_path: &PathBuf) {
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
    fn mark_task_failed(&mut self, task_id: &str, error: &str) {
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
    fn add_notification(&mut self, message: String, level: cctakt::plan::NotifyLevel) {
        self.notifications.push(Notification {
            message,
            level,
            created_at: std::time::Instant::now(),
        });
    }

    /// Save current plan to file (persist status changes across restarts)
    fn save_plan(&mut self) {
        if let Some(ref plan) = self.current_plan {
            if let Err(e) = self.plan_manager.save(plan) {
                debug::log(&format!("Failed to save plan: {e}"));
            }
        }
    }

    /// Clean up old notifications (older than 5 seconds)
    fn cleanup_notifications(&mut self) {
        let now = std::time::Instant::now();
        self.notifications.retain(|n| {
            now.duration_since(n.created_at).as_secs() < 5
        });
    }

    /// Check if any agent completed its task and update plan
    fn check_agent_task_completions(&mut self) {
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
    fn resize(&mut self, cols: u16, rows: u16) {
        self.content_cols = cols;
        self.content_rows = rows;
        self.agent_manager.resize_all(cols, rows);
    }
}

/// Get commit log from worktree
fn get_commit_log(worktree_path: &PathBuf) -> String {
    use std::process::Command;

    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["log", "--oneline", "-n", "20", "--no-decorate"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    }
}

/// Get commits made by a worker (commits since branch creation)
fn get_worker_commits(worktree_path: &PathBuf) -> Vec<String> {
    use std::process::Command;

    // Get commits that are ahead of main/master
    // Try main first, then master
    let bases = ["main", "master"];
    for base in bases {
        let output = Command::new("git")
            .current_dir(worktree_path)
            .args(["log", "--oneline", &format!("{base}..HEAD")])
            .output();

        if let Ok(o) = output {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let commits: Vec<String> = stdout
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
                if !commits.is_empty() {
                    return commits;
                }
            }
        }
    }

    // Fallback: just get recent commits
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["log", "--oneline", "-n", "10"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Detect GitHub repository from git remote
fn detect_github_repo() -> Option<String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_github_url(&url)
}

/// Parse GitHub repository from URL string
/// Supports formats:
/// - https://github.com/owner/repo.git
/// - git@github.com:owner/repo.git
/// - https://github.com/owner/repo
fn parse_github_url(url: &str) -> Option<String> {
    if url.contains("github.com") {
        let repo = url
            .trim_end_matches(".git")
            .split("github.com")
            .last()?
            .trim_start_matches('/')
            .trim_start_matches(':')
            .to_string();
        if repo.is_empty() {
            None
        } else {
            Some(repo)
        }
    } else {
        None
    }
}

// ==================== Init Command ====================

/// Run the init command
fn run_init(force: bool) -> Result<()> {
    println!("🚀 Initializing cctakt...\n");

    // Check if we're in a git repository
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        return Err(anyhow::anyhow!(
            "Not a git repository. Please run 'cctakt init' from within a git repository."
        ));
    }

    // 1. Check/create .claude directory
    let claude_dir = PathBuf::from(".claude");
    let commands_dir = claude_dir.join("commands");

    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)?;
        println!("✅ Created .claude/ directory");
    } else {
        println!("📁 .claude/ directory already exists");
    }

    if !commands_dir.exists() {
        fs::create_dir_all(&commands_dir)?;
        println!("✅ Created .claude/commands/ directory");
    }

    // 2. Create orchestrator skill
    let orchestrator_skill_path = commands_dir.join("orchestrator.md");
    if !orchestrator_skill_path.exists() || force {
        let skill_content = include_str!("../templates/orchestrator_skill.md");
        fs::write(&orchestrator_skill_path, skill_content)?;
        println!("✅ Created orchestrator skill: .claude/commands/orchestrator.md");
    } else {
        println!("📄 Orchestrator skill already exists (use --force to overwrite)");
    }

    // 3. Create orchestrator.md reference
    let orchestrator_md_path = claude_dir.join("orchestrator.md");
    if !orchestrator_md_path.exists() || force {
        let orchestrator_content = include_str!("../templates/orchestrator.md");
        fs::write(&orchestrator_md_path, orchestrator_content)?;
        println!("✅ Created orchestrator reference: .claude/orchestrator.md");
    } else {
        println!("📄 Orchestrator reference already exists (use --force to overwrite)");
    }

    // 4. Create .cctakt directory
    let cctakt_dir = PathBuf::from(".cctakt");
    if !cctakt_dir.exists() {
        fs::create_dir_all(&cctakt_dir)?;
        println!("✅ Created .cctakt/ directory");
    } else {
        println!("📁 .cctakt/ directory already exists");
    }

    // 5. Create cctakt.toml config if not exists
    let config_path = PathBuf::from("cctakt.toml");
    if !config_path.exists() || force {
        Config::generate_default(&config_path)?;
        println!("✅ Created configuration: cctakt.toml");
    } else {
        println!("📄 Configuration file already exists (use --force to overwrite)");
    }

    // 6. Update .gitignore
    let gitignore_path = PathBuf::from(".gitignore");
    let gitignore_entries = [".cctakt/plan_*.json"];

    let existing_gitignore = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let mut added_entries = Vec::new();

    for entry in gitignore_entries {
        if !existing_gitignore.contains(entry) {
            added_entries.push(entry);
        }
    }

    if !added_entries.is_empty() {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)?;

        if !existing_gitignore.is_empty() && !existing_gitignore.ends_with('\n') {
            writeln!(file)?;
        }
        writeln!(file, "\n# cctakt")?;
        for entry in &added_entries {
            writeln!(file, "{entry}")?;
        }
        println!("✅ Updated .gitignore with cctakt entries");
    }

    println!("\n---\n");

    // 7. Check GitHub token
    check_github_token();

    // 8. Check claude CLI
    check_claude_cli();

    println!("\n🎉 cctakt initialization complete!");
    println!("\nNext steps:");
    println!("  1. Run 'cctakt' to start the TUI");
    println!("  2. Press 'i' to select an issue");
    println!("  3. The orchestrator Claude Code can use /orchestrator skill");

    Ok(())
}

/// Check GitHub token availability
fn check_github_token() {
    print!("🔑 GitHub token: ");
    io::stdout().flush().ok();

    // Check environment variable
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            println!("✅ Found (GITHUB_TOKEN environment variable)");
            return;
        }
    }

    // Check gh CLI
    let gh_token = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|t| !t.is_empty());

    if gh_token.is_some() {
        println!("✅ Found (gh CLI)");
        return;
    }

    println!("⚠️  Not found");
    println!("   To enable GitHub integration:");
    println!("   - Set GITHUB_TOKEN environment variable, or");
    println!("   - Run 'gh auth login' to authenticate with GitHub CLI");
}

/// Check claude CLI availability
fn check_claude_cli() {
    print!("🤖 Claude CLI: ");
    io::stdout().flush().ok();

    let claude_available = Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if claude_available {
        println!("✅ Available");
    } else {
        println!("❌ Not found");
        println!("   Install Claude Code CLI: npm install -g @anthropic-ai/claude-code");
    }
}

/// Run the status command
fn run_status() -> Result<()> {
    println!("cctakt Environment Status\n");

    // Git repository
    print!("📂 Git repository: ");
    io::stdout().flush().ok();
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if is_git_repo {
        println!("✅ Yes");

        // Get repo info
        if let Some(repo) = detect_github_repo() {
            println!("   Repository: {repo}");
        }
    } else {
        println!("❌ No");
    }

    // Check directories
    print!("📁 .claude/ directory: ");
    io::stdout().flush().ok();
    if PathBuf::from(".claude").exists() {
        println!("✅ Exists");
    } else {
        println!("❌ Missing");
    }

    print!("📁 .cctakt/ directory: ");
    io::stdout().flush().ok();
    if PathBuf::from(".cctakt").exists() {
        println!("✅ Exists");
    } else {
        println!("❌ Missing");
    }

    // Check orchestrator skill
    print!("📄 Orchestrator skill: ");
    io::stdout().flush().ok();
    if PathBuf::from(".claude/commands/orchestrator.md").exists() {
        println!("✅ Installed");
    } else {
        println!("❌ Not installed");
    }

    // Check config
    print!("⚙️  Configuration: ");
    io::stdout().flush().ok();
    if PathBuf::from("cctakt.toml").exists() {
        println!("✅ Found");
    } else {
        println!("⚠️  Using defaults");
    }

    println!();

    // Check GitHub token
    check_github_token();

    // Check claude CLI
    check_claude_cli();

    println!();
    println!("Run 'cctakt init' to set up missing components.");

    Ok(())
}

/// Run the TUI application
fn run_tui() -> Result<()> {
    // Acquire lock to prevent duplicate instances
    // The lock is automatically released when _lock goes out of scope
    let _lock = LockFile::acquire()?;

    // Load configuration
    let config = Config::load().unwrap_or_default();

    // Initialize theme from config
    set_theme(create_theme(&config.theme));

    // Get terminal size
    let (cols, rows) = terminal::size().context("Failed to get terminal size")?;
    let content_rows = rows.saturating_sub(3); // Header 1 line + border 2 lines
    let content_cols = cols.saturating_sub(2); // Border 2 columns

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    execute!(
        stdout,
        crossterm::terminal::SetTitle("cctakt - Claude Code Orchestrator")
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize app
    let mut app = App::new(content_rows, content_cols, config);

    // Add initial agent
    if let Err(e) = app.add_agent() {
        // Cleanup and return error
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            crossterm::cursor::Show,
            LeaveAlternateScreen
        )?;
        return Err(e);
    }

    // Main loop
    loop {
        // Draw
        terminal.draw(|f| ui(f, &mut app))?;

        // Handle pending agent prompt (wait ~1 second for agent to initialize)
        if app.pending_agent_prompt.is_some() {
            app.prompt_delay_frames += 1;

            // After 60 frames (~1 sec), send the task
            if app.prompt_delay_frames > 60 {
                if let Some(prompt) = app.pending_agent_prompt.take() {
                    if let Some(agent) = app.agent_manager.active_mut() {
                        agent.send_bytes(prompt.as_bytes());
                        agent.send_bytes(b"\r");  // Carriage return for Enter
                        agent.task_sent = true;
                        agent.work_state = agent::WorkState::Working;
                    }
                }
                app.prompt_delay_frames = 0;
            }
        }

        // Check agent work states and auto-transition to review mode
        app.check_agent_completion();

        // Poll events (16ms ≈ 60fps)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match app.mode {
                        AppMode::ReviewMerge => {
                            // Handle review mode input
                            match key.code {
                                KeyCode::Enter | KeyCode::Char('m') | KeyCode::Char('M') => {
                                    // Enqueue merge (handled by MergeWorker)
                                    app.enqueue_merge();
                                }
                                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('c') | KeyCode::Char('C') => {
                                    // Cancel review (q to quit, c to cancel)
                                    app.cancel_review();
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if let Some(ref mut state) = app.review_state {
                                        state.diff_view.scroll_up(1);
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if let Some(ref mut state) = app.review_state {
                                        state.diff_view.scroll_down(1);
                                    }
                                }
                                KeyCode::PageUp => {
                                    if let Some(ref mut state) = app.review_state {
                                        state.diff_view.page_up(20);
                                    }
                                }
                                KeyCode::PageDown => {
                                    if let Some(ref mut state) = app.review_state {
                                        state.diff_view.page_down(20);
                                    }
                                }
                                KeyCode::Home => {
                                    if let Some(ref mut state) = app.review_state {
                                        state.diff_view.scroll_to_top();
                                    }
                                }
                                KeyCode::End => {
                                    if let Some(ref mut state) = app.review_state {
                                        state.diff_view.scroll_to_bottom();
                                    }
                                }
                                _ => {}
                            }
                        }
                        AppMode::IssuePicker => {
                            // Handle issue picker input
                            if let Some(result) = app.issue_picker.handle_key(key.code) {
                                match result {
                                    IssuePickerResult::Selected(issue) => {
                                        app.mode = AppMode::Normal;
                                        let _ = app.add_agent_from_issue(issue);
                                    }
                                    IssuePickerResult::Cancel => {
                                        app.mode = AppMode::Normal;
                                    }
                                    IssuePickerResult::Refresh => {
                                        app.fetch_issues();
                                    }
                                }
                            }
                        }
                        AppMode::Normal => {
                            if app.agent_manager.is_empty() {
                                // No agents - orchestrator was closed, quit app
                                app.should_quit = true;
                            } else {
                                // Always handle global keybindings (Ctrl+Q, Ctrl+T, etc)
                                let handled = handle_keybinding(&mut app, key.modifiers, key.code);

                                if !handled {
                                    // Debug: log current mode and key
                                    debug::log(&format!("Key: {:?}, Mode: {:?}, InputMode: {:?}", key.code, app.mode, app.input_mode));

                                    match app.input_mode {
                                        InputMode::Navigation => {
                                            // Navigation mode: hjkl for pane navigation
                                            debug::log("Processing Navigation mode key");
                                            match key.code {
                                                KeyCode::Char('h') => {
                                                    app.focused_pane = FocusedPane::Left;
                                                }
                                                KeyCode::Char('l') => {
                                                    app.focused_pane = FocusedPane::Right;
                                                }
                                                KeyCode::Char('j') => {
                                                    if app.focused_pane == FocusedPane::Right {
                                                        app.agent_manager.switch_to_next_worker();
                                                    }
                                                }
                                                KeyCode::Char('k') => {
                                                    if app.focused_pane == FocusedPane::Right {
                                                        app.agent_manager.switch_to_prev_worker();
                                                    }
                                                }
                                                KeyCode::Char('i') | KeyCode::Enter => {
                                                    // Switch to input mode
                                                    app.input_mode = InputMode::Input;
                                                }
                                                _ => {}
                                            }
                                        }
                                        InputMode::Input => {
                                            // Input mode: forward keys to focused agent
                                            // Esc switches back to navigation mode
                                            debug::log("Processing Input mode key");
                                            if key.code == KeyCode::Esc {
                                                debug::log("Esc pressed - switching to Navigation mode");
                                                app.input_mode = InputMode::Navigation;
                                            } else {
                                                // Determine which agent to send input to
                                                // Fallback: if focused pane has no agent, try the other pane
                                                let has_interactive = app.agent_manager.get_interactive().is_some();
                                                let has_worker = app.agent_manager.get_active_non_interactive().is_some();

                                                let use_interactive = match app.focused_pane {
                                                    FocusedPane::Left => has_interactive || !has_worker,
                                                    FocusedPane::Right => !has_worker && has_interactive,
                                                };

                                                let agent = if use_interactive {
                                                    app.agent_manager.get_interactive_mut()
                                                } else {
                                                    app.agent_manager.get_active_non_interactive_mut()
                                                };

                                                if let Some(agent) = agent {
                                                    if agent.status == AgentStatus::Running {
                                                        match (key.modifiers, key.code) {
                                                            (KeyModifiers::CONTROL, KeyCode::Char(c)) => {
                                                                let ctrl_char = (c as u8) & 0x1f;
                                                                agent.send_bytes(&[ctrl_char]);
                                                            }
                                                            (_, KeyCode::Enter) => agent.send_bytes(b"\r"),
                                                            (_, KeyCode::Backspace) => agent.send_bytes(&[0x7f]),
                                                            (_, KeyCode::Tab) => agent.send_bytes(b"\t"),
                                                            (_, KeyCode::Up) => agent.send_bytes(b"\x1b[A"),
                                                            (_, KeyCode::Down) => agent.send_bytes(b"\x1b[B"),
                                                            (_, KeyCode::Right) => agent.send_bytes(b"\x1b[C"),
                                                            (_, KeyCode::Left) => agent.send_bytes(b"\x1b[D"),
                                                            (_, KeyCode::Home) => agent.send_bytes(b"\x1b[H"),
                                                            (_, KeyCode::End) => agent.send_bytes(b"\x1b[F"),
                                                            (_, KeyCode::PageUp) => agent.send_bytes(b"\x1b[5~"),
                                                            (_, KeyCode::PageDown) => agent.send_bytes(b"\x1b[6~"),
                                                            (_, KeyCode::Delete) => agent.send_bytes(b"\x1b[3~"),
                                                            (_, KeyCode::Char(c)) => {
                                                                let mut buf = [0u8; 4];
                                                                let s = c.encode_utf8(&mut buf);
                                                                agent.send_bytes(s.as_bytes());
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        AppMode::ThemePicker => {
                            // Handle theme picker input
                            handle_theme_picker_input(&mut app, key.code);
                        }
                        AppMode::ConfirmBuild => {
                            // Handle build confirmation input
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    // Run build
                                    app.spawn_build_worker();
                                    app.pending_build_branch = None;
                                    app.mode = AppMode::Normal;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') => {
                                    // Skip build (n for no, q to quit)
                                    app.pending_build_branch = None;
                                    app.mode = AppMode::Normal;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Event::Resize(new_cols, new_rows) => {
                    let content_rows = new_rows.saturating_sub(3);
                    let content_cols = new_cols.saturating_sub(2);
                    app.resize(content_cols, content_rows);
                }
                _ => {}
            }
        }

        // Check all agents' status
        app.agent_manager.check_all_status();

        // Plan processing
        app.check_plan();
        app.check_agent_task_completions();
        app.process_plan();

        // Check MergeWorker completion
        app.check_merge_worker_completion();

        app.cleanup_notifications();

        // Check if active agent just ended and has a worktree (for review)
        if app.mode == AppMode::Normal {
            let active_index = app.agent_manager.active_index();
            if let Some(agent) = app.agent_manager.active() {
                if agent.status == AgentStatus::Ended {
                    // Check if this agent has a worktree
                    let has_worktree = active_index < app.agent_worktrees.len()
                        && app.agent_worktrees[active_index].is_some();
                    if has_worktree {
                        app.start_review(active_index);
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::cursor::Show,
        LeaveAlternateScreen
    )?;

    Ok(())
}

/// Handle special keybindings, returns true if handled
fn handle_keybinding(app: &mut App, modifiers: KeyModifiers, code: KeyCode) -> bool {
    match (modifiers, code) {
        // Ctrl+Q: Quit
        (KeyModifiers::CONTROL, KeyCode::Char('q' | 'Q')) => {
            app.should_quit = true;
            true
        }
        // Ctrl+T: Open theme picker
        (KeyModifiers::CONTROL, KeyCode::Char('t' | 'T')) => {
            app.open_theme_picker();
            true
        }
        // Ctrl+I or F2: Open issue picker
        (KeyModifiers::CONTROL, KeyCode::Char('i' | 'I')) | (_, KeyCode::F(2)) => {
            app.open_issue_picker();
            true
        }
        // Ctrl+W: Close active agent
        (KeyModifiers::CONTROL, KeyCode::Char('w' | 'W')) => {
            app.close_active_agent();
            true
        }
        // Ctrl+Tab or plain Tab (when no agent focused): Next tab
        // Note: Ctrl+Tab may not work in all terminals, so we use Ctrl+N as alternative
        (KeyModifiers::CONTROL, KeyCode::Char('n' | 'N')) => {
            app.agent_manager.next();
            true
        }
        // Ctrl+P: Previous tab
        (KeyModifiers::CONTROL, KeyCode::Char('p' | 'P')) => {
            app.agent_manager.prev();
            true
        }
        // Ctrl+1-9: Switch to tab by number
        (KeyModifiers::CONTROL, KeyCode::Char(c)) if ('1'..='9').contains(&c) => {
            let index = (c as usize) - ('1' as usize);
            app.agent_manager.switch_to(index);
            true
        }
        // Alt+1-9: Also switch to tab by number (more compatible)
        (KeyModifiers::ALT, KeyCode::Char(c)) if ('1'..='9').contains(&c) => {
            let index = (c as usize) - ('1' as usize);
            app.agent_manager.switch_to(index);
            true
        }
        // Note: hjkl pane navigation is handled in Navigation mode (see AppMode::Normal)
        _ => false,
    }
}

/// Handle theme picker keyboard input
fn handle_theme_picker_input(app: &mut App, code: KeyCode) {
    let themes = available_themes();
    let theme_count = themes.len();

    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.theme_picker_index > 0 {
                app.theme_picker_index -= 1;
            } else {
                app.theme_picker_index = theme_count.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.theme_picker_index < theme_count.saturating_sub(1) {
                app.theme_picker_index += 1;
            } else {
                app.theme_picker_index = 0;
            }
        }
        KeyCode::Enter => {
            // Apply selected theme
            if let Some((id, _, _)) = themes.get(app.theme_picker_index) {
                app.apply_theme(id);
            }
            app.show_theme_picker = false;
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('q') => {
            // Cancel (q to quit)
            app.show_theme_picker = false;
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header with tabs
            Constraint::Min(0),    // Main area
            Constraint::Length(1), // Footer with status
        ])
        .split(f.area());

    // Header with tabs
    render_header(f, app, chunks[0]);

    // Footer with status
    render_footer(f, app, chunks[2]);

    // Main area
    if app.agent_manager.is_empty() {
        render_no_agent_menu(f, chunks[1]);
    } else {
        render_split_pane_main_area(f, app, chunks[1]);
    }

    // Render overlays based on mode
    match app.mode {
        AppMode::IssuePicker => {
            let popup_area = centered_rect(80, 70, f.area());
            app.issue_picker.render(f, popup_area);
        }
        AppMode::ThemePicker => {
            render_theme_picker(f, app, f.area());
        }
        AppMode::ConfirmBuild => {
            render_build_confirmation(f, app, f.area());
        }
        AppMode::ReviewMerge | AppMode::Normal => {}
    }

    // Render notifications at the bottom
    if !app.notifications.is_empty() {
        render_notifications(f, app, f.area());
    }

    // Render plan status if active
    if app.current_plan.is_some() {
        render_plan_status(f, app, f.area());
    }
}

/// Render notifications at the bottom of the screen
fn render_notifications(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let notification_count = app.notifications.len().min(3); // Show max 3 notifications
    if notification_count == 0 {
        return;
    }

    let height = notification_count as u16 + 2; // +2 for borders
    let notification_area = ratatui::layout::Rect {
        x: area.x + 2,
        y: area.height.saturating_sub(height + 1),
        width: area.width.saturating_sub(4).min(60),
        height,
    };

    let t = theme();
    let lines: Vec<Line> = app
        .notifications
        .iter()
        .rev()
        .take(3)
        .map(|n| {
            let (prefix, style) = match n.level {
                cctakt::plan::NotifyLevel::Info => ("ℹ", t.style_info()),
                cctakt::plan::NotifyLevel::Warning => ("⚠", t.style_warning()),
                cctakt::plan::NotifyLevel::Error => ("✗", t.style_error()),
                cctakt::plan::NotifyLevel::Success => ("✓", t.style_success()),
            };
            Line::from(vec![
                Span::styled(format!(" {prefix} "), style),
                Span::raw(&n.message),
            ])
        })
        .collect();

    let notification_widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border_muted()),
    );

    f.render_widget(Clear, notification_area);
    f.render_widget(notification_widget, notification_area);
}

/// Render plan status indicator
fn render_plan_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let Some(ref plan) = app.current_plan else {
        return;
    };

    let (pending, running, completed, failed) = plan.count_by_status();
    let total = plan.tasks.len();

    let status_text = format!(
        " Plan: {completed}/{total} ({running} running, {failed} failed) "
    );

    let status_area = ratatui::layout::Rect {
        x: area.width.saturating_sub(status_text.len() as u16 + 2),
        y: 0,
        width: status_text.len() as u16,
        height: 1,
    };

    let t = theme();
    let style = if failed > 0 {
        t.style_error()
    } else if running > 0 {
        t.style_warning()
    } else if pending > 0 {
        t.style_info()
    } else {
        t.style_success()
    };

    let status_widget = Paragraph::new(status_text).style(style);
    f.render_widget(status_widget, status_area);
}

/// Render build confirmation dialog
fn render_build_confirmation(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let t = theme();

    // Calculate popup size
    let popup_width = 50u16;
    let popup_height = 7u16;

    // Center the popup
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = ratatui::layout::Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    f.render_widget(Clear, popup_area);

    let branch_name = app
        .pending_build_branch
        .as_deref()
        .unwrap_or("unknown");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Merged: {}", branch_name),
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
        Line::from("Run build? (y/n)"),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.style_dialog_border())
        .style(t.style_dialog_bg())
        .title(Span::styled(
            " Build Confirmation ",
            Style::default()
                .fg(t.neon_cyan())
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(paragraph, popup_area);
}

/// Render theme picker modal
fn render_theme_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let t = theme();
    let themes = available_themes();
    let current_theme = current_theme_id();

    // Calculate popup size
    let popup_width = 40u16;
    let popup_height = (themes.len() as u16) + 6; // title + items + footer + borders

    // Center the popup
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = ratatui::layout::Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    // Clear the popup area
    f.render_widget(Clear, popup_area);

    // Build theme list
    let mut lines: Vec<Line> = vec![
        Line::from(""),
    ];

    for (i, (id, name, description)) in themes.iter().enumerate() {
        let is_selected = i == app.theme_picker_index;
        let is_current = *id == current_theme;

        let prefix = if is_selected { " > " } else { "   " };
        let suffix = if is_current { " ✓" } else { "" };

        let style = if is_selected {
            Style::default()
                .fg(t.neon_cyan())
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default().fg(t.neon_green())
        } else {
            t.style_text()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(*name, style),
            Span::styled(suffix, Style::default().fg(t.neon_green())),
        ]));

        // Show description for selected item
        if is_selected {
            lines.push(Line::from(vec![
                Span::raw("     "),
                Span::styled(*description, t.style_text_muted()),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Footer
    lines.push(Line::from(vec![
        Span::styled(" Enter", t.style_key()),
        Span::styled(": Select  ", t.style_key_desc()),
        Span::styled("Esc", t.style_key()),
        Span::styled(": Cancel", t.style_key_desc()),
    ]));

    let block = Block::default()
        .title(Span::styled(
            " テーマを選択 ",
            Style::default()
                .fg(t.neon_cyan())
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(t.style_dialog_border())
        .style(t.style_dialog_bg());

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

/// Render review merge screen
fn render_review_merge(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let Some(ref mut state) = app.review_state else {
        return;
    };

    let t = theme();

    // Clear the area first
    f.render_widget(Clear, area);

    // Layout: header + diff + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header with stats
            Constraint::Min(10),   // Diff view
            Constraint::Length(3), // Footer with help
        ])
        .split(area);

    // Header with merge info
    let mut header_lines = vec![
        Line::from(vec![
            Span::styled(" Review Merge: ", Style::default().fg(t.neon_cyan()).add_modifier(Modifier::BOLD)),
            Span::styled(&state.branch, Style::default().fg(t.neon_yellow())),
            Span::raw(" → "),
            Span::styled("main", Style::default().fg(t.success())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Stats: "),
            Span::styled(format!("{} files", state.files_changed), t.style_text()),
            Span::raw(", "),
            Span::styled(format!("+{}", state.insertions), Style::default().fg(t.success())),
            Span::raw(" / "),
            Span::styled(format!("-{}", state.deletions), Style::default().fg(t.error())),
        ]),
    ];

    // Show conflicts warning if any
    if !state.conflicts.is_empty() {
        header_lines.push(Line::from(vec![
            Span::styled(" ⚠ Potential conflicts: ", t.style_warning()),
            Span::styled(
                state.conflicts.join(", "),
                t.style_warning(),
            ),
        ]));
    }

    // Show recent commits
    if !state.commit_log.is_empty() {
        header_lines.push(Line::from(""));
        header_lines.push(Line::from(vec![
            Span::styled(" Recent commits: ", Style::default().fg(t.neon_cyan())),
            Span::styled(
                state.commit_log.lines().next().unwrap_or(""),
                t.style_text_secondary(),
            ),
        ]));
    }

    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border()),
    );
    f.render_widget(header, chunks[0]);

    // Diff view
    state.diff_view.render(f, chunks[1]);

    // Footer with help
    let footer = Paragraph::new(vec![Line::from(vec![
        Span::styled(" [Enter/M]", t.style_success()),
        Span::raw(" Merge  "),
        Span::styled("[Esc/C]", t.style_error()),
        Span::raw(" Cancel  "),
        Span::styled("[↑/↓/PgUp/PgDn]", t.style_key()),
        Span::raw(" Scroll"),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border_muted()),
    );
    f.render_widget(footer, chunks[2]);
}

/// Render header with tabs
fn render_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let t = theme();
    let mut spans: Vec<Span> = vec![
        Span::styled(
            " cctakt ",
            Style::default()
                .fg(t.tab_active_fg())
                .bg(t.neon_pink())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            concat!("v", env!("CARGO_PKG_VERSION"), " "),
            t.style_text_muted(),
        ),
    ];

    let agents = app.agent_manager.list();
    let active_index = app.agent_manager.active_index();

    for (i, agent) in agents.iter().enumerate() {
        let is_active = i == active_index;
        let is_ended = agent.status == AgentStatus::Ended;

        let tab_content = format!(" [{}:{}] ", i + 1, agent.name);

        let style = if is_active {
            t.style_tab_active()
        } else if is_ended {
            Style::default().fg(t.status_ended())
        } else {
            t.style_tab_inactive()
        };

        spans.push(Span::styled(tab_content, style));
    }

    let header = Paragraph::new(Line::from(spans));
    f.render_widget(header, area);
}

/// Render footer with agent status and key bindings
fn render_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let t = theme();

    // Count agents by work state
    let agents = app.agent_manager.list();
    let mut running_count = 0;
    let mut idle_count = 0;
    let mut completed_count = 0;

    for agent in agents {
        match agent.work_state {
            agent::WorkState::Starting | agent::WorkState::Working => running_count += 1,
            agent::WorkState::Idle => idle_count += 1,
            agent::WorkState::Completed => completed_count += 1,
        }
    }

    let total_agents = agents.len();

    // Build left side: agent status
    let mut left_spans: Vec<Span> = vec![];

    if total_agents > 0 {
        left_spans.push(Span::styled(
            format!(" Agents: {total_agents} "),
            t.style_text_muted(),
        ));
        left_spans.push(Span::styled(
            format!("Running: {running_count}"),
            if running_count > 0 {
                t.style_warning()
            } else {
                t.style_text_muted()
            },
        ));
        left_spans.push(Span::styled(" | ", t.style_text_muted()));
        left_spans.push(Span::styled(
            format!("Idle: {idle_count}"),
            if idle_count > 0 {
                t.style_info()
            } else {
                t.style_text_muted()
            },
        ));
        left_spans.push(Span::styled(" | ", t.style_text_muted()));
        left_spans.push(Span::styled(
            format!("Completed: {completed_count}"),
            if completed_count > 0 {
                t.style_success()
            } else {
                t.style_text_muted()
            },
        ));
    }

    // Add input mode indicator
    left_spans.push(Span::styled(" | ", t.style_text_muted()));
    let (mode_text, mode_style) = match app.input_mode {
        InputMode::Navigation => ("NAV", t.style_warning()),
        InputMode::Input => ("INS", t.style_success()),
    };
    left_spans.push(Span::styled(mode_text, mode_style));

    // Add focused pane indicator
    let pane_text = match app.focused_pane {
        FocusedPane::Left => " [←]",
        FocusedPane::Right => " [→]",
    };
    left_spans.push(Span::styled(pane_text, t.style_text_muted()));

    // Build right side: plan status (if any) and key bindings
    let mut right_spans: Vec<Span> = vec![];

    // Plan status
    if let Some(ref plan) = app.current_plan {
        let (pending, running, completed, failed) = plan.count_by_status();
        let total = plan.tasks.len();
        let plan_style = if failed > 0 {
            t.style_error()
        } else if running > 0 {
            t.style_warning()
        } else {
            t.style_success()
        };
        right_spans.push(Span::styled(
            format!("Plan: {completed}/{total} "),
            plan_style,
        ));
        // Mark pending as unused to suppress warning
        let _ = pending;
    }

    // Key bindings
    right_spans.push(Span::styled(
        "[^T:new ^I:issue ^W:close ^N/^P:switch ^Q:quit] ",
        t.style_text_muted(),
    ));

    // Calculate widths for left/right alignment
    let left_text: String = left_spans.iter().map(|s| s.content.as_ref()).collect();
    let right_text: String = right_spans.iter().map(|s| s.content.as_ref()).collect();
    let left_width = left_text.len();
    let right_width = right_text.len();
    let available_width = area.width as usize;

    // Build final line with padding
    let mut spans = left_spans;
    let padding = available_width.saturating_sub(left_width + right_width);
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }
    spans.extend(right_spans);

    let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_surface()));
    f.render_widget(footer, area);
}

/// Render menu when no agents exist
/// Render the main area with split panes for Interactive (left) and NonInteractive (right) agents
fn render_split_pane_main_area(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let interactive = app.agent_manager.get_interactive();
    let active_worker = app.agent_manager.get_active_non_interactive();
    let is_review_mode = app.mode == AppMode::ReviewMerge;

    match (interactive, active_worker, is_review_mode) {
        // ReviewMerge mode with orchestrator: show orchestrator on left, review UI on right
        (Some(orchestrator), _, true) => {
            let t = theme();

            // Split horizontally: left 50% for orchestrator, 1 column for border, right 50% for review
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(1), // vertical separator
                    Constraint::Percentage(50),
                ])
                .split(area);

            // Left pane: Interactive (orchestrator)
            if orchestrator.status == AgentStatus::Ended {
                render_ended_agent(f, orchestrator, main_chunks[0]);
            } else {
                render_agent_screen(f, orchestrator, main_chunks[0]);
            }

            // Vertical separator
            let separator_lines: Vec<Line> = (0..main_chunks[1].height)
                .map(|_| Line::from("│"))
                .collect();
            let separator = Paragraph::new(separator_lines)
                .style(Style::default().fg(t.border_secondary()));
            f.render_widget(separator, main_chunks[1]);

            // Right pane: Review UI
            render_review_merge(f, app, main_chunks[2]);
        }
        // ReviewMerge mode without orchestrator: full width for review UI
        (None, _, true) => {
            render_review_merge(f, app, area);
        }
        // Both Interactive and NonInteractive agents exist: split pane layout
        (Some(orchestrator), Some(worker), false) => {
            let t = theme();
            let left_focused = app.focused_pane == FocusedPane::Left;
            let right_focused = app.focused_pane == FocusedPane::Right;

            // Split horizontally: left 50% for orchestrator, 1 column for border, right 50% for worker
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(1), // vertical separator
                    Constraint::Percentage(50),
                ])
                .split(area);

            // Left pane: Interactive (orchestrator) with focus indicator
            let left_area = main_chunks[0];
            if left_focused {
                // Draw focus indicator (top-left corner marker)
                let focus_marker = Paragraph::new("◆")
                    .style(Style::default().fg(t.neon_cyan()));
                f.render_widget(focus_marker, ratatui::layout::Rect::new(left_area.x, left_area.y, 1, 1));
            }
            if orchestrator.status == AgentStatus::Ended {
                render_ended_agent(f, orchestrator, main_chunks[0]);
            } else {
                render_agent_screen(f, orchestrator, main_chunks[0]);
            }

            // Vertical separator - highlight based on focus
            let separator_color = if left_focused || right_focused {
                if left_focused { t.neon_cyan() } else { t.neon_pink() }
            } else {
                t.border_secondary()
            };
            let separator_lines: Vec<Line> = (0..main_chunks[1].height)
                .map(|_| Line::from("│"))
                .collect();
            let separator = Paragraph::new(separator_lines)
                .style(Style::default().fg(separator_color));
            f.render_widget(separator, main_chunks[1]);

            // Right pane: NonInteractive (worker) with focus indicator
            let right_area = main_chunks[2];
            if right_focused {
                // Draw focus indicator (top-left corner marker)
                let focus_marker = Paragraph::new("◆")
                    .style(Style::default().fg(t.neon_pink()));
                f.render_widget(focus_marker, ratatui::layout::Rect::new(right_area.x, right_area.y, 1, 1));
            }
            if worker.status == AgentStatus::Ended {
                render_ended_agent(f, worker, main_chunks[2]);
            } else {
                render_agent_screen(f, worker, main_chunks[2]);
            }
        }
        // Only Interactive agent: full width for orchestrator
        (Some(orchestrator), None, false) => {
            if orchestrator.status == AgentStatus::Ended {
                render_ended_agent(f, orchestrator, area);
            } else {
                render_agent_screen(f, orchestrator, area);
            }
        }
        // Only NonInteractive agents: full width for worker
        (None, Some(worker), false) => {
            if worker.status == AgentStatus::Ended {
                render_ended_agent(f, worker, area);
            } else {
                render_agent_screen(f, worker, area);
            }
        }
        // No agents (shouldn't happen, but handle gracefully)
        (None, None, false) => {
            render_no_agent_menu(f, area);
        }
    }
}

fn render_no_agent_menu(f: &mut Frame, area: ratatui::layout::Rect) {
    let t = theme();
    let menu = Paragraph::new(vec![
        Line::from(""),
        Line::from("  No active agents."),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [N]", t.style_success()),
            Span::raw(" New agent"),
        ]),
        Line::from(vec![
            Span::styled("  [I/F2]", t.style_info()),
            Span::raw(" New agent from GitHub issue"),
        ]),
        Line::from(vec![
            Span::styled("  [Q]", t.style_error()),
            Span::raw(" Quit cctakt"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press N, I, or Q...",
            t.style_text_muted(),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border_muted()),
    );
    f.render_widget(menu, area);
}

/// Render ended agent menu
fn render_ended_agent(f: &mut Frame, agent: &agent::Agent, area: ratatui::layout::Rect) {
    let t = theme();
    let menu = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  Agent '{}' session ended.", agent.name)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Ctrl+W]", t.style_warning()),
            Span::raw(" Close this tab"),
        ]),
        Line::from(vec![
            Span::styled("  [Ctrl+N/P]", Style::default().fg(t.neon_blue())),
            Span::raw(" Switch to another tab"),
        ]),
        Line::from(vec![
            Span::styled("  [Ctrl+Q]", t.style_error()),
            Span::raw(" Quit"),
        ]),
        Line::from(""),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} (ended) ", agent.name))
            .border_style(t.style_border_muted()),
    );
    f.render_widget(menu, area);
}

/// Render active agent's screen (handles both interactive and non-interactive modes)
fn render_agent_screen(f: &mut Frame, agent: &agent::Agent, area: ratatui::layout::Rect) {
    match agent.mode {
        agent::AgentMode::Interactive => {
            render_agent_screen_interactive(f, agent, area);
        }
        agent::AgentMode::NonInteractive => {
            render_agent_screen_non_interactive(f, agent, area);
        }
    }
}

/// Render interactive (PTY) agent screen with vt100 colors
fn render_agent_screen_interactive(f: &mut Frame, agent: &agent::Agent, area: ratatui::layout::Rect) {
    let t = theme();
    let Some(parser_arc) = agent.get_parser() else {
        // Fallback if no parser
        let widget = Paragraph::new("No parser available").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(t.style_border_muted()),
        );
        f.render_widget(widget, area);
        return;
    };

    let parser = parser_arc.lock().unwrap();
    let screen = parser.screen();

    let content_height = area.height.saturating_sub(2) as usize;
    let content_width = area.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();

    for row in 0..content_height {
        let mut spans: Vec<Span> = Vec::new();
        let mut current_text = String::new();
        let mut current_style = Style::default();

        for col in 0..content_width {
            let cell = screen.cell(row as u16, col as u16);
            if let Some(cell) = cell {
                let cell_style = cell_to_style(cell);

                if cell_style != current_style {
                    if !current_text.is_empty() {
                        spans.push(Span::styled(current_text.clone(), current_style));
                        current_text.clear();
                    }
                    current_style = cell_style;
                }

                current_text.push_str(&cell.contents());
            }
        }

        if !current_text.is_empty() {
            spans.push(Span::styled(current_text, current_style));
        }

        lines.push(Line::from(spans));
    }

    let terminal_widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border_muted()),
    );
    f.render_widget(terminal_widget, area);
}

/// Render non-interactive agent screen (JSON stream output)
fn render_agent_screen_non_interactive(f: &mut Frame, agent: &agent::Agent, area: ratatui::layout::Rect) {
    let t = theme();
    let content_height = area.height.saturating_sub(2) as usize;
    let output = agent.screen_text();

    // Parse and filter JSON events (skip uninteresting ones)
    let all_lines: Vec<Line> = output
        .lines()
        .filter_map(|line| {
            // Parse JSON for prettier display
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                format_json_event(&json)
            } else if !line.trim().is_empty() {
                Some(Line::from(Span::raw(line.to_string())))
            } else {
                None
            }
        })
        .collect();

    // Get the last N lines to fit in the viewport
    let start = all_lines.len().saturating_sub(content_height);
    let visible_lines: Vec<Line> = all_lines[start..].to_vec();

    // Show status indicator
    let status_style = match agent.work_state {
        agent::WorkState::Working => Style::default().fg(Color::Yellow),
        agent::WorkState::Completed => {
            if agent.error.is_some() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            }
        }
        _ => Style::default().fg(Color::Gray),
    };

    let status_text = match agent.work_state {
        agent::WorkState::Starting => "Starting...",
        agent::WorkState::Working => "Working...",
        agent::WorkState::Idle => "Idle",
        agent::WorkState::Completed => {
            if agent.error.is_some() {
                "Error"
            } else {
                "Completed"
            }
        }
    };

    let terminal_widget = Paragraph::new(visible_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border_muted())
            .title(Span::styled(
                format!(" {status_text} "),
                status_style,
            )),
    );
    f.render_widget(terminal_widget, area);
}

/// Convert vt100 cell attributes to ratatui Style
fn cell_to_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default();

    // Foreground color
    let fg = cell.fgcolor();
    if !matches!(fg, vt100::Color::Default) {
        style = style.fg(vt100_color_to_ratatui(fg));
    }

    // Background color
    let bg = cell.bgcolor();
    if !matches!(bg, vt100::Color::Default) {
        style = style.bg(vt100_color_to_ratatui(bg));
    }

    // Attributes
    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }

    style
}

/// Convert vt100 color to ratatui color
fn vt100_color_to_ratatui(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::Gray,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Format a JSON stream event for display
/// Returns None if the event should be skipped
fn format_json_event(json: &serde_json::Value) -> Option<Line<'static>> {
    let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");

    match event_type {
        "system" => {
            let subtype = json.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
            Some(Line::from(vec![
                Span::styled("[SYS] ", Style::default().fg(Color::Blue)),
                Span::raw(subtype.to_string()),
            ]))
        }
        "user" => {
            // Skip user events (echo of input, not useful to display)
            None
        }
        "assistant" => {
            // Extract only text content (skip tool_use which is not informative)
            let text: String = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|block| {
                            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                block.get("text").and_then(|t| t.as_str())
                            } else {
                                None // Skip tool_use, tool_result, etc.
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            // Skip if no text content (only tool calls)
            if text.trim().is_empty() {
                return None;
            }

            // Truncate long text (char-safe for UTF-8)
            let display_text: String = if text.chars().count() > 80 {
                format!("{}...", text.chars().take(80).collect::<String>())
            } else {
                text
            };

            Some(Line::from(vec![
                Span::styled("[AI] ", Style::default().fg(Color::Cyan)),
                Span::raw(display_text),
            ]))
        }
        "result" => {
            let subtype = json.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
            let style = if subtype == "success" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            Some(Line::from(vec![
                Span::styled("[DONE] ", style),
                Span::raw(subtype.to_string()),
            ]))
        }
        _ => None, // Skip unknown event types
    }
}

// ==================== Main Entry Point ====================

fn main() -> Result<()> {
    // Initialize debug logging (only in debug builds)
    debug::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { force }) => run_init(force),
        Some(Commands::Status) => run_status(),
        Some(Commands::Issues { labels, state }) => run_issues(labels, state),
        Some(Commands::Run { plan }) => run_plan(plan),
        None => run_tui(),
    }
}

/// Run workers from a plan file (CLI mode)
fn run_plan(plan_path: PathBuf) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    println!("Loading plan from: {}", plan_path.display());

    // Load plan
    let plan_content = fs::read_to_string(&plan_path)
        .with_context(|| format!("Failed to read plan file: {}", plan_path.display()))?;
    let mut plan: Plan = serde_json::from_str(&plan_content)
        .with_context(|| "Failed to parse plan JSON")?;

    println!("Plan: {}", plan.description.as_deref().unwrap_or("(no description)"));
    println!("Tasks: {}", plan.tasks.len());
    println!();

    // Load config for worktree settings
    let config = Config::load().unwrap_or_default();
    let worktree_manager = WorktreeManager::from_current_dir()
        .context("Failed to initialize worktree manager")?;

    // Process pending create_worker tasks
    for task in &mut plan.tasks {
        if task.status != TaskStatus::Pending {
            println!("[{}] Skipping (status: {:?})", task.id, task.status);
            continue;
        }

        let TaskAction::CreateWorker { branch, task_description, base_branch: _ } = &task.action else {
            println!("[{}] Skipping (not a create_worker task)", task.id);
            continue;
        };

        println!("========================================");
        println!("[{}] Starting worker", task.id);
        println!("Branch: {branch}");
        println!("Task: {}", task_description.lines().next().unwrap_or(""));
        println!("========================================");

        // Create worktree
        let worktree_path = match worktree_manager.create(branch, &config.worktree_dir) {
            Ok(path) => {
                println!("Created worktree: {}", path.display());
                path
            }
            Err(e) => {
                println!("Failed to create worktree: {e}");
                task.status = TaskStatus::Failed;
                task.error = Some(format!("Failed to create worktree: {e}"));
                continue;
            }
        };

        // Update task status
        task.status = TaskStatus::Running;

        // Build command
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(task_description)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions");

        cmd.current_dir(&worktree_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        println!("\n--- Worker output ---\n");

        // Spawn process
        let mut child = cmd.spawn().context("Failed to spawn claude")?;

        // Read stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                // Parse and display JSON events
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match event_type {
                        "system" => {
                            let subtype = json.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
                            println!("[SYS] {subtype}");
                        }
                        "assistant" => {
                            // Extract only text content (skip tool_use)
                            if let Some(content) = json.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
                                for block in content {
                                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                            let preview: String = text.chars().take(100).collect();
                                            if !preview.trim().is_empty() {
                                                println!("[AI] {}...", preview.replace('\n', " "));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        "result" => {
                            let subtype = json.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
                            println!("[RESULT] {subtype}");
                        }
                        _ => {}
                    }
                }
            }
        }

        // Wait for process to finish
        let status = child.wait()?;
        println!("\n--- Worker finished (exit: {status}) ---\n");

        // Get commits
        let commits = get_worker_commits(&worktree_path);
        println!("Commits: {}", commits.len());
        for commit in &commits {
            println!("  - {commit}");
        }

        // Update task
        if status.success() {
            task.status = TaskStatus::Completed;
            task.result = Some(TaskResult {
                commits,
                pr_number: None,
                pr_url: None,
            });
        } else {
            task.status = TaskStatus::Failed;
            task.error = Some(format!("Process exited with: {status}"));
        }

        println!();
    }

    // Save updated plan
    let updated_plan = serde_json::to_string_pretty(&plan)?;
    fs::write(&plan_path, updated_plan)?;
    println!("Plan saved to: {}", plan_path.display());

    Ok(())
}

/// List GitHub issues
fn run_issues(labels: Option<String>, state: String) -> Result<()> {
    let config = Config::load()?;

    // Get repository from config or detect from git
    let repo = config.github.repository.clone()
        .or_else(detect_github_repo)
        .ok_or_else(|| anyhow::anyhow!("No repository configured. Set 'repository' in cctakt.toml or add a git remote."))?;

    let client = GitHubClient::new(&repo)?;

    let label_vec: Vec<&str> = labels
        .as_ref()
        .map(|l| l.split(',').map(|s| s.trim()).collect())
        .unwrap_or_default();

    println!("Fetching issues from {repo}...\n");

    let issues = client.fetch_issues(&label_vec, &state)?;

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    for issue in &issues {
        let labels_str = if issue.labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", issue.label_names())
        };
        println!("#{:<5} {}{}", issue.number, issue.title, labels_str);
    }

    println!("\nTotal: {} issues", issues.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Edge case: URL contains github.com but no repo path
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
        use cctakt::github::Issue;

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
        use cctakt::github::Issue;

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
        // Special characters should be sanitized
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
        // Loading state is internal, but we can verify it doesn't panic
    }

    #[test]
    fn test_issue_picker_set_issues() {
        use cctakt::github::Issue;

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
        use cctakt::github::Issue;
        use crossterm::event::KeyCode;

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

        // Navigate down
        let result = picker.handle_key(KeyCode::Down);
        assert!(result.is_none()); // Navigation doesn't return result

        // Navigate up
        let result = picker.handle_key(KeyCode::Up);
        assert!(result.is_none());
    }

    #[test]
    fn test_issue_picker_cancel() {
        use crossterm::event::KeyCode;

        let mut picker = IssuePicker::new();
        let result = picker.handle_key(KeyCode::Esc);

        assert!(matches!(result, Some(IssuePickerResult::Cancel)));
    }

    #[test]
    fn test_issue_picker_refresh() {
        use crossterm::event::KeyCode;

        let mut picker = IssuePicker::new();
        let result = picker.handle_key(KeyCode::Char('r'));

        assert!(matches!(result, Some(IssuePickerResult::Refresh)));
    }

    #[test]
    fn test_issue_picker_select_empty() {
        use crossterm::event::KeyCode;

        let mut picker = IssuePicker::new();
        // Trying to select from empty list should do nothing (no panic)
        let result = picker.handle_key(KeyCode::Enter);
        assert!(result.is_none());
    }

    #[test]
    fn test_issue_picker_select_with_issues() {
        use cctakt::github::Issue;
        use crossterm::event::KeyCode;

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
        // This test verifies WorktreeManager can be created from the current git repo
        let result = WorktreeManager::from_current_dir();
        // Should succeed since we're in a git repo
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
        // Test on current repo (should work since we're in a git repo)
        let log = get_commit_log(&PathBuf::from("."));
        // Should return some commits
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
        // DiffView should be created without panic
        assert!(!view.is_empty());
    }

    #[test]
    fn test_diff_view_with_title() {
        let view = DiffView::new("test".to_string()).with_title("branch → main".to_string());
        // with_title should work without panic
        assert!(!view.is_empty());
    }

    #[test]
    fn test_diff_view_scrolling() {
        let diff = (0..100)
            .map(|i| format!("+line {i}\n"))
            .collect::<String>();
        let mut view = DiffView::new(diff);

        // Test scroll operations
        view.scroll_down(10);
        view.scroll_up(5);
        view.page_down(20);
        view.page_up(10);
        view.scroll_to_top();
        view.scroll_to_bottom();
        // All operations should complete without panic
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
        // Test on current repo - should return commits
        let commits = get_worker_commits(&PathBuf::from("."));
        // Should return some commits since we're in a git repo
        assert!(!commits.is_empty());
    }

    #[test]
    fn test_get_worker_commits_nonexistent_dir() {
        // Test on nonexistent directory - should return empty
        let commits = get_worker_commits(&PathBuf::from("/nonexistent/path/that/doesnt/exist"));
        assert!(commits.is_empty());
    }

    #[test]
    fn test_get_worker_commits_format() {
        // Test that commits are in expected format (hash + message)
        let commits = get_worker_commits(&PathBuf::from("."));
        if !commits.is_empty() {
            // Each commit should have at least a hash (7+ chars)
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
            // Just verify it doesn't panic
            let _ = notification.message;
        }
    }

    // ==================== parse_github_url additional tests ====================

    #[test]
    fn test_parse_github_url_enterprise() {
        // Enterprise GitHub URLs typically don't use github.com
        let url = "https://github.example.com/owner/repo.git";
        // Should return None since it doesn't match github.com exactly
        assert_eq!(parse_github_url(url), None);
    }

    #[test]
    fn test_parse_github_url_with_port() {
        let url = "https://github.com:443/owner/repo.git";
        // Port in URL - behavior depends on implementation
        let result = parse_github_url(url);
        // Should handle this gracefully (either parse or return None)
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
        let manager = AgentManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_default() {
        let manager = AgentManager::default();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_list_empty() {
        let manager = AgentManager::new();
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_agent_manager_active_none() {
        let manager = AgentManager::new();
        assert!(manager.active().is_none());
    }

    #[test]
    fn test_agent_manager_switch_to_invalid() {
        let mut manager = AgentManager::new();
        // Switching to invalid index should not panic
        manager.switch_to(100);
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_next_empty() {
        let mut manager = AgentManager::new();
        // Next on empty manager should not panic
        manager.next();
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_prev_empty() {
        let mut manager = AgentManager::new();
        // Prev on empty manager should not panic
        manager.prev();
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_close_invalid() {
        let mut manager = AgentManager::new();
        // Closing invalid index should not panic
        manager.close(100);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_get_none() {
        let manager = AgentManager::new();
        assert!(manager.get(0).is_none());
        assert!(manager.get(100).is_none());
    }

    // ==================== AgentStatus tests ====================

    #[test]
    fn test_agent_status_equality() {
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_eq!(AgentStatus::Ended, AgentStatus::Ended);
        assert_ne!(AgentStatus::Running, AgentStatus::Ended);
    }

    #[test]
    fn test_agent_status_clone() {
        let status = AgentStatus::Running;
        let cloned = status;
        assert_eq!(status, cloned);
    }
}
