//=============================================
// plugin_host/src/guest_api.rs
//=============================================
// Author: Solvra Shell Team
// License: MIT
// Goal: Guest-side bindings expected by wasm plugins
// Objective: Outline traits that native hosts must satisfy
//=============================================

/// Trait implemented by guest modules to receive lifecycle events.
pub trait PluginGuest {
    /// Invoked when the host loads the plugin.
    fn on_load(&mut self);
    /// Invoked before the host unloads the plugin.
    fn on_unload(&mut self);
}
