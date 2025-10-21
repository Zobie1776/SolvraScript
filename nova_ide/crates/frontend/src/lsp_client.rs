use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tower_lsp::lsp_types::{
    ClientInfo, CompletionParams, Diagnostic, InitializeParams, Position, TextDocumentIdentifier,
    Url,
};

#[derive(Debug, Clone)]
pub struct LanguageServerDescriptor {
    pub language_id: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
}

#[derive(Debug)]
pub enum LspCommand {
    Initialize(InitializeParams),
    RequestCompletions(CompletionParams),
    Shutdown,
}

#[derive(Debug)]
pub struct LspSessionHandle {
    pub language_id: String,
    pub root: PathBuf,
    pub sender: mpsc::Sender<LspCommand>,
    pub task: JoinHandle<()>,
}

#[derive(Debug, Default)]
pub struct LspCoordinator {
    registry: HashMap<String, LanguageServerDescriptor>,
    active_sessions: HashMap<String, LspSessionHandle>,
}

impl LspCoordinator {
    pub fn new() -> Self {
        let mut coordinator = Self::default();
        coordinator.register_builtin_servers();
        coordinator
    }

    fn register_builtin_servers(&mut self) {
        let definitions = vec![
            ("nova_core", "nova-core-lsp"),
            ("nova_script", "nova-script-lsp"),
            ("rust", "rust-analyzer"),
            ("c", "clangd"),
            ("cpp", "clangd"),
            ("csharp", "omnisharp"),
            ("java", "jdtls"),
            ("javascript", "typescript-language-server"),
            ("typescript", "typescript-language-server"),
            ("html", "vscode-html-language-server"),
            ("python", "pyright-langserver"),
        ];
        for (language_id, executable) in definitions {
            self.registry.insert(
                language_id.to_string(),
                LanguageServerDescriptor {
                    language_id: language_id.to_string(),
                    executable: PathBuf::from(executable),
                    args: vec!["--stdio".to_string()],
                    file_extensions: Vec::new(),
                },
            );
        }
    }

    pub fn register_descriptor(&mut self, descriptor: LanguageServerDescriptor) {
        self.registry
            .insert(descriptor.language_id.clone(), descriptor);
    }

    pub async fn ensure_session(&mut self, language_id: &str, root: &Path) -> Result<()> {
        if self.active_sessions.contains_key(language_id) {
            return Ok(());
        }

        let descriptor = self
            .registry
            .get(language_id)
            .context("missing language server descriptor")?
            .clone();

        let (sender, mut receiver) = mpsc::channel::<LspCommand>(32);
        let root = root.to_path_buf();
        let language_id = language_id.to_string();
        let runtime_language_id = language_id.clone();
        let task = tokio::spawn(async move {
            let language_id = runtime_language_id;
            while let Some(command) = receiver.recv().await {
                match command {
                    LspCommand::Initialize(params) => {
                        log::info!(
                            "Initializing {} with params {:?}",
                            language_id,
                            params.root_uri
                        );
                        let _ = descriptor.executable.as_os_str();
                    }
                    LspCommand::RequestCompletions(params) => {
                        log::debug!(
                            "Completion request {:?}",
                            params.text_document_position.text_document.uri
                        );
                        let _ = params;
                    }
                    LspCommand::Shutdown => {
                        log::info!("Shutting down LSP session {}", language_id);
                        break;
                    }
                }
            }
        });

        let handle = LspSessionHandle {
            language_id: language_id.clone(),
            root: root.clone(),
            sender: sender.clone(),
            task,
        };
        self.active_sessions.insert(language_id.clone(), handle);

        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            #[allow(deprecated)]
            root_path: None,
            root_uri: Url::from_directory_path(&root).ok(),
            initialization_options: None,
            capabilities: Default::default(),
            trace: None,
            workspace_folders: None,
            client_info: Some(ClientInfo {
                name: "NovaIDE".into(),
                version: Some("0.1.0".into()),
            }),
            locale: None,
        };
        sender.send(LspCommand::Initialize(init_params)).await?;
        Ok(())
    }

    pub async fn request_completions(
        &mut self,
        language_id: &str,
        text_document: TextDocumentIdentifier,
        position: Position,
    ) -> Result<()> {
        if let Some(session) = self.active_sessions.get(language_id) {
            let params = CompletionParams {
                text_document_position: tower_lsp::lsp_types::TextDocumentPositionParams {
                    text_document,
                    position,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                context: None,
            };
            session
                .sender
                .send(LspCommand::RequestCompletions(params))
                .await?;
        }
        Ok(())
    }

    pub async fn diagnostics_for(&mut self, language_id: &str) -> Vec<Diagnostic> {
        if let Some(session) = self.active_sessions.get(language_id) {
            log::trace!("Diagnostics requested for {}", session.language_id);
        }
        Vec::new()
    }

    pub async fn shutdown(&mut self, language_id: &str) {
        if let Some(handle) = self.active_sessions.remove(language_id) {
            let _ = handle.sender.send(LspCommand::Shutdown).await;
            handle.task.abort();
        }
    }
}
