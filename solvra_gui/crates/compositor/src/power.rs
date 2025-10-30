//=============================================
// solvra_compositor/src/power.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Power management stubs for the compositor
// Objective: Track idle timeouts and surface hooks for suspend logic
//=============================================

use std::time::{Duration, Instant};

/// Tracks idle periods for the compositor process.
#[derive(Debug)]
pub struct IdleTracker {
    /// Timestamp of the last activity event.
    last_event: Instant,
    /// Threshold before requesting suspend.
    timeout: Duration,
}

impl IdleTracker {
    /// Create a new tracker with the provided timeout (seconds).
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            last_event: Instant::now(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Record user activity.
    pub fn ping(&mut self) {
        self.last_event = Instant::now();
    }

    /// Determine whether the timeout elapsed.
    pub fn is_idle(&self) -> bool {
        self.last_event.elapsed() >= self.timeout
    }
}
