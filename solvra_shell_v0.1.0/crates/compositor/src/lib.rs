//=============================================
// solvra_compositor/src/lib.rs
//=============================================
// Author: Solvra Shell Team
// License: MIT
// Goal: Library entry points shared by the compositor binary
// Objective: Wire smithay primitives with lightweight state managers
//=============================================

pub mod config;
pub mod inputs;
pub mod ipc;
pub mod power;
pub mod profile;
pub mod render_gl;
pub mod wlcore;
pub mod wm;

use crate::config::{load_from_file, CompositorConfig};
use crate::ipc::{IpcRouter, RpcResponse};
use crate::power::IdleTracker;
use crate::profile::Profile;
use crate::wlcore::create_backend;
use crate::wm::WorkspaceManager;
use anyhow::Result;
use calloop::LoopSignal;
use std::path::Path;
use tracing::{info, instrument};

//=============================================
// SECTION: Compositor Application
//=============================================

/// Shared compositor state used by the binary entry point.
pub struct Compositor {
    /// Wayland backend state.
    backend: wlcore::WlBackend,
    /// Runtime configuration snapshot.
    config: CompositorConfig,
    /// Workspace manager façade.
    wm: WorkspaceManager,
    /// IPC router for JSON-RPC commands.
    ipc: IpcRouter,
    /// Idle tracker gating power actions.
    idle: IdleTracker,
}

impl Compositor {
    /// Build a new compositor using the optional configuration path.
    #[instrument(name = "compositor_build")]
    pub fn build_with_config(path: Option<&Path>) -> Result<Self> {
        utils::logging::init("compositor");
        let config = match path {
            Some(path) => load_from_file(path)?,
            None => CompositorConfig::default(),
        };
        let backend = create_backend()?;
        let wm = WorkspaceManager::new();
        let ipc = IpcRouter::new();
        let idle = IdleTracker::new(300);
        info!(profile = %config.profile, "compositor constructed");
        Ok(Self {
            backend,
            config,
            wm,
            ipc,
            idle,
        })
    }

    /// Build a compositor using defaults.
    pub fn build() -> Result<Self> {
        Self::build_with_config(None)
    }

    /// Poll the compositor once; placeholder for the event loop wiring.
    #[instrument(name = "compositor_tick", skip(self))]
    pub fn tick(&mut self) {
        self.idle.ping();
        if self.idle.is_idle() {
            info!("system idle threshold reached");
        }
        let response: RpcResponse = self.ipc.handle_default();
        info!(?response, "ipc default response");
        info!(workspaces = self.wm.len(), "tick complete");
    }

    /// Active profile derived from configuration.
    pub fn profile(&self) -> Profile {
        self.config.profile()
    }

    /// Event loop stop signal.
    pub fn loop_signal(&self) -> LoopSignal {
        self.backend.loop_signal.clone()
    }
}

//=============================================
// SECTION: Utility Helpers
//=============================================

/// Initialize tracing subscribers for development builds.
pub fn init_tracing() {
    utils::logging::init("compositor-main");
}
