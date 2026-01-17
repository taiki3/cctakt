//! Color theme module for cctakt
//!
//! Provides multiple color themes with a trait-based design for easy switching.
//! Themes include Cyberpunk (default), Monokai, Dracula, Nord, and Minimal.

use ratatui::style::{Color, Modifier, Style};
use std::sync::RwLock;

// ==================== ColorTheme Trait ====================

/// Color theme trait defining all theme colors and styles
pub trait ColorTheme: Send + Sync {
    // ==================== Neon/Accent Colors ====================

    /// Primary accent color (hot pink in Cyberpunk)
    fn neon_pink(&self) -> Color;

    /// Secondary accent color (cyan in Cyberpunk)
    fn neon_cyan(&self) -> Color;

    /// Tertiary accent color (purple in Cyberpunk)
    fn neon_purple(&self) -> Color;

    /// Success/active color (green in Cyberpunk)
    fn neon_green(&self) -> Color;

    /// Warning color (yellow in Cyberpunk)
    fn neon_yellow(&self) -> Color;

    /// Highlight color (orange in Cyberpunk)
    fn neon_orange(&self) -> Color;

    /// Info color (blue in Cyberpunk)
    fn neon_blue(&self) -> Color;

    // ==================== Background Colors ====================

    /// Main background color
    fn bg_dark(&self) -> Color;

    /// Panel background color
    fn bg_panel(&self) -> Color;

    /// Surface/elevated background color
    fn bg_surface(&self) -> Color;

    /// Highlight background color
    fn bg_highlight(&self) -> Color;

    // ==================== Text Colors ====================

    /// Primary text color
    fn text_primary(&self) -> Color;

    /// Secondary text color
    fn text_secondary(&self) -> Color;

    /// Muted text color
    fn text_muted(&self) -> Color;

    // ==================== Semantic Colors ====================

    /// Success color
    fn success(&self) -> Color {
        self.neon_green()
    }

    /// Error color
    fn error(&self) -> Color;

    /// Warning color
    fn warning(&self) -> Color {
        self.neon_yellow()
    }

    /// Info color
    fn info(&self) -> Color {
        self.neon_cyan()
    }

    // ==================== Agent Status Colors ====================

    /// Running status color
    fn status_running(&self) -> Color {
        self.neon_green()
    }

    /// Idle status color
    fn status_idle(&self) -> Color {
        self.neon_yellow()
    }

    /// Ended status color
    fn status_ended(&self) -> Color;

    /// Error status color
    fn status_error(&self) -> Color {
        self.error()
    }

    // ==================== Border Colors ====================

    /// Primary border color
    fn border_primary(&self) -> Color {
        self.neon_cyan()
    }

    /// Secondary border color
    fn border_secondary(&self) -> Color;

    /// Active/focused border color
    fn border_active(&self) -> Color {
        self.neon_pink()
    }

    // ==================== Diff Colors ====================

    /// Addition background color
    fn diff_add_bg(&self) -> Color;

    /// Deletion background color
    fn diff_del_bg(&self) -> Color;

    /// Addition text color
    fn diff_addition(&self) -> Color {
        self.neon_green()
    }

    /// Deletion text color
    fn diff_deletion(&self) -> Color {
        self.error()
    }

    /// Context line color
    fn diff_context(&self) -> Color {
        self.text_primary()
    }

    /// Hunk header color
    fn diff_hunk_header(&self) -> Color {
        self.neon_cyan()
    }

    /// File header color
    fn diff_file_header(&self) -> Color {
        self.neon_yellow()
    }

    // ==================== UI Element Colors ====================

    /// Active tab background
    fn tab_active_bg(&self) -> Color {
        self.neon_cyan()
    }

    /// Active tab foreground
    fn tab_active_fg(&self) -> Color {
        Color::Rgb(0, 0, 0)
    }

    /// Selected item background
    fn selected_bg(&self) -> Color {
        self.bg_highlight()
    }

    /// Cursor background
    fn cursor_bg(&self) -> Color {
        self.neon_cyan()
    }

    /// Cursor foreground
    fn cursor_fg(&self) -> Color {
        Color::Rgb(0, 0, 0)
    }

