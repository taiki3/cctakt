//! Agent module - manages individual Claude Code sessions

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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

/// Represents a single Claude Code session
pub struct Agent {
    pub id: usize,
    pub name: String,
    pub working_dir: PathBuf,
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub pty_writer: Option<Box<dyn Write + Send>>,
    pub pty_master: Option<Box<dyn MasterPty + Send>>,
    pub status: AgentStatus,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    _output_thread: Option<JoinHandle<()>>,
    /// Last time output was received
    last_activity: Arc<Mutex<Instant>>,
    /// Current work state
    pub work_state: WorkState,
    /// Whether task has been sent to this agent
    pub task_sent: bool,
}

impl Agent {
    /// Create a new agent and spawn Claude Code
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

        // Spawn Claude Code in the specified working directory
        let mut cmd = CommandBuilder::new("claude");
        cmd.arg("--dangerously-skip-permissions");
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
                        // Update activity timestamp
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
            parser,
            pty_writer,
            pty_master,
            status: AgentStatus::Running,
            child: Some(child),
            _output_thread: Some(output_thread),
            last_activity,
            work_state: WorkState::Starting,
            task_sent: false,
        })
    }

    /// Send bytes to the PTY
    pub fn send_bytes(&mut self, bytes: &[u8]) {
        if let Some(ref mut writer) = self.pty_writer {
            let _ = writer.write_all(bytes);
            let _ = writer.flush();
        }
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) {
        // Resize vt100 parser
        {
            let mut parser = self.parser.lock().unwrap();
            parser.set_size(rows, cols);
        }
        // Resize PTY
        if let Some(ref master) = self.pty_master {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    /// Check if the process has ended
    pub fn check_status(&mut self) -> AgentStatus {
        if self.status == AgentStatus::Running {
            if let Some(ref mut child) = self.child {
                if let Ok(Some(_)) = child.try_wait() {
                    self.status = AgentStatus::Ended;
                }
            }
        }
        self.status
    }

    /// Get time since last activity
    pub fn idle_duration(&self) -> Duration {
        if let Ok(ts) = self.last_activity.lock() {
            ts.elapsed()
        } else {
            Duration::ZERO
        }
    }

    /// Get the current screen content as text
    pub fn screen_text(&self) -> String {
        if let Ok(parser) = self.parser.lock() {
            let screen = parser.screen();
            let mut text = String::new();
            for row in 0..screen.size().0 {
                let line = screen.contents_between(row, 0, row, screen.size().1);
                text.push_str(&line);
                text.push('\n');
            }
            text
        } else {
            String::new()
        }
    }

    /// Check work state and update based on activity
    /// Returns true if state changed to Completed
    pub fn update_work_state(&mut self, idle_threshold: Duration) -> bool {
        let old_state = self.work_state;

        // Don't change state if task hasn't been sent yet
        if !self.task_sent {
            return false;
        }

        let idle_time = self.idle_duration();
        let screen = self.screen_text();

        // Only check for definite completion: prompt waiting + commit detected
        let is_at_prompt = self.detect_prompt_waiting(&screen);
        let has_committed = self.detect_commit_success(&screen);

        match self.work_state {
            WorkState::Starting => {
                self.work_state = WorkState::Working;
            }
            WorkState::Working | WorkState::Idle => {
                // Only mark as completed when:
                // 1. Screen shows we're at a prompt (waiting for input)
                // 2. There's evidence of a commit
                // 3. Been idle for threshold time (to ensure stable state)
                if is_at_prompt && has_committed && idle_time >= idle_threshold {
                    self.work_state = WorkState::Completed;
                } else if idle_time >= idle_threshold {
                    self.work_state = WorkState::Idle;
                } else if idle_time < Duration::from_millis(500) {
                    self.work_state = WorkState::Working;
                }
            }
            WorkState::Completed => {
                // Stay completed
            }
        }

        old_state != WorkState::Completed && self.work_state == WorkState::Completed
    }

    /// Detect if screen shows a prompt waiting for input
    fn detect_prompt_waiting(&self, screen: &str) -> bool {
        // Get last few non-empty lines
        let lines: Vec<&str> = screen
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        if let Some(last_line) = lines.last() {
            let trimmed = last_line.trim();
            // Claude Code prompt patterns
            if trimmed.ends_with('>')
                || trimmed.ends_with('$')
                || trimmed.contains("❯")
                || trimmed.ends_with(':')  // "Enter your message:"
            {
                return true;
            }
        }
        false
    }

    /// Detect if there's been a successful commit
    fn detect_commit_success(&self, screen: &str) -> bool {
        let lower = screen.to_lowercase();

        // Commit success patterns
        let patterns = [
            "successfully committed",
            "changes committed",
            "created commit",
            "commit created",
            "[main",      // git output: [main abc1234]
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

    /// Add a new agent
    pub fn add(&mut self, name: String, working_dir: PathBuf, rows: u16, cols: u16) -> Result<usize> {
        let id = self.next_id;
        let agent = Agent::spawn(id, name, working_dir, rows, cols)?;
        self.agents.push(agent);
        self.next_id += 1;

        // Switch to the new agent
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

            // Adjust active index
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
