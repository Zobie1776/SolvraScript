use crate::app::{
    AppCapability, AppCategory, AppId, AppMetadata, AppPackage, UiComponent, UiComponentKind,
};
use crate::sandbox::{SandboxPermission, SandboxPolicy};
use semver::Version;

/// Return catalog metadata for SolvraSlides.
pub fn metadata() -> AppMetadata {
    let sandbox = SandboxPolicy::new()
        .allow_permission(SandboxPermission::FileRead)
        .allow_storage_root("~/Documents/SolvraSlides");

    let package = AppPackage::new(Version::new(1, 0, 0))
        .with_sandbox(sandbox)
        .with_capability(
            AppCapability::new(
                "solvraslides.presentation",
                "Composable presentation authoring with layout and media support",
            )
            .with_tag("presentation")
            .with_tag("slides"),
        )
        .with_ui_component(
            UiComponent::new(
                "solvraslides-editor",
                UiComponentKind::IdeView,
                "Slide designer with layout and animation timeline",
            )
            .with_entry_point("solvraslides::editor"),
        )
        .with_ui_component(UiComponent::new(
            "solvraslides-player",
            UiComponentKind::Immersive,
            "Fullscreen slide playback",
        ));

    AppMetadata::new(
        AppId::new("dev.solvra.slides").expect("valid id"),
        "SolvraSlides",
        "Design polished presentations with collaborative tooling",
        "SolvraSlides focuses on fast slide composition, templating, and live presentation tools including timers and speaker notes.",
        AppCategory::Productivity,
        "Solvra Labs",
        package,
    )
    .with_tag("presentations")
    .with_screenshot("screenshots/solvraslides.png")
}

/// Represents a slide deck comprised of ordered slides.
#[derive(Debug, Default, Clone)]
pub struct Presentation {
    slides: Vec<Slide>,
    theme: Theme,
}

impl Presentation {
    /// Create an empty presentation with the default theme.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the theme for the presentation.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Add a new slide to the deck and return a mutable reference to it.
    pub fn add_slide(&mut self, title: impl Into<String>) -> &mut Slide {
        self.slides.push(Slide::new(title));
        self.slides.last_mut().expect("slide inserted")
    }

    /// Iterate over slides in the order they will be presented.
    pub fn slides(&self) -> &[Slide] {
        &self.slides
    }

    /// Duplicate a slide at the specified index.
    pub fn duplicate_slide(&mut self, index: usize) -> Option<()> {
        let slide = self.slides.get(index)?.clone();
        self.slides.insert(index + 1, slide);
        Some(())
    }

    /// Apply a transition to the slide at the provided index.
    pub fn apply_transition(&mut self, index: usize, transition: Transition) -> Option<()> {
        let slide = self.slides.get_mut(index)?;
        slide.transition = Some(transition);
        Some(())
    }

    /// Render a simple textual preview of the presentation.
    pub fn preview_outline(&self) -> String {
        let mut outline = String::new();
        for (idx, slide) in self.slides.iter().enumerate() {
            outline.push_str(&format!("{}. {}\n", idx + 1, slide.title));
            for element in &slide.elements {
                match element {
                    SlideElement::Text(text) => {
                        outline.push_str(&format!("   • {}\n", text.content));
                    }
                    SlideElement::Image(image) => {
                        outline.push_str(&format!("   • [Image] {}\n", image.path));
                    }
                }
            }
        }
        outline
    }
}

/// Presentation theme describing fonts and palette.
#[derive(Debug, Clone)]
pub struct Theme {
    pub primary_font: String,
    pub accent_color: String,
    pub background_color: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary_font: "Solvra Sans".into(),
            accent_color: "#5B8DEF".into(),
            background_color: "#FFFFFF".into(),
        }
    }
}

/// A single slide containing positioned elements.
#[derive(Debug, Clone)]
pub struct Slide {
    pub title: String,
    pub elements: Vec<SlideElement>,
    pub notes: Option<String>,
    pub transition: Option<Transition>,
}

impl Slide {
    /// Create an empty slide with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            elements: Vec::new(),
            notes: None,
            transition: None,
        }
    }

    /// Add a text element to the slide.
    pub fn add_text(&mut self, content: impl Into<String>, position: Position, style: TextStyle) {
        self.elements.push(SlideElement::Text(TextElement::new(
            content, position, style,
        )));
    }

    /// Add an image element to the slide.
    pub fn add_image(&mut self, path: impl Into<String>, position: Position, size: Size) {
        self.elements
            .push(SlideElement::Image(ImageElement::new(path, position, size)));
    }

    /// Add speaker notes to the slide.
    pub fn set_notes(&mut self, notes: impl Into<String>) {
        self.notes = Some(notes.into());
    }
}

/// Visual transition applied when moving to a slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Fade,
    SlideLeft,
    Zoom,
}

/// Position expressed as normalized coordinates (0.0-1.0) relative to slide size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Size expressed as normalized width/height relative to slide size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Text styling for slide elements.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: u8,
    pub bold: bool,
    pub color: String,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Solvra Sans".into(),
            font_size: 28,
            bold: false,
            color: "#222222".into(),
        }
    }
}

impl TextStyle {
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

/// Slide elements supported by SolvraSlides.
#[derive(Debug, Clone, PartialEq)]
pub enum SlideElement {
    Text(TextElement),
    Image(ImageElement),
}

/// Text box rendered on a slide.
#[derive(Debug, Clone, PartialEq)]
pub struct TextElement {
    pub content: String,
    pub position: Position,
    pub style: TextStyle,
}

impl TextElement {
    pub fn new(content: impl Into<String>, position: Position, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            position,
            style,
        }
    }
}

/// Image rendered on a slide.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageElement {
    pub path: String,
    pub position: Position,
    pub size: Size,
}

impl ImageElement {
    pub fn new(path: impl Into<String>, position: Position, size: Size) -> Self {
        Self {
            path: path.into(),
            position,
            size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_presentation_outline() {
        let mut deck = Presentation::new();
        let slide = deck.add_slide("Welcome");
        slide.add_text(
            "Agenda",
            Position::new(0.1, 0.2),
            TextStyle::default().bold(),
        );
        slide.add_image(
            "images/intro.png",
            Position::new(0.5, 0.5),
            Size::new(0.3, 0.3),
        );
        deck.apply_transition(0, Transition::Fade).unwrap();

        let outline = deck.preview_outline();
        assert!(outline.contains("1. Welcome"));
        assert!(outline.contains("Agenda"));
        assert!(outline.contains("images/intro.png"));
    }
}