    /// Key binding color
    fn key_binding(&self) -> Color {
        self.neon_cyan()
    }

    /// Key description color
    fn key_description(&self) -> Color {
        self.text_muted()
    }

    /// Issue number color
    fn issue_number(&self) -> Color {
        self.neon_yellow()
    }

    /// Issue label color
    fn issue_label(&self) -> Color {
        self.neon_purple()
    }

    // ==================== Style Methods ====================

    /// Style for active tab
    fn style_tab_active(&self) -> Style {
        Style::default()
            .fg(self.tab_active_fg())
            .bg(self.tab_active_bg())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for inactive tab
    fn style_tab_inactive(&self) -> Style {
        Style::default()
            .fg(self.text_secondary())
            .bg(self.bg_surface())
    }

    /// Style for primary border
    fn style_border(&self) -> Style {
        Style::default().fg(self.border_primary())
    }

    /// Style for secondary (muted) border
    fn style_border_muted(&self) -> Style {
        Style::default().fg(self.border_secondary())
    }

    /// Style for dialog border
    fn style_dialog_border(&self) -> Style {
        Style::default().fg(self.neon_cyan())
    }

    /// Style for success text
    fn style_success(&self) -> Style {
        Style::default()
            .fg(self.success())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for error text
    fn style_error(&self) -> Style {
        Style::default()
            .fg(self.error())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for warning text
    fn style_warning(&self) -> Style {
        Style::default().fg(self.warning())
    }

    /// Style for info text
    fn style_info(&self) -> Style {
        Style::default().fg(self.info())
    }

    /// Style for primary text
    fn style_text(&self) -> Style {
        Style::default().fg(self.text_primary())
    }

    /// Style for secondary text
    fn style_text_secondary(&self) -> Style {
        Style::default().fg(self.text_secondary())
    }

    /// Style for muted text
    fn style_text_muted(&self) -> Style {
        Style::default().fg(self.text_muted())
    }

    /// Style for key bindings
    fn style_key(&self) -> Style {
        Style::default()
            .fg(self.key_binding())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for key descriptions
    fn style_key_desc(&self) -> Style {
        Style::default().fg(self.key_description())
    }

    /// Style for selected/highlighted items
    fn style_selected(&self) -> Style {
        Style::default()
            .bg(self.selected_bg())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for cursor
    fn style_cursor(&self) -> Style {
        Style::default()
            .fg(self.cursor_fg())
            .bg(self.cursor_bg())
    }

    /// Style for input text
    fn style_input(&self) -> Style {
        Style::default().fg(self.neon_yellow())
    }

    /// Style for loading indicator
    fn style_loading(&self) -> Style {
        Style::default().fg(self.neon_yellow())
    }

    /// Style for dialog background
    fn style_dialog_bg(&self) -> Style {
        Style::default().bg(self.bg_dark())
    }
}

// ==================== Global Theme Access ====================

static CURRENT_THEME: RwLock<Option<Box<dyn ColorTheme>>> = RwLock::new(None);

/// Static default theme for fallback
static DEFAULT_THEME: CyberpunkTheme = CyberpunkTheme;

/// Theme accessor wrapper that holds a read guard
pub struct ThemeGuard<'a> {
    guard: std::sync::RwLockReadGuard<'a, Option<Box<dyn ColorTheme>>>,
}

impl std::ops::Deref for ThemeGuard<'_> {
    type Target = dyn ColorTheme;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().map(|t| t.as_ref()).unwrap_or(&DEFAULT_THEME)
    }
}

/// Get the current theme
///
/// Returns a guard that provides access to the theme.
/// Falls back to CyberpunkTheme if none is set or if the lock is poisoned.
pub fn theme() -> ThemeGuard<'static> {
    ThemeGuard {
        guard: CURRENT_THEME.read().unwrap_or_else(|e| e.into_inner()),
    }
}

/// Set the global theme
///
/// This can be called multiple times to change the theme at runtime.
/// Returns true if the theme was set successfully.
pub fn set_theme(theme_impl: Box<dyn ColorTheme>) -> bool {
    match CURRENT_THEME.write() {
        Ok(mut guard) => {
            *guard = Some(theme_impl);
            true
        }
        Err(e) => {
            // Recover from poisoned lock
            let mut guard = e.into_inner();
            *guard = Some(theme_impl);
            true
        }
    }
}

/// Create a theme from its name
pub fn create_theme(name: &str) -> Box<dyn ColorTheme> {
    match name.to_lowercase().as_str() {
        "monokai" => Box::new(MonokaiTheme),
        "dracula" => Box::new(DraculaTheme),
        "nord" => Box::new(NordTheme),
        "arctic" | "aurora" | "arctic-aurora" => Box::new(ArcticAuroraTheme),
        "minimal" => Box::new(MinimalTheme),
        _ => Box::new(CyberpunkTheme),
    }
}

/// Available themes with their names and descriptions
///
/// Returns a list of (id, display_name, description) tuples.
pub fn available_themes() -> &'static [(&'static str, &'static str, &'static str)] {
    &[
        ("cyberpunk", "Cyberpunk", "ネオンカラーのサイバーパンク風"),
        ("monokai", "Monokai", "クラシックなエディタカラー"),
        ("dracula", "Dracula", "人気のダークテーマ"),
        ("nord", "Nord", "北欧の青みのあるパレット"),
        ("arctic", "Arctic Aurora", "オーロラ風の幻想的なテーマ"),
        ("minimal", "Minimal", "控えめでプロフェッショナル"),
    ]
}

