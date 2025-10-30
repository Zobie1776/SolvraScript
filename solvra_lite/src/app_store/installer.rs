#![allow(dead_code)]

use std::collections::VecDeque;
use std::time::Duration;

use tokio::time::sleep;
use tracing::info;

use super::catalog::AppMetadata;

#[derive(Debug, Clone)]
pub struct InstallJob {
    pub app: AppMetadata,
    pub progress: f32,
    pub state: InstallState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallState {
    Pending,
    Installing,
    Completed,
}

#[derive(Debug, Default)]
pub struct InstallerQueue {
    queue: VecDeque<InstallJob>,
}

impl InstallerQueue {
    pub fn enqueue(&mut self, app: AppMetadata) {
        self.queue.push_back(InstallJob {
            app,
            progress: 0.0,
            state: InstallState::Pending,
        });
    }

    pub fn jobs(&self) -> impl Iterator<Item = &InstallJob> {
        self.queue.iter()
    }

    pub async fn process(&mut self) {
        if let Some(mut job) = self.queue.pop_front() {
            info!("installing {}", job.app.id);
            job.state = InstallState::Installing;
            for step in 1..=5 {
                sleep(Duration::from_millis(120)).await;
                job.progress = step as f32 / 5.0;
            }
            job.state = InstallState::Completed;
            self.queue.push_back(job);
        }
    }

    pub fn process_sync(&mut self) {
        if let Some(mut job) = self.queue.pop_front() {
            job.state = InstallState::Completed;
            job.progress = 1.0;
            self.queue.push_back(job);
        }
    }
}
