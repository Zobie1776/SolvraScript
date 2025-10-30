use crossbeam_deque::{Injector, Steal, Worker};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Work-stealing scheduler backed by crossbeam-deque.
#[derive(Debug)]
pub struct WorkStealingScheduler {
    injector: Arc<Injector<Job>>,
    shutdown: Arc<AtomicBool>,
    handles: Vec<thread::JoinHandle<()>>,
}

impl WorkStealingScheduler {
    pub fn new(worker_count: usize) -> Self {
        let injector = Arc::new(Injector::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::new();
        for _ in 0..worker_count.max(1) {
            workers.push(Worker::new_fifo());
        }
        let stealers: Vec<_> = workers.iter().map(|worker| worker.stealer()).collect();
        let stealers = Arc::new(stealers);
        let mut handles = Vec::new();
        for (index, worker) in workers.into_iter().enumerate() {
            let injector = injector.clone();
            let shutdown = shutdown.clone();
            let stealers = stealers.clone();
            handles.push(thread::spawn(move || {
                worker_loop(index, worker, injector, shutdown, stealers)
            }));
        }
        Self {
            injector,
            shutdown,
            handles,
        }
    }

    pub fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.injector.push(Box::new(job));
    }
}

impl Drop for WorkStealingScheduler {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        for _ in &self.handles {
            self.injector.push(Box::new(|| {}));
        }
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

fn worker_loop(
    index: usize,
    worker: Worker<Job>,
    injector: Arc<Injector<Job>>,
    shutdown: Arc<AtomicBool>,
    stealers: Arc<Vec<crossbeam_deque::Stealer<Job>>>,
) {
    while !shutdown.load(Ordering::SeqCst) {
        if let Some(job) = worker.pop() {
            job();
            continue;
        }
        match injector.steal_batch_and_pop(&worker) {
            Steal::Success(job) => {
                job();
                continue;
            }
            Steal::Retry => continue,
            Steal::Empty => {}
        }

        let mut stolen = None;
        for (i, stealer) in stealers.iter().enumerate() {
            if i == index {
                continue;
            }
            match stealer.steal() {
                Steal::Success(job) => {
                    stolen = Some(job);
                    break;
                }
                Steal::Retry => continue,
                Steal::Empty => continue,
            }
        }

        if let Some(job) = stolen {
            job();
        } else {
            thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn executes_tasks() {
        let scheduler = WorkStealingScheduler::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..8 {
            let counter = counter.clone();
            scheduler.spawn(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
        // Give workers some time to run.
        thread::sleep(Duration::from_millis(50));
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }
}