/// Get the theme ID from the current theme
///
/// Returns the theme name that can be used with create_theme().
pub fn current_theme_id() -> &'static str {
    // We can't easily get the theme ID from the trait object,
    // so we compare colors to identify the theme
    let t = theme();
    let bg = t.bg_dark();

    match bg {
        Color::Rgb(13, 13, 26) => "cyberpunk",
        Color::Rgb(39, 40, 34) => "monokai",
        Color::Rgb(40, 42, 54) => "dracula",
        Color::Rgb(46, 52, 64) => "nord",
        Color::Rgb(28, 35, 49) => "arctic",
        Color::Rgb(26, 26, 26) => "minimal",
        _ => "cyberpunk",
    }
}

// ==================== Cyberpunk Theme ====================

/// Cyberpunk color palette
///
/// A neon-infused color palette inspired by cyberpunk aesthetics.
pub struct CyberpunkTheme;

impl ColorTheme for CyberpunkTheme {
    fn neon_pink(&self) -> Color {
        Color::Rgb(255, 0, 128)
    }

    fn neon_cyan(&self) -> Color {
        Color::Rgb(0, 255, 255)
    }

    fn neon_purple(&self) -> Color {
        Color::Rgb(157, 78, 221)
    }

    fn neon_green(&self) -> Color {
        Color::Rgb(57, 255, 20)
    }

    fn neon_yellow(&self) -> Color {
        Color::Rgb(255, 255, 0)
    }

    fn neon_orange(&self) -> Color {
        Color::Rgb(255, 165, 0)
    }

    fn neon_blue(&self) -> Color {
        Color::Rgb(0, 191, 255)
    }

    fn bg_dark(&self) -> Color {
        Color::Rgb(13, 13, 26)
    }

    fn bg_panel(&self) -> Color {
        Color::Rgb(26, 26, 46)
    }

    fn bg_surface(&self) -> Color {
        Color::Rgb(35, 35, 60)
    }

    fn bg_highlight(&self) -> Color {
        Color::Rgb(50, 50, 80)
    }

    fn text_primary(&self) -> Color {
        Color::Rgb(230, 230, 240)
    }

    fn text_secondary(&self) -> Color {
        Color::Rgb(140, 140, 160)
    }

    fn text_muted(&self) -> Color {
        Color::Rgb(90, 90, 110)
    }

    fn error(&self) -> Color {
        Color::Rgb(255, 50, 80)
    }

    fn status_ended(&self) -> Color {
        Color::Rgb(100, 80, 120)
    }

    fn border_secondary(&self) -> Color {
        Color::Rgb(60, 60, 90)
    }

    fn diff_add_bg(&self) -> Color {
        Color::Rgb(0, 50, 20)
    }

