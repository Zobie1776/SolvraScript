//! System abstraction layer for NovaCore.

pub mod fs;
pub mod net;

#[cfg(feature = "gpu")]
pub mod gpu;
