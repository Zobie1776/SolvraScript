//! Memory management utilities used by SolvraRuntime.

pub mod arena;
pub mod contract;
pub mod gc;

pub use contract::{MemoryContract, MemoryError, MemoryHandle, MemoryStats};