    fn diff_del_bg(&self) -> Color {
        Color::Rgb(50, 0, 20)
    }
}

// ==================== Monokai Theme ====================

/// Monokai color palette
///
/// Classic Monokai colors used in many editors.
pub struct MonokaiTheme;

impl ColorTheme for MonokaiTheme {
    fn neon_pink(&self) -> Color {
        Color::Rgb(249, 38, 114) // #F92672
    }

    fn neon_cyan(&self) -> Color {
        Color::Rgb(102, 217, 239) // #66D9EF
    }

    fn neon_purple(&self) -> Color {
        Color::Rgb(174, 129, 255) // #AE81FF
    }

    fn neon_green(&self) -> Color {
        Color::Rgb(166, 226, 46) // #A6E22E
    }

    fn neon_yellow(&self) -> Color {
        Color::Rgb(230, 219, 116) // #E6DB74
    }

    fn neon_orange(&self) -> Color {
        Color::Rgb(253, 151, 31) // #FD971F
    }

    fn neon_blue(&self) -> Color {
        Color::Rgb(102, 217, 239) // #66D9EF (same as cyan)
    }

    fn bg_dark(&self) -> Color {
        Color::Rgb(39, 40, 34) // #272822
    }

    fn bg_panel(&self) -> Color {
        Color::Rgb(49, 50, 44) // slightly lighter
    }

    fn bg_surface(&self) -> Color {
        Color::Rgb(59, 60, 54) // more elevated
    }

    fn bg_highlight(&self) -> Color {
        Color::Rgb(73, 72, 62) // #49483E
    }

    fn text_primary(&self) -> Color {
        Color::Rgb(248, 248, 242) // #F8F8F2
    }

    fn text_secondary(&self) -> Color {
        Color::Rgb(175, 175, 165)
    }

    fn text_muted(&self) -> Color {
        Color::Rgb(117, 113, 94) // #75715E
    }

    fn error(&self) -> Color {
        Color::Rgb(249, 38, 114) // #F92672 (pink in Monokai)
    }

    fn status_ended(&self) -> Color {
        Color::Rgb(117, 113, 94) // muted
    }

    fn border_secondary(&self) -> Color {
        Color::Rgb(73, 72, 62) // #49483E
    }

    fn diff_add_bg(&self) -> Color {
        Color::Rgb(30, 50, 20)
    }

    fn diff_del_bg(&self) -> Color {
        Color::Rgb(50, 20, 30)
    }
}

// ==================== Dracula Theme ====================

/// Dracula color palette
///
/// The popular Dracula theme colors.
pub struct DraculaTheme;

impl ColorTheme for DraculaTheme {
    fn neon_pink(&self) -> Color {
        Color::Rgb(255, 121, 198) // #FF79C6
    }

    fn neon_cyan(&self) -> Color {
        Color::Rgb(139, 233, 253) // #8BE9FD
    }

    fn neon_purple(&self) -> Color {
        Color::Rgb(189, 147, 249) // #BD93F9
    }

    fn neon_green(&self) -> Color {
        Color::Rgb(80, 250, 123) // #50FA7B
    }

    fn neon_yellow(&self) -> Color {
        Color::Rgb(241, 250, 140) // #F1FA8C
    }

    fn neon_orange(&self) -> Color {
        Color::Rgb(255, 184, 108) // #FFB86C
    }

    fn neon_blue(&self) -> Color {
        Color::Rgb(139, 233, 253) // #8BE9FD (same as cyan)
    }

    fn bg_dark(&self) -> Color {
        Color::Rgb(40, 42, 54) // #282A36
    }

    fn bg_panel(&self) -> Color {
        Color::Rgb(50, 52, 64)
    }

    fn bg_surface(&self) -> Color {
        Color::Rgb(68, 71, 90) // #44475A
    }

    fn bg_highlight(&self) -> Color {
        Color::Rgb(68, 71, 90) // #44475A
    }

    fn text_primary(&self) -> Color {
        Color::Rgb(248, 248, 242) // #F8F8F2
    }

    fn text_secondary(&self) -> Color {
        Color::Rgb(200, 200, 200)
    }

