//! Status bar module for cctakt
//!
//! Provides a status bar widget that displays the status of all agents
//! in a compact format at the bottom of the screen.

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Status kind for an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatusKind {
    /// Agent is actively running
    Running,
    /// Agent is idle (waiting for input)
    Idle,
    /// Agent has finished its task
    Ended,
    /// Agent encountered an error
    Error,
}

impl AgentStatusKind {
    /// Get the status indicator symbol
    fn symbol(&self) -> &'static str {
        match self {
            AgentStatusKind::Running => "\u{25cf}", // ●
            AgentStatusKind::Idle => "\u{25cb}",    // ○
            AgentStatusKind::Ended => "\u{25cb}",   // ○
            AgentStatusKind::Error => "\u{2717}",   // ✗
        }
    }

    /// Get the color for this status
    fn color(&self) -> Color {
        match self {
            AgentStatusKind::Running => Theme::STATUS_RUNNING,
            AgentStatusKind::Idle => Theme::STATUS_IDLE,
            AgentStatusKind::Ended => Theme::STATUS_ENDED,
            AgentStatusKind::Error => Theme::STATUS_ERROR,
        }
    }

    /// Get the status text
    fn text(&self) -> &'static str {
        match self {
            AgentStatusKind::Running => "Running",
            AgentStatusKind::Idle => "Idle",
            AgentStatusKind::Ended => "Ended",
            AgentStatusKind::Error => "Error",
        }
    }
}

/// Information about an agent's status
#[derive(Debug, Clone)]
pub struct AgentStatusInfo {
    /// Agent ID
    pub id: usize,
    /// Agent name (usually branch name)
    pub name: String,
    /// Current status
    pub status: AgentStatusKind,
    /// Whether this agent is currently active/selected
    pub is_active: bool,
}

impl AgentStatusInfo {
    /// Create a new agent status info
    pub fn new(id: usize, name: impl Into<String>, status: AgentStatusKind, is_active: bool) -> Self {
        Self {
            id,
            name: name.into(),
            status,
            is_active,
        }
    }
}

/// Status bar widget for displaying agent statuses
///
/// # Example
/// ```ignore
/// let mut statusbar = StatusBar::new();
/// statusbar.update(vec![
///     AgentStatusInfo::new(1, "feat/auth", AgentStatusKind::Running, true),
///     AgentStatusInfo::new(2, "fix/api", AgentStatusKind::Idle, false),
/// ]);
/// statusbar.render(f, area);
/// ```
pub struct StatusBar {
    agents: Vec<AgentStatusInfo>,
}

