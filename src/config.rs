//! Configuration file support for cctakt
//!
//! Handles `.cctakt.toml` configuration file loading and saving.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default configuration file name
const CONFIG_FILE_NAME: &str = ".cctakt.toml";

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base directory for creating worktrees
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: PathBuf,

    /// Default branch prefix
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,

    /// Color theme name: "cyberpunk", "monokai", "dracula", "nord", "minimal"
    #[serde(default = "default_theme")]
    pub theme: String,

    /// GitHub configuration
    #[serde(default)]
    pub github: GitHubConfig,

    /// Anthropic API configuration
    #[serde(default)]
    pub anthropic: AnthropicConfig,

    /// Keybinding configuration
    #[serde(default)]
    pub keybindings: KeyBindings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worktree_dir: default_worktree_dir(),
            branch_prefix: default_branch_prefix(),
            theme: default_theme(),
            github: GitHubConfig::default(),
            anthropic: AnthropicConfig::default(),
            keybindings: KeyBindings::default(),
        }
    }
}

fn default_theme() -> String {
    "cyberpunk".to_string()
}

/// GitHub-related configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    /// Whether to automatically fetch issues
    #[serde(default)]
    pub auto_fetch_issues: bool,

    /// Target repository in owner/repo format
    #[serde(default)]
    pub repository: Option<String>,

    /// Labels to filter issues (e.g., "cctakt", "good first issue")
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Anthropic API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// API key (can also be set via ANTHROPIC_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model to use (default: claude-sonnet-4-20250514)
    #[serde(default = "default_anthropic_model")]
    pub model: String,

    /// Max tokens for response (default: 1024)
    #[serde(default = "default_anthropic_max_tokens")]
    pub max_tokens: u32,

    /// Whether to auto-generate PR descriptions
    #[serde(default = "default_auto_generate_pr")]
    pub auto_generate_pr_description: bool,
}

fn default_anthropic_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_anthropic_max_tokens() -> u32 {
    1024
}

fn default_auto_generate_pr() -> bool {
    true
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_anthropic_model(),
            max_tokens: default_anthropic_max_tokens(),
            auto_generate_pr_description: default_auto_generate_pr(),
        }
    }
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Key to create new agent (default: "ctrl+t")
    #[serde(default = "default_new_agent")]
    pub new_agent: String,

    /// Key to close agent (default: "ctrl+w")
    #[serde(default = "default_close_agent")]
    pub close_agent: String,

    /// Key to switch to next tab (default: "tab")
    #[serde(default = "default_next_tab")]
    pub next_tab: String,

    /// Key to switch to previous tab (default: "shift+tab")
    #[serde(default = "default_prev_tab")]
    pub prev_tab: String,

    /// Key to quit application (default: "ctrl+q")
    #[serde(default = "default_quit")]
    pub quit: String,
}

// Default value functions
fn default_worktree_dir() -> PathBuf {
    PathBuf::from(".worktrees")
}

fn default_branch_prefix() -> String {
    "cctakt".to_string()
}

fn default_new_agent() -> String {
    "ctrl+t".to_string()
}

fn default_close_agent() -> String {
    "ctrl+w".to_string()
}

fn default_next_tab() -> String {
    "tab".to_string()
}

fn default_prev_tab() -> String {
    "shift+tab".to_string()
}

fn default_quit() -> String {
    "ctrl+q".to_string()
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            new_agent: default_new_agent(),
            close_agent: default_close_agent(),
            next_tab: default_next_tab(),
            prev_tab: default_prev_tab(),
            quit: default_quit(),
        }
    }
}

