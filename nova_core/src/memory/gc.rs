use std::collections::HashSet;

/// Handle used by the collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcHandle(u32);

impl GcHandle {
    fn new(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Minimal tri-colour mark sweep collector.
#[derive(Debug, Default)]
pub struct Collector<T> {
    objects: Vec<Option<T>>,
}

impl<T> Collector<T> {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn allocate(&mut self, value: T) -> GcHandle {
        let handle = GcHandle::new(self.objects.len());
        self.objects.push(Some(value));
        handle
    }

    pub fn get(&self, handle: GcHandle) -> Option<&T> {
        self.objects
            .get(handle.index())
            .and_then(|slot| slot.as_ref())
    }

    pub fn collect<I>(&mut self, roots: I)
    where
        I: IntoIterator<Item = GcHandle>,
    {
        let mut marked = HashSet::new();
        for root in roots {
            marked.insert(root.index());
        }
        for (index, slot) in self.objects.iter_mut().enumerate() {
            if !marked.contains(&index) {
                *slot = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_sweeps_unmarked_objects() {
        let mut collector = Collector::new();
        let keep = collector.allocate("keep");
        let drop = collector.allocate("drop");
        collector.collect([keep]);
        assert!(collector.get(drop).is_none());
        assert_eq!(collector.get(keep), Some(&"keep"));
    }
}