impl StatusBar {
    /// Create a new empty status bar
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }

    /// Update the agent information
    pub fn update(&mut self, agents: Vec<AgentStatusInfo>) {
        self.agents = agents;
    }

    /// Get the number of agents
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Get agents by status
    pub fn agents_with_status(&self, status: AgentStatusKind) -> impl Iterator<Item = &AgentStatusInfo> {
        self.agents.iter().filter(move |a| a.status == status)
    }

    /// Render the status bar
    ///
    /// The status bar displays all agents in a single line with their status indicators.
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.agents.is_empty() {
            // Show empty state
            let empty = Paragraph::new(Line::from(vec![
                Span::styled(
                    "\u{2500}".repeat(3), // ───
                    Style::default().fg(Theme::BORDER_SECONDARY),
                ),
                Span::styled(
                    " No agents running ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled(
                    "\u{2500}".repeat(area.width.saturating_sub(25) as usize),
                    Style::default().fg(Theme::BORDER_SECONDARY),
                ),
            ]));
            f.render_widget(empty, area);
            return;
        }

        let mut spans: Vec<Span> = Vec::new();

        // Separator at start
        spans.push(Span::styled(
            "\u{2500} ", // ─
            Style::default().fg(Theme::BORDER_SECONDARY),
        ));

        for (idx, agent) in self.agents.iter().enumerate() {
            if idx > 0 {
                spans.push(Span::styled(
                    "  ",
                    Style::default().fg(Theme::BORDER_SECONDARY),
                ));
            }

            // Agent number with bracket
            let bracket_style = if agent.is_active {
                Style::default()
                    .fg(Theme::NEON_CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            };

            spans.push(Span::styled("[", bracket_style));
            spans.push(Span::styled(agent.id.to_string(), bracket_style));
            spans.push(Span::styled("] ", bracket_style));

            // Agent name
            let name_style = if agent.is_active {
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_SECONDARY)
            };

            // Truncate name if too long
            let max_name_len = 15;
            let display_name = if agent.name.len() > max_name_len {
                format!("{}...", &agent.name[..max_name_len - 3])
            } else {
                agent.name.clone()
            };
            spans.push(Span::styled(display_name, name_style));

            spans.push(Span::raw(" "));

            // Status indicator
            spans.push(Span::styled(
                agent.status.symbol(),
                Style::default().fg(agent.status.color()),
            ));

            spans.push(Span::raw(" "));

            // Status text
            spans.push(Span::styled(
                agent.status.text(),
                Style::default().fg(agent.status.color()),
            ));
        }

        // Fill remaining space with separator
        let content_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let remaining = area.width.saturating_sub(content_len as u16 + 1);
        if remaining > 0 {
            spans.push(Span::styled(
                format!(" {}", "\u{2500}".repeat(remaining as usize)),
                Style::default().fg(Theme::BORDER_SECONDARY),
            ));
        }

        let statusbar = Paragraph::new(Line::from(spans));
        f.render_widget(statusbar, area);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn test_statusbar_new() {
        let statusbar = StatusBar::new();
        assert_eq!(statusbar.agent_count(), 0);
    }

    #[test]
    fn test_statusbar_update() {
        let mut statusbar = StatusBar::new();
        statusbar.update(vec![
            AgentStatusInfo::new(1, "feat/auth", AgentStatusKind::Running, true),
            AgentStatusInfo::new(2, "fix/api", AgentStatusKind::Idle, false),
        ]);
        assert_eq!(statusbar.agent_count(), 2);
    }

    #[test]
    fn test_agent_status_info_new() {
        let info = AgentStatusInfo::new(1, "test-branch", AgentStatusKind::Running, true);
        assert_eq!(info.id, 1);
        assert_eq!(info.name, "test-branch");
        assert_eq!(info.status, AgentStatusKind::Running);
        assert!(info.is_active);
    }

    #[test]
    fn test_status_kind_symbol() {
        assert_eq!(AgentStatusKind::Running.symbol(), "\u{25cf}");
        assert_eq!(AgentStatusKind::Idle.symbol(), "\u{25cb}");
        assert_eq!(AgentStatusKind::Ended.symbol(), "\u{25cb}");
        assert_eq!(AgentStatusKind::Error.symbol(), "\u{2717}");
    }

    #[test]
    fn test_status_kind_color() {
        assert_eq!(AgentStatusKind::Running.color(), Theme::STATUS_RUNNING);
        assert_eq!(AgentStatusKind::Idle.color(), Theme::STATUS_IDLE);
        assert_eq!(AgentStatusKind::Ended.color(), Theme::STATUS_ENDED);
        assert_eq!(AgentStatusKind::Error.color(), Theme::STATUS_ERROR);
    }

    #[test]
    fn test_status_kind_text() {
        assert_eq!(AgentStatusKind::Running.text(), "Running");
        assert_eq!(AgentStatusKind::Idle.text(), "Idle");
        assert_eq!(AgentStatusKind::Ended.text(), "Ended");
        assert_eq!(AgentStatusKind::Error.text(), "Error");
    }

    #[test]
    fn test_agents_with_status() {
        let mut statusbar = StatusBar::new();
        statusbar.update(vec![
            AgentStatusInfo::new(1, "agent1", AgentStatusKind::Running, true),
            AgentStatusInfo::new(2, "agent2", AgentStatusKind::Idle, false),
            AgentStatusInfo::new(3, "agent3", AgentStatusKind::Running, false),
            AgentStatusInfo::new(4, "agent4", AgentStatusKind::Ended, false),
        ]);

        let running: Vec<_> = statusbar.agents_with_status(AgentStatusKind::Running).collect();
        assert_eq!(running.len(), 2);

        let idle: Vec<_> = statusbar.agents_with_status(AgentStatusKind::Idle).collect();
        assert_eq!(idle.len(), 1);

        let ended: Vec<_> = statusbar.agents_with_status(AgentStatusKind::Ended).collect();
        assert_eq!(ended.len(), 1);
    }

    #[test]
    fn test_statusbar_default() {
        let statusbar = StatusBar::default();
        assert_eq!(statusbar.agent_count(), 0);
    }
}
