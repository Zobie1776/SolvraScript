//=============================================
// nova_compositor/src/profile.rs
//=============================================
// Author: Nova GUI Team
// License: MIT
// Goal: Describe compositor runtime profiles
// Objective: Provide helper utilities for feature toggles
//=============================================

/// Supported compositor profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Full NovaOS experience (effects enabled).
    Full,
    /// Nova Lite preset (compact metrics, reduced effects).
    Lite,
    /// Tablet preset (touch gestures, on-screen keyboard).
    Tablet,
}

impl Profile {
    /// Parse from string; defaults to lite.
    pub fn from_str(value: &str) -> Self {
        match value {
            "full" => Self::Full,
            "tablet" => Self::Tablet,
            _ => Self::Lite,
        }
    }
}
