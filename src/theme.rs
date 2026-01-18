//! Color theme module for cctakt
//!
//! Provides multiple color themes with a struct-based design for easy switching.
//! Themes include Cyberpunk (default), Monokai, Dracula, Nord, Arctic Aurora, and Minimal.

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::RwLock;

// ==================== ThemeId Enum ====================

/// Theme identifier for type-safe theme selection
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ThemeId {
    #[default]
    Cyberpunk,
    Monokai,
    Dracula,
    Nord,
    ArcticAurora,
    Minimal,
}

impl ThemeId {
    /// Get the display name of the theme
    pub fn name(&self) -> &'static str {
        match self {
            ThemeId::Cyberpunk => "Cyberpunk",
            ThemeId::Monokai => "Monokai",
            ThemeId::Dracula => "Dracula",
            ThemeId::Nord => "Nord",
            ThemeId::ArcticAurora => "Arctic Aurora",
            ThemeId::Minimal => "Minimal",
        }
    }

    /// Get the description of the theme
    pub fn description(&self) -> &'static str {
        match self {
            ThemeId::Cyberpunk => "ネオンカラーのサイバーパンク風",
            ThemeId::Monokai => "クラシックなエディタカラー",
            ThemeId::Dracula => "人気のダークテーマ",
            ThemeId::Nord => "北欧の青みのあるパレット",
            ThemeId::ArcticAurora => "オーロラ風の幻想的なテーマ",
            ThemeId::Minimal => "控えめでプロフェッショナル",
        }
    }

    /// Get the ID string for this theme (for serialization)
    pub fn id(&self) -> &'static str {
        match self {
            ThemeId::Cyberpunk => "cyberpunk",
            ThemeId::Monokai => "monokai",
            ThemeId::Dracula => "dracula",
            ThemeId::Nord => "nord",
            ThemeId::ArcticAurora => "arctic",
            ThemeId::Minimal => "minimal",
        }
    }

    /// Get all available theme IDs
    pub fn all() -> &'static [ThemeId] {
        &[
            ThemeId::Cyberpunk,
            ThemeId::Monokai,
            ThemeId::Dracula,
            ThemeId::Nord,
            ThemeId::ArcticAurora,
            ThemeId::Minimal,
        ]
    }
}

impl FromStr for ThemeId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cyberpunk" => Ok(ThemeId::Cyberpunk),
            "monokai" => Ok(ThemeId::Monokai),
            "dracula" => Ok(ThemeId::Dracula),
            "nord" => Ok(ThemeId::Nord),
            "arctic" | "aurora" | "arctic-aurora" | "arcticaurora" => Ok(ThemeId::ArcticAurora),
            "minimal" => Ok(ThemeId::Minimal),
            _ => Err(()),
        }
    }
}

impl Display for ThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

// ==================== ThemeColors Struct ====================

/// Complete color theme definition
#[derive(Clone, Debug)]
pub struct ThemeColors {
    /// Theme identifier
    pub id: ThemeId,

    // ==================== Neon/Accent Colors ====================
    /// Primary accent color (hot pink in Cyberpunk)
    pub neon_pink: Color,
    /// Secondary accent color (cyan in Cyberpunk)
    pub neon_cyan: Color,
    /// Tertiary accent color (purple in Cyberpunk)
    pub neon_purple: Color,
    /// Success/active color (green in Cyberpunk)
    pub neon_green: Color,
    /// Warning color (yellow in Cyberpunk)
    pub neon_yellow: Color,
    /// Highlight color (orange in Cyberpunk)
    pub neon_orange: Color,
    /// Info color (blue in Cyberpunk)
    pub neon_blue: Color,

    // ==================== Background Colors ====================
    /// Main background color
    pub bg_dark: Color,
    /// Panel background color
    pub bg_panel: Color,
    /// Surface/elevated background color
    pub bg_surface: Color,
    /// Highlight background color
    pub bg_highlight: Color,

    // ==================== Text Colors ====================
    /// Primary text color
    pub text_primary: Color,
    /// Secondary text color
    pub text_secondary: Color,
    /// Muted text color
    pub text_muted: Color,

    // ==================== Semantic Colors ====================
    /// Error color
    pub error: Color,
    /// Status ended color
    pub status_ended: Color,

    // ==================== Border Colors ====================
    /// Secondary border color
    pub border_secondary: Color,

    // ==================== Diff Colors ====================
    /// Addition background color
    pub diff_add_bg: Color,
    /// Deletion background color
    pub diff_del_bg: Color,

    // ==================== Optional Overrides ====================
    /// Override for border_primary (defaults to neon_cyan)
    pub border_primary_override: Option<Color>,
    /// Override for tab_active_bg (defaults to neon_cyan)
    pub tab_active_bg_override: Option<Color>,
    /// Override for warning (defaults to neon_yellow)
    pub warning_override: Option<Color>,
}

impl ThemeColors {
    // ==================== Accessor Methods ====================

    /// Primary accent color
    pub fn neon_pink(&self) -> Color {
        self.neon_pink
    }

    /// Secondary accent color
    pub fn neon_cyan(&self) -> Color {
        self.neon_cyan
    }

    /// Tertiary accent color
    pub fn neon_purple(&self) -> Color {
        self.neon_purple
    }

    /// Success/active color
    pub fn neon_green(&self) -> Color {
        self.neon_green
    }

    /// Warning color
    pub fn neon_yellow(&self) -> Color {
        self.neon_yellow
    }

    /// Highlight color
    pub fn neon_orange(&self) -> Color {
        self.neon_orange
    }

    /// Info color
    pub fn neon_blue(&self) -> Color {
        self.neon_blue
    }

    /// Main background color
    pub fn bg_dark(&self) -> Color {
        self.bg_dark
    }

    /// Panel background color
    pub fn bg_panel(&self) -> Color {
        self.bg_panel
    }

    /// Surface/elevated background color
    pub fn bg_surface(&self) -> Color {
        self.bg_surface
    }

    /// Highlight background color
    pub fn bg_highlight(&self) -> Color {
        self.bg_highlight
    }

    /// Primary text color
    pub fn text_primary(&self) -> Color {
        self.text_primary
    }

