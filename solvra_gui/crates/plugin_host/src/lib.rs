//=============================================
// plugin_host/src/lib.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Plugin host orchestrator for Solvra GUI
// Objective: Manage module lifecycle and expose host/guest APIs
//=============================================

pub mod guest_api;
pub mod host_api;

use anyhow::Result;
use plugin_sdk::{compile, new_store, PluginDescriptor};
use wasmtime::Engine;

/// Load a plugin descriptor and compile the module.
pub fn load_plugin(engine: &Engine, bytes: &[u8]) -> Result<PluginDescriptor> {
    let module = compile(engine, bytes)?;
    let store = new_store(engine);
    let _imports = module.imports().len();
    let _exports = module.exports().len();
    drop(store);
    Ok(PluginDescriptor {
        name: "demo".into(),
        version: "0.0.1".into(),
    })
}
