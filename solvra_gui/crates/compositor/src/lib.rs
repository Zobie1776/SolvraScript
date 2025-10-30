//=============================================
// solvra_compositor/src/lib.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Library entry points shared by the Solvra compositor binary
// Objective: Bootstrap smithay backend, IPC router, and window manager façade
//=============================================

pub mod config;
pub mod inputs;
pub mod ipc;
pub mod power;
pub mod profile;
pub mod render_gl;
pub mod wlcore;
pub mod wm;

use crate::config::CompositorConfig;
use crate::ipc::{IpcRouter, RpcResponse};
use crate::power::IdleTracker;
use crate::profile::Profile;
use crate::wlcore::WlBackend;
use crate::wm::WorkspaceManager;
use anyhow::Result;
use calloop::LoopSignal;
use std::path::Path;
use tracing::{info, instrument};

//=============================================
// SECTION: Compositor State
//=============================================

/// State container wiring smithay, window manager, and IPC layers.
pub struct Compositor {
    /// Wayland backend (display + calloop loop).
    backend: WlBackend,
    /// Loaded configuration snapshot.
    config: CompositorConfig,
    /// IPC router dispatching JSON-RPC methods.
    ipc: IpcRouter,
    /// Workspace manager responsible for tiling/focus bookkeeping.
    wm: WorkspaceManager,
    /// Idle tracker for power-management hooks.
    idle: IdleTracker,
}

impl Compositor {
    /// Build a compositor using optional configuration file.
    #[instrument(name = "compositor_build")]
    pub fn build_with_config(path: Option<&Path>) -> Result<Self> {
        utils::logging::init("compositor");
        let config = if let Some(path) = path {
            config::load_from_file(path)?
        } else {
            CompositorConfig::default()
        };
        let backend = wlcore::create_backend()?;
        let wm = WorkspaceManager::new();
        let ipc = IpcRouter::new(&config.socket_path);
        let idle = IdleTracker::new(config.power.idle_timeout_secs);
        info!(profile = %config.profile, theme = %config.theme, "compositor initialised");
        Ok(Self {
            backend,
            config,
            ipc,
            wm,
            idle,
        })
    }

    /// Build a compositor using defaults.
    pub fn build() -> Result<Self> {
        Self::build_with_config(None)
    }

    /// Single-tick handler; the real compositor will install smithay event sources.
    #[instrument(name = "compositor_tick", skip(self))]
    pub fn tick(&mut self) {
        let response: RpcResponse = self.ipc.handle_default();
        info!(?response, workspaces = self.wm.len(), "tick");
        self.idle.ping();
        if self.idle.is_idle() {
            info!("idle threshold reached");
        }
    }

    /// Access the active profile enum.
    pub fn profile(&self) -> Profile {
        self.config.profile()
    }

    /// Borrow the loop signal to stop calloop.
    pub fn loop_signal(&self) -> LoopSignal {
        self.backend.loop_signal.clone()
    }

    /// Expose mutable workspace manager (used by IPC handlers).
    pub fn workspaces_mut(&mut self) -> &mut WorkspaceManager {
        &mut self.wm
    }
}

//=============================================
// SECTION: Feature Helpers
//=============================================

/// Enable plugin subsystem when feature is active.
#[cfg(feature = "plugins")]
pub mod plugins {
    use plugin_host::load_plugin;
    use plugin_sdk::PluginDescriptor;
    use wasmtime::Engine;

    /// Compile a plugin module from raw bytes.
    pub fn compile_plugin(bytes: &[u8]) -> anyhow::Result<PluginDescriptor> {
        let engine = Engine::default();
        load_plugin(&engine, bytes)
    }
}

/// Offline helpers stub out heavy dependencies for no-network builds.
#[cfg(feature = "offline")]
pub mod offline {
    /// Return a canned JSON response for unit tests without smithay.
    pub fn mock_response() -> String {
        "{\"jsonrpc\":\"2.0\",\"result\":\"offline\",\"id\":0}".into()
    }
}

//=============================================
// SECTION: Tracing Bootstrap
//=============================================

/// Initialize tracing subscribers for the compositor binary.
pub fn init_tracing() {
    utils::logging::init("compositor-main");
}
