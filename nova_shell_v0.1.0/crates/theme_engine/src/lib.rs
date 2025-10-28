//=============================================
// theme_engine/src/lib.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Theme parsing and token distribution
// Objective: Provide shared data structures for compositor and UI crates
//=============================================

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

//=============================================
// SECTION: Theme Tokens
//=============================================

/// Shared theme information consumed by multiple binaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDocument {
    /// Display label for UI selection lists.
    pub name: ThemeName,
    /// Color palette tokens.
    pub colors: ThemeColors,
    /// Typography scale values.
    pub typography: Typography,
    /// Visual effect configuration.
    pub effects: ThemeEffects,
}

impl ThemeDocument {
    /// Load the document from the provided path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        let doc = toml::from_str::<Self>(&data)?;
        Ok(doc)
    }
}

/// Metadata for the theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeName {
    /// Machine readable identifier.
    pub label: String,
    /// Human-readable description.
    pub description: String,
}

/// Describes theme colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Base surface color.
    pub surface: String,
    /// Overlay color.
    pub overlay: String,
    /// Primary accent color.
    pub primary: String,
    /// Accent highlight.
    pub accent: String,
    /// Danger tone.
    pub danger: String,
    /// Warning tone.
    pub warn: String,
    /// Success tone.
    pub success: String,
}

/// Typographic measurements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    /// Base font size in dp.
    pub base_size: u16,
    /// Scaling factor between steps.
    pub scale: f32,
}

/// Visual effects metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEffects {
    /// Blur radius for drop shadows.
    pub shadow_blur: u16,
    /// Default corner radius.
    pub corner_radius: u16,
}

//=============================================
// SECTION: Runtime Tokens
//=============================================

/// Flattened runtime tokens ready for consumption.
#[derive(Debug, Clone)]
pub struct ThemeTokens {
    /// Primary colors used by compositor + iced apps.
    pub palette: ThemeColors,
    /// Typography metrics.
    pub typography: Typography,
    /// Glow/shadow configuration.
    pub effects: ThemeEffects,
}

impl From<ThemeDocument> for ThemeTokens {
    fn from(doc: ThemeDocument) -> Self {
        Self {
            palette: doc.colors,
            typography: doc.typography,
            effects: doc.effects,
        }
    }
}
