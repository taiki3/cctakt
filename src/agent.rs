//! Agent module - manages individual Claude Code sessions
//!
//! Supports two modes:
//! - Interactive (PTY): For orchestrator sessions with human interaction
//! - Non-interactive (stream-json): For worker sessions with deterministic completion

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cctakt::stream_parser::{StreamEvent, StreamParser};

/// Agent execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    /// Interactive mode using PTY (for orchestrator)
    Interactive,
    /// Non-interactive mode using stream-json (for workers)
    NonInteractive,
}

/// Status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Ended,
}

/// Work state of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    /// Agent is starting up
    Starting,
    /// Agent is actively working (receiving output)
    Working,
    /// Agent is idle (no output for a while, waiting for input)
    Idle,
    /// Agent has completed work (detected completion patterns)
    Completed,
}

/// Internal state for interactive (PTY) mode
struct InteractiveState {
    parser: Arc<Mutex<vt100::Parser>>,
    pty_writer: Option<Box<dyn Write + Send>>,
    pty_master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    last_activity: Arc<Mutex<Instant>>,
}

/// Internal state for non-interactive mode
struct NonInteractiveState {
    parser: Arc<Mutex<StreamParser>>,
    child: Option<Child>,
    output_buffer: Arc<Mutex<String>>,
}

/// Represents a single Claude Code session
pub struct Agent {
    pub id: usize,
    pub name: String,
    pub working_dir: PathBuf,
    pub status: AgentStatus,
    pub work_state: WorkState,
    pub task_sent: bool,
    pub mode: AgentMode,
    /// Error message if failed (non-interactive only)
    pub error: Option<String>,
    /// Result text if completed (non-interactive only)
    pub result: Option<String>,
    /// Interactive state (PTY)
    interactive: Option<InteractiveState>,
    /// Non-interactive state (stream-json)
    non_interactive: Option<NonInteractiveState>,
    /// Output reading thread handle
    _output_thread: Option<JoinHandle<()>>,
}

