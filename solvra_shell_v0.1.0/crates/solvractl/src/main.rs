//=============================================
// solvractl/src/main.rs
//=============================================
// Author: Solvra Shell Team
// License: MIT
// Goal: Command line control surface for Solvra Shell
// Objective: Emit JSON-RPC envelopes to control the compositor
//=============================================

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;

//=============================================
// SECTION: CLI Definitions
//=============================================

#[derive(Debug, Parser)]
#[command(name = "solvractl", version, about = "Solvra Shell control tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Reload compositor configuration
    Reload,
    /// Focus the next workspace
    FocusNext,
    /// Focus the previous workspace
    FocusPrev,
    /// Set the layout strategy
    Layout { layout: String },
    /// Apply a theme
    Theme { name: String },
}

//=============================================
// SECTION: JSON-RPC Envelope
//=============================================

#[derive(Clone, Debug, Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    id: i64,
}

//=============================================
// SECTION: Entry Point
//=============================================

fn main() -> Result<()> {
    utils::logging::init("solvractl");
    let cli = Cli::parse();
    let request = build_request(&cli.command);
    println!("{}", serde_json::to_string_pretty(&request)?);
    Ok(())
}

fn build_request(command: &Command) -> RpcRequest<'_> {
    match command {
        Command::Reload => RpcRequest {
            jsonrpc: "2.0",
            method: "config_reload",
            params: None,
            id: 1,
        },
        Command::FocusNext => RpcRequest {
            jsonrpc: "2.0",
            method: "focus_next",
            params: None,
            id: 2,
        },
        Command::FocusPrev => RpcRequest {
            jsonrpc: "2.0",
            method: "focus_prev",
            params: None,
            id: 3,
        },
        Command::Layout { layout } => RpcRequest {
            jsonrpc: "2.0",
            method: "layout_set",
            params: Some(serde_json::json!({"layout": layout})),
            id: 4,
        },
        Command::Theme { name } => RpcRequest {
            jsonrpc: "2.0",
            method: "theme_set",
            params: Some(serde_json::json!({"theme": name})),
            id: 5,
        },
    }
}
