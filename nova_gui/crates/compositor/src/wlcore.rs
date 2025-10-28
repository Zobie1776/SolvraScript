//=============================================
// nova_compositor/src/wlcore.rs
//=============================================
// Author: Nova GUI Team
// License: MIT
// Goal: Host smithay backend primitives for the compositor
// Objective: Provide helpers to bootstrap the Wayland display, seats, and event loop
//=============================================

use crate::render_gl::GlowRenderer;
use anyhow::Result;
use calloop::{EventLoop, LoopSignal};
use smithay::backend::egl::EglGlesBackend;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::reexports::wayland_server::Display;

//=============================================
// SECTION: Backend State
//=============================================

/// State stored alongside the smithay display.
#[derive(Debug)]
pub struct WlState {
    /// Glow renderer instance.
    pub renderer: GlowRenderer,
}

/// Wrapper combining smithay display and calloop event loop.
#[derive(Debug)]
pub struct WlBackend {
    /// Wayland display handle.
    pub display: Display<WlState>,
    /// Calloop event loop for backend.
    pub event_loop: EventLoop<WlState>,
    /// Loop signal (used to quit event loop).
    pub loop_signal: LoopSignal,
    /// State shared with smithay callbacks.
    pub state: WlState,
}

//=============================================
// SECTION: Backend Factory
//=============================================

/// Create a backend featuring an EGL/Glow renderer and calloop loop.
pub fn create_backend() -> Result<WlBackend> {
    let mut event_loop: EventLoop<WlState> = EventLoop::try_new()?;
    let loop_signal = event_loop.get_signal();
    let display: Display<WlState> = Display::new()?;

    // Initialise minimal EGL backend and renderer (stub).
    let egl_backend = EglGlesBackend::new(None)?;
    let renderer = GlesRenderer::new(egl_backend)?;
    let state = WlState {
        renderer: GlowRenderer::new(renderer),
    };

    Ok(WlBackend {
        display,
        event_loop,
        loop_signal,
        state,
    })
}
