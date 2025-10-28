//=============================================
// nova_compositor/src/profile.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Describe compositor runtime profiles
// Objective: Provide helper utilities for feature toggles
//=============================================

/// Supported compositor profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Full NovaOS experience.
    Full,
    /// Nova Lite preset.
    Lite,
    /// Tablet preset enabling gestures.
    Tablet,
}

impl Profile {
    /// Parse a profile string, falling back to lite.
    pub fn from_str(value: &str) -> Self {
        match value {
            "full" => Self::Full,
            "tablet" => Self::Tablet,
            _ => Self::Lite,
        }
    }
}
