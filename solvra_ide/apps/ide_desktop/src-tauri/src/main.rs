#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Context;
use solvra_fail_safe::{register_passphrase, verify};
use solvra_ide_core::lsp::{
    CompletionItem, Diagnostic, HoverResult, SolvraLanguageServer, TextPosition,
};
use solvra_ide_core::tasks::{RunTaskPayload, TaskOutcome, TaskRunner};
use solvra_ide_core::tree::{ProjectNode, ProjectTreeBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{async_runtime::Mutex, Manager, State};
use tokio::fs;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Serialize, Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub content: String,
}

#[tauri::command]
async fn open_file(path: String) -> Result<FilePayload, String> {
    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {path}"))
        .map_err(|err| err.to_string())?;
    Ok(FilePayload { path, content })
}

#[tauri::command]
async fn save_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content)
        .await
        .with_context(|| format!("failed to save {path}"))
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn run_task(payload: RunTaskPayload) -> Result<TaskOutcome, String> {
    let runner = TaskRunner::new();
    let (command, options) = payload.into_parts();
    runner
        .run(&command, options)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn read_project_tree(root: String) -> Result<ProjectNode, String> {
    let builder = ProjectTreeBuilder::default();
    let node = builder
        .build(Path::new(&root))
        .map_err(|err| err.to_string())?;
    Ok(node)
}

#[tauri::command]
fn show_error(window: tauri::Window, message: String) -> Result<(), String> {
    window
        .emit("solvra://error", message)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn verify_publish_passphrase(passphrase: String) -> Result<(), String> {
    verify(&passphrase).map_err(|err| err.to_string())
}

#[tauri::command]
fn configure_fail_safe(passphrase: String) -> Result<(), String> {
    register_passphrase(&passphrase)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn init_tracing() {
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
}

type SharedLsp = Mutex<SolvraLanguageServer>;

#[tauri::command]
async fn lsp_complete(
    state: State<'_, SharedLsp>,
    source: String,
    line: usize,
    character: usize,
) -> Result<Vec<CompletionItem>, String> {
    let mut guard = state.lock().await;
    guard
        .complete(&source, TextPosition { line, character })
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn lsp_hover(
    state: State<'_, SharedLsp>,
    source: String,
    line: usize,
    character: usize,
) -> Result<Option<HoverResult>, String> {
    let mut guard = state.lock().await;
    guard
        .hover(&source, TextPosition { line, character })
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn lsp_goto_definition(
    state: State<'_, SharedLsp>,
    source: String,
    symbol: String,
) -> Result<Option<TextPosition>, String> {
    let mut guard = state.lock().await;
    guard
        .goto_definition(&source, &symbol)
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn lsp_diagnostics(
    state: State<'_, SharedLsp>,
    source: String,
) -> Result<Vec<Diagnostic>, String> {
    let guard = state.lock().await;
    guard.diagnostics(&source).map_err(|err| err.to_string())
}

pub fn run() {
    init_tracing();
    tauri::Builder::default()
        .manage(Mutex::new(SolvraLanguageServer::new()))
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            open_file,
            save_file,
            run_task,
            read_project_tree,
            show_error,
            verify_publish_passphrase,
            configure_fail_safe,
            lsp_complete,
            lsp_hover,
            lsp_goto_definition,
            lsp_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running SolvraIDE desktop");
}

fn main() {
    run();
}
