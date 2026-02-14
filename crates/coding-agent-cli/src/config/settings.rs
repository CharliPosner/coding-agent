//! Configuration settings for the coding-agent CLI

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Permission settings
    pub permissions: PermissionsConfig,
    /// Model settings
    pub model: ModelConfig,
    /// Theme settings
    pub theme: ThemeConfig,
    /// Persistence settings
    pub persistence: PersistenceConfig,
    /// Behavior settings
    pub behavior: BehaviorConfig,
    /// Error recovery settings
    pub error_recovery: ErrorRecoveryConfig,
    /// Integration settings
    pub integrations: IntegrationsConfig,
}

/// Permission settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PermissionsConfig {
    /// Paths that are trusted (no confirmation needed)
    pub trusted_paths: Vec<String>,
    /// Whether to auto-allow read operations
    pub auto_read: bool,
}

/// Model settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModelConfig {
    /// Default model to use
    pub default: String,
    /// Available models
    pub available: Vec<String>,
    /// Context window size in tokens
    pub context_window: u32,
}

/// Theme settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ThemeConfig {
    /// Theme style: minimal, colorful, or monochrome
    pub style: String,
}

/// Persistence settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PersistenceConfig {
    /// Whether persistence is enabled
    pub enabled: bool,
    /// Format: specstory, markdown, or json
    pub format: String,
    /// Path for history files
    pub path: String,
}

/// Behavior settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BehaviorConfig {
    /// Whether to stream responses
    pub streaming: bool,
    /// Tool verbosity level: minimal, standard, or verbose
    pub tool_verbosity: String,
    /// Whether to show the context bar
    pub show_context_bar: bool,
    /// Whether to show fun facts during long waits
    pub fun_facts: bool,
    /// Delay in seconds before showing fun facts
    pub fun_fact_delay: u32,
    /// Maximum number of tool iterations before stopping
    pub max_tool_iterations: usize,
}

/// Error recovery settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ErrorRecoveryConfig {
    /// Whether to auto-fix errors
    pub auto_fix: bool,
    /// Whether to generate tests for fixes
    pub generate_tests: bool,
    /// Maximum number of retry attempts
    pub max_retry_attempts: u32,
}

/// Integration settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct IntegrationsConfig {
    /// Obsidian integration settings
    pub obsidian: ObsidianConfig,
    /// Git integration settings
    pub git: GitConfig,
}

/// Obsidian vault settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ObsidianConfig {
    /// Path to Obsidian vault
    pub vault_path: String,
}

/// Git integration settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GitConfig {
    /// Whether to auto-stage files during commit
    pub auto_stage: bool,
    /// Commit message style: purpose, conventional, or simple
    pub commit_style: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            permissions: PermissionsConfig::default(),
            model: ModelConfig::default(),
            theme: ThemeConfig::default(),
            persistence: PersistenceConfig::default(),
            behavior: BehaviorConfig::default(),
            error_recovery: ErrorRecoveryConfig::default(),
            integrations: IntegrationsConfig::default(),
        }
    }
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            trusted_paths: vec![],
            auto_read: true,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: "claude-3-opus".to_string(),
            available: vec!["claude-3-opus".to_string(), "claude-3-sonnet".to_string()],
            context_window: 200000,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            style: "minimal".to_string(),
        }
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "specstory".to_string(),
            path: ".specstory/history/".to_string(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            streaming: true,
            tool_verbosity: "standard".to_string(),
            show_context_bar: true,
            fun_facts: true,
            fun_fact_delay: 10,
            max_tool_iterations: 50,
        }
    }
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            auto_fix: true,
            generate_tests: true,
            max_retry_attempts: 3,
        }
    }
}

impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            obsidian: ObsidianConfig::default(),
            git: GitConfig::default(),
        }
    }
}

impl Default for ObsidianConfig {
    fn default() -> Self {
        Self {
            vault_path: "~/Documents/Personal/".to_string(),
        }
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            auto_stage: false,
            commit_style: "purpose".to_string(),
        }
    }
}