impl Config {
    /// Load configuration file (returns default if not found)
    ///
    /// Searches for `.cctakt.toml` in the current directory.
    pub fn load() -> Result<Self> {
        let config_path = PathBuf::from(CONFIG_FILE_NAME);

        if config_path.exists() {
            Self::load_from(&config_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from specified path
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse configuration file: {}", path.display()))?;

        Ok(config)
    }

    /// Save configuration to file
    ///
    /// Saves to `.cctakt.toml` in the current directory.
    pub fn save(&self) -> Result<()> {
        let config_path = PathBuf::from(CONFIG_FILE_NAME);
        self.save_to(&config_path)
    }

    /// Save configuration to specified path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write configuration file: {}", path.display()))?;

        Ok(())
    }

    /// Generate default configuration file
    pub fn generate_default(path: &Path) -> Result<()> {
        let config = Config::default();
        config.save_to(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.worktree_dir, PathBuf::from(".worktrees"));
        assert_eq!(config.branch_prefix, "cctakt");
        assert_eq!(config.theme, "cyberpunk");
        assert!(!config.github.auto_fetch_issues);
        assert!(config.github.repository.is_none());
        assert!(config.github.labels.is_empty());
        assert_eq!(config.keybindings.new_agent, "ctrl+t");
        assert_eq!(config.keybindings.quit, "ctrl+q");
        // Anthropic defaults
        assert!(config.anthropic.api_key.is_none());
        assert_eq!(config.anthropic.model, "claude-sonnet-4-20250514");
        assert_eq!(config.anthropic.max_tokens, 1024);
        assert!(config.anthropic.auto_generate_pr_description);
    }

    #[test]
    fn test_load_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
worktree_dir = ".custom-worktrees"
branch_prefix = "custom"

[github]
auto_fetch_issues = true
repository = "user/repo"
labels = ["bug", "enhancement"]

[keybindings]
new_agent = "ctrl+n"
quit = "ctrl+c"
"#
        )
        .unwrap();

        let config = Config::load_from(temp_file.path()).unwrap();

        assert_eq!(config.worktree_dir, PathBuf::from(".custom-worktrees"));
        assert_eq!(config.branch_prefix, "custom");
        assert!(config.github.auto_fetch_issues);
        assert_eq!(config.github.repository, Some("user/repo".to_string()));
        assert_eq!(config.github.labels, vec!["bug", "enhancement"]);
        assert_eq!(config.keybindings.new_agent, "ctrl+n");
        assert_eq!(config.keybindings.quit, "ctrl+c");
        // Default values should be used for unspecified keys
        assert_eq!(config.keybindings.close_agent, "ctrl+w");
    }

    #[test]
    fn test_save_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut config = Config::default();
        config.branch_prefix = "test-prefix".to_string();
        config.github.repository = Some("test/repo".to_string());

        config.save_to(path).unwrap();

        let loaded = Config::load_from(path).unwrap();
        assert_eq!(loaded.branch_prefix, "test-prefix");
        assert_eq!(loaded.github.repository, Some("test/repo".to_string()));
    }

    #[test]
    fn test_generate_default() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        Config::generate_default(path).unwrap();

        let loaded = Config::load_from(path).unwrap();
        assert_eq!(loaded.worktree_dir, PathBuf::from(".worktrees"));
        assert_eq!(loaded.branch_prefix, "cctakt");
    }

    #[test]
    fn test_partial_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Only specify some values, rest should use defaults
        writeln!(
            temp_file,
            r#"
branch_prefix = "partial"
"#
        )
        .unwrap();

        let config = Config::load_from(temp_file.path()).unwrap();

        assert_eq!(config.branch_prefix, "partial");
        // Default values
        assert_eq!(config.worktree_dir, PathBuf::from(".worktrees"));
        assert_eq!(config.keybindings.new_agent, "ctrl+t");
    }

    #[test]
    fn test_anthropic_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[anthropic]
api_key = "sk-ant-test-key"
model = "claude-3-opus"
max_tokens = 2048
auto_generate_pr_description = false
"#
        )
        .unwrap();

        let config = Config::load_from(temp_file.path()).unwrap();

        assert_eq!(config.anthropic.api_key, Some("sk-ant-test-key".to_string()));
        assert_eq!(config.anthropic.model, "claude-3-opus");
        assert_eq!(config.anthropic.max_tokens, 2048);
        assert!(!config.anthropic.auto_generate_pr_description);
    }

    #[test]
    fn test_anthropic_config_default() {
        let config = AnthropicConfig::default();

        assert!(config.api_key.is_none());
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_tokens, 1024);
        assert!(config.auto_generate_pr_description);
    }

    #[test]
    fn test_theme_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
theme = "dracula"
"#
        )
        .unwrap();

        let config = Config::load_from(temp_file.path()).unwrap();
        assert_eq!(config.theme, "dracula");
    }

    #[test]
    fn test_theme_config_default() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
branch_prefix = "test"
"#
        )
        .unwrap();

        let config = Config::load_from(temp_file.path()).unwrap();
        assert_eq!(config.theme, "cyberpunk");
    }
}
