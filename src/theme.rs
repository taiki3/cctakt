//! Cyberpunk color theme module for cctakt
//!
//! Provides a neon-infused color palette inspired by cyberpunk aesthetics.

use ratatui::style::{Color, Modifier, Style};

/// Cyberpunk color palette
///
/// A carefully crafted palette featuring neon colors against dark backgrounds,
/// inspired by the cyberpunk aesthetic.
pub struct CyberpunkTheme;

impl CyberpunkTheme {
    // ==================== Primary Neon Colors ====================

    /// Hot neon pink - primary accent color
    pub const NEON_PINK: Color = Color::Rgb(255, 0, 128);

    /// Electric cyan - secondary accent color
    pub const NEON_CYAN: Color = Color::Rgb(0, 255, 255);

    /// Vivid purple - tertiary accent color
    pub const NEON_PURPLE: Color = Color::Rgb(157, 78, 221);

    /// Toxic green - success/active indicator
    pub const NEON_GREEN: Color = Color::Rgb(57, 255, 20);

    /// Warning yellow - attention grabbing
    pub const NEON_YELLOW: Color = Color::Rgb(255, 255, 0);

    /// Hot orange - for warnings and highlights
    pub const NEON_ORANGE: Color = Color::Rgb(255, 165, 0);

    /// Electric blue - for information
    pub const NEON_BLUE: Color = Color::Rgb(0, 191, 255);

    // ==================== Background Colors ====================

    /// Deep dark background - main background
    pub const BG_DARK: Color = Color::Rgb(13, 13, 26);

    /// Slightly lighter background for panels
    pub const BG_PANEL: Color = Color::Rgb(26, 26, 46);

    /// Elevated surface color
    pub const BG_SURFACE: Color = Color::Rgb(35, 35, 60);

    /// Highlight background
    pub const BG_HIGHLIGHT: Color = Color::Rgb(50, 50, 80);

    // ==================== Diff Colors ====================

    /// Addition background - dark green tint
    pub const DIFF_ADD_BG: Color = Color::Rgb(0, 50, 20);

    /// Deletion background - dark red tint
    pub const DIFF_DEL_BG: Color = Color::Rgb(50, 0, 20);

    // ==================== Text Colors ====================

    /// Primary text color - bright white
    pub const TEXT_PRIMARY: Color = Color::Rgb(230, 230, 240);

    /// Secondary text color - muted
    pub const TEXT_SECONDARY: Color = Color::Rgb(140, 140, 160);

    /// Muted text color - for less important info
    pub const TEXT_MUTED: Color = Color::Rgb(90, 90, 110);

    // ==================== Semantic Colors ====================

    /// Success color
    pub const SUCCESS: Color = Self::NEON_GREEN;

    /// Error color
    pub const ERROR: Color = Color::Rgb(255, 50, 80);

    /// Warning color
    pub const WARNING: Color = Self::NEON_YELLOW;

    /// Info color
    pub const INFO: Color = Self::NEON_CYAN;

    // ==================== Agent Status Colors ====================

    /// Running status - animated green glow
    pub const STATUS_RUNNING: Color = Self::NEON_GREEN;

    /// Idle status - waiting yellow
    pub const STATUS_IDLE: Color = Self::NEON_YELLOW;

    /// Ended status - muted purple
    pub const STATUS_ENDED: Color = Color::Rgb(100, 80, 120);

    /// Error status - alarm red
    pub const STATUS_ERROR: Color = Self::ERROR;

    // ==================== Border Colors ====================

    /// Primary border - cyan glow
    pub const BORDER_PRIMARY: Color = Self::NEON_CYAN;

    /// Secondary border - muted
    pub const BORDER_SECONDARY: Color = Color::Rgb(60, 60, 90);

    /// Active/focused border
    pub const BORDER_ACTIVE: Color = Self::NEON_PINK;

    // ==================== UI Element Colors ====================

    /// Header tab active background
    pub const TAB_ACTIVE_BG: Color = Self::NEON_CYAN;

    /// Header tab active foreground
    pub const TAB_ACTIVE_FG: Color = Color::Rgb(0, 0, 0);

    /// Header tab inactive background
    pub const TAB_INACTIVE_BG: Color = Self::BG_SURFACE;

    /// Header tab inactive foreground
    pub const TAB_INACTIVE_FG: Color = Self::TEXT_SECONDARY;

    /// Selected item background
    pub const SELECTED_BG: Color = Self::BG_HIGHLIGHT;

    /// Cursor color
    pub const CURSOR_BG: Color = Self::NEON_CYAN;
    pub const CURSOR_FG: Color = Color::Rgb(0, 0, 0);

    // ==================== Help Text Colors ====================

    /// Key binding color
    pub const KEY_BINDING: Color = Self::NEON_CYAN;

    /// Key description color
    pub const KEY_DESCRIPTION: Color = Self::TEXT_MUTED;

    // ==================== Diff Line Type Colors ====================

    /// Context line - unchanged
    pub const DIFF_CONTEXT: Color = Self::TEXT_PRIMARY;

    /// Addition line
    pub const DIFF_ADDITION: Color = Self::NEON_GREEN;

    /// Deletion line
    pub const DIFF_DELETION: Color = Self::ERROR;

    /// Hunk header
    pub const DIFF_HUNK_HEADER: Color = Self::NEON_CYAN;

    /// File header
    pub const DIFF_FILE_HEADER: Color = Self::NEON_YELLOW;

    // ==================== Issue Picker Colors ====================

    /// Issue number color
    pub const ISSUE_NUMBER: Color = Self::NEON_YELLOW;

