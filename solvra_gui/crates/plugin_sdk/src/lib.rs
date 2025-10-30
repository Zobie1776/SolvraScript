//=============================================
// plugin_sdk/src/lib.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Shared SDK helpers for Solvra GUI wasm plugins
// Objective: Provide helper functions to compile and instantiate wasm modules
//=============================================

use anyhow::Result;
use wasmtime::{Engine, Module, Store};

/// Metadata describing a plugin.
#[derive(Debug, Clone)]
pub struct PluginDescriptor {
    /// Human readable name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
}

/// Compile a wasm module using a shared engine.
pub fn compile(engine: &Engine, bytes: &[u8]) -> Result<Module> {
    let module = Module::from_binary(engine, bytes)?;
    Ok(module)
}

/// Create a new store for plugin execution.
pub fn new_store(engine: &Engine) -> Store<()> {
    Store::new(engine, ())
}
