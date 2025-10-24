#![allow(dead_code)]

mod auth;

pub use auth::{AuthProvider, AuthState};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use egui::Ui;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientOptions {
    pub base_url: String,
    pub workspace: Option<String>,
    pub timeout: Duration,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            base_url: "https://api.novaos.dev".into(),
            workspace: None,
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiMode {
    Chat,
    Code,
    Automations,
}

#[derive(Debug, Default)]
struct ConnectionState {
    authenticated: bool,
    last_error: Option<String>,
}

pub struct NovaAiClient {
    http: Client,
    options: ClientOptions,
    auth: AuthState,
    connection: Arc<Mutex<ConnectionState>>,
    mode: AiMode,
    history: Vec<AiInteraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInteraction {
    pub title: String,
    pub body: String,
    pub mode: AiMode,
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("authentication required")]
    AuthenticationRequired,
    #[error("request failed: {0}")]
    Request(String),
}

impl NovaAiClient {
    pub async fn new(options: ClientOptions) -> Result<Self> {
        let http = Client::builder().timeout(options.timeout).build()?;
        let auth = AuthState::new(None);
        let client = Self {
            http,
            options,
            auth,
            connection: Arc::new(Mutex::new(ConnectionState::default())),
            mode: AiMode::Chat,
            history: Vec::with_capacity(16),
        };
        Ok(client)
    }

    pub async fn with_auth_provider(
        options: ClientOptions,
        provider: Arc<dyn AuthProvider>,
    ) -> Result<Self> {
        let http = Client::builder().timeout(options.timeout).build()?;
        let auth = AuthState::new(Some(provider));
        Ok(Self {
            http,
            options,
            auth,
            connection: Arc::new(Mutex::new(ConnectionState::default())),
            mode: AiMode::Chat,
            history: Vec::with_capacity(16),
        })
    }

    pub async fn ensure_authenticated(&self) -> Result<()> {
        let mut connection = self.connection.lock().unwrap();
        match self.auth.token().await {
            Ok(token) => {
                connection.authenticated = token.is_some();
                connection.last_error = None;
                Ok(())
            }
            Err(err) => {
                connection.authenticated = false;
                connection.last_error = Some(err.to_string());
                Err(ApiError::AuthenticationRequired.into())
            }
        }
    }

    pub fn connection_state(&self) -> String {
        let state = self.connection.lock().unwrap();
        if state.authenticated {
            "connected".into()
        } else {
            state.last_error.clone().unwrap_or_else(|| "offline".into())
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.connection.lock().unwrap().authenticated
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            AiMode::Chat => AiMode::Code,
            AiMode::Code => AiMode::Automations,
            AiMode::Automations => AiMode::Chat,
        };
        info!("nova_ai_mode {:?}", self.mode);
    }

    pub async fn request_completion(&mut self, prompt: &str) -> Result<String> {
        self.ensure_authenticated().await?;
        debug!("nova_ai_prompt mode={:?} prompt={}", self.mode, prompt);
        let response = format!("[NovaAI {:?}] {prompt}", self.mode);
        self.history.push(AiInteraction {
            title: "Prompt".into(),
            body: response.clone(),
            mode: self.mode,
        });
        Ok(response)
    }

    pub fn show_compact(&mut self, ui: &mut Ui) {
        ui.heading("NovaAI");
        ui.label(format!("Mode: {:?}", self.mode));
        for item in self.history.iter().rev().take(3) {
            ui.label(format!("{}", item.body));
        }
    }

    pub fn show_full(&mut self, ui: &mut Ui) {
        ui.heading("NovaAI Workspace");
        ui.label(format!("Endpoint: {}", self.options.base_url));
        ui.label(format!("Mode: {:?}", self.mode));
        if ui.button("Cycle Mode").clicked() {
            self.cycle_mode();
        }
        ui.separator();
        for item in self.history.iter().rev() {
            ui.collapsing(&item.title, |ui| {
                ui.label(&item.body);
            });
        }
    }
}
