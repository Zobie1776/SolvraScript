//=============================================
// nova_compositor/src/power.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Power management stubs for the compositor
// Objective: Track idle timeouts that will later control suspend hooks
//=============================================

use std::time::{Duration, Instant};

//=============================================
// SECTION: Idle Tracker
//=============================================

/// Tracks idle periods for the compositor process.
#[derive(Debug)]
pub struct IdleTracker {
    /// Timestamp of the last input event.
    last_event: Instant,
    /// Threshold before requesting suspend.
    timeout: Duration,
}

impl IdleTracker {
    /// Create a new tracker.
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            last_event: Instant::now(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Record activity.
    pub fn ping(&mut self) {
        self.last_event = Instant::now();
    }

    /// Determine whether the timeout elapsed.
    pub fn is_idle(&self) -> bool {
        self.last_event.elapsed() >= self.timeout
    }
}