impl Agent {
    /// Create a new agent in interactive (PTY) mode
    pub fn spawn(id: usize, name: String, working_dir: PathBuf, rows: u16, cols: u16) -> Result<Self> {
        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000)));

        // Setup PTY
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open pty")?;

        // Spawn Claude Code in the specified working directory (orchestrator mode)
        let mut cmd = CommandBuilder::new("claude");
        cmd.arg("--dangerously-skip-permissions");
        cmd.arg("--append-system-prompt");
        cmd.arg(
            "You are the ORCHESTRATOR. Your job is to coordinate work, NOT implement it yourself.\n\
            RULES:\n\
            1. DO NOT write implementation code - Workers do that\n\
            2. DO NOT edit source files (*.rs, *.ts, *.py, etc.)\n\
            3. Your ONLY job is to create plans in .cctakt/plan.json\n\
            4. Use /orchestrator command to see plan format\n\
            When user requests a feature, write a plan.json with create_worker tasks."
        );
        cmd.cwd(&working_dir);

        let child = pair.slave.spawn_command(cmd).context("Failed to spawn claude")?;
        drop(pair.slave);

        // Setup PTY reader/writer
        let reader = pair.master.try_clone_reader().context("Failed to clone reader")?;
        let pty_writer = Some(pair.master.take_writer().context("Failed to take writer")?);
        let pty_master = Some(pair.master);

        // Activity tracking
        let last_activity = Arc::new(Mutex::new(Instant::now()));
        let last_activity_clone = Arc::clone(&last_activity);

        // Spawn output reading thread
        let parser_clone = Arc::clone(&parser);
        let output_thread = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut ts) = last_activity_clone.lock() {
                            *ts = Instant::now();
                        }
                        let mut parser = parser_clone.lock().unwrap();
                        parser.process(&buf[..n]);
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            id,
            name,
            working_dir,
            status: AgentStatus::Running,
            work_state: WorkState::Starting,
            task_sent: false,
            mode: AgentMode::Interactive,
            error: None,
            result: None,
            interactive: Some(InteractiveState {
                parser,
                pty_writer,
                pty_master,
                child: Some(child),
                last_activity,
            }),
            non_interactive: None,
            _output_thread: Some(output_thread),
        })
    }

    /// Create a new agent in non-interactive mode
    pub fn spawn_non_interactive(
        id: usize,
        name: String,
        working_dir: PathBuf,
        task_description: &str,
        max_turns: Option<u32>,
    ) -> Result<Self> {
        let parser = Arc::new(Mutex::new(StreamParser::new()));
        let output_buffer = Arc::new(Mutex::new(String::new()));

        // Build command
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(task_description)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions");

        if let Some(turns) = max_turns {
            cmd.arg("--max-turns").arg(turns.to_string());
        }

        cmd.current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn claude process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        // Spawn output reading thread
        let parser_clone = Arc::clone(&parser);
        let output_buffer_clone = Arc::clone(&output_buffer);
        let output_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Ok(mut p) = parser_clone.lock() {
                        p.feed(&format!("{}\n", line));
                    }
                    if let Ok(mut buf) = output_buffer_clone.lock() {
                        buf.push_str(&line);
                        buf.push('\n');
                    }
                }
            }
        });

        // Spawn stderr reading thread
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[agent stderr] {}", line);
                }
            }
        });

        Ok(Self {
            id,
            name,
            working_dir,
            status: AgentStatus::Running,
            work_state: WorkState::Working,
            task_sent: true,
            mode: AgentMode::NonInteractive,
            error: None,
            result: None,
            interactive: None,
            non_interactive: Some(NonInteractiveState {
                parser,
                child: Some(child),
                output_buffer,
            }),
            _output_thread: Some(output_thread),
        })
    }

    /// Send bytes to the agent (interactive mode only)
    pub fn send_bytes(&mut self, bytes: &[u8]) {
        if let Some(ref mut state) = self.interactive {
            if let Some(ref mut writer) = state.pty_writer {
                let _ = writer.write_all(bytes);
                let _ = writer.flush();
            }
        }
    }

    /// Resize the PTY (interactive mode only)
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if let Some(ref mut state) = self.interactive {
            {
                let mut parser = state.parser.lock().unwrap();
                parser.set_size(rows, cols);
            }
            if let Some(ref master) = state.pty_master {
                let _ = master.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
        }
    }

    /// Check if the process has ended
    pub fn check_status(&mut self) -> AgentStatus {
        if self.status == AgentStatus::Running {
            match self.mode {
                AgentMode::Interactive => {
                    if let Some(ref mut state) = self.interactive {
                        if let Some(ref mut child) = state.child {
                            if let Ok(Some(_)) = child.try_wait() {
                                self.status = AgentStatus::Ended;
                            }
                        }
                    }
                }
                AgentMode::NonInteractive => {
                    if let Some(ref mut state) = self.non_interactive {
                        if let Some(ref mut child) = state.child {
                            if let Ok(Some(exit_status)) = child.try_wait() {
                                self.status = AgentStatus::Ended;
                                if let Ok(p) = state.parser.lock() {
                                    if p.completed {
                                        self.work_state = WorkState::Completed;
                                        self.result = p.result.clone();
                                        self.error = p.error.clone();
                                    }
                                }
                                if !exit_status.success() && self.error.is_none() {
                                    self.error = Some(format!("Process exited with status: {}", exit_status));
                                }
                            }
                        }
                    }
                }
            }
        }
        self.status
    }

    /// Get time since last activity (interactive mode)
    pub fn idle_duration(&self) -> Duration {
        if let Some(ref state) = self.interactive {
            if let Ok(ts) = state.last_activity.lock() {
                return ts.elapsed();
            }
        }
        Duration::ZERO
    }

    /// Get the current screen content as text
    pub fn screen_text(&self) -> String {
        match self.mode {
            AgentMode::Interactive => {
                if let Some(ref state) = self.interactive {
                    if let Ok(parser) = state.parser.lock() {
                        let screen = parser.screen();
                        let mut text = String::new();
                        for row in 0..screen.size().0 {
                            let line = screen.contents_between(row, 0, row, screen.size().1);
                            text.push_str(&line);
                            text.push('\n');
                        }
                        return text;
                    }
                }
                String::new()
            }
            AgentMode::NonInteractive => {
                if let Some(ref state) = self.non_interactive {
                    if let Ok(buf) = state.output_buffer.lock() {
                        return buf.clone();
                    }
                }
                String::new()
            }
        }
    }

    /// Check work state and update based on activity
    /// Returns true if state changed to Completed
    pub fn update_work_state(&mut self, idle_threshold: Duration) -> bool {
        let old_state = self.work_state;

        match self.mode {
            AgentMode::Interactive => {
                if !self.task_sent {
                    return false;
                }

                let idle_time = self.idle_duration();
                let screen = self.screen_text();
                let is_at_prompt = self.detect_prompt_waiting(&screen);
                let has_committed = self.detect_commit_success(&screen);

                match self.work_state {
                    WorkState::Starting => {
                        self.work_state = WorkState::Working;
                    }
                    WorkState::Working | WorkState::Idle => {
                        if is_at_prompt && has_committed && idle_time >= idle_threshold {
                            self.work_state = WorkState::Completed;
                        } else if idle_time >= idle_threshold {
                            self.work_state = WorkState::Idle;
                        } else if idle_time < Duration::from_millis(500) {
                            self.work_state = WorkState::Working;
                        }
                    }
                    WorkState::Completed => {}
                }
            }
            AgentMode::NonInteractive => {
                if let Some(ref state) = self.non_interactive {
                    if let Ok(p) = state.parser.lock() {
                        if p.completed {
                            self.work_state = WorkState::Completed;
                            self.result = p.result.clone();
                            self.error = p.error.clone();
                        }
                    }
                }
            }
        }

        old_state != WorkState::Completed && self.work_state == WorkState::Completed
    }

    /// Detect if screen shows a prompt waiting for input (interactive mode)
    fn detect_prompt_waiting(&self, screen: &str) -> bool {
        let lines: Vec<&str> = screen
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        if let Some(last_line) = lines.last() {
            let trimmed = last_line.trim();
            if trimmed.ends_with('>')
                || trimmed.ends_with('$')
                || trimmed.contains("❯")
                || trimmed.ends_with(':')
            {
                return true;
            }
        }
        false
    }

    /// Detect if there's been a successful commit (interactive mode)
    fn detect_commit_success(&self, screen: &str) -> bool {
        let lower = screen.to_lowercase();
        let patterns = [
            "successfully committed",
            "changes committed",
            "created commit",
            "commit created",
            "[main",
            "[master",
            "files changed",
            "insertions(+)",
            "deletions(-)",
        ];

        for pattern in patterns {
            if lower.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Get the vt100 parser (interactive mode only)
    pub fn get_parser(&self) -> Option<&Arc<Mutex<vt100::Parser>>> {
        self.interactive.as_ref().map(|s| &s.parser)
    }

    /// Get stream events (non-interactive mode only)
    pub fn events(&self) -> Vec<StreamEvent> {
        if let Some(ref state) = self.non_interactive {
            if let Ok(p) = state.parser.lock() {
                return p.events.clone();
            }
        }
        Vec::new()
    }

    /// Check if completed successfully (non-interactive mode)
    pub fn is_success(&self) -> bool {
        self.work_state == WorkState::Completed && self.error.is_none()
    }

    /// Check if completed with error (non-interactive mode)
    pub fn is_error(&self) -> bool {
        self.work_state == WorkState::Completed && self.error.is_some()
    }
}

/// Manages multiple agents
pub struct AgentManager {
    agents: Vec<Agent>,
    active_index: usize,
    next_id: usize,
}

impl AgentManager {
    /// Create a new empty AgentManager
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            active_index: 0,
            next_id: 1,
        }
    }

    /// Add a new agent in interactive (PTY) mode
    pub fn add(&mut self, name: String, working_dir: PathBuf, rows: u16, cols: u16) -> Result<usize> {
        let id = self.next_id;
        let agent = Agent::spawn(id, name, working_dir, rows, cols)?;
        self.agents.push(agent);
        self.next_id += 1;
        self.active_index = self.agents.len() - 1;
        Ok(id)
    }

    /// Add a new agent in non-interactive mode
    pub fn add_non_interactive(
        &mut self,
        name: String,
        working_dir: PathBuf,
        task_description: &str,
        max_turns: Option<u32>,
    ) -> Result<usize> {
        let id = self.next_id;
        let agent = Agent::spawn_non_interactive(id, name, working_dir, task_description, max_turns)?;
        self.agents.push(agent);
        self.next_id += 1;
        self.active_index = self.agents.len() - 1;
        Ok(id)
    }

    /// Get the active agent
    pub fn active(&self) -> Option<&Agent> {
        self.agents.get(self.active_index)
    }

    /// Get the active agent mutably
    pub fn active_mut(&mut self) -> Option<&mut Agent> {
        self.agents.get_mut(self.active_index)
    }

    /// Get an agent by index
    pub fn get(&self, index: usize) -> Option<&Agent> {
        self.agents.get(index)
    }

    /// Get an agent by index mutably
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Agent> {
        self.agents.get_mut(index)
    }

    /// Switch to a specific agent by index
    pub fn switch_to(&mut self, index: usize) {
        if index < self.agents.len() {
            self.active_index = index;
        }
    }

    /// Switch to the next agent
    pub fn next(&mut self) {
        if !self.agents.is_empty() {
            self.active_index = (self.active_index + 1) % self.agents.len();
        }
    }

    /// Switch to the previous agent
    pub fn prev(&mut self) {
        if !self.agents.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.agents.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /// Close an agent by index
    pub fn close(&mut self, index: usize) {
        if index < self.agents.len() {
            self.agents.remove(index);
            if self.agents.is_empty() {
                self.active_index = 0;
            } else if self.active_index >= self.agents.len() {
                self.active_index = self.agents.len() - 1;
            } else if index < self.active_index {
                self.active_index -= 1;
            }
        }
    }

    /// Get all agents
    pub fn list(&self) -> &[Agent] {
        &self.agents
    }

    /// Get the current active index
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Check if there are any agents
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Check status of all agents
    pub fn check_all_status(&mut self) {
        for agent in &mut self.agents {
            agent.check_status();
        }
    }

    /// Resize all agents' PTYs
    pub fn resize_all(&mut self, cols: u16, rows: u16) {
        for agent in &mut self.agents {
            agent.resize(cols, rows);
        }
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AgentMode tests ====================

    #[test]
    fn test_agent_mode_values() {
        assert_eq!(AgentMode::Interactive, AgentMode::Interactive);
        assert_eq!(AgentMode::NonInteractive, AgentMode::NonInteractive);
        assert_ne!(AgentMode::Interactive, AgentMode::NonInteractive);
    }

    // ==================== AgentStatus tests ====================

    #[test]
    fn test_agent_status_running() {
        let status = AgentStatus::Running;
        assert_eq!(status, AgentStatus::Running);
    }

    #[test]
    fn test_agent_status_ended() {
        let status = AgentStatus::Ended;
        assert_eq!(status, AgentStatus::Ended);
    }

    #[test]
    fn test_agent_status_copy() {
        let status = AgentStatus::Running;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_agent_status_clone() {
        let status = AgentStatus::Ended;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_agent_status_debug() {
        let status = AgentStatus::Running;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Running"));
    }

    // ==================== WorkState tests ====================

    #[test]
    fn test_work_state_values() {
        assert_eq!(WorkState::Starting, WorkState::Starting);
        assert_eq!(WorkState::Working, WorkState::Working);
        assert_eq!(WorkState::Idle, WorkState::Idle);
        assert_eq!(WorkState::Completed, WorkState::Completed);
    }

    #[test]
    fn test_work_state_inequality() {
        assert_ne!(WorkState::Starting, WorkState::Working);
        assert_ne!(WorkState::Working, WorkState::Completed);
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
    fn test_agent_manager_active_empty() {
        let manager = AgentManager::new();
        assert!(manager.active().is_none());
    }

    #[test]
    fn test_agent_manager_active_mut_empty() {
        let mut manager = AgentManager::new();
        assert!(manager.active_mut().is_none());
    }

    #[test]
    fn test_agent_manager_get_empty() {
        let manager = AgentManager::new();
        assert!(manager.get(0).is_none());
        assert!(manager.get(100).is_none());
    }

    #[test]
    fn test_agent_manager_get_mut_empty() {
        let mut manager = AgentManager::new();
        assert!(manager.get_mut(0).is_none());
        assert!(manager.get_mut(100).is_none());
    }

    #[test]
    fn test_agent_manager_switch_to_empty() {
        let mut manager = AgentManager::new();
        manager.switch_to(0);
        assert_eq!(manager.active_index(), 0);
        manager.switch_to(100);
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_next_empty() {
        let mut manager = AgentManager::new();
        manager.next();
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_prev_empty() {
        let mut manager = AgentManager::new();
        manager.prev();
        assert_eq!(manager.active_index(), 0);
    }

    #[test]
    fn test_agent_manager_close_empty() {
        let mut manager = AgentManager::new();
        manager.close(0);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_close_invalid_index() {
        let mut manager = AgentManager::new();
        manager.close(100);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_check_all_status_empty() {
        let mut manager = AgentManager::new();
        manager.check_all_status();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_agent_manager_resize_all_empty() {
        let mut manager = AgentManager::new();
        manager.resize_all(80, 24);
        assert!(manager.is_empty());
    }
}
