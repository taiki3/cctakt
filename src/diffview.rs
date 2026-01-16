//! Diff viewer module for cctakt
//!
//! Provides a scrollable diff viewer widget for reviewing changes
//! before merging branches.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// A scrollable diff viewer widget
///
/// # Example
/// ```ignore
/// let diff_content = merger.diff("feat/auth")?;
/// let mut diffview = DiffView::new(diff_content);
///
/// // Handle scroll
/// diffview.scroll_down(5);
///
/// // Render
/// diffview.render(f, area);
/// ```
pub struct DiffView {
    /// The raw diff content
    diff_content: String,
    /// Parsed and styled lines
    lines: Vec<DiffLine>,
    /// Current scroll position
    scroll: u16,
    /// Whether syntax highlighting is enabled
    syntax_highlight: bool,
    /// Title for the diff view (e.g., "feat/auth -> main")
    title: Option<String>,
}

/// A parsed diff line with its type
#[derive(Debug, Clone)]
struct DiffLine {
    content: String,
    line_type: DiffLineType,
}

/// Type of diff line for styling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffLineType {
    /// Regular context line
    Context,
    /// Added line (+)
    Addition,
    /// Removed line (-)
    Deletion,
    /// Hunk header (@@)
    HunkHeader,
    /// File header (diff --git, ---, +++)
    FileHeader,
    /// Empty line
    Empty,
}

impl DiffLineType {
    /// Get the foreground color for this line type
    fn color(&self) -> Color {
        match self {
            DiffLineType::Context => Color::White,
            DiffLineType::Addition => Color::Green,
            DiffLineType::Deletion => Color::Red,
            DiffLineType::HunkHeader => Color::Cyan,
            DiffLineType::FileHeader => Color::Yellow,
            DiffLineType::Empty => Color::White,
        }
    }

    /// Get the background color for this line type (if any)
    fn bg_color(&self) -> Option<Color> {
        match self {
            DiffLineType::Addition => Some(Color::Rgb(0, 40, 0)),
            DiffLineType::Deletion => Some(Color::Rgb(40, 0, 0)),
            _ => None,
        }
    }
}

impl DiffView {
    /// Create a new diff view with the given content
    pub fn new(diff: String) -> Self {
        let lines = parse_diff(&diff);
        Self {
            diff_content: diff,
            lines,
            scroll: 0,
            syntax_highlight: true,
            title: None,
        }
    }

    /// Set the title for the diff view
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Enable or disable syntax highlighting
    pub fn set_syntax_highlight(&mut self, enabled: bool) {
        self.syntax_highlight = enabled;
    }

