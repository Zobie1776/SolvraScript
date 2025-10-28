//=============================================
// nova_compositor/src/render_gl.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Placeholder glow renderer wiring
// Objective: Reference smithay's Glow renderer without spinning a real EGL context
//=============================================

use smithay::backend::renderer::gles::GlesRenderer;
use std::marker::PhantomData;

//=============================================
// SECTION: Renderer Skeleton
//=============================================

/// Simple descriptor for a glow-powered render pass.
#[derive(Debug, Clone)]
pub struct GlowRenderer {
    /// Marker to keep the smithay renderer type alive.
    _marker: PhantomData<GlesRenderer>,
}

impl GlowRenderer {
    /// Build a renderer stub.
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