    /// Issue label color
    pub const ISSUE_LABEL: Color = Self::NEON_PURPLE;

    // ==================== Style Constructors ====================

    /// Create style for active tab
    pub fn style_tab_active() -> Style {
        Style::default()
            .fg(Self::TAB_ACTIVE_FG)
            .bg(Self::TAB_ACTIVE_BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Create style for inactive tab
    pub fn style_tab_inactive() -> Style {
        Style::default()
            .fg(Self::TAB_INACTIVE_FG)
            .bg(Self::TAB_INACTIVE_BG)
    }

    /// Create style for primary border
    pub fn style_border() -> Style {
        Style::default().fg(Self::BORDER_PRIMARY)
    }

    /// Create style for secondary (muted) border
    pub fn style_border_muted() -> Style {
        Style::default().fg(Self::BORDER_SECONDARY)
    }

    /// Create style for dialog border
    pub fn style_dialog_border() -> Style {
        Style::default().fg(Self::NEON_CYAN)
    }

    /// Create style for success text
    pub fn style_success() -> Style {
        Style::default()
            .fg(Self::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    /// Create style for error text
    pub fn style_error() -> Style {
        Style::default().fg(Self::ERROR).add_modifier(Modifier::BOLD)
    }

    /// Create style for warning text
    pub fn style_warning() -> Style {
        Style::default().fg(Self::WARNING)
    }

    /// Create style for info text
    pub fn style_info() -> Style {
        Style::default().fg(Self::INFO)
    }

    /// Create style for primary text
    pub fn style_text() -> Style {
        Style::default().fg(Self::TEXT_PRIMARY)
    }

    /// Create style for secondary text
    pub fn style_text_secondary() -> Style {
        Style::default().fg(Self::TEXT_SECONDARY)
    }

    /// Create style for muted text
    pub fn style_text_muted() -> Style {
        Style::default().fg(Self::TEXT_MUTED)
    }

    /// Create style for key bindings
    pub fn style_key() -> Style {
        Style::default()
            .fg(Self::KEY_BINDING)
            .add_modifier(Modifier::BOLD)
    }

    /// Create style for key descriptions
    pub fn style_key_desc() -> Style {
        Style::default().fg(Self::KEY_DESCRIPTION)
    }

    /// Create style for selected/highlighted items
    pub fn style_selected() -> Style {
        Style::default()
            .bg(Self::SELECTED_BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Create style for cursor
    pub fn style_cursor() -> Style {
        Style::default().fg(Self::CURSOR_FG).bg(Self::CURSOR_BG)
    }

    /// Create style for input text
    pub fn style_input() -> Style {
        Style::default().fg(Self::NEON_YELLOW)
    }

    /// Create style for loading indicator
    pub fn style_loading() -> Style {
        Style::default().fg(Self::NEON_YELLOW)
    }

    /// Create style for dialog background
    pub fn style_dialog_bg() -> Style {
        Style::default().bg(Self::BG_DARK)
    }
}

/// Helper type alias for commonly used theme
pub type Theme = CyberpunkTheme;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neon_colors_are_rgb() {
        // Verify key colors are properly defined as RGB
        assert!(matches!(Theme::NEON_PINK, Color::Rgb(255, 0, 128)));
        assert!(matches!(Theme::NEON_CYAN, Color::Rgb(0, 255, 255)));
        assert!(matches!(Theme::NEON_GREEN, Color::Rgb(57, 255, 20)));
    }

    #[test]
    fn test_style_constructors() {
        // Ensure style constructors don't panic
        let _ = Theme::style_tab_active();
        let _ = Theme::style_tab_inactive();
        let _ = Theme::style_border();
        let _ = Theme::style_success();
        let _ = Theme::style_error();
        let _ = Theme::style_warning();
        let _ = Theme::style_info();
        let _ = Theme::style_text();
        let _ = Theme::style_key();
        let _ = Theme::style_selected();
    }

    #[test]
    fn test_semantic_colors() {
        // Success should be green
        assert!(matches!(Theme::SUCCESS, Color::Rgb(57, 255, 20)));

        // Error should be red-ish
        assert!(matches!(Theme::ERROR, Color::Rgb(255, 50, 80)));

        // Warning should be yellow
        assert!(matches!(Theme::WARNING, Color::Rgb(255, 255, 0)));

        // Info should be cyan
        assert!(matches!(Theme::INFO, Color::Rgb(0, 255, 255)));
    }

    #[test]
    fn test_diff_colors() {
        // Addition should be green
        assert!(matches!(Theme::DIFF_ADDITION, Color::Rgb(57, 255, 20)));

        // Deletion should be red-ish
        assert!(matches!(Theme::DIFF_DELETION, Color::Rgb(255, 50, 80)));

        // Hunk header should be cyan
        assert!(matches!(Theme::DIFF_HUNK_HEADER, Color::Rgb(0, 255, 255)));

        // File header should be yellow
        assert!(matches!(Theme::DIFF_FILE_HEADER, Color::Rgb(255, 255, 0)));
    }

    #[test]
    fn test_status_colors() {
        assert!(matches!(Theme::STATUS_RUNNING, Color::Rgb(57, 255, 20)));
        assert!(matches!(Theme::STATUS_IDLE, Color::Rgb(255, 255, 0)));
        assert!(matches!(Theme::STATUS_ENDED, Color::Rgb(100, 80, 120)));
        assert!(matches!(Theme::STATUS_ERROR, Color::Rgb(255, 50, 80)));
    }
}
