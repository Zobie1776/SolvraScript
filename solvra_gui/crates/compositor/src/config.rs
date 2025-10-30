//=============================================
// solvra_compositor/src/config.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Represent compositor configuration values
// Objective: Load TOML data and expose strongly typed accessors
//=============================================

use crate::profile::Profile;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

//=============================================
// SECTION: Data Model
//=============================================

/// Root configuration document for the compositor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorConfig {
    /// Theme identifier to load.
    pub theme: String,
    /// Selected profile (full/lite/tablet).
    pub profile: String,
    /// IPC socket path.
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
    /// Power configuration block.
    #[serde(default)]
    pub power: PowerConfig,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            theme: "Minimal".into(),
            profile: "lite".into(),
            socket_path: default_socket_path(),
            power: PowerConfig::default(),
        }
    }
}

impl CompositorConfig {
    /// Resolve the profile enumeration.
    pub fn profile(&self) -> Profile {
        Profile::from_str(&self.profile)
    }
}

/// Power management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    /// Idle timeout in seconds.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: default_idle_timeout(),
        }
    }
}

//=============================================
// SECTION: IO Helpers
//=============================================

/// Load configuration from a TOML file.
pub fn load_from_file(path: impl AsRef<Path>) -> Result<CompositorConfig> {
    let data = fs::read_to_string(path)?;
    let config = toml::from_str::<CompositorConfig>(&data)?;
    Ok(config)
}

//=============================================
// SECTION: Defaults
//=============================================

fn default_socket_path() -> String {
    "/run/user/1000/solvra-gui.sock".into()
}

fn default_idle_timeout() -> u64 {
    300
}
