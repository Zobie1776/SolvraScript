mod code_editor;
mod file_explorer;

pub use code_editor::CodeEditorPanel;
pub use file_explorer::{FileEntry, FileExplorerPanel};

use egui::Ui;

#[derive(Debug)]
pub struct MobileIde {
    file_explorer: FileExplorerPanel,
    code_editor: CodeEditorPanel,
    active_tab: IdeTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdeTab {
    Explorer,
    Editor,
}

impl Default for MobileIde {
    fn default() -> Self {
        let explorer = FileExplorerPanel::with_entries(vec![
            FileEntry {
                path: "src/main.novas".into(),
                is_dir: false,
            },
            FileEntry {
                path: "scripts/build.novas".into(),
                is_dir: false,
            },
            FileEntry {
                path: "assets".into(),
                is_dir: true,
            },
        ]);

        Self {
            file_explorer: explorer,
            code_editor: CodeEditorPanel::default(),
            active_tab: IdeTab::Editor,
        }
    }
}

impl MobileIde {
    pub fn render_compact(&mut self, ui: &mut Ui) {
        ui.heading("NovaIDE Mobile");
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.active_tab == IdeTab::Explorer, "Files")
                .clicked()
            {
                self.active_tab = IdeTab::Explorer;
            }
            if ui
                .selectable_label(self.active_tab == IdeTab::Editor, "Editor")
                .clicked()
            {
                self.active_tab = IdeTab::Editor;
            }
        });
        ui.separator();

        match self.active_tab {
            IdeTab::Explorer => self.file_explorer.show(ui),
            IdeTab::Editor => self.code_editor.show(ui),
        }
    }

    pub fn render_split(&mut self, ui: &mut Ui, _ratio: f32) {
        ui.vertical(|ui| {
            ui.heading("NovaIDE");
            ui.separator();
            self.file_explorer.show(ui);
            ui.separator();
            self.code_editor.show(ui);
        });
    }

    pub fn render_expanded(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("NovaIDE Workspace");
            ui.separator();
            ui.columns(2, |columns| {
                self.file_explorer.show(&mut columns[0]);
                self.code_editor.show(&mut columns[1]);
            });
        });
    }

    pub fn handle_tap(&mut self, position: egui::Pos2) {
        match self.active_tab {
            IdeTab::Explorer => self.file_explorer.handle_tap(position),
            IdeTab::Editor => self.code_editor.handle_tap(position),
        }
    }
}
