//=============================================
// nova_compositor/src/wlcore.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Host smithay backend primitives for the compositor
// Objective: Provide helpers to bootstrap the Wayland display and seats
//=============================================

use crate::render_gl::GlowRenderer;
use anyhow::Result;
use calloop::{EventLoop, LoopSignal};
use smithay::reexports::wayland_server::Display;

//=============================================
// SECTION: Backend State
//=============================================

/// State stored alongside the smithay display.
#[derive(Debug, Default)]
pub struct WlState {
    /// Placeholder renderer handle.
    pub renderer: GlowRenderer,
}

/// Wrapper combining the smithay display and calloop event loop.
pub struct WlBackend {
    /// Wayland display handle.
    pub display: Display<WlState>,
    /// Calloop event loop.
    pub event_loop: EventLoop<WlState>,
    /// Signal used to stop the event loop.
    pub loop_signal: LoopSignal,
    /// Global state shared with smithay callbacks.
    pub state: WlState,
}

//=============================================
// SECTION: Display Factory
//=============================================

/// Create an empty smithay display with the compositor backend state.
pub fn create_backend() -> Result<WlBackend> {
    let mut event_loop: EventLoop<WlState> = EventLoop::try_new()?;
    let loop_signal = event_loop.get_signal();
    let display: Display<WlState> = Display::new()?;
    let state = WlState {
        renderer: GlowRenderer::new(),
    };
    Ok(WlBackend {
        display,
        event_loop,
        loop_signal,
        state,
    })
}
