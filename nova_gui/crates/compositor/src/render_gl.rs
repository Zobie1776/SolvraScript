//=============================================
// nova_compositor/src/render_gl.rs
//=============================================
// Author: Nova GUI Team
// License: MIT
// Goal: Placeholder Glow renderer wiring
// Objective: Reference smithay's Glow renderer and provide hooks for passes
//=============================================

use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Logical, Rectangle};

//=============================================
// SECTION: Renderer Skeleton
//=============================================

/// Definition of a glow render pass (clear + border draw).
#[derive(Debug, Clone)]
pub struct GlowPass {
    /// Region to clear before presenting.
    pub target: Rectangle<i32, Logical>,
}

impl GlowPass {
    /// Build a pass that covers the entire surface.
    pub fn fullscreen(width: i32, height: i32) -> Self {
        Self {
            target: Rectangle::from_loc_and_size((0, 0), (width, height)),
        }
    }
}

/// Convenience wrapper for smithay's GLES renderer.
#[derive(Debug)]
pub struct GlowRenderer {
    renderer: GlesRenderer,
}

impl GlowRenderer {
    /// Create an instance from smithay renderer.
    pub fn new(renderer: GlesRenderer) -> Self {
        Self { renderer }
    }

    /// Access the underlying renderer (e.g., for draw calls).
    pub fn inner(&self) -> &GlesRenderer {
        &self.renderer
    }
}
