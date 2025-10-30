//=============================================
// theme_engine/src/lib.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Theme parsing and token distribution
// Objective: Provide shared data structures consumed by compositor and iced apps
//=============================================

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

//=============================================
// SECTION: Theme Document
//=============================================

/// Complete theme document loaded from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDocument {
    /// Theme metadata.
    pub name: ThemeName,
    /// Color tokens.
    pub colors: ThemeColors,
    /// Typography metrics.
    pub typography: Typography,
    /// Visual effects configuration.
    pub effects: ThemeEffects,
}

impl ThemeDocument {
    /// Load the document from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        Ok(toml::from_str::<Self>(&data)?)
    }
}

/// Theme metadata struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeName {
    /// Machine readable identifier.
    pub label: String,
    /// Human readable description.
    pub description: String,
}

/// Color palette tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub surface: String,
    pub overlay: String,
    pub primary: String,
    pub accent: String,
    pub danger: String,
    pub warn: String,
    pub success: String,
}

/// Typography tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    pub base_size: u16,
    pub scale: f32,
}

/// Visual effects tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEffects {
    pub shadow_blur: u16,
    pub corner_radius: u16,
}

//=============================================
// SECTION: Runtime Tokens
//=============================================

/// Flattened theme tokens consumed at runtime.
#[derive(Debug, Clone)]
pub struct ThemeTokens {
    pub colors: ThemeColors,
    pub typography: Typography,
    pub effects: ThemeEffects,
}

impl From<ThemeDocument> for ThemeTokens {
    fn from(doc: ThemeDocument) -> Self {
        Self {
            colors: doc.colors,
            typography: doc.typography,
            effects: doc.effects,
        }
    }
}
