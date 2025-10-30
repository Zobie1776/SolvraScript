//=============================================
// solvra_compositor/src/config.rs
//=============================================
// Author: Solvra Shell Team
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

/// Root configuration document for the compositor process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorConfig {
    /// Active theme identifier.
    pub theme: String,
    /// Active performance profile.
    pub profile: String,
    /// Path to the IPC socket.
    pub socket_path: String,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            theme: "Minimal".into(),
            profile: "lite".into(),
            socket_path: "/run/user/1000/solvra-shell.sock".into(),
        }
    }
}

impl CompositorConfig {
    /// Resolve the profile enum.
    pub fn profile(&self) -> Profile {
        Profile::from_str(&self.profile)
    }
}

//=============================================
// SECTION: IO Helpers
//=============================================

/// Load the configuration from a TOML file.
pub fn load_from_file(path: impl AsRef<Path>) -> Result<CompositorConfig> {
    let data = fs::read_to_string(path)?;
    let config = toml::from_str::<CompositorConfig>(&data)?;
    Ok(config)
}
