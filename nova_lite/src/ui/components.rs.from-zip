use egui::{Color32, RichText, Ui};

#[derive(Debug, Clone)]
pub struct StatusBadge {
    pub label: String,
    pub color: Color32,
}

impl StatusBadge {
    pub fn show(&self, ui: &mut Ui) {
        let text = RichText::new(&self.label).color(Color32::WHITE);
        ui.colored_label(self.color, text);
    }
}

#[derive(Debug, Clone)]
pub struct ToolbarAction {
    pub label: String,
    pub tooltip: String,
}

impl ToolbarAction {
    pub fn show(&self, ui: &mut Ui) -> bool {
        let response = ui.button(&self.label);
        let clicked = response.clicked();
        if !self.tooltip.is_empty() {
            response.on_hover_text(&self.tooltip);
        }
        clicked
    }
}
