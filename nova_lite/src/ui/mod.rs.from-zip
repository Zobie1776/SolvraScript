use std::ops::ControlFlow;
use std::time::Duration;

use egui::{vec2, Color32, Context, RawInput};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::api::NovaAiClient;
use crate::app_store::MobileAppStore;
use crate::ide::MobileIde;

pub mod components;
pub mod gestures;
pub mod layout;

pub use components::{StatusBadge, ToolbarAction};
pub use gestures::{Gesture, GestureSystem, TouchEvent};
pub use layout::{LayoutClass, ResponsiveLayout};

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub refresh_interval: Duration,
    pub surface_size: egui::Vec2,
    pub accent_color: Color32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_interval: Duration::from_millis(16),
            surface_size: vec2(1024.0, 768.0),
            accent_color: Color32::from_rgb(105, 89, 205),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UiError {
    #[error("UI error: {0}")]
    General(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub enum UiCommand {
    ToggleIde,
    ToggleAppStore,
    ToggleAiPanel,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum UiEvent {
    Touch(TouchEvent),
    Command(UiCommand),
    Resize(egui::Vec2),
    Shutdown,
}

pub struct LiteUiManager {
    config: UiConfig,
    context: Context,
    layout: ResponsiveLayout,
    gestures: GestureSystem,
    event_tx: UnboundedSender<UiEvent>,
    ide_visible: bool,
    app_store_visible: bool,
    ai_panel_visible: bool,
}

impl LiteUiManager {
    pub async fn bootstrap(
        config: UiConfig,
    ) -> Result<(Self, UnboundedReceiver<UiEvent>), UiError> {
        let (event_tx, event_rx) = unbounded_channel();
        let manager = Self {
            config,
            context: Context::default(),
            layout: ResponsiveLayout::default(),
            gestures: GestureSystem::new(),
            event_tx,
            ide_visible: true,
            app_store_visible: true,
            ai_panel_visible: true,
        };
        Ok((manager, event_rx))
    }

    pub fn event_sender(&self) -> UnboundedSender<UiEvent> {
        self.event_tx.clone()
    }

    pub async fn render_frame(
        &mut self,
        ide: &mut MobileIde,
        app_store: &mut MobileAppStore,
        ai_client: &mut NovaAiClient,
    ) -> Result<(), UiError> {
        let min_surface = self.layout.minimum_surface();
        let surface_size = egui::Vec2::new(
            self.config.surface_size.x.max(min_surface.x),
            self.config.surface_size.y.max(min_surface.y),
        );
        let raw_input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, surface_size)),
            ..Default::default()
        };

        let accent = self.config.accent_color;

        let layout = &self.layout;
        let refresh_interval = self.config.refresh_interval;
        let _output = self.context.run(raw_input, |ctx| {
            ctx.request_repaint_after(refresh_interval);
            let surface = ctx.screen_rect().size();
            let class = layout.classify(surface);

            egui::TopBottomPanel::top("nova_lite_toolbar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("NovaLite");
                    ui.separator();
                    let status = StatusBadge {
                        label: format!("AI: {}", ai_client.connection_state()),
                        color: if ai_client.is_authenticated() {
                            accent
                        } else {
                            Color32::from_rgb(180, 58, 58)
                        },
                    };
                    status.show(ui);

                    ui.add_space(12.0);
                    let ide_toggle = ToolbarAction {
                        label: "IDE".into(),
                        tooltip: "Toggle NovaIDE panels".into(),
                    };
                    if ide_toggle.show(ui) {
                        let _ = self.event_tx.send(UiEvent::Command(UiCommand::ToggleIde));
                    }
                    let store_toggle = ToolbarAction {
                        label: "Store".into(),
                        tooltip: "Toggle App Store".into(),
                    };
                    if store_toggle.show(ui) {
                        let _ = self
                            .event_tx
                            .send(UiEvent::Command(UiCommand::ToggleAppStore));
                    }
                    let ai_toggle = ToolbarAction {
                        label: "NovaAI".into(),
                        tooltip: "Toggle NovaAI panel".into(),
                    };
                    if ai_toggle.show(ui) {
                        let _ = self
                            .event_tx
                            .send(UiEvent::Command(UiCommand::ToggleAiPanel));
                    }
                });
            });

            egui::CentralPanel::default().show(ctx, |ui| match class {
                LayoutClass::Compact => {
                    ui.vertical(|ui| {
                        if self.ide_visible {
                            ide.render_compact(ui);
                        }
                        if self.app_store_visible {
                            ui.separator();
                            app_store.show_compact(ui);
                        }
                        if self.ai_panel_visible {
                            ui.separator();
                            ai_client.show_compact(ui);
                        }
                    });
                }
                LayoutClass::Medium => {
                    ui.horizontal(|ui| {
                        if self.ide_visible {
                            ui.vertical(|ui| {
                                ide.render_split(ui, 0.6);
                            });
                        }
                        if self.app_store_visible {
                            ui.separator();
                            ui.vertical(|ui| {
                                app_store.show_catalog(ui);
                            });
                        }
                    });
                    if self.ai_panel_visible {
                        ui.separator();
                        ai_client.show_compact(ui);
                    }
                }
                LayoutClass::Expanded => {
                    let _ = layout.ideal_panel_split(surface, class);
                    ui.columns(3, |columns| {
                        if self.ide_visible {
                            ide.render_expanded(&mut columns[0]);
                        }
                        if self.app_store_visible {
                            columns[1].vertical(|ui| app_store.show_full(ui));
                        }
                        if self.ai_panel_visible {
                            columns[2].vertical(|ui| ai_client.show_full(ui));
                        }
                    });
                }
            });
        });

        Ok(())
    }

    pub async fn handle_event(
        &mut self,
        event: UiEvent,
        ide: &mut MobileIde,
        app_store: &mut MobileAppStore,
        ai_client: &mut NovaAiClient,
    ) -> Result<ControlFlow<()>, UiError> {
        match event {
            UiEvent::Touch(touch) => {
                if let Some(gesture) = self.gestures.record(touch) {
                    match gesture {
                        Gesture::Tap { position } => {
                            ide.handle_tap(position);
                            app_store.handle_tap(position);
                        }
                        Gesture::LongPress { position } => {
                            app_store.handle_long_press(position);
                        }
                        Gesture::Swipe { start: _, end: _ } => {
                            ai_client.cycle_mode();
                        }
                    }
                }
            }
            UiEvent::Command(cmd) => match cmd {
                UiCommand::ToggleIde => self.ide_visible = !self.ide_visible,
                UiCommand::ToggleAppStore => self.app_store_visible = !self.app_store_visible,
                UiCommand::ToggleAiPanel => self.ai_panel_visible = !self.ai_panel_visible,
            },
            UiEvent::Resize(size) => {
                self.config.surface_size = size.max(self.layout.minimum_surface());
            }
            UiEvent::Shutdown => {
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    pub async fn shutdown(&mut self) {
        self.context = Context::default();
    }
}
