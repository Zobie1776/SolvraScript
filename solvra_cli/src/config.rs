//! Configuration handling for SolvraCLI including loading and defaults.

use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Default configuration path relative to the user's config directory.
const CONFIG_FILE: &str = "cli.toml";

/// Configuration model for the CLI loaded from TOML.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CliConfig {
    /// Custom prompt string (supports environment expansion).
    pub prompt: String,
    /// Alias definitions for commands.
    pub aliases: HashMap<String, String>,
    /// Environment variables exported on startup.
    pub env: HashMap<String, String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            prompt: "\u{001b}[1;36msolvra\u{001b}[0m> ".to_string(),
            aliases: HashMap::new(),
            env: HashMap::new(),
        }
    }
}

impl CliConfig {
    /// Load configuration from disk or fall back to defaults when absent.
    pub fn load() -> anyhow::Result<(Self, PathBuf)> {
        let dirs = ProjectDirs::from("dev", "Solvra", "solvra")
            .ok_or_else(|| anyhow::anyhow!("unable to determine configuration directory"))?;
        let config_dir = dirs.config_dir();
        fs::create_dir_all(config_dir).context("creating Solvra config directory")?;
        let path = config_dir.join(CONFIG_FILE);
        if !path.exists() {
            return Ok((Self::default(), path));
        }
        let data = fs::read_to_string(&path)
            .with_context(|| format!("reading configuration from {}", path.display()))?;
        let cfg: Self = toml::from_str(&data)
            .with_context(|| format!("parsing configuration {}", path.display()))?;
        Ok((cfg, path))
    }

    /// Persist the configuration back to disk.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let serialized = toml::to_string_pretty(self)?;
        fs::write(path, serialized)
            .with_context(|| format!("writing configuration to {}", path.display()))?;
        Ok(())
    }
}
