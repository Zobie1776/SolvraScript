use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::{egui, App};
use egui::TextEdit;

use crate::debugger::DebuggerHub;
use crate::file_explorer::{ExplorerNode, FileExplorer};
use crate::git_panel::GitPanelState;
use crate::lsp_client::LspCoordinator;
use crate::settings::SettingsStore;
use crate::solvra_ai::{CompletionContext, SolvraAiService};
use crate::theme::ThemeManager;

pub struct GuiLaunchOptions {
    pub workspace_root: PathBuf,
}

pub struct SolvraIdeContext {
    pub runtime: Arc<tokio::runtime::Runtime>,
    pub workspace_root: PathBuf,
}

pub trait IdePlugin: Send {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn show(&mut self, ctx: &egui::Context, app: &mut SolvraIdeApp);
}

pub struct SolvraIdeApp {
    pub context: SolvraIdeContext,
    pub file_explorer: FileExplorer,
    pub editor: EditorState,
    pub terminal: TerminalPane,
    pub diagnostics: Vec<DiagnosticItem>,
    pub git_panel: GitPanelState,
    pub ai_service: SolvraAiService,
    pub lsp: LspCoordinator,
    pub debugger: DebuggerHub,
    pub settings: SettingsStore,
    pub theme: ThemeManager,
    pub show_terminal: bool,
    pub show_ai_panel: bool,
    pub show_git_panel: bool,
    pub show_settings: bool,
    pub show_command_palette: bool,
    pub command_filter: String,
    pub plugins: Vec<Box<dyn IdePlugin>>,
}

