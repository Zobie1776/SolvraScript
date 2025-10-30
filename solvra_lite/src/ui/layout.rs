use egui::{vec2, Vec2};

/// Logical layout classes to adapt the UI for various screen sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    Compact,
    Medium,
    Expanded,
}

/// Handles responsive breakpoints and utility helpers for arranging panels.
#[derive(Debug, Clone)]
pub struct ResponsiveLayout {
    breakpoints: [f32; 2],
    pub minimum_panel_width: f32,
}

impl Default for ResponsiveLayout {
    fn default() -> Self {
        Self {
            breakpoints: [480.0, 900.0],
            minimum_panel_width: 320.0,
        }
    }
}

impl ResponsiveLayout {
    pub fn classify(&self, surface: Vec2) -> LayoutClass {
        if surface.x < self.breakpoints[0] {
            LayoutClass::Compact
        } else if surface.x < self.breakpoints[1] {
            LayoutClass::Medium
        } else {
            LayoutClass::Expanded
        }
    }

    pub fn ideal_panel_split(&self, surface: Vec2, class: LayoutClass) -> (f32, f32) {
        let total = surface.x.max(self.minimum_panel_width * 2.0);
        match class {
            LayoutClass::Compact => (total, 0.0),
            LayoutClass::Medium => (total * 0.55, total * 0.45),
            LayoutClass::Expanded => (total * 0.45, total * 0.55),
        }
    }

    pub fn minimum_surface(&self) -> Vec2 {
        vec2(self.minimum_panel_width * 2.0, 640.0)
    }
}