/// Errors that can occur during configuration operations
#[derive(Debug)]
pub enum ConfigError {
    /// Failed to find config directory
    NoConfigDir,
    /// Failed to read config file
    ReadError(std::io::Error),
    /// Failed to write config file
    WriteError(std::io::Error),
    /// Failed to parse TOML
    ParseError(toml::de::Error),
    /// Failed to serialize TOML
    SerializeError(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoConfigDir => write!(f, "Could not determine config directory"),
            ConfigError::ReadError(e) => write!(f, "Failed to read config file: {}", e),
            ConfigError::WriteError(e) => write!(f, "Failed to write config file: {}", e),
            ConfigError::ParseError(e) => write!(f, "Failed to parse config file: {}", e),
            ConfigError::SerializeError(e) => write!(f, "Failed to serialize config: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::ReadError(e) => Some(e),
            ConfigError::WriteError(e) => Some(e),
            ConfigError::ParseError(e) => Some(e),
            ConfigError::SerializeError(e) => Some(e),
            _ => None,
        }
    }
}

impl Config {
    /// Get the default config file path
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(config_dir.join("coding-agent").join("config.toml"))
    }

    /// Load config from the default path, creating default if it doesn't exist
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Load config from a specific path, creating default if it doesn't exist
    pub fn load_from(path: &PathBuf) -> Result<Self, ConfigError> {
        if !path.exists() {
            // Create default config
            let config = Config::default();
            config.save_to(path)?;
            return Ok(config);
        }

        let contents = fs::read_to_string(path).map_err(ConfigError::ReadError)?;
        Self::parse(&contents)
    }

    /// Parse config from a TOML string
    pub fn parse(contents: &str) -> Result<Self, ConfigError> {
        // Parse TOML and merge with defaults
        // This allows partial configs to work correctly
        let partial: toml::Value = toml::from_str(contents).map_err(ConfigError::ParseError)?;

        // Serialize defaults to Value, merge, then deserialize
        let default = Config::default();
        let default_value = toml::Value::try_from(&default).map_err(ConfigError::SerializeError)?;

        let merged = merge_toml_values(default_value, partial);

        merged.try_into().map_err(ConfigError::ParseError)
    }

    /// Save config to the default path
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::default_path()?;
        self.save_to(&path)
    }

    /// Save config to a specific path
    pub fn save_to(&self, path: &PathBuf) -> Result<(), ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ConfigError::WriteError)?;
        }

        let contents = toml::to_string_pretty(self).map_err(ConfigError::SerializeError)?;
        fs::write(path, contents).map_err(ConfigError::WriteError)
    }

    /// Convert config to TOML string
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::SerializeError)
    }

    /// Add a trusted path to the configuration and save to disk
    ///
    /// This is called when a user responds with "always" to a permission prompt.
    /// The path is added to the trusted_paths list and the config is saved.
    pub fn add_trusted_path(&mut self, path: &str) -> Result<(), ConfigError> {
        // Only add if not already present
        if !self.permissions.trusted_paths.contains(&path.to_string()) {
            self.permissions.trusted_paths.push(path.to_string());
            self.save()?;
        }
        Ok(())
    }

    /// Add a trusted path to the configuration and save to a specific path
    ///
    /// This variant allows specifying a custom config file path (useful for testing).
    pub fn add_trusted_path_to(
        &mut self,
        path: &str,
        config_path: &PathBuf,
    ) -> Result<(), ConfigError> {
        // Only add if not already present
        if !self.permissions.trusted_paths.contains(&path.to_string()) {
            self.permissions.trusted_paths.push(path.to_string());
            self.save_to(config_path)?;
        }
        Ok(())
    }
}

