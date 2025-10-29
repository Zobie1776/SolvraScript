use parking_lot::Mutex;
use std::any::Any;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryHandle(u32);

impl MemoryHandle {
    pub fn index(&self) -> usize {
        self.0 as usize
    }

    pub fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub capacity_bytes: usize,
    pub used_bytes: usize,
    pub allocation_count: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemoryError {
    #[error(
        "memory contract capacity exceeded: requested {requested} bytes, {available} bytes available"
    )]
    CapacityExceeded { requested: usize, available: usize },
    #[error("invalid memory handle {handle}")]
    InvalidHandle { handle: u32 },
    #[error("type mismatch for handle {handle}")]
    TypeMismatch { handle: u32 },
}

#[derive(Debug)]
struct Allocation {
    payload: Arc<dyn Any + Send + Sync>,
    size_hint: usize,
}

#[derive(Debug, Default)]
struct MemoryState {
    slots: Vec<Option<Allocation>>,
    used: usize,
}

/// Deterministic memory arena shared across NovaCore and NovaScript.
#[derive(Debug)]
pub struct MemoryContract {
    capacity: usize,
    state: Mutex<MemoryState>,
}

impl MemoryContract {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            state: Mutex::new(MemoryState::default()),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn allocate_arc(
        &self,
        payload: Arc<dyn Any + Send + Sync>,
        size_hint: usize,
    ) -> Result<MemoryHandle, MemoryError> {
        let size_hint = size_hint.max(std::mem::size_of::<usize>());
        let mut state = self.state.lock();
        if state.used + size_hint > self.capacity {
            return Err(MemoryError::CapacityExceeded {
                requested: size_hint,
                available: self.capacity.saturating_sub(state.used),
            });
        }

        let entry = Allocation { payload, size_hint };
        let index = if let Some((index, slot)) = state
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
        {
            *slot = Some(entry);
            index
        } else {
            state.slots.push(Some(entry));
            state.slots.len() - 1
        };

        state.used += size_hint;
        Ok(MemoryHandle(index as u32))
    }

    pub fn release(&self, handle: MemoryHandle) -> bool {
        let mut state = self.state.lock();
        let index = handle.index();
        if let Some(slot) = state.slots.get_mut(index) {
            if let Some(allocation) = slot.take() {
                state.used = state.used.saturating_sub(allocation.size_hint);
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> MemoryStats {
        let state = self.state.lock();
        MemoryStats {
            capacity_bytes: self.capacity,
            used_bytes: state.used,
            allocation_count: state.slots.iter().filter(|slot| slot.is_some()).count(),
        }
    }

    pub fn downcast_arc<T>(&self, handle: MemoryHandle) -> Result<Arc<T>, MemoryError>
    where
        T: Any + Send + Sync,
    {
        let arc = {
            let state = self.state.lock();
            let Some(Some(allocation)) = state.slots.get(handle.index()).map(|slot| slot.as_ref())
            else {
                return Err(MemoryError::InvalidHandle { handle: handle.0 });
            };
            allocation.payload.clone()
        };

        match Arc::downcast::<T>(arc) {
            Ok(value) => Ok(value),
            Err(_) => Err(MemoryError::TypeMismatch { handle: handle.0 }),
        }
    }
}

impl fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} used / {} capacity ({} allocations)",
            self.used_bytes, self.capacity_bytes, self.allocation_count
        )
    }
}
