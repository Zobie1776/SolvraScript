use egui::{TextEdit, Ui};

#[derive(Debug, Clone)]
pub struct CodeEditorPanel {
    pub file_name: String,
    pub buffer: String,
    pub language: String,
}

impl Default for CodeEditorPanel {
    fn default() -> Self {
        Self {
            file_name: "main.svs".into(),
            buffer: "// Start hacking with SolvraScript".into(),
            language: "solvrascript".into(),
        }
    }
}

impl CodeEditorPanel {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading(&self.file_name);
        ui.label(format!("Language: {}", self.language));
        ui.add_space(4.0);
        let editor = TextEdit::multiline(&mut self.buffer)
            .desired_width(f32::INFINITY)
            .code_editor();
        ui.add(editor);
    }

    pub fn handle_tap(&mut self, position: egui::Pos2) {
        tracing::trace!("code_editor_tap x={} y={}", position.x, position.y);
    }
}
