//=============================================
// plugin_sdk/src/lib.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Shared SDK helpers for Nova Shell wasm plugins
// Objective: Provide simple host-call registration scaffolding
//=============================================

use anyhow::Result;
use wasmtime::{Engine, Module, Store};

//=============================================
// SECTION: Plugin Descriptor
//=============================================

/// Metadata describing a plugin.
#[derive(Debug, Clone)]
pub struct PluginDescriptor {
    /// Human readable name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
}

//=============================================
// SECTION: Loader
//=============================================

/// Compile a wasm module using a shared engine.
pub fn compile(engine: &Engine, bytes: &[u8]) -> Result<Module> {
    let module = Module::from_binary(engine, bytes)?;
    Ok(module)
}

/// Create a new store for plugin execution.
pub fn new_store(engine: &Engine) -> Store<()> {
    Store::new(engine, ())
}