    /// Secondary text color
    pub fn text_secondary(&self) -> Color {
        self.text_secondary
    }

    /// Muted text color
    pub fn text_muted(&self) -> Color {
        self.text_muted
    }

    /// Error color
    pub fn error(&self) -> Color {
        self.error
    }

    /// Status ended color
    pub fn status_ended(&self) -> Color {
        self.status_ended
    }

    /// Secondary border color
    pub fn border_secondary(&self) -> Color {
        self.border_secondary
    }

    /// Addition background color
    pub fn diff_add_bg(&self) -> Color {
        self.diff_add_bg
    }

    /// Deletion background color
    pub fn diff_del_bg(&self) -> Color {
        self.diff_del_bg
    }

    // ==================== Computed Colors ====================

    /// Success color (defaults to neon_green)
    pub fn success(&self) -> Color {
        self.neon_green
    }

    /// Warning color (defaults to neon_yellow, can be overridden)
    pub fn warning(&self) -> Color {
        self.warning_override.unwrap_or(self.neon_yellow)
    }

    /// Info color (defaults to neon_cyan)
    pub fn info(&self) -> Color {
        self.neon_cyan
    }

    /// Running status color
    pub fn status_running(&self) -> Color {
        self.neon_green
    }

    /// Idle status color
    pub fn status_idle(&self) -> Color {
        self.neon_yellow
    }

    /// Error status color
    pub fn status_error(&self) -> Color {
        self.error
    }

    /// Primary border color (defaults to neon_cyan, can be overridden)
    pub fn border_primary(&self) -> Color {
        self.border_primary_override.unwrap_or(self.neon_cyan)
    }

    /// Active/focused border color
    pub fn border_active(&self) -> Color {
        self.neon_pink
    }

    /// Addition text color
    pub fn diff_addition(&self) -> Color {
        self.neon_green
    }

    /// Deletion text color
    pub fn diff_deletion(&self) -> Color {
        self.error
    }

    /// Context line color
    pub fn diff_context(&self) -> Color {
        self.text_primary
    }

    /// Hunk header color
    pub fn diff_hunk_header(&self) -> Color {
        self.neon_cyan
    }

    /// File header color
    pub fn diff_file_header(&self) -> Color {
        self.neon_yellow
    }

    /// Active tab background (defaults to neon_cyan, can be overridden)
    pub fn tab_active_bg(&self) -> Color {
        self.tab_active_bg_override.unwrap_or(self.neon_cyan)
    }

    /// Active tab foreground
    pub fn tab_active_fg(&self) -> Color {
        Color::Rgb(0, 0, 0)
    }

    /// Selected item background
    pub fn selected_bg(&self) -> Color {
        self.bg_highlight
    }

    /// Cursor background
    pub fn cursor_bg(&self) -> Color {
        self.neon_cyan
    }

    /// Cursor foreground
    pub fn cursor_fg(&self) -> Color {
        Color::Rgb(0, 0, 0)
    }

    /// Key binding color
    pub fn key_binding(&self) -> Color {
        self.neon_cyan
    }

    /// Key description color
    pub fn key_description(&self) -> Color {
        self.text_muted
    }

    /// Issue number color
    pub fn issue_number(&self) -> Color {
        self.neon_yellow
    }

    /// Issue label color
    pub fn issue_label(&self) -> Color {
        self.neon_purple
    }

    // ==================== Style Methods ====================