impl SolvraIdeApp {
    fn new(context: SolvraIdeContext) -> Self {
        let mut file_explorer = FileExplorer::default();
        file_explorer.load_from_root(context.workspace_root.clone(), 3);

        let settings = SettingsStore::load_or_default();
        let theme = ThemeManager::new(settings.data.theme.clone());
        let mut git_panel = GitPanelState::new(context.workspace_root.clone());
        git_panel.refresh();

        let ai_service = SolvraAiService::default();
        let lsp = LspCoordinator::new();
        let debugger = DebuggerHub::new();

        Self {
            context,
            file_explorer,
            editor: EditorState::default(),
            terminal: TerminalPane::default(),
            diagnostics: Vec::new(),
            git_panel,
            ai_service,
            lsp,
            debugger,
            settings,
            theme,
            show_terminal: true,
            show_ai_panel: true,
            show_git_panel: true,
            show_settings: false,
            show_command_palette: false,
            command_filter: String::new(),
            plugins: Vec::new(),
        }
    }

    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Open...").clicked() {
                self.file_explorer.refresh(3);
            }
            if ui.button("Command Palette").clicked() {
                self.show_command_palette = true;
            }
            if ui.button("New Terminal").clicked() {
                self.show_terminal = true;
                self.terminal.output.push(String::from("Spawned terminal."));
            }
            ui.separator();
            ui.checkbox(&mut self.show_terminal, "Terminal");
            ui.checkbox(&mut self.show_ai_panel, "SolvraAI");
            ui.checkbox(&mut self.show_git_panel, "Git");
            ui.checkbox(&mut self.show_settings, "Settings");
        });
    }

    fn render_file_explorer(&mut self, ui: &mut egui::Ui) {
        if let Some(root) = self.file_explorer.root.as_mut() {
            ui.heading("Project");
            let active = self.editor.active_path();
            let mut open_requests = Vec::new();
            Self::render_explorer_node(ui, root, &mut open_requests, active.as_ref());
            for path in open_requests {
                self.open_tab(path);
            }
        } else {
            ui.label("No workspace root.");
        }
    }

    fn render_explorer_node(
        ui: &mut egui::Ui,
        node: &mut ExplorerNode,
        open_requests: &mut Vec<PathBuf>,
        active_path: Option<&PathBuf>,
    ) {
        if node.is_dir {
            let header = egui::CollapsingHeader::new(node.name.clone())
                .default_open(node.expanded)
                .show(ui, |ui| {
                    for child in node.children.iter_mut() {
                        Self::render_explorer_node(ui, child, open_requests, active_path);
                    }
                });
            node.expanded = header.fully_open();
        } else {
            let is_active = active_path.map(|path| path == &node.path).unwrap_or(false);
            if ui.selectable_label(is_active, &node.name).clicked() {
                open_requests.push(node.path.clone());
            }
        }
    }

    fn open_tab(&mut self, path: PathBuf) {
        if let Some(index) = self.editor.tab_index(&path) {
            self.editor.active = Some(index);
            return;
        }
        let contents = fs::read_to_string(&path).unwrap_or_default();
        let language = detect_language(&path);
        self.editor.tabs.push(EditorTab {
            path,
            contents,
            language,
            dirty: false,
        });
        self.editor.active = Some(self.editor.tabs.len() - 1);
        let workspace_root = self.context.workspace_root.clone();
        if let Some(language) = self.editor.tabs.last().map(|tab| tab.language.clone()) {
            if let Err(err) = self
                .context
                .runtime
                .block_on(self.lsp.ensure_session(&language, &workspace_root))
            {
                self.terminal.output.push(format!(
                    "Failed to initialize LSP for {}: {}",
                    language, err
                ));
            }
        }
    }

    fn render_editor(&mut self, ui: &mut egui::Ui) {
        if self.editor.tabs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("Open a file from the explorer to begin editing.");
            });
            return;
        }

        ui.horizontal(|ui| {
            let mut remove_index = None;
            for (index, tab) in self.editor.tabs.iter().enumerate() {
                let mut label = tab
                    .path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| tab.path.display().to_string());
                if tab.dirty {
                    label.push('*');
                }
                let selected = Some(index) == self.editor.active;
                if ui.selectable_label(selected, label).clicked() {
                    self.editor.active = Some(index);
                }
                if ui.small_button("×").clicked() {
                    remove_index = Some(index);
                }
            }
            if let Some(index) = remove_index {
                self.editor.tabs.remove(index);
                if self.editor.tabs.is_empty() {
                    self.editor.active = None;
                } else if let Some(active) = self.editor.active {
                    if active >= self.editor.tabs.len() {
                        self.editor.active = Some(self.editor.tabs.len() - 1);
                    }
                }
            }
        });

        ui.separator();
        if let Some(active) = self.editor.active {
            let tab = &mut self.editor.tabs[active];
            let edit = TextEdit::multiline(&mut tab.contents)
                .desired_rows(32)
                .lock_focus(true)
                .code_editor();
            let response = ui.add(edit);
            if response.changed() {
                tab.dirty = true;
                if self.settings.data.auto_save {
                    let path = tab.path.clone();
                    if let Err(err) = fs::write(&path, &tab.contents) {
                        self.terminal.output.push(format!(
                            "Failed to save {}: {}",
                            path.display(),
                            err
                        ));
                    } else {
                        tab.dirty = false;
                    }
                }
            }
        }
    }

    fn render_terminal(&mut self, ui: &mut egui::Ui) {
        ui.heading("Terminal");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for line in &self.terminal.output {
                    ui.label(line);
                }
            });
    }

    fn render_diagnostics(&mut self, ui: &mut egui::Ui) {
        ui.heading("Diagnostics");
        if self.diagnostics.is_empty() {
            ui.label("No diagnostics reported.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for diagnostic in &self.diagnostics {
                    ui.group(|ui| {
                        ui.label(format!("{}:{}", diagnostic.file.display(), diagnostic.line));
                        ui.label(&diagnostic.message);
                    });
                }
            });
        }
    }

    fn render_ai_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("SolvraAI");
        if !self.settings.data.enable_solvra_ai {
            ui.label("SolvraAI is disabled in settings.");
            return;
        }
        if let Some(active_path) = self.editor.active_path() {
            if ui.button("Request Completion").clicked() {
                let position = (0, 0);
                let context = CompletionContext {
                    file: active_path.display().to_string(),
                    position,
                    prefix: self.editor.active_text().unwrap_or_default(),
                };
                if let Ok(suggestion) = self
                    .context
                    .runtime
                    .block_on(self.ai_service.debounce_completion(context))
                {
                    self.terminal
                        .output
                        .push(format!("AI suggestion: {}", suggestion.body));
                }
            }
        }
        ui.separator();
        for suggestion in self.ai_service.history() {
            ui.collapsing(&suggestion.title, |ui| {
                ui.label(&suggestion.body);
                ui.label(format!("confidence: {:.2}", suggestion.confidence));
            });
        }
    }

    fn render_git_panel(&mut self, ctx: &egui::Context) {
        if self.show_git_panel {
            egui::Window::new("Git")
                .open(&mut self.show_git_panel)
                .show(ctx, |ui| {
                    self.git_panel.refresh();
                    if let Some(branch) = &self.git_panel.summary.branch {
                        ui.label(format!("Branch: {}", branch));
                    } else {
                        ui.label("Git repository not detected");
                    }
                    ui.separator();
                    if self.git_panel.summary.changes.is_empty() {
                        ui.label("Working tree clean");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for change in &self.git_panel.summary.changes {
                                ui.label(format!("{:?} {}", change.status, change.path.display()));
                            }
                        });
                    }
                });
        }
    }

    fn render_settings(&mut self, ctx: &egui::Context) {
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    ui.label("Theme");
                    let themes = self.theme.available_themes();
                    for theme in themes {
                        if ui
                            .selectable_label(self.settings.data.theme == theme, &theme)
                            .clicked()
                        {
                            self.settings.set_theme(theme.clone());
                            self.theme.active_theme = theme;
                            let _ = self.settings.save();
                        }
                    }
                    ui.separator();
                    ui.label("Font size");
                    let mut font_size = self.settings.data.font_size;
                    if ui
                        .add(egui::Slider::new(&mut font_size, 10.0..=24.0))
                        .changed()
                    {
                        self.settings.update_font_size(font_size);
                        let _ = self.settings.save();
                    }
                    ui.checkbox(&mut self.settings.data.enable_solvra_ai, "Enable SolvraAI");
                });
        }
    }

    fn render_command_palette(&mut self, ctx: &egui::Context) {
        if self.show_command_palette {
            let entries = self.command_entries();
            let mut open = self.show_command_palette;
            let mut should_close = false;
            egui::Window::new("Command Palette")
                .collapsible(false)
                .default_width(400.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.command_filter).hint_text("Search commands"),
                    );
                    ui.separator();
                    let filter = self.command_filter.to_lowercase();
                    for entry in &entries {
                        if entry.label.to_lowercase().contains(&filter)
                            && ui.button(&entry.label).clicked()
                        {
                            (entry.action)(self);
                            should_close = true;
                        }
                    }
                });
            self.show_command_palette = open && !should_close;
        }
    }

    fn command_entries(&self) -> Vec<CommandPaletteEntry> {
        let mut entries = vec![
            CommandPaletteEntry::new("Toggle Terminal", |app| {
                app.show_terminal = !app.show_terminal
            }),
            CommandPaletteEntry::new("Toggle SolvraAI", |app| {
                app.show_ai_panel = !app.show_ai_panel
            }),
            CommandPaletteEntry::new("Toggle Git Panel", |app| {
                app.show_git_panel = !app.show_git_panel
            }),
            CommandPaletteEntry::new("Refresh Explorer", |app| app.file_explorer.refresh(3)),
        ];
        for plugin in &self.plugins {
            let id = plugin.id();
            let name = plugin.name().to_string();
            entries.push(CommandPaletteEntry::new(name, move |app| {
                app.terminal.output.push(format!("Plugin {} activated", id));
            }));
        }
        entries
    }
}

