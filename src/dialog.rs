//! Input dialog module for cctakt
//!
//! Provides a generic input dialog widget for user input operations
//! such as adding agents, entering task descriptions, etc.

use crate::theme::theme;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Result of dialog interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    /// User submitted the input
    Submit(String),
    /// User cancelled the dialog
    Cancel,
}

/// A generic input dialog widget
///
/// # Example
/// ```ignore
/// let mut dialog = InputDialog::new("New Agent", "Enter task description:");
/// dialog.show();
///
/// // In event loop:
/// if let Some(result) = dialog.handle_key(key_code) {
///     match result {
///         DialogResult::Submit(value) => { /* use value */ }
///         DialogResult::Cancel => { /* cancelled */ }
///     }
/// }
/// ```
pub struct InputDialog {
    title: String,
    prompt: String,
    input: String,
    cursor_position: usize,
    visible: bool,
}

impl InputDialog {
    /// Create a new input dialog
    pub fn new(title: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            input: String::new(),
            cursor_position: 0,
            visible: false,
        }
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if the dialog is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the current input value
    pub fn value(&self) -> &str {
        &self.input
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Handle key input
    ///
    /// Returns `Some(DialogResult)` when the dialog should close,
    /// `None` when the dialog should stay open.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<DialogResult> {
        if !self.visible {
            return None;
        }

        match key {
            KeyCode::Enter => {
                let value = self.input.clone();
                self.hide();
                self.clear();
                Some(DialogResult::Submit(value))
            }
            KeyCode::Esc => {
                self.hide();
                self.clear();
                Some(DialogResult::Cancel)
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                None
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.input.remove(self.cursor_position);
                }
                None
            }
            KeyCode::Delete => {
                if self.cursor_position < self.input.len() {
                    self.input.remove(self.cursor_position);
                }
                None
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                None
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
                None
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                None
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
                None
            }
            _ => None,
        }
    }

    /// Render the dialog
    ///
    /// The dialog is rendered as a centered popup over the given area.
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let t = theme();

        // Calculate dialog dimensions
        let dialog_width = 50.min(area.width.saturating_sub(4));
        let dialog_height = 9;

        // Center the dialog
        let dialog_area = centered_rect(dialog_width, dialog_height, area);

        // Clear the area behind the dialog
        f.render_widget(Clear, dialog_area);

        // Dialog block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(t.style_dialog_border())
            .style(t.style_dialog_bg());

        f.render_widget(block.clone(), dialog_area);

        // Inner area for content
        let inner = block.inner(dialog_area);

        // Layout for content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Prompt
                Constraint::Length(1), // Spacing
                Constraint::Length(3), // Input field
                Constraint::Length(1), // Spacing
                Constraint::Length(1), // Help text
            ])
            .split(inner);

        // Prompt text
        let prompt = Paragraph::new(self.prompt.as_str())
            .style(t.style_text());
        f.render_widget(prompt, chunks[0]);

        // Input field with cursor
        let input_display = if self.cursor_position < self.input.len() {
            let (before, after) = self.input.split_at(self.cursor_position);
            let cursor_char = after.chars().next().unwrap_or(' ');
            let remaining = if after.len() > 1 { &after[cursor_char.len_utf8()..] } else { "" };
            Line::from(vec![
                Span::raw(before),
                Span::styled(
                    cursor_char.to_string(),
                    t.style_cursor(),
                ),
                Span::raw(remaining),
            ])
        } else {
            Line::from(vec![
                Span::raw(&self.input),
                Span::styled(
                    " ",
                    t.style_cursor(),
                ),
            ])
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(t.style_border_muted());

        let input_field = Paragraph::new(input_display)
            .block(input_block)
            .style(t.style_input());
        f.render_widget(input_field, chunks[2]);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", t.style_success()),
            Span::raw(" Submit  "),
            Span::styled("[Esc]", t.style_error()),
            Span::raw(" Cancel"),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(help, chunks[4]);
    }
}

/// Create a centered rectangle with given width and height
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_new() {
        let dialog = InputDialog::new("Test Title", "Test Prompt");
        assert_eq!(dialog.title, "Test Title");
        assert_eq!(dialog.prompt, "Test Prompt");
        assert!(!dialog.is_visible());
        assert!(dialog.value().is_empty());
    }

    #[test]
    fn test_dialog_show_hide() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        assert!(!dialog.is_visible());

        dialog.show();
        assert!(dialog.is_visible());

        dialog.hide();
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_dialog_input() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        dialog.show();

        // Type "hello"
        dialog.handle_key(KeyCode::Char('h'));
        dialog.handle_key(KeyCode::Char('e'));
        dialog.handle_key(KeyCode::Char('l'));
        dialog.handle_key(KeyCode::Char('l'));
        dialog.handle_key(KeyCode::Char('o'));

        assert_eq!(dialog.value(), "hello");
    }

    #[test]
    fn test_dialog_backspace() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        dialog.show();

        dialog.handle_key(KeyCode::Char('h'));
        dialog.handle_key(KeyCode::Char('i'));
        dialog.handle_key(KeyCode::Backspace);

        assert_eq!(dialog.value(), "h");
    }

    #[test]
    fn test_dialog_submit() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        dialog.show();

        dialog.handle_key(KeyCode::Char('t'));
        dialog.handle_key(KeyCode::Char('e'));
        dialog.handle_key(KeyCode::Char('s'));
        dialog.handle_key(KeyCode::Char('t'));

        let result = dialog.handle_key(KeyCode::Enter);
        assert_eq!(result, Some(DialogResult::Submit("test".to_string())));
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_dialog_cancel() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        dialog.show();

        dialog.handle_key(KeyCode::Char('t'));
        dialog.handle_key(KeyCode::Char('e'));

        let result = dialog.handle_key(KeyCode::Esc);
        assert_eq!(result, Some(DialogResult::Cancel));
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_dialog_cursor_movement() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        dialog.show();

        dialog.handle_key(KeyCode::Char('a'));
        dialog.handle_key(KeyCode::Char('b'));
        dialog.handle_key(KeyCode::Char('c'));
        // cursor at end: abc|

        dialog.handle_key(KeyCode::Left);
        // cursor: ab|c

        dialog.handle_key(KeyCode::Char('x'));
        // cursor: abx|c

        assert_eq!(dialog.value(), "abxc");

        dialog.handle_key(KeyCode::Home);
        dialog.handle_key(KeyCode::Char('z'));
        // cursor: z|abxc

        assert_eq!(dialog.value(), "zabxc");

        dialog.handle_key(KeyCode::End);
        dialog.handle_key(KeyCode::Char('!'));
        // cursor: zabxc!|

        assert_eq!(dialog.value(), "zabxc!");
    }

    #[test]
    fn test_dialog_clear() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        dialog.show();

        dialog.handle_key(KeyCode::Char('t'));
        dialog.handle_key(KeyCode::Char('e'));
        dialog.handle_key(KeyCode::Char('s'));
        dialog.handle_key(KeyCode::Char('t'));

        dialog.clear();
        assert!(dialog.value().is_empty());
    }

    #[test]
    fn test_dialog_not_visible_ignores_input() {
        let mut dialog = InputDialog::new("Test", "Prompt");
        // Dialog is not visible

        let result = dialog.handle_key(KeyCode::Char('x'));
        assert!(result.is_none());
        assert!(dialog.value().is_empty());
    }
}