    /// Get the current scroll position
    pub fn scroll_position(&self) -> u16 {
        self.scroll
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Scroll up by the specified number of lines
    pub fn scroll_up(&mut self, lines: u16) {
        self.scroll = self.scroll.saturating_sub(lines);
    }

    /// Scroll down by the specified number of lines
    pub fn scroll_down(&mut self, lines: u16) {
        let max_scroll = self.lines.len().saturating_sub(1) as u16;
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    /// Scroll to the top
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    /// Scroll to the bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.lines.len().saturating_sub(1) as u16;
    }

    /// Page up (scroll by viewport height)
    pub fn page_up(&mut self, viewport_height: u16) {
        self.scroll_up(viewport_height.saturating_sub(2));
    }

    /// Page down (scroll by viewport height)
    pub fn page_down(&mut self, viewport_height: u16) {
        self.scroll_down(viewport_height.saturating_sub(2));
    }

    /// Get the raw diff content
    pub fn content(&self) -> &str {
        &self.diff_content
    }

    /// Check if the diff is empty
    pub fn is_empty(&self) -> bool {
        self.diff_content.is_empty()
    }

    /// Render the diff view
    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Calculate content area (excluding borders)
        let content_height = area.height.saturating_sub(4) as usize; // borders + help line

        // Build the block
        let title = self.title.clone().unwrap_or_else(|| "Diff".to_string());
        let block = Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        // Build styled lines
        let visible_lines: Vec<Line> = self
            .lines
            .iter()
            .skip(self.scroll as usize)
            .take(content_height)
            .map(|line| {
                if self.syntax_highlight {
                    style_diff_line(line)
                } else {
                    Line::from(line.content.clone())
                }
            })
            .collect();

        // Add empty lines if needed
        let all_lines = visible_lines;

        // Add help line at the bottom
        let help_line = Line::from(vec![
            Span::styled(
                "[",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "\u{2191}/\u{2193}",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "] Scroll  ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "[Enter]",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Merge  ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "[Esc]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Cancel",
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        // Render main content
        let diff_widget = Paragraph::new(all_lines)
            .block(block)
            .alignment(Alignment::Left);

        f.render_widget(diff_widget, area);

        // Render help line at the bottom inside the border
        let help_area = Rect::new(
            area.x + 2,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(4),
            1,
        );
        let help_widget = Paragraph::new(help_line).alignment(Alignment::Center);
        f.render_widget(help_widget, help_area);

        // Render scrollbar if needed
        if self.lines.len() > content_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("\u{25b2}")) // ▲
                .end_symbol(Some("\u{25bc}"));   // ▼

            let mut scrollbar_state = ScrollbarState::new(self.lines.len())
                .position(self.scroll as usize);

            let scrollbar_area = Rect::new(
                area.x + area.width.saturating_sub(1),
                area.y + 1,
                1,
                area.height.saturating_sub(3),
            );

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }

    /// Render with a custom block (for embedding in other widgets)
    pub fn render_with_block(&self, f: &mut Frame, area: Rect, block: Block) {
        let inner = block.inner(area);
        f.render_widget(block, area);

        let content_height = inner.height as usize;

        let visible_lines: Vec<Line> = self
            .lines
            .iter()
            .skip(self.scroll as usize)
            .take(content_height)
            .map(|line| {
                if self.syntax_highlight {
                    style_diff_line(line)
                } else {
                    Line::from(line.content.clone())
                }
            })
            .collect();

        let diff_widget = Paragraph::new(visible_lines);
        f.render_widget(diff_widget, inner);
    }
}

/// Parse diff content into typed lines
fn parse_diff(diff: &str) -> Vec<DiffLine> {
    diff.lines()
        .map(|line| {
            let line_type = classify_diff_line(line);
            DiffLine {
                content: line.to_string(),
                line_type,
            }
        })
        .collect()
}

/// Classify a diff line by its prefix
fn classify_diff_line(line: &str) -> DiffLineType {
    if line.is_empty() {
        DiffLineType::Empty
    } else if line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("---")
        || line.starts_with("+++")
    {
        DiffLineType::FileHeader
    } else if line.starts_with("@@") {
        DiffLineType::HunkHeader
    } else if line.starts_with('+') {
        DiffLineType::Addition
    } else if line.starts_with('-') {
        DiffLineType::Deletion
    } else {
        DiffLineType::Context
    }
}

/// Style a diff line based on its type
fn style_diff_line(line: &DiffLine) -> Line<'static> {
    let mut style = Style::default().fg(line.line_type.color());

    if let Some(bg) = line.line_type.bg_color() {
        style = style.bg(bg);
    }

    // Add bold for headers
    if matches!(
        line.line_type,
        DiffLineType::FileHeader | DiffLineType::HunkHeader
    ) {
        style = style.add_modifier(Modifier::BOLD);
    }

    Line::from(Span::styled(line.content.clone(), style))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diffview_new() {
        let diff = "+ added line\n- removed line\n context".to_string();
        let view = DiffView::new(diff.clone());
        assert_eq!(view.content(), diff);
        assert_eq!(view.line_count(), 3);
        assert_eq!(view.scroll_position(), 0);
    }

    #[test]
    fn test_diffview_with_title() {
        let view = DiffView::new("".to_string()).with_title("feat/auth -> main");
        assert_eq!(view.title, Some("feat/auth -> main".to_string()));
    }

    #[test]
    fn test_diffview_scroll_down() {
        let diff = (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let mut view = DiffView::new(diff);

        assert_eq!(view.scroll_position(), 0);

        view.scroll_down(10);
        assert_eq!(view.scroll_position(), 10);

        view.scroll_down(1000);
        assert_eq!(view.scroll_position(), 99); // max is line_count - 1
    }

    #[test]
    fn test_diffview_scroll_up() {
        let diff = (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let mut view = DiffView::new(diff);

        view.scroll_down(50);
        assert_eq!(view.scroll_position(), 50);

        view.scroll_up(20);
        assert_eq!(view.scroll_position(), 30);

        view.scroll_up(100);
        assert_eq!(view.scroll_position(), 0); // min is 0
    }

    #[test]
    fn test_diffview_scroll_to_top_bottom() {
        let diff = (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let mut view = DiffView::new(diff);

        view.scroll_down(50);
        view.scroll_to_top();
        assert_eq!(view.scroll_position(), 0);

        view.scroll_to_bottom();
        assert_eq!(view.scroll_position(), 99);
    }

    #[test]
    fn test_diffview_is_empty() {
        let empty = DiffView::new("".to_string());
        assert!(empty.is_empty());

        let non_empty = DiffView::new("some content".to_string());
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_classify_diff_line() {
        assert_eq!(classify_diff_line(""), DiffLineType::Empty);
        assert_eq!(classify_diff_line("diff --git a/file b/file"), DiffLineType::FileHeader);
        assert_eq!(classify_diff_line("--- a/file"), DiffLineType::FileHeader);
        assert_eq!(classify_diff_line("+++ b/file"), DiffLineType::FileHeader);
        assert_eq!(classify_diff_line("index abc123..def456"), DiffLineType::FileHeader);
        assert_eq!(classify_diff_line("@@ -1,5 +1,6 @@"), DiffLineType::HunkHeader);
        assert_eq!(classify_diff_line("+ added line"), DiffLineType::Addition);
        assert_eq!(classify_diff_line("- removed line"), DiffLineType::Deletion);
        assert_eq!(classify_diff_line(" context line"), DiffLineType::Context);
        assert_eq!(classify_diff_line("plain text"), DiffLineType::Context);
    }

    #[test]
    fn test_diff_line_type_color() {
        assert_eq!(DiffLineType::Context.color(), Color::White);
        assert_eq!(DiffLineType::Addition.color(), Color::Green);
        assert_eq!(DiffLineType::Deletion.color(), Color::Red);
        assert_eq!(DiffLineType::HunkHeader.color(), Color::Cyan);
        assert_eq!(DiffLineType::FileHeader.color(), Color::Yellow);
    }

    #[test]
    fn test_syntax_highlight_toggle() {
        let mut view = DiffView::new("+ line".to_string());
        assert!(view.syntax_highlight);

        view.set_syntax_highlight(false);
        assert!(!view.syntax_highlight);
    }

    #[test]
    fn test_page_up_down() {
        let diff = (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let mut view = DiffView::new(diff);

        view.page_down(20);
        assert_eq!(view.scroll_position(), 18); // 20 - 2

        view.page_up(20);
        assert_eq!(view.scroll_position(), 0);
    }
}
