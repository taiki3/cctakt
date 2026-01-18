//! TUI command implementation

use crate::agent::{AgentStatus, WorkState};
use crate::app::{App, AppMode, FocusedPane, InputMode};
use crate::tui::{handle_keybinding, handle_navigation_mode, handle_theme_picker_input, ui};
use anyhow::{Context, Result};
use cctakt::{create_theme, debug, set_theme, Config, IssuePickerResult, LockFile};
use crossterm::{
    cursor::Hide,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

/// Run the TUI application
pub fn run_tui() -> Result<()> {
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
                        agent.send_bytes(b"\r"); // Carriage return for Enter
                        agent.task_sent = true;
                        agent.work_state = WorkState::Working;
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
                    // Debug: log every key event received
                    debug::log(&format!(
                        "KEY_EVENT: {:?}, mode={:?}, input_mode={:?}",
                        key.code, app.mode, app.input_mode
                    ));
                    match app.mode {
                        AppMode::ReviewMerge => {
                            // Handle review mode input
                            match key.code {
                                KeyCode::Enter | KeyCode::Char('m') | KeyCode::Char('M') => {
                                    // Enqueue merge (handled by MergeWorker)
                                    app.enqueue_merge();
                                }
                                KeyCode::Char('q')
                                | KeyCode::Char('Q')
                                | KeyCode::Char('c')
                                | KeyCode::Char('C') => {
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
                                // Pane navigation with h/l
                                KeyCode::Char('h') => {
                                    app.focused_pane = FocusedPane::Left;
                                }
                                KeyCode::Char('l') => {
                                    app.focused_pane = FocusedPane::Right;
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
                            debug::log("Entering AppMode::Normal branch");
                            if app.agent_manager.is_empty() {
                                // No agents - orchestrator was closed, quit app
                                debug::log("agent_manager.is_empty() = true, quitting");
                                app.should_quit = true;
                            } else {
                                // Always handle global keybindings (Ctrl+Q, Ctrl+T, etc)
                                let handled = handle_keybinding(&mut app, key.modifiers, key.code);
                                debug::log(&format!("handle_keybinding returned: {}", handled));

                                if !handled {
                                    // Debug: log current mode and key
                                    debug::log(&format!(
                                        "Key: {:?}, Mode: {:?}, InputMode: {:?}",
                                        key.code, app.mode, app.input_mode
                                    ));

                                    match app.input_mode {
                                        InputMode::Navigation => {
                                            // Navigation mode: hjkl for pane navigation
                                            debug::log("Processing Navigation mode key");
                                            handle_navigation_mode(&mut app, key.code);
                                        }
                                        InputMode::Input => {
                                            // Input mode: forward keys to focused agent
                                            // Esc switches back to navigation mode
                                            debug::log("Processing Input mode key");
                                            if key.code == KeyCode::Esc {
                                                debug::log(
                                                    "Esc pressed - switching to Navigation mode",
                                                );
                                                app.input_mode = InputMode::Navigation;
                                            } else {
                                                // Determine which agent to send input to
                                                // Fallback: if focused pane has no agent, try the other pane
                                                let has_interactive =
                                                    app.agent_manager.get_interactive().is_some();
                                                let has_worker = app
                                                    .agent_manager
                                                    .get_active_non_interactive()
                                                    .is_some();

                                                let use_interactive = match app.focused_pane {
                                                    FocusedPane::Left => {
                                                        has_interactive || !has_worker
                                                    }
                                                    FocusedPane::Right => {
                                                        !has_worker && has_interactive
                                                    }
                                                };

                                                let agent = if use_interactive {
                                                    app.agent_manager.get_interactive_mut()
                                                } else {
                                                    app.agent_manager.get_active_non_interactive_mut()
                                                };

                                                if let Some(agent) = agent {
                                                    if agent.status == AgentStatus::Running {
                                                        match (key.modifiers, key.code) {
                                                            (
                                                                KeyModifiers::CONTROL,
                                                                KeyCode::Char(c),
                                                            ) => {
                                                                let ctrl_char = (c as u8) & 0x1f;
                                                                agent.send_bytes(&[ctrl_char]);
                                                            }
                                                            (_, KeyCode::Enter) => {
                                                                agent.send_bytes(b"\r")
                                                            }
                                                            (_, KeyCode::Backspace) => {
                                                                agent.send_bytes(&[0x7f])
                                                            }
                                                            (_, KeyCode::Tab) => {
                                                                agent.send_bytes(b"\t")
                                                            }
                                                            (_, KeyCode::Up) => {
                                                                agent.send_bytes(b"\x1b[A")
                                                            }
                                                            (_, KeyCode::Down) => {
                                                                agent.send_bytes(b"\x1b[B")
                                                            }
                                                            (_, KeyCode::Right) => {
                                                                agent.send_bytes(b"\x1b[C")
                                                            }
                                                            (_, KeyCode::Left) => {
                                                                agent.send_bytes(b"\x1b[D")
                                                            }
                                                            (_, KeyCode::Home) => {
                                                                agent.send_bytes(b"\x1b[H")
                                                            }
                                                            (_, KeyCode::End) => {
                                                                agent.send_bytes(b"\x1b[F")
                                                            }
                                                            (_, KeyCode::PageUp) => {
                                                                agent.send_bytes(b"\x1b[5~")
                                                            }
                                                            (_, KeyCode::PageDown) => {
                                                                agent.send_bytes(b"\x1b[6~")
                                                            }
                                                            (_, KeyCode::Delete) => {
                                                                agent.send_bytes(b"\x1b[3~")
                                                            }
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
                                    let branch = app.pending_build_branch.take().unwrap_or_else(|| "unknown".to_string());
                                    app.spawn_build_worker(branch);
                                    app.mode = AppMode::Normal;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') => {
                                    // Skip build - show task complete
                                    let branch = app.pending_build_branch.take().unwrap_or_else(|| "unknown".to_string());
                                    app.show_task_complete(branch, false, None);
                                }
                                _ => {}
                            }
                        }
                        AppMode::TaskComplete => {
                            // Handle task complete screen input
                            match key.code {
                                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                                    app.close_task_complete();
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

        // Check BuildWorker completion
        app.check_build_worker_completion();

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
