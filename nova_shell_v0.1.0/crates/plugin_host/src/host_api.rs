//=============================================
// plugin_host/src/host_api.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Host-side functions exposed to wasm plugins
// Objective: Provide simple logging hook for experimentation
//=============================================

/// Log a message from a plugin.
pub fn log(message: &str) {
    println!("plugin log: {message}");
}