    fn text_muted(&self) -> Color {
        Color::Rgb(98, 114, 164) // #6272A4
    }

    fn error(&self) -> Color {
        Color::Rgb(255, 85, 85) // #FF5555
    }

    fn status_ended(&self) -> Color {
        Color::Rgb(98, 114, 164) // #6272A4
    }

    fn border_secondary(&self) -> Color {
        Color::Rgb(68, 71, 90) // #44475A
    }

    fn diff_add_bg(&self) -> Color {
        Color::Rgb(30, 60, 40)
    }

    fn diff_del_bg(&self) -> Color {
        Color::Rgb(60, 30, 30)
    }
}

// ==================== Nord Theme ====================

/// Nord color palette
///
/// The Arctic, north-bluish color palette.
pub struct NordTheme;

impl ColorTheme for NordTheme {
    fn neon_pink(&self) -> Color {
        Color::Rgb(180, 142, 173) // #B48EAD (Nord Aurora purple-pink)
    }

    fn neon_cyan(&self) -> Color {
        Color::Rgb(136, 192, 208) // #88C0D0 (Nord Frost)
    }

    fn neon_purple(&self) -> Color {
        Color::Rgb(180, 142, 173) // #B48EAD
    }

    fn neon_green(&self) -> Color {
        Color::Rgb(163, 190, 140) // #A3BE8C
    }

    fn neon_yellow(&self) -> Color {
        Color::Rgb(235, 203, 139) // #EBCB8B
    }

    fn neon_orange(&self) -> Color {
        Color::Rgb(208, 135, 112) // #D08770
    }

    fn neon_blue(&self) -> Color {
        Color::Rgb(94, 129, 172) // #5E81AC
    }

    fn bg_dark(&self) -> Color {
        Color::Rgb(46, 52, 64) // #2E3440 (Nord Polar Night)
    }

    fn bg_panel(&self) -> Color {
        Color::Rgb(59, 66, 82) // #3B4252
    }

    fn bg_surface(&self) -> Color {
        Color::Rgb(67, 76, 94) // #434C5E
    }

    fn bg_highlight(&self) -> Color {
        Color::Rgb(76, 86, 106) // #4C566A
    }

    fn text_primary(&self) -> Color {
        Color::Rgb(236, 239, 244) // #ECEFF4 (Nord Snow Storm)
    }

    fn text_secondary(&self) -> Color {
        Color::Rgb(216, 222, 233) // #D8DEE9
    }

    fn text_muted(&self) -> Color {
        Color::Rgb(129, 161, 193) // #81A1C1
    }

    fn error(&self) -> Color {
        Color::Rgb(191, 97, 106) // #BF616A (Nord Aurora red)
    }

    fn status_ended(&self) -> Color {
        Color::Rgb(76, 86, 106) // #4C566A
    }

    fn border_secondary(&self) -> Color {
        Color::Rgb(67, 76, 94) // #434C5E
    }

    fn diff_add_bg(&self) -> Color {
        Color::Rgb(40, 55, 45)
    }

    fn diff_del_bg(&self) -> Color {
        Color::Rgb(55, 40, 45)
    }

    fn border_primary(&self) -> Color {
        Color::Rgb(129, 161, 193) // #81A1C1 (softer than cyan)
    }

    fn tab_active_bg(&self) -> Color {
        Color::Rgb(136, 192, 208) // #88C0D0
    }
}

// ==================== Arctic Aurora Theme ====================

/// Arctic Aurora color palette
///
/// A Nordic theme inspired by the aurora borealis, featuring
/// blue-green to purple gradient-like colors against a deep night sky.
pub struct ArcticAuroraTheme;

impl ColorTheme for ArcticAuroraTheme {
    fn neon_pink(&self) -> Color {
        Color::Rgb(201, 160, 220) // #C9A0DC - Soft purple (aurora pink)
    }

    fn neon_cyan(&self) -> Color {
        Color::Rgb(127, 219, 202) // #7FDBCA - Teal (aurora green)
    }

    fn neon_purple(&self) -> Color {
        Color::Rgb(157, 142, 201) // #9D8EC9 - Purple (aurora purple)
    }

