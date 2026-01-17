//! Issue selection UI for cctakt
//!
//! Provides a TUI component for selecting GitHub issues.

use crate::github::Issue;
use crate::theme::theme;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Result of issue picker interaction
#[derive(Debug, Clone)]
pub enum IssuePickerResult {
    /// User selected an issue
    Selected(Issue),
    /// User cancelled the picker
    Cancel,
    /// User requested refresh
    Refresh,
}

/// Issue picker UI component
pub struct IssuePicker {
    /// Available issues
    issues: Vec<Issue>,

    /// Currently selected index
    selected_index: usize,

    /// Scroll offset for display
    scroll_offset: usize,

    /// Whether data is being loaded
    loading: bool,

    /// Error message if any
    error: Option<String>,

    /// List state for ratatui
    list_state: ListState,
}

impl IssuePicker {
    /// Create a new issue picker
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            issues: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            loading: false,
            error: None,
            list_state,
        }
    }

    /// Set the list of issues
    pub fn set_issues(&mut self, issues: Vec<Issue>) {
        self.issues = issues;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_state.select(Some(0));
        self.error = None;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading {
            self.error = None;
        }
    }

    /// Set error message
    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
        self.loading = false;
    }

    /// Get current selection
    pub fn selected(&self) -> Option<&Issue> {
        self.issues.get(self.selected_index)
    }

    /// Handle key input
    ///
    /// Returns `Some(result)` if an action should be taken,
    /// `None` if the picker should continue.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<IssuePickerResult> {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                None
            }
            KeyCode::Enter => {
                self.selected().cloned().map(IssuePickerResult::Selected)
            }
            KeyCode::Esc | KeyCode::Char('q') => Some(IssuePickerResult::Cancel),
            KeyCode::Char('r') => Some(IssuePickerResult::Refresh),
            KeyCode::Home => {
                self.selected_index = 0;
                self.list_state.select(Some(0));
                None
            }
            KeyCode::End => {
                if !self.issues.is_empty() {
                    self.selected_index = self.issues.len() - 1;
                    self.list_state.select(Some(self.selected_index));
                }
                None
            }
            KeyCode::PageUp => {
                self.selected_index = self.selected_index.saturating_sub(10);
                self.list_state.select(Some(self.selected_index));
                None
            }
            KeyCode::PageDown => {
                if !self.issues.is_empty() {
                    self.selected_index =
                        (self.selected_index + 10).min(self.issues.len() - 1);
                    self.list_state.select(Some(self.selected_index));
                }
                None
            }
            _ => None,
        }
    }

    /// Move selection up
    fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Move selection down
    fn move_down(&mut self) {
        if self.selected_index + 1 < self.issues.len() {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    /// Render the issue picker
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let t = theme();

        // Clear background
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" Select Issue ")
            .borders(Borders::ALL)
            .border_style(t.style_border());

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // Handle loading state
        if self.loading {
            let loading_text = Paragraph::new("Loading issues...")
                .style(t.style_loading());
            f.render_widget(loading_text, inner_area);
            return;
        }

        // Handle error state
        if let Some(ref error) = self.error {
            let error_text = Paragraph::new(format!("Error: {error}"))
                .style(Style::default().fg(t.error()));
            f.render_widget(error_text, inner_area);
            return;
        }

        // Handle empty state
        if self.issues.is_empty() {
            let empty_text = Paragraph::new("No issues found")
                .style(t.style_text_muted());
            f.render_widget(empty_text, inner_area);
            return;
        }

        // Calculate areas
        let list_height = inner_area.height.saturating_sub(2);
        let list_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: inner_area.width,
            height: list_height,
        };
        let help_area = Rect {
            x: inner_area.x,
            y: inner_area.y + list_height,
            width: inner_area.width,
            height: 2,
        };

        // Render issue list
        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|issue| {
                let labels = if issue.labels.is_empty() {
                    String::new()
                } else {
                    format!(
                        "[{}] ",
                        issue
                            .labels
                            .iter()
                            .map(|l| l.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("#{:<5} ", issue.number),
                        Style::default().fg(t.issue_number()),
                    ),
                    Span::styled(labels, Style::default().fg(t.issue_label())),
                    Span::raw(&issue.title),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(t.style_selected())
            .highlight_symbol("> ");

        f.render_stateful_widget(list, list_area, &mut self.list_state);

        // Render help text
        let help_text = Line::from(vec![
            Span::styled("[", t.style_text_muted()),
            Span::styled("Up/Down", t.style_key()),
            Span::styled("] Navigate  [", t.style_text_muted()),
            Span::styled("Enter", t.style_key()),
            Span::styled("] Select  [", t.style_text_muted()),
            Span::styled("r", t.style_key()),
            Span::styled("] Refresh  [", t.style_text_muted()),
            Span::styled("Esc", t.style_key()),
            Span::styled("] Cancel", t.style_text_muted()),
        ]);

        let help = Paragraph::new(help_text);
        f.render_widget(help, help_area);
    }

    /// Get the number of issues
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Check if there are no issues
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Get error if any
    pub fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl Default for IssuePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate centered popup area
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let popup_height = area.height * percent_y / 100;

    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;

    Rect {
        x: area.x + x,
        y: area.y + y,
        width: popup_width,
        height: popup_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::Label;

    fn create_test_issues() -> Vec<Issue> {
        vec![
            Issue {
                number: 123,
                title: "Fix login validation error".to_string(),
                body: Some("Login fails when...".to_string()),
                labels: vec![Label {
                    name: "bug".to_string(),
                    color: "d73a4a".to_string(),
                }],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/123".to_string(),
            },
            Issue {
                number: 456,
                title: "Add dark mode support".to_string(),
                body: Some("Users want dark mode".to_string()),
                labels: vec![Label {
                    name: "feature".to_string(),
                    color: "a2eeef".to_string(),
                }],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/456".to_string(),
            },
            Issue {
                number: 789,
                title: "Update README".to_string(),
                body: None,
                labels: vec![Label {
                    name: "docs".to_string(),
                    color: "0075ca".to_string(),
                }],
                state: "open".to_string(),
                html_url: "https://github.com/test/repo/issues/789".to_string(),
            },
        ]
    }

    #[test]
    fn test_new_picker() {
        let picker = IssuePicker::new();

        assert!(picker.is_empty());
        assert!(!picker.is_loading());
        assert!(picker.get_error().is_none());
    }

    #[test]
    fn test_set_issues() {
        let mut picker = IssuePicker::new();
        let issues = create_test_issues();

        picker.set_issues(issues);

        assert_eq!(picker.len(), 3);
        assert!(!picker.is_empty());
        assert_eq!(picker.selected().unwrap().number, 123);
    }

    #[test]
    fn test_navigation() {
        let mut picker = IssuePicker::new();
        picker.set_issues(create_test_issues());

        // Initial selection
        assert_eq!(picker.selected().unwrap().number, 123);

        // Move down
        picker.handle_key(KeyCode::Down);
        assert_eq!(picker.selected().unwrap().number, 456);

        // Move down again
        picker.handle_key(KeyCode::Char('j'));
        assert_eq!(picker.selected().unwrap().number, 789);

        // Move down at bottom (should stay)
        picker.handle_key(KeyCode::Down);
        assert_eq!(picker.selected().unwrap().number, 789);

        // Move up
        picker.handle_key(KeyCode::Up);
        assert_eq!(picker.selected().unwrap().number, 456);

        // Move up with k
        picker.handle_key(KeyCode::Char('k'));
        assert_eq!(picker.selected().unwrap().number, 123);

        // Move up at top (should stay)
        picker.handle_key(KeyCode::Up);
        assert_eq!(picker.selected().unwrap().number, 123);
    }

    #[test]
    fn test_selection() {
        let mut picker = IssuePicker::new();
        picker.set_issues(create_test_issues());

        picker.handle_key(KeyCode::Down);
        let result = picker.handle_key(KeyCode::Enter);

        match result {
            Some(IssuePickerResult::Selected(issue)) => {
                assert_eq!(issue.number, 456);
            }
            _ => panic!("Expected Selected result"),
        }
    }

    #[test]
    fn test_cancel() {
        let mut picker = IssuePicker::new();
        picker.set_issues(create_test_issues());

        let result = picker.handle_key(KeyCode::Esc);
        assert!(matches!(result, Some(IssuePickerResult::Cancel)));

        let result = picker.handle_key(KeyCode::Char('q'));
        assert!(matches!(result, Some(IssuePickerResult::Cancel)));
    }

    #[test]
    fn test_refresh() {
        let mut picker = IssuePicker::new();
        picker.set_issues(create_test_issues());

        let result = picker.handle_key(KeyCode::Char('r'));
        assert!(matches!(result, Some(IssuePickerResult::Refresh)));
    }

    #[test]
    fn test_loading_state() {
        let mut picker = IssuePicker::new();

        picker.set_loading(true);
        assert!(picker.is_loading());

        picker.set_loading(false);
        assert!(!picker.is_loading());
    }

    #[test]
    fn test_error_state() {
        let mut picker = IssuePicker::new();

        picker.set_error(Some("Network error".to_string()));
        assert_eq!(picker.get_error(), Some("Network error"));
        assert!(!picker.is_loading());

        picker.set_error(None);
        assert!(picker.get_error().is_none());
    }

    #[test]
    fn test_home_end_keys() {
        let mut picker = IssuePicker::new();
        picker.set_issues(create_test_issues());

        // Go to end
        picker.handle_key(KeyCode::End);
        assert_eq!(picker.selected().unwrap().number, 789);

        // Go to home
        picker.handle_key(KeyCode::Home);
        assert_eq!(picker.selected().unwrap().number, 123);
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 50,
        };

        let popup = centered_rect(80, 60, area);

        assert_eq!(popup.width, 80);
        assert_eq!(popup.height, 30);
        assert_eq!(popup.x, 10);
        assert_eq!(popup.y, 10);
    }
}
