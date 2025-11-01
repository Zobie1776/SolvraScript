use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct AsyncControl {
    state: Arc<Mutex<HashMap<u64, TaskState>>>,
}

struct TaskState {
    cancelled: bool,
    deadline: Option<Instant>,
}

impl AsyncControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, task_id: u64) {
        let mut state = self.state.lock().expect("async control state poisoned");
        state.insert(
            task_id,
            TaskState {
                cancelled: false,
                deadline: None,
            },
        );
    }

    pub fn cancel(&self, task_id: u64) -> bool {
        let mut state = self.state.lock().expect("async control state poisoned");
        if let Some(entry) = state.get_mut(&task_id) {
            entry.cancelled = true;
            true
        } else {
            false
        }
    }

    pub fn set_deadline(&self, task_id: u64, duration: Duration) -> bool {
        let mut state = self.state.lock().expect("async control state poisoned");
        if let Some(entry) = state.get_mut(&task_id) {
            entry.deadline = Some(Instant::now() + duration);
            true
        } else {
            false
        }
    }

    pub fn clear_deadline(&self, task_id: u64) -> bool {
        let mut state = self.state.lock().expect("async control state poisoned");
        if let Some(entry) = state.get_mut(&task_id) {
            entry.deadline = None;
            true
        } else {
            false
        }
    }

    pub fn deadline(&self, task_id: u64) -> Option<Instant> {
        self.state
            .lock()
            .expect("async control state poisoned")
            .get(&task_id)
            .and_then(|entry| entry.deadline)
    }

    pub fn is_cancelled(&self, task_id: u64) -> bool {
        self.state
            .lock()
            .expect("async control state poisoned")
            .get(&task_id)
            .map(|entry| entry.cancelled)
            .unwrap_or(false)
    }

    pub fn complete(&self, task_id: u64) {
        self.state
            .lock()
            .expect("async control state poisoned")
            .remove(&task_id);
    }
}
