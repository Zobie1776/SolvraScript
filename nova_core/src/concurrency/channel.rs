use crossbeam_queue::SegQueue;
use std::sync::Arc;

/// Lock-free multi-producer multi-consumer queue built on top of `SegQueue`.
#[derive(Debug, Clone)]
pub struct MpmcQueue<T> {
    inner: Arc<SegQueue<T>>,
}

impl<T> MpmcQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SegQueue::new()),
        }
    }

    pub fn push(&self, value: T) {
        self.inner.push(value);
    }

    pub fn pop(&self) -> Option<T> {
        self.inner.pop()
    }
}

impl<T> Default for MpmcQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn queue_supports_multiple_threads() {
        let queue = MpmcQueue::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..4 {
            let queue = queue.clone();
            let counter = counter.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    queue.push(1usize);
                }
                while let Some(value) = queue.pop() {
                    counter.fetch_add(value, Ordering::SeqCst);
                }
            }));
        }
        for handle in handles {
            handle.join().expect("thread");
        }
        assert!(counter.load(Ordering::SeqCst) > 0);
    }
}
