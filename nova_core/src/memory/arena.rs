use std::marker::PhantomData;

/// Handle returned by the arena allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaHandle<T> {
    index: u32,
    _marker: PhantomData<T>,
}

impl<T> ArenaHandle<T> {
    fn new(index: usize) -> Self {
        Self {
            index: index as u32,
            _marker: PhantomData,
        }
    }

    pub fn index(&self) -> usize {
        self.index as usize
    }
}

/// A simple bump allocation arena.
#[derive(Debug, Default)]
pub struct Arena<T> {
    entries: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn allocate(&mut self, value: T) -> ArenaHandle<T> {
        let handle = ArenaHandle::new(self.entries.len());
        self.entries.push(value);
        handle
    }

    pub fn get(&self, handle: ArenaHandle<T>) -> Option<&T> {
        self.entries.get(handle.index())
    }

    pub fn get_mut(&mut self, handle: ArenaHandle<T>) -> Option<&mut T> {
        self.entries.get_mut(handle.index())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_roundtrip() {
        let mut arena = Arena::new();
        let handle_a = arena.allocate("hello");
        let handle_b = arena.allocate("world");
        assert_eq!(arena.get(handle_a), Some(&"hello"));
        assert_eq!(arena.get(handle_b), Some(&"world"));
    }
}
