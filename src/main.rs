mod agent;

use agent::{AgentManager, AgentStatus};
use anyhow::{Context, Result};
use cctakt::{
    Config, GitHubClient, Issue, IssuePicker, IssuePickerResult, WorktreeManager,
    issue_picker::centered_rect, suggest_branch_name,
};
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::env;
use std::io;
use std::time::Duration;

/// Application mode
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    /// Normal mode - agent PTY view
    Normal,
    /// Issue picker mode
    IssuePicker,
}

/// Application state
struct App {
    agent_manager: AgentManager,
    should_quit: bool,
    content_rows: u16,
    content_cols: u16,
    /// Current application mode
    mode: AppMode,
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
            config,
            worktree_manager,
            github_client,
            issue_picker: IssuePicker::new(),
            agent_issues: Vec::new(),
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
            self.fetch_issues();
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
                    self.issue_picker.set_issues(issues);
                }
                Err(e) => {
                    self.issue_picker.set_error(Some(e.to_string()));
                }
            }
        }
    }

    /// Add a new agent from a selected issue
    fn add_agent_from_issue(&mut self, issue: Issue) -> Result<()> {
        let branch_name = suggest_branch_name(&issue, &self.config.branch_prefix);

        // Create worktree if available
        let working_dir = if let Some(ref wt_manager) = self.worktree_manager {
            match wt_manager.create(&branch_name, &self.config.worktree_dir) {
                Ok(path) => path,
                Err(_) => env::current_dir().context("Failed to get current directory")?,
            }
        } else {
            env::current_dir().context("Failed to get current directory")?
        };

        let name = format!("#{}", issue.number);
        self.agent_manager
            .add(name, working_dir, self.content_rows, self.content_cols)?;
        self.agent_issues.push(Some(issue));

        Ok(())
    }

    /// Add a new agent with the current directory
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

        self.agent_manager
            .add(display_name, working_dir, self.content_rows, self.content_cols)?;
        self.agent_issues.push(None);
        Ok(())
    }

    /// Close the active agent
    fn close_active_agent(&mut self) {
        let index = self.agent_manager.active_index();
        self.agent_manager.close(index);
        if index < self.agent_issues.len() {
            self.agent_issues.remove(index);
        }
    }

    /// Resize all agents
    fn resize(&mut self, cols: u16, rows: u16) {
        self.content_cols = cols;
        self.content_rows = rows;
        self.agent_manager.resize_all(cols, rows);
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

    // Parse GitHub URL formats:
    // - https://github.com/owner/repo.git
    // - git@github.com:owner/repo.git
    // - https://github.com/owner/repo
    if url.contains("github.com") {
        let repo = url
            .trim_end_matches(".git")
            .split("github.com")
            .last()?
            .trim_start_matches('/')
            .trim_start_matches(':')
            .to_string();
        Some(repo)
    } else {
        None
    }
}

