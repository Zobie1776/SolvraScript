use std::collections::VecDeque;

use crate::Value;

/// Handle referencing an object stored inside the garbage collected heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcHandle(u32);

impl GcHandle {
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Reference to a garbage collected object that can be cloned freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcRef {
    handle: GcHandle,
}

impl GcRef {
    pub fn new(handle: GcHandle) -> Self {
        Self { handle }
    }

    pub fn handle(&self) -> GcHandle {
        self.handle
    }
}

/// Objects managed by the collector.
#[derive(Debug)]
pub enum GcObject {
    List(Vec<Value>),
    Native(Box<dyn std::any::Any + Send>),
}

impl GcObject {
    pub fn trace(&self, visitor: &mut dyn FnMut(GcRef)) {
        match self {
            GcObject::List(items) => {
                for item in items {
                    item.trace(visitor);
                }
            }
            GcObject::Native(_) => {}
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            GcObject::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            GcObject::List(list) => Some(list),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Entry {
    value: Option<GcObject>,
    marked: bool,
}

impl Entry {
    fn new(value: GcObject) -> Self {
        Self {
            value: Some(value),
            marked: false,
        }
    }
}

/// Mark and sweep collector used by the virtual machine.
#[derive(Debug)]
pub struct Collector {
    objects: Vec<Entry>,
    threshold: usize,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            threshold: 64,
        }
    }

    pub fn allocate(&mut self, value: GcObject) -> GcRef {
        let index = self.objects.len();
        self.objects.push(Entry::new(value));
        GcRef::new(GcHandle(index as u32))
    }

    pub fn get(&self, reference: GcRef) -> Option<&GcObject> {
        self.objects
            .get(reference.handle.index())
            .and_then(|entry| entry.value.as_ref())
    }

    pub fn get_mut(&mut self, reference: GcRef) -> Option<&mut GcObject> {
        self.objects
            .get_mut(reference.handle.index())
            .and_then(|entry| entry.value.as_mut())
    }

    pub fn collect(&mut self, roots: impl IntoIterator<Item = GcRef>) {
        self.mark(roots);
        self.sweep();
        if self.objects.len() > self.threshold {
            self.threshold = self.objects.len() * 2;
        }
    }

    pub fn allocated(&self) -> usize {
        self.objects
            .iter()
            .filter(|entry| entry.value.is_some())
            .count()
    }

    fn mark(&mut self, roots: impl IntoIterator<Item = GcRef>) {
        let mut worklist: VecDeque<GcRef> = roots.into_iter().collect();
        while let Some(reference) = worklist.pop_front() {
            if let Some(entry) = self.objects.get_mut(reference.handle.index()) {
                if entry.marked {
                    continue;
                }
                entry.marked = true;
                if let Some(value) = entry.value.as_ref() {
                    let mut enqueue = |child: GcRef| worklist.push_back(child);
                    value.trace(&mut enqueue);
                }
            }
        }
    }

    fn sweep(&mut self) {
        for entry in &mut self.objects {
            if entry.marked {
                entry.marked = false;
            } else {
                entry.value = None;
            }
        }
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}
