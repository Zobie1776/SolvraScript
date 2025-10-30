use std::collections::HashMap;

use eframe::egui::{self, Color32, Context, Style, Visuals};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

static BUILTIN_THEMES: Lazy<HashMap<&'static str, ThemeDefinition>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "solvra-dark",
        ThemeDefinition {
            name: "Solvra Dark".into(),
            visuals: Visuals::dark(),
            accent: Color32::from_rgb(100, 170, 255),
        },
    );
    map.insert(
        "solvra-light",
        ThemeDefinition {
            name: "Solvra Light".into(),
            visuals: Visuals::light(),
            accent: Color32::from_rgb(60, 90, 200),
        },
    );
    map
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDefinition {
    pub name: String,
    #[serde(skip)]
    pub visuals: Visuals,
    #[serde(skip)]
    pub accent: Color32,
}

#[derive(Debug, Default, Clone)]
pub struct ThemeManager {
    pub active_theme: String,
    pub custom_themes: HashMap<String, ThemeDefinition>,
}

impl ThemeManager {
    pub fn new(active: impl Into<String>) -> Self {
        Self {
            active_theme: active.into(),
            custom_themes: HashMap::new(),
        }
    }

    pub fn apply(&self, ctx: &Context, font_size: f32) {
        let visuals = self
            .resolve_theme()
            .map(|definition| definition.visuals.clone())
            .unwrap_or_else(Visuals::dark);

        let mut style: Style = (*ctx.style()).clone();
        style.visuals = visuals;
        if let Some(text_style) = style.text_styles.get_mut(&egui::TextStyle::Body) {
            text_style.size = font_size;
        }
        ctx.set_style(style);
    }

    pub fn resolve_theme(&self) -> Option<ThemeDefinition> {
        if let Some(theme) = self.custom_themes.get(&self.active_theme) {
            Some(theme.clone())
        } else {
            BUILTIN_THEMES.get(self.active_theme.as_str()).cloned()
        }
    }

    pub fn register_custom_theme(&mut self, name: impl Into<String>, definition: ThemeDefinition) {
        self.custom_themes.insert(name.into(), definition);
    }

    pub fn available_themes(&self) -> Vec<String> {
        let mut all: Vec<String> = BUILTIN_THEMES.keys().map(|key| key.to_string()).collect();
        all.extend(self.custom_themes.keys().cloned());
        all.sort();
        all
    }
}
