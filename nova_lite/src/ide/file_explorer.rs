use egui::{CollapsingHeader, Label, RichText, Sense, Ui};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Default)]
pub struct FileExplorerPanel {
    pub entries: Vec<FileEntry>,
    pub selected: Option<String>,
}

impl FileExplorerPanel {
    pub fn with_entries(entries: Vec<FileEntry>) -> Self {
        Self {
            entries,
            selected: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("Files")
            .default_open(true)
            .show(ui, |ui| {
                for entry in &self.entries {
                    let mut text = RichText::new(&entry.path);
                    if let Some(selected) = &self.selected {
                        if selected == &entry.path {
                            text = text.color(egui::Color32::from_rgb(120, 180, 250));
                        }
                    }

                    let response = ui.add(Label::new(text).sense(Sense::click()));
                    if response.clicked() {
                        self.selected = Some(entry.path.clone());
                    }
                }
            });
    }

    pub fn handle_tap(&mut self, position: egui::Pos2) {
        // For now we just log the event; a real implementation would map the position to entries.
        tracing::debug!("file_explorer_tap x={} y={}", position.x, position.y);
    }
}
