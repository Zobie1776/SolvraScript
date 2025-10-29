use super::scheduler::WorkStealingScheduler;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopState {
    Pending,
    Idle,
}

#[derive(Debug, Clone)]
pub struct TaskHandle {
    completed: Arc<AtomicBool>,
}

impl TaskHandle {
    pub fn is_complete(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }

    pub fn wait(&self) {
        while !self.is_complete() {
            std::thread::yield_now();
        }
    }

    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while !self.is_complete() {
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::yield_now();
        }
        true
    }
}

#[derive(Debug)]
pub struct TaskExecutor {
    scheduler: Arc<WorkStealingScheduler>,
    pending: Arc<AtomicUsize>,
}

impl TaskExecutor {
    pub fn new(worker_count: usize) -> Self {
        let workers = worker_count.max(1);
        Self {
            scheduler: Arc::new(WorkStealingScheduler::new(workers)),
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn spawn<F>(&self, job: F) -> TaskHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let completed = Arc::new(AtomicBool::new(false));
        let flag = completed.clone();
        let pending = self.pending.clone();
        pending.fetch_add(1, Ordering::SeqCst);
        self.scheduler.spawn(move || {
            job();
            pending.fetch_sub(1, Ordering::SeqCst);
            flag.store(true, Ordering::SeqCst);
        });
        TaskHandle { completed }
    }

    pub fn poll_once(&self) -> LoopState {
        if self.pending.load(Ordering::SeqCst) == 0 {
            LoopState::Idle
        } else {
            std::thread::yield_now();
            LoopState::Pending
        }
    }
}

impl Clone for TaskExecutor {
    fn clone(&self) -> Self {
        Self {
            scheduler: self.scheduler.clone(),
            pending: self.pending.clone(),
        }
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self::new(workers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn executes_job() {
        let executor = TaskExecutor::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        let flag = counter.clone();
        let handle = executor.spawn(move || {
            flag.fetch_add(1, Ordering::SeqCst);
        });
        handle.wait();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn timeout_works() {
        let executor = TaskExecutor::new(1);
        let handle = executor.spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
        });
        assert!(!handle.wait_timeout(Duration::from_millis(1)));
        assert!(handle.wait_timeout(Duration::from_millis(100)));
    }

    #[test]
    fn poll_once_reports_state() {
        let executor = TaskExecutor::new(1);
        assert_eq!(executor.poll_once(), LoopState::Idle);

        let handle = executor.spawn(|| std::thread::sleep(Duration::from_millis(10)));
        assert_eq!(executor.poll_once(), LoopState::Pending);
        handle.wait();
        assert_eq!(executor.poll_once(), LoopState::Idle);
    }
}