    fn neon_green(&self) -> Color {
        Color::Rgb(163, 217, 165) // #A3D9A5 - Pale green (aurora green)
    }

    fn neon_yellow(&self) -> Color {
        Color::Rgb(240, 230, 140) // #F0E68C - Khaki (subtle yellow)
    }

    fn neon_orange(&self) -> Color {
        Color::Rgb(232, 184, 157) // #E8B89D - Salmon pink
    }

    fn neon_blue(&self) -> Color {
        Color::Rgb(135, 206, 235) // #87CEEB - Sky blue
    }

    fn bg_dark(&self) -> Color {
        Color::Rgb(28, 35, 49) // #1C2331 - Deep navy
    }

    fn bg_panel(&self) -> Color {
        Color::Rgb(37, 45, 59) // #252D3B - Slightly lighter navy
    }

    fn bg_surface(&self) -> Color {
        Color::Rgb(46, 56, 71) // #2E3847 - Panel surface
    }

    fn bg_highlight(&self) -> Color {
        Color::Rgb(58, 69, 86) // #3A4556 - Highlight
    }

    fn text_primary(&self) -> Color {
        Color::Rgb(232, 238, 245) // #E8EEF5 - Snow white
    }

    fn text_secondary(&self) -> Color {
        Color::Rgb(184, 197, 214) // #B8C5D6 - Light blue-gray
    }

    fn text_muted(&self) -> Color {
        Color::Rgb(107, 122, 143) // #6B7A8F - Muted blue
    }

    fn error(&self) -> Color {
        Color::Rgb(229, 115, 115) // #E57373 - Soft red
    }

    fn status_ended(&self) -> Color {
        Color::Rgb(93, 107, 126) // #5D6B7E - Gray
    }

    fn border_secondary(&self) -> Color {
        Color::Rgb(58, 69, 86) // #3A4556
    }

    fn diff_add_bg(&self) -> Color {
        Color::Rgb(30, 58, 47) // #1E3A2F - Dark green
    }

    fn diff_del_bg(&self) -> Color {
        Color::Rgb(58, 40, 50) // #3A2832 - Dark red
    }

    fn border_primary(&self) -> Color {
        Color::Rgb(127, 219, 202) // #7FDBCA - Teal (aurora accent)
    }

    fn tab_active_bg(&self) -> Color {
        Color::Rgb(127, 219, 202) // #7FDBCA - Teal
    }
}

// ==================== Minimal Theme ====================

/// Minimal color palette
///
/// A subdued, professional color scheme with dark gray background.
pub struct MinimalTheme;

impl ColorTheme for MinimalTheme {
    fn neon_pink(&self) -> Color {
        Color::Rgb(200, 120, 150) // Muted pink
    }

    fn neon_cyan(&self) -> Color {
        Color::Rgb(74, 158, 255) // #4A9EFF - Calm blue
    }

    fn neon_purple(&self) -> Color {
        Color::Rgb(150, 130, 180)
    }

    fn neon_green(&self) -> Color {
        Color::Rgb(76, 175, 80) // #4CAF50 - Material green
    }

    fn neon_yellow(&self) -> Color {
        Color::Rgb(255, 152, 0) // #FF9800 - Material orange (less harsh than yellow)
    }

    fn neon_orange(&self) -> Color {
        Color::Rgb(255, 152, 0) // #FF9800
    }

    fn neon_blue(&self) -> Color {
        Color::Rgb(74, 158, 255) // #4A9EFF
    }

    fn bg_dark(&self) -> Color {
        Color::Rgb(26, 26, 26) // #1A1A1A
    }

    fn bg_panel(&self) -> Color {
        Color::Rgb(35, 35, 35) // #232323
    }

    fn bg_surface(&self) -> Color {
        Color::Rgb(45, 45, 45) // #2D2D2D
    }

    fn bg_highlight(&self) -> Color {
        Color::Rgb(55, 55, 55) // #373737
    }

    fn text_primary(&self) -> Color {
        Color::Rgb(224, 224, 224) // #E0E0E0
    }

