//! Agent module - manages individual Claude Code sessions

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Ended,
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
        cmd.cwd(&working_dir);

        let child = pair.slave.spawn_command(cmd).context("Failed to spawn claude")?;
        drop(pair.slave);

        // Setup PTY reader/writer
        let reader = pair.master.try_clone_reader().context("Failed to clone reader")?;
        let pty_writer = Some(pair.master.take_writer().context("Failed to take writer")?);
        let pty_master = Some(pair.master);

        // Spawn output reading thread
        let parser_clone = Arc::clone(&parser);
        let output_thread = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
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
