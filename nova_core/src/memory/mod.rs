//! Memory management utilities used by NovaRuntime.

pub mod arena;
pub mod contract;
pub mod gc;

pub use contract::{MemoryContract, MemoryError, MemoryHandle, MemoryStats};
