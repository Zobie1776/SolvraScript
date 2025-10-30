use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub installed: bool,
    pub rating: f32,
}

impl AppMetadata {
    pub fn new(id: &str, name: &str, summary: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            summary: summary.to_string(),
            installed: false,
            rating: 0.0,
        }
    }
}

#[derive(Debug, Default)]
pub struct AppCatalog {
    pub apps: Vec<AppMetadata>,
}

impl AppCatalog {
    pub fn bootstrap() -> Self {
        Self {
            apps: vec![
                AppMetadata::new(
                    "solvra.ide.mobile",
                    "SolvraIDE Mobile",
                    "A streamlined IDE tuned for touchscreen workflows.",
                ),
                AppMetadata::new(
                    "solvra.term",
                    "SolvraTerm",
                    "Terminal emulator with SolvraScript integration.",
                ),
                AppMetadata::new(
                    "solvra.play",
                    "SolvraPlay",
                    "Media and gaming launcher optimized for SolvraLite.",
                ),
            ],
        }
    }

    pub fn toggle_install(&mut self, id: &str) {
        if let Some(app) = self.apps.iter_mut().find(|app| app.id == id) {
            app.installed = !app.installed;
        }
    }
}
