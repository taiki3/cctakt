//! Input handling for TUI

use crate::app::{App, AppMode, FocusedPane, InputMode};
use cctakt::{available_themes, plan::NotifyLevel};
use crossterm::event::{KeyCode, KeyModifiers};

/// Handle special keybindings, returns true if handled
pub fn handle_keybinding(app: &mut App, modifiers: KeyModifiers, code: KeyCode) -> bool {
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
        // Ctrl+R: Restart conductor (orchestrator)
        (KeyModifiers::CONTROL, KeyCode::Char('r' | 'R')) => {
            match app.restart_conductor() {
                Ok(()) => {
                    app.add_notification(
                        "Conductor restarted".to_string(),
                        NotifyLevel::Success,
                    );
                }
                Err(e) => {
                    app.add_notification(
                        format!("Failed to restart conductor: {e}"),
                        NotifyLevel::Error,
                    );
                }
            }
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
pub fn handle_theme_picker_input(app: &mut App, code: KeyCode) {
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

/// Handle navigation mode keys (hjkl)
pub fn handle_navigation_mode(app: &mut App, code: KeyCode) {
    match code {
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