impl App for SolvraIdeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx, self.settings.data.font_size);

        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.render_menu_bar(ui));
        egui::SidePanel::left("explorer").show(ctx, |ui| self.render_file_explorer(ui));

        if self.show_ai_panel {
            egui::SidePanel::right("ai").show(ctx, |ui| self.render_ai_panel(ui));
        }

        egui::CentralPanel::default().show(ctx, |ui| self.render_editor(ui));

        egui::TopBottomPanel::bottom("diagnostics").show(ctx, |ui| self.render_diagnostics(ui));
        if self.show_terminal {
            egui::TopBottomPanel::bottom("terminal").show(ctx, |ui| self.render_terminal(ui));
        }

        self.render_git_panel(ctx);
        self.render_settings(ctx);
        self.render_command_palette(ctx);

        if ctx.input(|input| {
            input.key_pressed(egui::Key::F1)
                || (input.modifiers.command && input.key_pressed(egui::Key::P))
        }) {
            self.show_command_palette = true;
        }
    }
}

#[derive(Default)]
pub struct TerminalPane {
    pub output: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticItem {
    pub file: PathBuf,
    pub line: u32,
    pub message: String,
}

#[derive(Default)]
pub struct EditorState {
    pub tabs: Vec<EditorTab>,
    pub active: Option<usize>,
}

impl EditorState {
    pub fn tab_index(&self, path: &Path) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.path == path)
    }

    pub fn active_path(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.tabs.get(index).map(|tab| tab.path.clone()))
    }

    pub fn active_text(&self) -> Option<String> {
        self.active
            .and_then(|index| self.tabs.get(index))
            .map(|tab| tab.contents.clone())
    }
}

#[derive(Debug, Clone)]
pub struct EditorTab {
    pub path: PathBuf,
    pub contents: String,
    pub language: String,
    pub dirty: bool,
}

pub struct CommandPaletteEntry {
    pub label: String,
    pub action: Box<dyn Fn(&mut SolvraIdeApp) + Send + Sync>,
}

impl CommandPaletteEntry {
    fn new(
        label: impl Into<String>,
        action: impl Fn(&mut SolvraIdeApp) + Send + Sync + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            action: Box::new(action),
        }
    }
}

pub fn run_gui(options: GuiLaunchOptions) -> eframe::Result<()> {
    let runtime = Arc::new(tokio::runtime::Runtime::new().expect("tokio runtime"));
    let mut context = Some(SolvraIdeContext {
        runtime: runtime.clone(),
        workspace_root: options.workspace_root.clone(),
    });
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "SolvraIDE v2",
        native_options,
        Box::new(move |_cc| Box::new(SolvraIdeApp::new(context.take().expect("app context")))),
    )
}

fn detect_language(path: &Path) -> String {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "rs" => "rust",
        "c" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "h" => "cpp",
        "cs" => "csharp",
        "java" => "java",
        "js" => "javascript",
        "ts" => "typescript",
        "html" | "htm" => "html",
        "py" => "python",
        "svs" => "solvra_script",
        "svc" => "solvra_core",
        other => other,
    }
    .to_string()
}
