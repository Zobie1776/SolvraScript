mod catalog;
mod installer;

pub use catalog::{AppCatalog, AppMetadata};
pub use installer::{InstallState, InstallerQueue};

use egui::{ProgressBar, Ui};
use tracing::info;

pub struct MobileAppStore {
    catalog: AppCatalog,
    installer: InstallerQueue,
    selected: Option<String>,
}

impl Default for MobileAppStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileAppStore {
    pub fn new() -> Self {
        Self {
            catalog: AppCatalog::bootstrap(),
            installer: InstallerQueue::default(),
            selected: None,
        }
    }

    pub fn show_compact(&mut self, ui: &mut Ui) {
        ui.heading("App Store");
        for app in &self.catalog.apps {
            let button = ui.selectable_label(self.selected.as_deref() == Some(&app.id), &app.name);
            if button.clicked() {
                self.selected = Some(app.id.clone());
            }
        }
    }

    pub fn show_catalog(&mut self, ui: &mut Ui) {
        ui.heading("Catalog");
        let mut requested_installs = Vec::new();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for app in &mut self.catalog.apps {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&app.name);
                        if app.installed {
                            ui.label("Installed");
                        }
                    });
                    ui.label(&app.summary);
                    let install_label = if app.installed { "Remove" } else { "Install" };
                    if ui.button(install_label).clicked() {
                        requested_installs.push(app.clone());
                    }
                });
            }
        });
        for app in requested_installs {
            self.queue_install(app);
        }
        self.render_queue(ui);
    }

    pub fn show_full(&mut self, ui: &mut Ui) {
        ui.heading("Nova App Store");
        self.show_catalog(ui);
    }

    pub fn handle_tap(&mut self, _position: egui::Pos2) {}

    pub fn handle_long_press(&mut self, _position: egui::Pos2) {
        if let Some(selected) = self.selected.clone() {
            self.catalog.toggle_install(&selected);
        }
    }

    fn queue_install(&mut self, app: AppMetadata) {
        let app_id = app.id.clone();
        info!("queue_install {}", app_id);
        self.installer.enqueue(app);
        self.installer.process_sync();
        self.catalog.toggle_install(&app_id);
    }

    fn render_queue(&mut self, ui: &mut Ui) {
        if self.installer.jobs().next().is_none() {
            return;
        }
        ui.separator();
        ui.heading("Downloads");
        for job in self.installer.jobs() {
            ui.label(&job.app.name);
            let label = match job.state {
                InstallState::Pending => "Pending",
                InstallState::Installing => "Installing",
                InstallState::Completed => "Completed",
            };
            ui.label(label);
            ui.add(ProgressBar::new(job.progress));
        }
    }
}