    fn text_secondary(&self) -> Color {
        Color::Rgb(158, 158, 158) // #9E9E9E
    }

    fn text_muted(&self) -> Color {
        Color::Rgb(97, 97, 97) // #616161
    }

    fn error(&self) -> Color {
        Color::Rgb(244, 67, 54) // #F44336 - Material red
    }

    fn status_ended(&self) -> Color {
        Color::Rgb(97, 97, 97) // #616161
    }

    fn border_secondary(&self) -> Color {
        Color::Rgb(66, 66, 66) // #424242
    }

    fn diff_add_bg(&self) -> Color {
        Color::Rgb(25, 45, 30)
    }

    fn diff_del_bg(&self) -> Color {
        Color::Rgb(50, 25, 25)
    }

    fn border_primary(&self) -> Color {
        Color::Rgb(74, 158, 255) // Match accent blue
    }

    fn warning(&self) -> Color {
        Color::Rgb(255, 152, 0) // #FF9800
    }

    fn tab_active_bg(&self) -> Color {
        Color::Rgb(74, 158, 255) // #4A9EFF
    }
}

// ==================== Legacy Type Alias ====================

/// Helper type alias for backward compatibility
///
/// Use `theme()` function instead for dynamic theme access.
#[deprecated(since = "0.2.0", note = "Use theme() function for dynamic theme access")]
pub type Theme = CyberpunkTheme;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyberpunk_theme_colors() {
        let theme = CyberpunkTheme;
        assert!(matches!(theme.neon_pink(), Color::Rgb(255, 0, 128)));
        assert!(matches!(theme.neon_cyan(), Color::Rgb(0, 255, 255)));
        assert!(matches!(theme.neon_green(), Color::Rgb(57, 255, 20)));
    }

    #[test]
    fn test_monokai_theme_colors() {
        let theme = MonokaiTheme;
        assert!(matches!(theme.neon_pink(), Color::Rgb(249, 38, 114)));
        assert!(matches!(theme.neon_green(), Color::Rgb(166, 226, 46)));
        assert!(matches!(theme.bg_dark(), Color::Rgb(39, 40, 34)));
    }

    #[test]
    fn test_dracula_theme_colors() {
        let theme = DraculaTheme;
        assert!(matches!(theme.neon_pink(), Color::Rgb(255, 121, 198)));
        assert!(matches!(theme.neon_green(), Color::Rgb(80, 250, 123)));
        assert!(matches!(theme.bg_dark(), Color::Rgb(40, 42, 54)));
    }

    #[test]
    fn test_nord_theme_colors() {
        let theme = NordTheme;
        assert!(matches!(theme.neon_cyan(), Color::Rgb(136, 192, 208)));
        assert!(matches!(theme.neon_green(), Color::Rgb(163, 190, 140)));
        assert!(matches!(theme.bg_dark(), Color::Rgb(46, 52, 64)));
    }

    #[test]
    fn test_minimal_theme_colors() {
        let theme = MinimalTheme;
        assert!(matches!(theme.neon_cyan(), Color::Rgb(74, 158, 255)));
        assert!(matches!(theme.neon_green(), Color::Rgb(76, 175, 80)));
        assert!(matches!(theme.bg_dark(), Color::Rgb(26, 26, 26)));
    }

    #[test]
    fn test_arctic_aurora_theme_colors() {
        let theme = ArcticAuroraTheme;
        // Accent colors (aurora)
        assert!(matches!(theme.neon_pink(), Color::Rgb(201, 160, 220))); // Soft purple
        assert!(matches!(theme.neon_cyan(), Color::Rgb(127, 219, 202))); // Teal
        assert!(matches!(theme.neon_purple(), Color::Rgb(157, 142, 201))); // Purple
        assert!(matches!(theme.neon_green(), Color::Rgb(163, 217, 165))); // Pale green
        assert!(matches!(theme.neon_yellow(), Color::Rgb(240, 230, 140))); // Khaki
        assert!(matches!(theme.neon_orange(), Color::Rgb(232, 184, 157))); // Salmon
        assert!(matches!(theme.neon_blue(), Color::Rgb(135, 206, 235))); // Sky blue
        // Background colors (deep night sky)
        assert!(matches!(theme.bg_dark(), Color::Rgb(28, 35, 49))); // Deep navy
        assert!(matches!(theme.bg_panel(), Color::Rgb(37, 45, 59)));
        assert!(matches!(theme.bg_surface(), Color::Rgb(46, 56, 71)));
        assert!(matches!(theme.bg_highlight(), Color::Rgb(58, 69, 86)));
        // Text colors
        assert!(matches!(theme.text_primary(), Color::Rgb(232, 238, 245))); // Snow white
        assert!(matches!(theme.text_secondary(), Color::Rgb(184, 197, 214)));
        assert!(matches!(theme.text_muted(), Color::Rgb(107, 122, 143)));
        // Other colors
        assert!(matches!(theme.error(), Color::Rgb(229, 115, 115))); // Soft red
        assert!(matches!(theme.status_ended(), Color::Rgb(93, 107, 126)));
    }

    #[test]
    fn test_style_constructors() {
        let theme = CyberpunkTheme;
        // Ensure style constructors don't panic
        let _ = theme.style_tab_active();
        let _ = theme.style_tab_inactive();
        let _ = theme.style_border();
        let _ = theme.style_success();
        let _ = theme.style_error();
        let _ = theme.style_warning();
        let _ = theme.style_info();
        let _ = theme.style_text();
        let _ = theme.style_key();
        let _ = theme.style_selected();
    }

    #[test]
    fn test_semantic_color_defaults() {
        let theme = CyberpunkTheme;
        // Test default implementations
        assert_eq!(theme.success(), theme.neon_green());
        assert_eq!(theme.warning(), theme.neon_yellow());
        assert_eq!(theme.info(), theme.neon_cyan());
        assert_eq!(theme.status_running(), theme.neon_green());
        assert_eq!(theme.status_idle(), theme.neon_yellow());
        assert_eq!(theme.status_error(), theme.error());
    }

    #[test]
    fn test_create_theme() {
        let cyberpunk = create_theme("cyberpunk");
        assert!(matches!(cyberpunk.neon_pink(), Color::Rgb(255, 0, 128)));

        let monokai = create_theme("monokai");
        assert!(matches!(monokai.neon_pink(), Color::Rgb(249, 38, 114)));

        let dracula = create_theme("Dracula"); // Test case insensitivity
        assert!(matches!(dracula.neon_pink(), Color::Rgb(255, 121, 198)));

        let nord = create_theme("NORD");
        assert!(matches!(nord.bg_dark(), Color::Rgb(46, 52, 64)));

        let minimal = create_theme("minimal");
        assert!(matches!(minimal.bg_dark(), Color::Rgb(26, 26, 26)));

        // Arctic Aurora theme can be selected by multiple names
        let arctic = create_theme("arctic");
        assert!(matches!(arctic.bg_dark(), Color::Rgb(28, 35, 49)));
        let aurora = create_theme("aurora");
        assert!(matches!(aurora.bg_dark(), Color::Rgb(28, 35, 49)));
        let arctic_aurora = create_theme("arctic-aurora");
        assert!(matches!(arctic_aurora.bg_dark(), Color::Rgb(28, 35, 49)));

        // Unknown theme defaults to Cyberpunk
        let unknown = create_theme("unknown");
        assert!(matches!(unknown.neon_pink(), Color::Rgb(255, 0, 128)));
    }

    #[test]
    fn test_diff_colors() {
        let theme = CyberpunkTheme;
        assert_eq!(theme.diff_addition(), theme.neon_green());
        assert_eq!(theme.diff_deletion(), theme.error());
        assert_eq!(theme.diff_hunk_header(), theme.neon_cyan());
        assert_eq!(theme.diff_file_header(), theme.neon_yellow());
    }

    #[test]
    fn test_status_colors() {
        let theme = CyberpunkTheme;
        assert_eq!(theme.status_running(), theme.neon_green());
        assert_eq!(theme.status_idle(), theme.neon_yellow());
        assert!(matches!(theme.status_ended(), Color::Rgb(100, 80, 120)));
        assert_eq!(theme.status_error(), theme.error());
    }
}