/// Merge two TOML values, with the second taking precedence
fn merge_toml_values(base: toml::Value, overlay: toml::Value) -> toml::Value {
    match (base, overlay) {
        (toml::Value::Table(mut base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                let merged_value = if let Some(base_value) = base_table.remove(&key) {
                    merge_toml_values(base_value, overlay_value)
                } else {
                    overlay_value
                };
                base_table.insert(key, merged_value);
            }
            toml::Value::Table(base_table)
        }
        // For non-table values, overlay takes precedence
        (_, overlay) => overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default_generation() {
        let config = Config::default();

        // Verify defaults are set
        assert_eq!(config.model.default, "claude-3-opus");
        assert!(config.permissions.auto_read);
        assert_eq!(config.behavior.tool_verbosity, "standard");
        assert!(config.persistence.enabled);
    }

    #[test]
    fn test_config_load_valid() {
        let toml = r#"
            [model]
            default = "claude-3-sonnet"
            context_window = 100000

            [behavior]
            streaming = false
        "#;

        let config = Config::parse(toml).expect("Should parse valid TOML");

        assert_eq!(config.model.default, "claude-3-sonnet");
        assert_eq!(config.model.context_window, 100000);
        assert!(!config.behavior.streaming);
        // Defaults should still be applied for missing fields
        assert!(config.permissions.auto_read);
    }

    #[test]
    fn test_config_load_invalid() {
        let toml = r#"
            [model
            default = "broken
        "#;

        let result = Config::parse(toml);
        assert!(result.is_err());

        if let Err(ConfigError::ParseError(_)) = result {
            // Expected
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_config_merge_partial() {
        // Only specify a few fields
        let toml = r#"
            [model]
            default = "gpt-4"
        "#;

        let config = Config::parse(toml).expect("Should parse partial config");

        // Specified field should be overridden
        assert_eq!(config.model.default, "gpt-4");

        // Unspecified fields should use defaults
        assert_eq!(config.model.context_window, 200000);
        assert!(config.behavior.streaming);
        assert_eq!(config.theme.style, "minimal");
    }

    #[test]
    fn test_config_unknown_keys_ignored() {
        let toml = r#"
            [model]
            default = "claude-3-opus"

            [unknown_section]
            unknown_key = "unknown_value"
        "#;

        // Should parse without error, ignoring unknown keys
        let result = Config::parse(toml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.model.default = "test-model".to_string();
        config.behavior.streaming = false;

        // Save config
        config.save_to(&config_path).expect("Should save config");

        // Verify file exists
        assert!(config_path.exists());

        // Load config
        let loaded = Config::load_from(&config_path).expect("Should load config");

        assert_eq!(loaded.model.default, "test-model");
        assert!(!loaded.behavior.streaming);
    }

    #[test]
    fn test_config_creates_parent_dir() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("config.toml");

        let config = Config::default();
        config.save_to(&config_path).expect("Should save config");

        assert!(config_path.exists());
    }

    #[test]
    fn test_config_load_creates_default() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        // File doesn't exist yet
        assert!(!config_path.exists());

        // Load should create default
        let config = Config::load_from(&config_path).expect("Should create default config");

        // File should now exist
        assert!(config_path.exists());

        // Config should be default
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_config_to_toml() {
        let config = Config::default();
        let toml = config.to_toml().expect("Should serialize to TOML");

        // Should contain expected sections
        assert!(toml.contains("[model]"));
        assert!(toml.contains("[behavior]"));
        assert!(toml.contains("[permissions]"));
    }

    #[test]
    fn test_always_adds_to_config() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        // Create a config with no trusted paths
        let mut config = Config::default();
        assert!(config.permissions.trusted_paths.is_empty());

        // Add a trusted path (simulating "always" response)
        config
            .add_trusted_path_to("/Users/test/projects", &config_path)
            .expect("Should add trusted path");

        // Verify in-memory config updated
        assert!(config
            .permissions
            .trusted_paths
            .contains(&"/Users/test/projects".to_string()));

        // Verify persisted to disk
        let loaded = Config::load_from(&config_path).expect("Should load config");
        assert!(loaded
            .permissions
            .trusted_paths
            .contains(&"/Users/test/projects".to_string()));
    }

    #[test]
    fn test_always_adds_to_config_no_duplicates() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();

        // Add the same path twice
        config
            .add_trusted_path_to("/Users/test/projects", &config_path)
            .expect("Should add trusted path");
        config
            .add_trusted_path_to("/Users/test/projects", &config_path)
            .expect("Should handle duplicate");

        // Should only have one entry
        let count = config
            .permissions
            .trusted_paths
            .iter()
            .filter(|p| *p == "/Users/test/projects")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_always_adds_multiple_paths() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();

        // Add multiple paths
        config
            .add_trusted_path_to("/Users/test/projects", &config_path)
            .expect("Should add first path");
        config
            .add_trusted_path_to("/Users/test/documents", &config_path)
            .expect("Should add second path");

        // Both paths should be present
        assert_eq!(config.permissions.trusted_paths.len(), 2);
        assert!(config
            .permissions
            .trusted_paths
            .contains(&"/Users/test/projects".to_string()));
        assert!(config
            .permissions
            .trusted_paths
            .contains(&"/Users/test/documents".to_string()));

        // Verify persisted
        let loaded = Config::load_from(&config_path).expect("Should load config");
        assert_eq!(loaded.permissions.trusted_paths.len(), 2);
    }

    #[test]
    fn test_tool_iterations_default() {
        let config = Config::default();
        assert_eq!(config.behavior.max_tool_iterations, 50);
    }

    #[test]
    fn test_tool_iterations_configurable() {
        let toml = r#"
            [behavior]
            max_tool_iterations = 100
        "#;

        let config = Config::parse(toml).expect("Should parse config");
        assert_eq!(config.behavior.max_tool_iterations, 100);
    }
}
