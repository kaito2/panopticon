use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level configuration for panopticon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanopticonConfig {
    /// State directory path.
    #[serde(default = "default_state_dir")]
    pub state_dir: String,

    /// Default Claude model for plan/execute commands.
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Permission mode for Claude CLI.
    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,

    /// Allowed tools for Claude CLI (empty = all).
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Minimum reputation threshold for agent assignment.
    #[serde(default = "default_min_reputation")]
    pub min_reputation_threshold: f64,

    /// Default decomposition strategy: "sequential", "parallel", "hybrid".
    #[serde(default = "default_decomposition_strategy")]
    pub decomposition_strategy: String,

    /// Max turns for Claude agent execution.
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,

    /// Claude model used for the natural-language intent router.
    #[serde(default = "default_router_model")]
    pub router_model: String,

    /// Maximum number of conversation messages retained in the REPL session.
    #[serde(default = "default_max_context_messages")]
    pub max_context_messages: u32,
}

fn default_state_dir() -> String {
    "~/.panopticon".to_string()
}

fn default_model() -> String {
    "sonnet".to_string()
}

fn default_permission_mode() -> String {
    "bypassPermissions".to_string()
}

fn default_min_reputation() -> f64 {
    0.3
}

fn default_decomposition_strategy() -> String {
    "hybrid".to_string()
}

fn default_max_turns() -> u32 {
    10
}

fn default_router_model() -> String {
    "haiku".to_string()
}

fn default_max_context_messages() -> u32 {
    20
}

impl Default for PanopticonConfig {
    fn default() -> Self {
        Self {
            state_dir: default_state_dir(),
            default_model: default_model(),
            permission_mode: default_permission_mode(),
            allowed_tools: Vec::new(),
            min_reputation_threshold: default_min_reputation(),
            decomposition_strategy: default_decomposition_strategy(),
            max_turns: default_max_turns(),
            router_model: default_router_model(),
            max_context_messages: default_max_context_messages(),
        }
    }
}

impl PanopticonConfig {
    /// Config file path within the state directory.
    pub fn config_path(state_dir: &Path) -> PathBuf {
        state_dir.join("config.toml")
    }

    /// Load config from disk. Returns default if not found.
    pub fn load(state_dir: &Path) -> Result<Self> {
        let path = Self::config_path(state_dir);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path).context("Failed to read config file")?;
        let config: Self = toml::from_str(&content).context("Failed to parse config file")?;
        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self, state_dir: &Path) -> Result<()> {
        let path = Self::config_path(state_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content).context("Failed to write config file")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = PanopticonConfig::default();
        assert_eq!(config.default_model, "sonnet");
        assert_eq!(config.max_turns, 10);
    }

    #[test]
    fn test_config_roundtrip() {
        let dir = tempdir().unwrap();
        let config = PanopticonConfig::default();
        config.save(dir.path()).unwrap();
        let loaded = PanopticonConfig::load(dir.path()).unwrap();
        assert_eq!(loaded.default_model, config.default_model);
        assert_eq!(loaded.max_turns, config.max_turns);
    }

    #[test]
    fn test_config_toml_serialization() {
        let config = PanopticonConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("default_model"));
        assert!(toml_str.contains("sonnet"));
    }
}