    /// Style for active tab
    pub fn style_tab_active(&self) -> Style {
        Style::default()
            .fg(self.tab_active_fg())
            .bg(self.tab_active_bg())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for inactive tab
    pub fn style_tab_inactive(&self) -> Style {
        Style::default()
            .fg(self.text_secondary)
            .bg(self.bg_surface)
    }

    /// Style for primary border
    pub fn style_border(&self) -> Style {
        Style::default().fg(self.border_primary())
    }

    /// Style for secondary (muted) border
    pub fn style_border_muted(&self) -> Style {
        Style::default().fg(self.border_secondary)
    }

    /// Style for dialog border
    pub fn style_dialog_border(&self) -> Style {
        Style::default().fg(self.neon_cyan)
    }

    /// Style for success text
    pub fn style_success(&self) -> Style {
        Style::default()
            .fg(self.success())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for error text
    pub fn style_error(&self) -> Style {
        Style::default()
            .fg(self.error)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for warning text
    pub fn style_warning(&self) -> Style {
        Style::default().fg(self.warning())
    }

    /// Style for info text
    pub fn style_info(&self) -> Style {
        Style::default().fg(self.info())
    }

    /// Style for primary text
    pub fn style_text(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Style for secondary text
    pub fn style_text_secondary(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Style for muted text
    pub fn style_text_muted(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    /// Style for key bindings
    pub fn style_key(&self) -> Style {
        Style::default()
            .fg(self.key_binding())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for key descriptions
    pub fn style_key_desc(&self) -> Style {
        Style::default().fg(self.key_description())
    }

    /// Style for selected/highlighted items
    pub fn style_selected(&self) -> Style {
        Style::default()
            .bg(self.selected_bg())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for cursor
    pub fn style_cursor(&self) -> Style {
        Style::default()
            .fg(self.cursor_fg())
            .bg(self.cursor_bg())
    }

    /// Style for input text
    pub fn style_input(&self) -> Style {
        Style::default().fg(self.neon_yellow)
    }

    /// Style for loading indicator
    pub fn style_loading(&self) -> Style {
        Style::default().fg(self.neon_yellow)
    }

    /// Style for dialog background
    pub fn style_dialog_bg(&self) -> Style {
        Style::default().bg(self.bg_dark)
    }
}

// ==================== Theme Definitions ====================

/// Cyberpunk theme - neon colors with dark background
pub const CYBERPUNK: ThemeColors = ThemeColors {
    id: ThemeId::Cyberpunk,
    neon_pink: Color::Rgb(255, 0, 128),
    neon_cyan: Color::Rgb(0, 255, 255),
    neon_purple: Color::Rgb(157, 78, 221),
    neon_green: Color::Rgb(57, 255, 20),
    neon_yellow: Color::Rgb(255, 255, 0),
    neon_orange: Color::Rgb(255, 165, 0),
    neon_blue: Color::Rgb(0, 191, 255),
    bg_dark: Color::Rgb(13, 13, 26),
    bg_panel: Color::Rgb(26, 26, 46),
    bg_surface: Color::Rgb(35, 35, 60),
    bg_highlight: Color::Rgb(50, 50, 80),
    text_primary: Color::Rgb(230, 230, 240),
    text_secondary: Color::Rgb(140, 140, 160),
    text_muted: Color::Rgb(90, 90, 110),
    error: Color::Rgb(255, 50, 80),
    status_ended: Color::Rgb(100, 80, 120),
    border_secondary: Color::Rgb(60, 60, 90),
    diff_add_bg: Color::Rgb(0, 50, 20),
    diff_del_bg: Color::Rgb(50, 0, 20),
    border_primary_override: None,
    tab_active_bg_override: None,
    warning_override: None,
};

/// Monokai theme - classic editor colors
pub const MONOKAI: ThemeColors = ThemeColors {
    id: ThemeId::Monokai,
    neon_pink: Color::Rgb(249, 38, 114),
    neon_cyan: Color::Rgb(102, 217, 239),
    neon_purple: Color::Rgb(174, 129, 255),
    neon_green: Color::Rgb(166, 226, 46),
    neon_yellow: Color::Rgb(230, 219, 116),
    neon_orange: Color::Rgb(253, 151, 31),
    neon_blue: Color::Rgb(102, 217, 239),
    bg_dark: Color::Rgb(39, 40, 34),
    bg_panel: Color::Rgb(49, 50, 44),
    bg_surface: Color::Rgb(59, 60, 54),
    bg_highlight: Color::Rgb(73, 72, 62),
    text_primary: Color::Rgb(248, 248, 242),
    text_secondary: Color::Rgb(175, 175, 165),
    text_muted: Color::Rgb(117, 113, 94),
    error: Color::Rgb(249, 38, 114),
    status_ended: Color::Rgb(117, 113, 94),
    border_secondary: Color::Rgb(73, 72, 62),
    diff_add_bg: Color::Rgb(30, 50, 20),
    diff_del_bg: Color::Rgb(50, 20, 30),
    border_primary_override: None,
    tab_active_bg_override: None,
    warning_override: None,
};

/// Dracula theme - popular dark theme
pub const DRACULA: ThemeColors = ThemeColors {
    id: ThemeId::Dracula,
    neon_pink: Color::Rgb(255, 121, 198),
    neon_cyan: Color::Rgb(139, 233, 253),
    neon_purple: Color::Rgb(189, 147, 249),
    neon_green: Color::Rgb(80, 250, 123),
    neon_yellow: Color::Rgb(241, 250, 140),
    neon_orange: Color::Rgb(255, 184, 108),
    neon_blue: Color::Rgb(139, 233, 253),
    bg_dark: Color::Rgb(40, 42, 54),
    bg_panel: Color::Rgb(50, 52, 64),
    bg_surface: Color::Rgb(68, 71, 90),
    bg_highlight: Color::Rgb(68, 71, 90),
    text_primary: Color::Rgb(248, 248, 242),
    text_secondary: Color::Rgb(200, 200, 200),
    text_muted: Color::Rgb(98, 114, 164),
    error: Color::Rgb(255, 85, 85),
    status_ended: Color::Rgb(98, 114, 164),
    border_secondary: Color::Rgb(68, 71, 90),
    diff_add_bg: Color::Rgb(30, 60, 40),
    diff_del_bg: Color::Rgb(60, 30, 30),
    border_primary_override: None,
    tab_active_bg_override: None,
    warning_override: None,
};

/// Nord theme - arctic, north-bluish colors
pub const NORD: ThemeColors = ThemeColors {
    id: ThemeId::Nord,
    neon_pink: Color::Rgb(180, 142, 173),
    neon_cyan: Color::Rgb(136, 192, 208),
    neon_purple: Color::Rgb(180, 142, 173),
    neon_green: Color::Rgb(163, 190, 140),
    neon_yellow: Color::Rgb(235, 203, 139),
    neon_orange: Color::Rgb(208, 135, 112),
    neon_blue: Color::Rgb(94, 129, 172),
    bg_dark: Color::Rgb(46, 52, 64),
    bg_panel: Color::Rgb(59, 66, 82),
    bg_surface: Color::Rgb(67, 76, 94),
    bg_highlight: Color::Rgb(76, 86, 106),
    text_primary: Color::Rgb(236, 239, 244),
    text_secondary: Color::Rgb(216, 222, 233),
    text_muted: Color::Rgb(129, 161, 193),
    error: Color::Rgb(191, 97, 106),
    status_ended: Color::Rgb(76, 86, 106),
    border_secondary: Color::Rgb(67, 76, 94),
    diff_add_bg: Color::Rgb(40, 55, 45),
    diff_del_bg: Color::Rgb(55, 40, 45),
    border_primary_override: Some(Color::Rgb(129, 161, 193)),
    tab_active_bg_override: Some(Color::Rgb(136, 192, 208)),
    warning_override: None,
};

/// Arctic Aurora theme - aurora borealis inspired
pub const ARCTIC_AURORA: ThemeColors = ThemeColors {
    id: ThemeId::ArcticAurora,
    neon_pink: Color::Rgb(201, 160, 220),
    neon_cyan: Color::Rgb(127, 219, 202),
    neon_purple: Color::Rgb(157, 142, 201),
    neon_green: Color::Rgb(163, 217, 165),
    neon_yellow: Color::Rgb(240, 230, 140),
    neon_orange: Color::Rgb(232, 184, 157),
    neon_blue: Color::Rgb(135, 206, 235),
    bg_dark: Color::Rgb(28, 35, 49),
    bg_panel: Color::Rgb(37, 45, 59),
    bg_surface: Color::Rgb(46, 56, 71),
    bg_highlight: Color::Rgb(58, 69, 86),
    text_primary: Color::Rgb(232, 238, 245),
    text_secondary: Color::Rgb(184, 197, 214),
    text_muted: Color::Rgb(107, 122, 143),
    error: Color::Rgb(229, 115, 115),
    status_ended: Color::Rgb(93, 107, 126),
    border_secondary: Color::Rgb(58, 69, 86),
    diff_add_bg: Color::Rgb(30, 58, 47),
    diff_del_bg: Color::Rgb(58, 40, 50),
    border_primary_override: Some(Color::Rgb(127, 219, 202)),
    tab_active_bg_override: Some(Color::Rgb(127, 219, 202)),
    warning_override: None,
};

/// Minimal theme - subdued, professional colors
pub const MINIMAL: ThemeColors = ThemeColors {
    id: ThemeId::Minimal,
    neon_pink: Color::Rgb(200, 120, 150),
    neon_cyan: Color::Rgb(74, 158, 255),
    neon_purple: Color::Rgb(150, 130, 180),
    neon_green: Color::Rgb(76, 175, 80),
    neon_yellow: Color::Rgb(255, 152, 0),
    neon_orange: Color::Rgb(255, 152, 0),
    neon_blue: Color::Rgb(74, 158, 255),
    bg_dark: Color::Rgb(26, 26, 26),
    bg_panel: Color::Rgb(35, 35, 35),
    bg_surface: Color::Rgb(45, 45, 45),
    bg_highlight: Color::Rgb(55, 55, 55),
    text_primary: Color::Rgb(224, 224, 224),
    text_secondary: Color::Rgb(158, 158, 158),
    text_muted: Color::Rgb(97, 97, 97),
    error: Color::Rgb(244, 67, 54),
    status_ended: Color::Rgb(97, 97, 97),
    border_secondary: Color::Rgb(66, 66, 66),
    diff_add_bg: Color::Rgb(25, 45, 30),
    diff_del_bg: Color::Rgb(50, 25, 25),
    border_primary_override: Some(Color::Rgb(74, 158, 255)),
    tab_active_bg_override: Some(Color::Rgb(74, 158, 255)),
    warning_override: Some(Color::Rgb(255, 152, 0)),
};

// ==================== Global Theme Management ====================

/// Current theme ID
static CURRENT_THEME_ID: RwLock<ThemeId> = RwLock::new(ThemeId::Cyberpunk);

/// Get the ThemeColors for a given ThemeId
pub fn get_theme_colors(id: ThemeId) -> &'static ThemeColors {
    match id {
        ThemeId::Cyberpunk => &CYBERPUNK,
        ThemeId::Monokai => &MONOKAI,
        ThemeId::Dracula => &DRACULA,
        ThemeId::Nord => &NORD,
        ThemeId::ArcticAurora => &ARCTIC_AURORA,
        ThemeId::Minimal => &MINIMAL,
    }
}

/// Get the current theme
///
/// Returns a reference to the current theme colors.
pub fn theme() -> &'static ThemeColors {
    let id = CURRENT_THEME_ID.read().unwrap_or_else(|e| e.into_inner());
    get_theme_colors(*id)
}

/// Get the current theme ID
pub fn current_theme_id() -> ThemeId {
    *CURRENT_THEME_ID.read().unwrap_or_else(|e| e.into_inner())
}

/// Get the current theme ID as a string
pub fn current_theme_id_str() -> &'static str {
    current_theme_id().id()
}

/// Set the global theme by ID
///
/// This can be called multiple times to change the theme at runtime.
/// Returns true if the theme was set successfully.
pub fn set_theme_by_id(id: ThemeId) -> bool {
    match CURRENT_THEME_ID.write() {
        Ok(mut guard) => {
            *guard = id;
            true
        }
        Err(e) => {
            // Recover from poisoned lock
            let mut guard = e.into_inner();
            *guard = id;
            true
        }
    }
}

/// Set the global theme from a theme name string
///
/// For backwards compatibility with existing code.
pub fn set_theme_from_str(name: &str) -> bool {
    let id = name.parse().unwrap_or(ThemeId::Cyberpunk);
    set_theme_by_id(id)
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

// ==================== Legacy API Compatibility ====================

/// ColorTheme trait for backwards compatibility
///
/// This trait is implemented by ThemeColors to maintain compatibility
/// with existing code that uses the trait-based API.
pub trait ColorTheme: Send + Sync {
    fn neon_pink(&self) -> Color;
    fn neon_cyan(&self) -> Color;
    fn neon_purple(&self) -> Color;
    fn neon_green(&self) -> Color;
    fn neon_yellow(&self) -> Color;
    fn neon_orange(&self) -> Color;
    fn neon_blue(&self) -> Color;
    fn bg_dark(&self) -> Color;
    fn bg_panel(&self) -> Color;
    fn bg_surface(&self) -> Color;
    fn bg_highlight(&self) -> Color;
    fn text_primary(&self) -> Color;
    fn text_secondary(&self) -> Color;
    fn text_muted(&self) -> Color;
    fn error(&self) -> Color;
    fn status_ended(&self) -> Color;
    fn border_secondary(&self) -> Color;
    fn diff_add_bg(&self) -> Color;
    fn diff_del_bg(&self) -> Color;
    fn success(&self) -> Color { self.neon_green() }
    fn warning(&self) -> Color { self.neon_yellow() }
    fn info(&self) -> Color { self.neon_cyan() }
    fn status_running(&self) -> Color { self.neon_green() }
    fn status_idle(&self) -> Color { self.neon_yellow() }
    fn status_error(&self) -> Color { self.error() }
    fn border_primary(&self) -> Color { self.neon_cyan() }
    fn border_active(&self) -> Color { self.neon_pink() }
    fn diff_addition(&self) -> Color { self.neon_green() }
    fn diff_deletion(&self) -> Color { self.error() }
    fn diff_context(&self) -> Color { self.text_primary() }
    fn diff_hunk_header(&self) -> Color { self.neon_cyan() }
    fn diff_file_header(&self) -> Color { self.neon_yellow() }
    fn tab_active_bg(&self) -> Color { self.neon_cyan() }
    fn tab_active_fg(&self) -> Color { Color::Rgb(0, 0, 0) }
    fn selected_bg(&self) -> Color { self.bg_highlight() }
    fn cursor_bg(&self) -> Color { self.neon_cyan() }
    fn cursor_fg(&self) -> Color { Color::Rgb(0, 0, 0) }
    fn key_binding(&self) -> Color { self.neon_cyan() }
    fn key_description(&self) -> Color { self.text_muted() }
    fn issue_number(&self) -> Color { self.neon_yellow() }
    fn issue_label(&self) -> Color { self.neon_purple() }
    fn style_tab_active(&self) -> Style {
        Style::default()
            .fg(self.tab_active_fg())
            .bg(self.tab_active_bg())
            .add_modifier(Modifier::BOLD)
    }
    fn style_tab_inactive(&self) -> Style {
        Style::default()
            .fg(self.text_secondary())
            .bg(self.bg_surface())
    }
    fn style_border(&self) -> Style { Style::default().fg(self.border_primary()) }
    fn style_border_muted(&self) -> Style { Style::default().fg(self.border_secondary()) }
    fn style_dialog_border(&self) -> Style { Style::default().fg(self.neon_cyan()) }
    fn style_success(&self) -> Style {
        Style::default()
            .fg(self.success())
            .add_modifier(Modifier::BOLD)
    }
    fn style_error(&self) -> Style {
        Style::default()
            .fg(self.error())
            .add_modifier(Modifier::BOLD)
    }
    fn style_warning(&self) -> Style { Style::default().fg(self.warning()) }
    fn style_info(&self) -> Style { Style::default().fg(self.info()) }
    fn style_text(&self) -> Style { Style::default().fg(self.text_primary()) }
    fn style_text_secondary(&self) -> Style { Style::default().fg(self.text_secondary()) }
    fn style_text_muted(&self) -> Style { Style::default().fg(self.text_muted()) }
    fn style_key(&self) -> Style {
        Style::default()
            .fg(self.key_binding())
            .add_modifier(Modifier::BOLD)
    }
    fn style_key_desc(&self) -> Style { Style::default().fg(self.key_description()) }
    fn style_selected(&self) -> Style {
        Style::default()
            .bg(self.selected_bg())
            .add_modifier(Modifier::BOLD)
    }
    fn style_cursor(&self) -> Style {
        Style::default()
            .fg(self.cursor_fg())
            .bg(self.cursor_bg())
    }
    fn style_input(&self) -> Style { Style::default().fg(self.neon_yellow()) }
    fn style_loading(&self) -> Style { Style::default().fg(self.neon_yellow()) }
    fn style_dialog_bg(&self) -> Style { Style::default().bg(self.bg_dark()) }
}

impl ColorTheme for ThemeColors {
    fn neon_pink(&self) -> Color { self.neon_pink }
    fn neon_cyan(&self) -> Color { self.neon_cyan }
    fn neon_purple(&self) -> Color { self.neon_purple }
    fn neon_green(&self) -> Color { self.neon_green }
    fn neon_yellow(&self) -> Color { self.neon_yellow }
    fn neon_orange(&self) -> Color { self.neon_orange }
    fn neon_blue(&self) -> Color { self.neon_blue }
    fn bg_dark(&self) -> Color { self.bg_dark }
    fn bg_panel(&self) -> Color { self.bg_panel }
    fn bg_surface(&self) -> Color { self.bg_surface }
    fn bg_highlight(&self) -> Color { self.bg_highlight }
    fn text_primary(&self) -> Color { self.text_primary }
    fn text_secondary(&self) -> Color { self.text_secondary }
    fn text_muted(&self) -> Color { self.text_muted }
    fn error(&self) -> Color { self.error }
    fn status_ended(&self) -> Color { self.status_ended }
    fn border_secondary(&self) -> Color { self.border_secondary }
    fn diff_add_bg(&self) -> Color { self.diff_add_bg }
    fn diff_del_bg(&self) -> Color { self.diff_del_bg }
    fn border_primary(&self) -> Color { ThemeColors::border_primary(self) }
    fn tab_active_bg(&self) -> Color { ThemeColors::tab_active_bg(self) }
    fn warning(&self) -> Color { ThemeColors::warning(self) }
}

/// Legacy theme struct names for backwards compatibility
pub struct CyberpunkTheme;
pub struct MonokaiTheme;
pub struct DraculaTheme;
pub struct NordTheme;
pub struct ArcticAuroraTheme;
pub struct MinimalTheme;

impl ColorTheme for CyberpunkTheme {
    fn neon_pink(&self) -> Color { CYBERPUNK.neon_pink }
    fn neon_cyan(&self) -> Color { CYBERPUNK.neon_cyan }
    fn neon_purple(&self) -> Color { CYBERPUNK.neon_purple }
    fn neon_green(&self) -> Color { CYBERPUNK.neon_green }
    fn neon_yellow(&self) -> Color { CYBERPUNK.neon_yellow }
    fn neon_orange(&self) -> Color { CYBERPUNK.neon_orange }
    fn neon_blue(&self) -> Color { CYBERPUNK.neon_blue }
    fn bg_dark(&self) -> Color { CYBERPUNK.bg_dark }
    fn bg_panel(&self) -> Color { CYBERPUNK.bg_panel }
    fn bg_surface(&self) -> Color { CYBERPUNK.bg_surface }
    fn bg_highlight(&self) -> Color { CYBERPUNK.bg_highlight }
    fn text_primary(&self) -> Color { CYBERPUNK.text_primary }
    fn text_secondary(&self) -> Color { CYBERPUNK.text_secondary }
    fn text_muted(&self) -> Color { CYBERPUNK.text_muted }
    fn error(&self) -> Color { CYBERPUNK.error }
    fn status_ended(&self) -> Color { CYBERPUNK.status_ended }
    fn border_secondary(&self) -> Color { CYBERPUNK.border_secondary }
    fn diff_add_bg(&self) -> Color { CYBERPUNK.diff_add_bg }
    fn diff_del_bg(&self) -> Color { CYBERPUNK.diff_del_bg }
}

impl ColorTheme for MonokaiTheme {
    fn neon_pink(&self) -> Color { MONOKAI.neon_pink }
    fn neon_cyan(&self) -> Color { MONOKAI.neon_cyan }
    fn neon_purple(&self) -> Color { MONOKAI.neon_purple }
    fn neon_green(&self) -> Color { MONOKAI.neon_green }
    fn neon_yellow(&self) -> Color { MONOKAI.neon_yellow }
    fn neon_orange(&self) -> Color { MONOKAI.neon_orange }
    fn neon_blue(&self) -> Color { MONOKAI.neon_blue }
    fn bg_dark(&self) -> Color { MONOKAI.bg_dark }
    fn bg_panel(&self) -> Color { MONOKAI.bg_panel }
    fn bg_surface(&self) -> Color { MONOKAI.bg_surface }
    fn bg_highlight(&self) -> Color { MONOKAI.bg_highlight }
    fn text_primary(&self) -> Color { MONOKAI.text_primary }
    fn text_secondary(&self) -> Color { MONOKAI.text_secondary }
    fn text_muted(&self) -> Color { MONOKAI.text_muted }
    fn error(&self) -> Color { MONOKAI.error }
    fn status_ended(&self) -> Color { MONOKAI.status_ended }
    fn border_secondary(&self) -> Color { MONOKAI.border_secondary }
    fn diff_add_bg(&self) -> Color { MONOKAI.diff_add_bg }
    fn diff_del_bg(&self) -> Color { MONOKAI.diff_del_bg }
}

impl ColorTheme for DraculaTheme {
    fn neon_pink(&self) -> Color { DRACULA.neon_pink }
    fn neon_cyan(&self) -> Color { DRACULA.neon_cyan }
    fn neon_purple(&self) -> Color { DRACULA.neon_purple }
    fn neon_green(&self) -> Color { DRACULA.neon_green }
    fn neon_yellow(&self) -> Color { DRACULA.neon_yellow }
    fn neon_orange(&self) -> Color { DRACULA.neon_orange }
    fn neon_blue(&self) -> Color { DRACULA.neon_blue }
    fn bg_dark(&self) -> Color { DRACULA.bg_dark }
    fn bg_panel(&self) -> Color { DRACULA.bg_panel }
    fn bg_surface(&self) -> Color { DRACULA.bg_surface }
    fn bg_highlight(&self) -> Color { DRACULA.bg_highlight }
    fn text_primary(&self) -> Color { DRACULA.text_primary }
    fn text_secondary(&self) -> Color { DRACULA.text_secondary }
    fn text_muted(&self) -> Color { DRACULA.text_muted }
    fn error(&self) -> Color { DRACULA.error }
    fn status_ended(&self) -> Color { DRACULA.status_ended }
    fn border_secondary(&self) -> Color { DRACULA.border_secondary }
    fn diff_add_bg(&self) -> Color { DRACULA.diff_add_bg }
    fn diff_del_bg(&self) -> Color { DRACULA.diff_del_bg }
}

impl ColorTheme for NordTheme {
    fn neon_pink(&self) -> Color { NORD.neon_pink }
    fn neon_cyan(&self) -> Color { NORD.neon_cyan }
    fn neon_purple(&self) -> Color { NORD.neon_purple }
    fn neon_green(&self) -> Color { NORD.neon_green }
    fn neon_yellow(&self) -> Color { NORD.neon_yellow }
    fn neon_orange(&self) -> Color { NORD.neon_orange }
    fn neon_blue(&self) -> Color { NORD.neon_blue }
    fn bg_dark(&self) -> Color { NORD.bg_dark }
    fn bg_panel(&self) -> Color { NORD.bg_panel }
    fn bg_surface(&self) -> Color { NORD.bg_surface }
    fn bg_highlight(&self) -> Color { NORD.bg_highlight }
    fn text_primary(&self) -> Color { NORD.text_primary }
    fn text_secondary(&self) -> Color { NORD.text_secondary }
    fn text_muted(&self) -> Color { NORD.text_muted }
    fn error(&self) -> Color { NORD.error }
    fn status_ended(&self) -> Color { NORD.status_ended }
    fn border_secondary(&self) -> Color { NORD.border_secondary }
    fn diff_add_bg(&self) -> Color { NORD.diff_add_bg }
    fn diff_del_bg(&self) -> Color { NORD.diff_del_bg }
    fn border_primary(&self) -> Color { NORD.border_primary() }
    fn tab_active_bg(&self) -> Color { NORD.tab_active_bg() }
}

impl ColorTheme for ArcticAuroraTheme {
    fn neon_pink(&self) -> Color { ARCTIC_AURORA.neon_pink }
    fn neon_cyan(&self) -> Color { ARCTIC_AURORA.neon_cyan }
    fn neon_purple(&self) -> Color { ARCTIC_AURORA.neon_purple }
    fn neon_green(&self) -> Color { ARCTIC_AURORA.neon_green }
    fn neon_yellow(&self) -> Color { ARCTIC_AURORA.neon_yellow }
    fn neon_orange(&self) -> Color { ARCTIC_AURORA.neon_orange }
    fn neon_blue(&self) -> Color { ARCTIC_AURORA.neon_blue }
    fn bg_dark(&self) -> Color { ARCTIC_AURORA.bg_dark }
    fn bg_panel(&self) -> Color { ARCTIC_AURORA.bg_panel }
    fn bg_surface(&self) -> Color { ARCTIC_AURORA.bg_surface }
    fn bg_highlight(&self) -> Color { ARCTIC_AURORA.bg_highlight }
    fn text_primary(&self) -> Color { ARCTIC_AURORA.text_primary }
    fn text_secondary(&self) -> Color { ARCTIC_AURORA.text_secondary }
    fn text_muted(&self) -> Color { ARCTIC_AURORA.text_muted }
    fn error(&self) -> Color { ARCTIC_AURORA.error }
    fn status_ended(&self) -> Color { ARCTIC_AURORA.status_ended }
    fn border_secondary(&self) -> Color { ARCTIC_AURORA.border_secondary }
    fn diff_add_bg(&self) -> Color { ARCTIC_AURORA.diff_add_bg }
    fn diff_del_bg(&self) -> Color { ARCTIC_AURORA.diff_del_bg }
    fn border_primary(&self) -> Color { ARCTIC_AURORA.border_primary() }
    fn tab_active_bg(&self) -> Color { ARCTIC_AURORA.tab_active_bg() }
}

impl ColorTheme for MinimalTheme {
    fn neon_pink(&self) -> Color { MINIMAL.neon_pink }
    fn neon_cyan(&self) -> Color { MINIMAL.neon_cyan }
    fn neon_purple(&self) -> Color { MINIMAL.neon_purple }
    fn neon_green(&self) -> Color { MINIMAL.neon_green }
    fn neon_yellow(&self) -> Color { MINIMAL.neon_yellow }
    fn neon_orange(&self) -> Color { MINIMAL.neon_orange }
    fn neon_blue(&self) -> Color { MINIMAL.neon_blue }
    fn bg_dark(&self) -> Color { MINIMAL.bg_dark }
    fn bg_panel(&self) -> Color { MINIMAL.bg_panel }
    fn bg_surface(&self) -> Color { MINIMAL.bg_surface }
    fn bg_highlight(&self) -> Color { MINIMAL.bg_highlight }
    fn text_primary(&self) -> Color { MINIMAL.text_primary }
    fn text_secondary(&self) -> Color { MINIMAL.text_secondary }
    fn text_muted(&self) -> Color { MINIMAL.text_muted }
    fn error(&self) -> Color { MINIMAL.error }
    fn status_ended(&self) -> Color { MINIMAL.status_ended }
    fn border_secondary(&self) -> Color { MINIMAL.border_secondary }
    fn diff_add_bg(&self) -> Color { MINIMAL.diff_add_bg }
    fn diff_del_bg(&self) -> Color { MINIMAL.diff_del_bg }
    fn border_primary(&self) -> Color { MINIMAL.border_primary() }
    fn tab_active_bg(&self) -> Color { MINIMAL.tab_active_bg() }
    fn warning(&self) -> Color { MINIMAL.warning() }
}

/// Create a theme from its name (legacy compatibility)
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

/// Set the global theme (legacy compatibility)
pub fn set_theme(theme_impl: Box<dyn ColorTheme>) -> bool {
    // Identify the theme by its bg_dark color
    let bg = theme_impl.bg_dark();
    let id = match bg {
        Color::Rgb(13, 13, 26) => ThemeId::Cyberpunk,
        Color::Rgb(39, 40, 34) => ThemeId::Monokai,
        Color::Rgb(40, 42, 54) => ThemeId::Dracula,
        Color::Rgb(46, 52, 64) => ThemeId::Nord,
        Color::Rgb(28, 35, 49) => ThemeId::ArcticAurora,
        Color::Rgb(26, 26, 26) => ThemeId::Minimal,
        _ => ThemeId::Cyberpunk,
    };
    set_theme_by_id(id)
}

/// Helper type alias for backward compatibility
#[deprecated(since = "0.2.0", note = "Use theme() function for dynamic theme access")]
pub type Theme = CyberpunkTheme;

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_id_parse() {
        assert_eq!("cyberpunk".parse::<ThemeId>().unwrap(), ThemeId::Cyberpunk);
        assert_eq!("monokai".parse::<ThemeId>().unwrap(), ThemeId::Monokai);
        assert_eq!("dracula".parse::<ThemeId>().unwrap(), ThemeId::Dracula);
        assert_eq!("nord".parse::<ThemeId>().unwrap(), ThemeId::Nord);
        assert_eq!("arctic".parse::<ThemeId>().unwrap(), ThemeId::ArcticAurora);
        assert_eq!("aurora".parse::<ThemeId>().unwrap(), ThemeId::ArcticAurora);
        assert_eq!("arctic-aurora".parse::<ThemeId>().unwrap(), ThemeId::ArcticAurora);
        assert_eq!("minimal".parse::<ThemeId>().unwrap(), ThemeId::Minimal);
        assert!("unknown".parse::<ThemeId>().is_err());
    }

    #[test]
    fn test_theme_id_display() {
        assert_eq!(ThemeId::Cyberpunk.to_string(), "cyberpunk");
        assert_eq!(ThemeId::Monokai.to_string(), "monokai");
        assert_eq!(ThemeId::Dracula.to_string(), "dracula");
        assert_eq!(ThemeId::Nord.to_string(), "nord");
        assert_eq!(ThemeId::ArcticAurora.to_string(), "arctic");
        assert_eq!(ThemeId::Minimal.to_string(), "minimal");
    }

    #[test]
    fn test_theme_id_all() {
        let all = ThemeId::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&ThemeId::Cyberpunk));
        assert!(all.contains(&ThemeId::Monokai));
        assert!(all.contains(&ThemeId::Dracula));
        assert!(all.contains(&ThemeId::Nord));
        assert!(all.contains(&ThemeId::ArcticAurora));
        assert!(all.contains(&ThemeId::Minimal));
    }

    #[test]
    fn test_cyberpunk_theme_colors() {
        let t = &CYBERPUNK;
        assert!(matches!(t.neon_pink, Color::Rgb(255, 0, 128)));
        assert!(matches!(t.neon_cyan, Color::Rgb(0, 255, 255)));
        assert!(matches!(t.neon_green, Color::Rgb(57, 255, 20)));
    }

    #[test]
    fn test_monokai_theme_colors() {
        let t = &MONOKAI;
        assert!(matches!(t.neon_pink, Color::Rgb(249, 38, 114)));
        assert!(matches!(t.neon_green, Color::Rgb(166, 226, 46)));
        assert!(matches!(t.bg_dark, Color::Rgb(39, 40, 34)));
    }

    #[test]
    fn test_dracula_theme_colors() {
        let t = &DRACULA;
        assert!(matches!(t.neon_pink, Color::Rgb(255, 121, 198)));
        assert!(matches!(t.neon_green, Color::Rgb(80, 250, 123)));
        assert!(matches!(t.bg_dark, Color::Rgb(40, 42, 54)));
    }

    #[test]
    fn test_nord_theme_colors() {
        let t = &NORD;
        assert!(matches!(t.neon_cyan, Color::Rgb(136, 192, 208)));
        assert!(matches!(t.neon_green, Color::Rgb(163, 190, 140)));
        assert!(matches!(t.bg_dark, Color::Rgb(46, 52, 64)));
    }

    #[test]
    fn test_minimal_theme_colors() {
        let t = &MINIMAL;
        assert!(matches!(t.neon_cyan, Color::Rgb(74, 158, 255)));
        assert!(matches!(t.neon_green, Color::Rgb(76, 175, 80)));
        assert!(matches!(t.bg_dark, Color::Rgb(26, 26, 26)));
    }

    #[test]
    fn test_arctic_aurora_theme_colors() {
        let t = &ARCTIC_AURORA;
        assert!(matches!(t.neon_pink, Color::Rgb(201, 160, 220)));
        assert!(matches!(t.neon_cyan, Color::Rgb(127, 219, 202)));
        assert!(matches!(t.neon_purple, Color::Rgb(157, 142, 201)));
        assert!(matches!(t.neon_green, Color::Rgb(163, 217, 165)));
        assert!(matches!(t.neon_yellow, Color::Rgb(240, 230, 140)));
        assert!(matches!(t.neon_orange, Color::Rgb(232, 184, 157)));
        assert!(matches!(t.neon_blue, Color::Rgb(135, 206, 235)));
        assert!(matches!(t.bg_dark, Color::Rgb(28, 35, 49)));
        assert!(matches!(t.bg_panel, Color::Rgb(37, 45, 59)));
        assert!(matches!(t.bg_surface, Color::Rgb(46, 56, 71)));
        assert!(matches!(t.bg_highlight, Color::Rgb(58, 69, 86)));
        assert!(matches!(t.text_primary, Color::Rgb(232, 238, 245)));
        assert!(matches!(t.text_secondary, Color::Rgb(184, 197, 214)));
        assert!(matches!(t.text_muted, Color::Rgb(107, 122, 143)));
        assert!(matches!(t.error, Color::Rgb(229, 115, 115)));
        assert!(matches!(t.status_ended, Color::Rgb(93, 107, 126)));
    }

    #[test]
    fn test_style_constructors() {
        let t = theme();
        let _ = t.style_tab_active();
        let _ = t.style_tab_inactive();
        let _ = t.style_border();
        let _ = t.style_success();
        let _ = t.style_error();
        let _ = t.style_warning();
        let _ = t.style_info();
        let _ = t.style_text();
        let _ = t.style_key();
        let _ = t.style_selected();
    }

    #[test]
    fn test_semantic_color_defaults() {
        let t = theme();
        assert_eq!(t.success(), t.neon_green);
        assert_eq!(t.info(), t.neon_cyan);
        assert_eq!(t.status_running(), t.neon_green);
    }

    #[test]
    fn test_create_theme() {
        let cyberpunk = create_theme("cyberpunk");
        assert!(matches!(cyberpunk.neon_pink(), Color::Rgb(255, 0, 128)));

        let monokai = create_theme("monokai");
        assert!(matches!(monokai.neon_pink(), Color::Rgb(249, 38, 114)));

        let dracula = create_theme("Dracula");
        assert!(matches!(dracula.neon_pink(), Color::Rgb(255, 121, 198)));

        let nord = create_theme("NORD");
        assert!(matches!(nord.bg_dark(), Color::Rgb(46, 52, 64)));

        let minimal = create_theme("minimal");
        assert!(matches!(minimal.bg_dark(), Color::Rgb(26, 26, 26)));

        let arctic = create_theme("arctic");
        assert!(matches!(arctic.bg_dark(), Color::Rgb(28, 35, 49)));

        let aurora = create_theme("aurora");
        assert!(matches!(aurora.bg_dark(), Color::Rgb(28, 35, 49)));

        let arctic_aurora = create_theme("arctic-aurora");
        assert!(matches!(arctic_aurora.bg_dark(), Color::Rgb(28, 35, 49)));

        let unknown = create_theme("unknown");
        assert!(matches!(unknown.neon_pink(), Color::Rgb(255, 0, 128)));
    }

    #[test]
    fn test_diff_colors() {
        let t = theme();
        assert_eq!(t.diff_addition(), t.neon_green);
        assert_eq!(t.diff_deletion(), t.error);
        assert_eq!(t.diff_hunk_header(), t.neon_cyan);
        assert_eq!(t.diff_file_header(), t.neon_yellow);
    }

    #[test]
    fn test_status_colors() {
        let t = theme();
        assert_eq!(t.status_running(), t.neon_green);
        assert_eq!(t.status_error(), t.error);
    }

    #[test]
    fn test_get_theme_colors() {
        assert_eq!(get_theme_colors(ThemeId::Cyberpunk).id, ThemeId::Cyberpunk);
        assert_eq!(get_theme_colors(ThemeId::Monokai).id, ThemeId::Monokai);
        assert_eq!(get_theme_colors(ThemeId::Dracula).id, ThemeId::Dracula);
        assert_eq!(get_theme_colors(ThemeId::Nord).id, ThemeId::Nord);
        assert_eq!(get_theme_colors(ThemeId::ArcticAurora).id, ThemeId::ArcticAurora);
        assert_eq!(get_theme_colors(ThemeId::Minimal).id, ThemeId::Minimal);
    }

    #[test]
    fn test_theme_colors_border_override() {
        // Nord has border_primary_override
        assert!(matches!(NORD.border_primary(), Color::Rgb(129, 161, 193)));
        // Cyberpunk uses default (neon_cyan)
        assert!(matches!(CYBERPUNK.border_primary(), Color::Rgb(0, 255, 255)));
    }

    #[test]
    fn test_available_themes() {
        let themes = available_themes();
        assert_eq!(themes.len(), 6);
        assert_eq!(themes[0].0, "cyberpunk");
        assert_eq!(themes[1].0, "monokai");
        assert_eq!(themes[2].0, "dracula");
        assert_eq!(themes[3].0, "nord");
        assert_eq!(themes[4].0, "arctic");
        assert_eq!(themes[5].0, "minimal");
    }
}