fn main() -> Result<()> {
    // Load configuration
    let config = Config::load().unwrap_or_default();

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

        // Poll events (16ms ≈ 60fps)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match app.mode {
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
                                // No agents - show menu
                                match key.code {
                                    KeyCode::Char('n') | KeyCode::Char('N') => {
                                        let _ = app.add_agent();
                                    }
                                    KeyCode::Char('i') | KeyCode::Char('I') => {
                                        app.open_issue_picker();
                                    }
                                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                        app.should_quit = true;
                                    }
                                    _ => {}
                                }
                            } else {
                                // Handle keybindings
                                let handled = handle_keybinding(&mut app, key.modifiers, key.code);

                                if !handled {
                                    // Forward to active agent's PTY
                                    if let Some(agent) = app.agent_manager.active_mut() {
                                        if agent.status == AgentStatus::Running {
                                            match (key.modifiers, key.code) {
                                                (KeyModifiers::CONTROL, KeyCode::Char(c)) => {
                                                    let ctrl_char = (c as u8) & 0x1f;
                                                    agent.send_bytes(&[ctrl_char]);
                                                }
                                                (_, KeyCode::Enter) => agent.send_bytes(b"\r"),
                                                (_, KeyCode::Backspace) => agent.send_bytes(&[0x7f]),
                                                (_, KeyCode::Tab) => agent.send_bytes(b"\t"),
                                                (_, KeyCode::Esc) => agent.send_bytes(&[0x1b]),
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
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
            app.should_quit = true;
            true
        }
        // Ctrl+T: New agent
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            let _ = app.add_agent();
            true
        }
        // Ctrl+I: Open issue picker
        (KeyModifiers::CONTROL, KeyCode::Char('i')) => {
            app.open_issue_picker();
            true
        }
        // Ctrl+W: Close active agent
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            app.close_active_agent();
            true
        }
        // Ctrl+Tab or plain Tab (when no agent focused): Next tab
        // Note: Ctrl+Tab may not work in all terminals, so we use Ctrl+N as alternative
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            app.agent_manager.next();
            true
        }
        // Ctrl+P: Previous tab
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
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
        _ => false,
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header with tabs
            Constraint::Min(1),    // Main area
        ])
        .split(f.area());

    // Header with tabs
    render_header(f, app, chunks[0]);

    // Main area
    if app.agent_manager.is_empty() {
        render_no_agent_menu(f, chunks[1]);
    } else if let Some(agent) = app.agent_manager.active() {
        if agent.status == AgentStatus::Ended {
            render_ended_agent(f, agent, chunks[1]);
        } else {
            render_agent_screen(f, agent, chunks[1]);
        }
    }

    // Render issue picker overlay if active
    if app.mode == AppMode::IssuePicker {
        let popup_area = centered_rect(80, 70, f.area());
        app.issue_picker.render(f, popup_area);
    }
}

/// Render header with tabs
fn render_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut spans: Vec<Span> = vec![
        Span::styled(
            " cctakt ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let agents = app.agent_manager.list();
    let active_index = app.agent_manager.active_index();

    for (i, agent) in agents.iter().enumerate() {
        let is_active = i == active_index;
        let is_ended = agent.status == AgentStatus::Ended;

        let tab_content = format!(" [{}:{}] ", i + 1, agent.name);

        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_ended {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Gray)
        };

        spans.push(Span::styled(tab_content, style));
    }

    // Add help text on the right
    spans.push(Span::styled(
        " [^T:new ^I:issue ^W:close ^N/^P:switch ^Q:quit]",
        Style::default().fg(Color::DarkGray),
    ));

    let header = Paragraph::new(Line::from(spans));
    f.render_widget(header, area);
}

/// Render menu when no agents exist
fn render_no_agent_menu(f: &mut Frame, area: ratatui::layout::Rect) {
    let menu = Paragraph::new(vec![
        Line::from(""),
        Line::from("  No active agents."),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [N]", Style::default().fg(Color::Green)),
            Span::raw(" New agent"),
        ]),
        Line::from(vec![
            Span::styled("  [I]", Style::default().fg(Color::Cyan)),
            Span::raw(" New agent from GitHub issue"),
        ]),
        Line::from(vec![
            Span::styled("  [Q]", Style::default().fg(Color::Red)),
            Span::raw(" Quit cctakt"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press N, I, or Q...",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(menu, area);
}

/// Render ended agent menu
fn render_ended_agent(f: &mut Frame, agent: &agent::Agent, area: ratatui::layout::Rect) {
    let menu = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  Agent '{}' session ended.", agent.name)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Ctrl+T]", Style::default().fg(Color::Green)),
            Span::raw(" New agent"),
        ]),
        Line::from(vec![
            Span::styled("  [Ctrl+W]", Style::default().fg(Color::Yellow)),
            Span::raw(" Close this tab"),
        ]),
        Line::from(vec![
            Span::styled("  [Ctrl+N/P]", Style::default().fg(Color::Blue)),
            Span::raw(" Switch to another tab"),
        ]),
        Line::from(""),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} (ended) ", agent.name))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(menu, area);
}

/// Render active agent's vt100 screen
fn render_agent_screen(f: &mut Frame, agent: &agent::Agent, area: ratatui::layout::Rect) {
    let parser = agent.parser.lock().unwrap();
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
            .border_style(Style::default().fg(Color::DarkGray)),
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
