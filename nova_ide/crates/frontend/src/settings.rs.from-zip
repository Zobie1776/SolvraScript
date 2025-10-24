use std::{fs, path::PathBuf};

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeSettings {
    pub theme: String,
    pub enable_nova_ai: bool,
    pub auto_save: bool,
    pub font_size: f32,
    pub lsp_overrides: Vec<LspOverride>,
}

impl Default for IdeSettings {
    fn default() -> Self {
        Self {
            theme: "nova-dark".to_string(),
            enable_nova_ai: true,
            auto_save: true,
            font_size: 15.0,
            lsp_overrides: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspOverride {
    pub language_id: String,
    pub server_path: PathBuf,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    pub config_path: PathBuf,
    pub data: IdeSettings,
}

impl SettingsStore {
    pub fn load_or_default() -> Self {
        let dirs = ProjectDirs::from("com", "Nova", "NovaIDE")
            .expect("project directories must be resolvable");
        let config_dir = dirs.config_dir();
        let config_path = config_dir.join("settings.json");

        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let data = fs::read_to_string(&config_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

        Self { config_path, data }
    }

    pub fn save(&self) -> Result<()> {
        let serialized = serde_json::to_string_pretty(&self.data)?;
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.config_path, serialized)?;
        Ok(())
    }

    pub fn toggle_auto_save(&mut self) {
        self.data.auto_save = !self.data.auto_save;
    }

    pub fn set_theme(&mut self, name: impl Into<String>) {
        self.data.theme = name.into();
    }

    pub fn update_font_size(&mut self, font_size: f32) {
        self.data.font_size = font_size;
    }

    pub fn nova_ai_enabled(&self) -> bool {
        self.data.enable_nova_ai
    }
}
