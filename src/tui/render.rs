//! TUI rendering functions

use crate::agent::{Agent, AgentMode, AgentStatus, WorkState};
use crate::app::{App, AppMode, FocusedPane, InputMode};
use cctakt::{available_themes, current_theme_id, issue_picker::centered_rect, theme};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Main UI rendering function
pub fn ui(f: &mut Frame, app: &mut App) {
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
pub fn render_notifications(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
pub fn render_plan_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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

/// Render theme picker modal
pub fn render_theme_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let t = theme();
    let themes = available_themes();
    let current_theme_id_str = current_theme_id().id();

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
    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, (id, name, description)) in themes.iter().enumerate() {
        let is_selected = i == app.theme_picker_index;
        let is_current = *id == current_theme_id_str;

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
pub fn render_review_merge(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
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
            Span::styled(
                " Review Merge: ",
                Style::default()
                    .fg(t.neon_cyan())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&state.branch, Style::default().fg(t.neon_yellow())),
            Span::raw(" → "),
            Span::styled("main", Style::default().fg(t.success())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Stats: "),
            Span::styled(format!("{} files", state.files_changed), t.style_text()),
            Span::raw(", "),
            Span::styled(
                format!("+{}", state.insertions),
                Style::default().fg(t.success()),
            ),
            Span::raw(" / "),
            Span::styled(
                format!("-{}", state.deletions),
                Style::default().fg(t.error()),
            ),
        ]),
    ];

    // Show conflicts warning if any
    if !state.conflicts.is_empty() {
        header_lines.push(Line::from(vec![
            Span::styled(" ⚠ Potential conflicts: ", t.style_warning()),
            Span::styled(state.conflicts.join(", "), t.style_warning()),
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
pub fn render_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
pub fn render_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let t = theme();

    // Count agents by work state
    let agents = app.agent_manager.list();
    let mut running_count = 0;
    let mut idle_count = 0;
    let mut completed_count = 0;

    for agent in agents {
        match agent.work_state {
            WorkState::Starting | WorkState::Working => running_count += 1,
            WorkState::Idle => idle_count += 1,
            WorkState::Completed => completed_count += 1,
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
        InputMode::Navigation => ("NAV(i:入力)", t.style_warning()),
        InputMode::Input => ("INS(Esc:移動)", t.style_success()),
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
pub fn render_split_pane_main_area(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
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

            // Left pane: Interactive (orchestrator) - no focus color in review mode
            if orchestrator.status == AgentStatus::Ended {
                render_ended_agent(f, orchestrator, main_chunks[0], None);
            } else {
                render_agent_screen(f, orchestrator, main_chunks[0], None);
            }

            // Vertical separator
            let separator_lines: Vec<Line> = (0..main_chunks[1].height)
                .map(|_| Line::from("│"))
                .collect();
            let separator =
                Paragraph::new(separator_lines).style(Style::default().fg(t.border_secondary()));
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

            // Determine focus colors
            let left_focus_color = if left_focused {
                Some(t.neon_cyan())
            } else {
                None
            };
            let right_focus_color = if right_focused {
                Some(t.neon_pink())
            } else {
                None
            };

            // Split horizontally: left 50% for orchestrator, 1 column for border, right 50% for worker
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
                render_ended_agent(f, orchestrator, main_chunks[0], left_focus_color);
            } else {
                render_agent_screen(f, orchestrator, main_chunks[0], left_focus_color);
            }

            // Vertical separator - highlight based on focus
            let separator_color = if left_focused || right_focused {
                if left_focused {
                    t.neon_cyan()
                } else {
                    t.neon_pink()
                }
            } else {
                t.border_secondary()
            };
            let separator_lines: Vec<Line> = (0..main_chunks[1].height)
                .map(|_| Line::from("│"))
                .collect();
            let separator =
                Paragraph::new(separator_lines).style(Style::default().fg(separator_color));
            f.render_widget(separator, main_chunks[1]);

            // Right pane: NonInteractive (worker)
            if worker.status == AgentStatus::Ended {
                render_ended_agent(f, worker, main_chunks[2], right_focus_color);
            } else {
                render_agent_screen(f, worker, main_chunks[2], right_focus_color);
            }
        }
        // Only Interactive agent: full width for orchestrator (always highlighted as single pane)
        (Some(orchestrator), None, false) => {
            let t = theme();
            let focus_color = Some(t.neon_cyan());
            if orchestrator.status == AgentStatus::Ended {
                render_ended_agent(f, orchestrator, area, focus_color);
            } else {
                render_agent_screen(f, orchestrator, area, focus_color);
            }
        }
        // Only NonInteractive agents: full width for worker (always highlighted as single pane)
        (None, Some(worker), false) => {
            let t = theme();
            let focus_color = Some(t.neon_pink());
            if worker.status == AgentStatus::Ended {
                render_ended_agent(f, worker, area, focus_color);
            } else {
                render_agent_screen(f, worker, area, focus_color);
            }
        }
        // No agents (shouldn't happen, but handle gracefully)
        (None, None, false) => {
            render_no_agent_menu(f, area);
        }
    }
}

pub fn render_no_agent_menu(f: &mut Frame, area: ratatui::layout::Rect) {
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
/// `focus_color`: Some(Color) to highlight border with that color, None for muted border
pub fn render_ended_agent(
    f: &mut Frame,
    agent: &Agent,
    area: ratatui::layout::Rect,
    focus_color: Option<Color>,
) {
    let t = theme();
    let border_style = match focus_color {
        Some(color) => Style::default().fg(color),
        None => t.style_border_muted(),
    };
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
            .border_style(border_style),
    );
    f.render_widget(menu, area);
}

/// Render active agent's screen (handles both interactive and non-interactive modes)
/// `focus_color`: Some(Color) to highlight border with that color, None for muted border
pub fn render_agent_screen(
    f: &mut Frame,
    agent: &Agent,
    area: ratatui::layout::Rect,
    focus_color: Option<Color>,
) {
    match agent.mode {
        AgentMode::Interactive => {
            render_agent_screen_interactive(f, agent, area, focus_color);
        }
        AgentMode::NonInteractive => {
            render_agent_screen_non_interactive(f, agent, area, focus_color);
        }
    }
}

/// Render interactive (PTY) agent screen with vt100 colors
/// `focus_color`: Some(Color) to highlight border with that color, None for muted border
pub fn render_agent_screen_interactive(
    f: &mut Frame,
    agent: &Agent,
    area: ratatui::layout::Rect,
    focus_color: Option<Color>,
) {
    let t = theme();
    let border_style = match focus_color {
        Some(color) => Style::default().fg(color),
        None => t.style_border_muted(),
    };
    let Some(parser_arc) = agent.get_parser() else {
        // Fallback if no parser
        let widget = Paragraph::new("No parser available").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style),
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
            .border_style(border_style),
    );
    f.render_widget(terminal_widget, area);
}

/// Render non-interactive agent screen (JSON stream output)
/// `focus_color`: Some(Color) to highlight border with that color, None for muted border
pub fn render_agent_screen_non_interactive(
    f: &mut Frame,
    agent: &Agent,
    area: ratatui::layout::Rect,
    focus_color: Option<Color>,
) {
    let t = theme();
    let border_style = match focus_color {
        Some(color) => Style::default().fg(color),
        None => t.style_border_muted(),
    };
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
        WorkState::Working => Style::default().fg(Color::Yellow),
        WorkState::Completed => {
            if agent.error.is_some() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            }
        }
        _ => Style::default().fg(Color::Gray),
    };

    let status_text = match agent.work_state {
        WorkState::Starting => "Starting...",
        WorkState::Working => "Working...",
        WorkState::Idle => "Idle",
        WorkState::Completed => {
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
            .border_style(border_style)
            .title(Span::styled(format!(" {status_text} "), status_style)),
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
