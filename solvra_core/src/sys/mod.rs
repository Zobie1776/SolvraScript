//! System abstraction layer for SolvraCore.

pub mod drivers;
pub mod fs;
pub mod hal;
pub mod net;

#[cfg(feature = "gpu")]
pub mod gpu;
