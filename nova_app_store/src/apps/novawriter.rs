use crate::app::{
    AppCapability, AppCategory, AppId, AppMetadata, AppPackage, UiComponent, UiComponentKind,
};
use crate::sandbox::{SandboxPermission, SandboxPolicy};
use semver::Version;
use std::collections::BTreeSet;

/// Return the catalog metadata for NovaWriter.
pub fn metadata() -> AppMetadata {
    let sandbox = SandboxPolicy::new()
        .allow_permission(SandboxPermission::FileRead)
        .allow_permission(SandboxPermission::FileWrite)
        .allow_storage_root("~/Documents/NovaWriter");

    let package = AppPackage::new(Version::new(1, 0, 0))
        .with_sandbox(sandbox)
        .with_capability(
            AppCapability::new(
                "novawriter.document",
                "Rich text document authoring and serialization",
            )
            .with_tag("document")
            .with_tag("editor"),
        )
        .with_ui_component(
            UiComponent::new(
                "novawriter-editor",
                UiComponentKind::IdeView,
                "Document editing surface with style controls",
            )
            .with_entry_point("novawriter::editor"),
        );

    AppMetadata::new(
        AppId::new("dev.nova.writer").expect("valid app id"),
        "NovaWriter",
        "Rich document editor for long-form writing",
        "NovaWriter provides a distraction-free editing experience with styling, export, and collaboration hooks.",
        AppCategory::Productivity,
        "Nova Labs",
        package,
    )
    .with_tag("writing")
    .with_tag("productivity")
    .with_screenshot("screenshots/novawriter.png")
}

/// Represents a styled text run within a paragraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub content: String,
    pub style: TextStyle,
}

impl TextRun {
    /// Convenience constructor for a run with plain text.
    pub fn plain(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::default(),
        }
    }
}

/// Text style attributes applied to a run of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: u8,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            font_size: 12,
        }
    }
}

impl TextStyle {
    /// Create a new style with a custom font size.
    pub fn sized(font_size: u8) -> Self {
        Self {
            font_size,
            ..Self::default()
        }
    }

    /// Enable bold styling.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Enable italic styling.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Enable underline styling.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

/// Paragraph alignment options supported by NovaWriter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justified,
}

/// A document paragraph comprised of styled text runs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub runs: Vec<TextRun>,
    pub alignment: Alignment,
}

impl Paragraph {
    /// Push a new run of text into the paragraph.
    pub fn push_run(&mut self, run: TextRun) {
        self.runs.push(run);
    }

    /// Helper to push a simple run with inline style.
    pub fn push_text(&mut self, content: impl Into<String>, style: TextStyle) {
        self.push_run(TextRun {
            content: content.into(),
            style,
        });
    }
}

/// Represents a complete NovaWriter document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    title: Option<String>,
    paragraphs: Vec<Paragraph>,
    keywords: BTreeSet<String>,
}

impl Document {
    /// Create an empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the document title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Append a keyword used for semantic search.
    pub fn add_keyword(&mut self, keyword: impl Into<String>) {
        self.keywords.insert(keyword.into());
    }

    /// Begin a new paragraph and return a mutable reference to it.
    pub fn add_paragraph(&mut self) -> &mut Paragraph {
        self.paragraphs.push(Paragraph::default());
        self.paragraphs.last_mut().expect("paragraph just inserted")
    }

    /// Iterate over paragraphs immutably.
    pub fn paragraphs(&self) -> &[Paragraph] {
        &self.paragraphs
    }

    /// Count total words across all paragraphs.
    pub fn word_count(&self) -> usize {
        self.paragraphs
            .iter()
            .flat_map(|p| p.runs.iter())
            .flat_map(|run| run.content.split_whitespace())
            .count()
    }

    /// Render the document into Markdown for interoperability.
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();
        if let Some(title) = &self.title {
            output.push_str(&format!("# {}\n\n", title));
        }
        for paragraph in &self.paragraphs {
            let mut paragraph_text = String::new();
            for run in &paragraph.runs {
                let mut text = run.content.clone();
                if run.style.bold {
                    text = format!("**{}**", text);
                }
                if run.style.italic {
                    text = format!("_{}_", text);
                }
                if run.style.underline {
                    text = format!("<u>{}</u>", text);
                }
                paragraph_text.push_str(&text);
            }
            output.push_str(&paragraph_text);
            output.push_str("\n\n");
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_document_and_exports_markdown() {
        let mut doc = Document::new().with_title("Test Doc");
        doc.add_keyword("example");
        let para = doc.add_paragraph();
        para.push_text("Hello", TextStyle::default().bold());
        para.push_text(" world", TextStyle::default());
        assert_eq!(doc.word_count(), 2);
        let markdown = doc.to_markdown();
        assert!(markdown.contains("# Test Doc"));
        assert!(markdown.contains("**Hello**"));
    }
}
