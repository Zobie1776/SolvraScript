//=============================================
// nova_compositor/src/inputs.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Capture keyboard and pointer input descriptions
// Objective: Offer wrappers around smithay input types
//=============================================

use smithay::input::keyboard::ModifiersState;
use smithay::input::pointer::MotionEvent;

//=============================================
// SECTION: Keyboard Event
//=============================================

/// Simplified keyboard event used during scaffolding.
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    /// Raw keycode from the backend.
    pub keycode: u32,
    /// Reported modifier state.
    pub modifiers: ModifiersState,
}

impl KeyboardEvent {
    /// Create a new keyboard event.
    pub fn new(keycode: u32, modifiers: ModifiersState) -> Self {
        Self { keycode, modifiers }
    }
}

//=============================================
// SECTION: Pointer Event
//=============================================

/// Wrapper for smithay pointer motion events.
#[derive(Debug, Clone)]
pub struct PointerMotion {
    /// Embedded smithay event.
    pub event: MotionEvent,
}

impl PointerMotion {
    /// Construct from a smithay motion event.
    pub fn new(event: MotionEvent) -> Self {
        Self { event }
    }
}
