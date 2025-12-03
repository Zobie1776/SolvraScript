//=====================================================
// File: vm/profiling.rs
//=====================================================
// Author: Codex Agent
// License: Duality Public License (DPL v1.0)
// Goal: Establish lightweight profiling scaffolding for future JIT/AOT work
// Objective: Track execution timing and hot function counts without impacting
//            the existing VM semantics
//=====================================================

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Captures coarse runtime timing data for a single VM execution.
#[derive(Clone, Debug)]
pub struct RuntimeProfile {
    start: Option<Instant>,
    finish: Option<Instant>,
    pub total_duration: Option<Duration>,
    pub hot_functions: HotFunctionTable,
}

impl RuntimeProfile {
    pub fn new() -> Self {
        Self {
            start: None,
            finish: None,
            total_duration: None,
            hot_functions: HotFunctionTable::default(),
        }
    }

    pub fn begin(&mut self) {
        self.start = Some(Instant::now());
        self.total_duration = None;
        self.finish = None;
    }

    pub fn end(&mut self) {
        if let Some(started) = self.start.take() {
            let duration = started.elapsed();
            self.total_duration = Some(duration);
            self.finish = Some(Instant::now());
        }
    }

    pub fn record_function(&mut self, name: &str) -> u64 {
        self.hot_functions.record_call(name)
    }
}

/// Basic frequency table for identifying hot functions during execution.
#[derive(Clone, Debug)]
pub struct HotFunctionTable {
    pub threshold: u64,
    hits: HashMap<String, u64>,
}

impl HotFunctionTable {
    pub const DEFAULT_HOT_THRESHOLD: u64 = 50;

    pub fn new() -> Self {
        Self {
            threshold: Self::DEFAULT_HOT_THRESHOLD,
            hits: HashMap::new(),
        }
    }

    pub fn record_call(&mut self, name: &str) -> u64 {
        let counter = self.hits.entry(name.to_string()).or_insert(0);
        *counter += 1;
        *counter
    }

    pub fn is_hot(&self, name: &str) -> bool {
        let threshold = self.threshold.max(1);
        self.hits.get(name).copied().unwrap_or(0) >= threshold
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        self.hits.clone()
    }
}

impl Default for HotFunctionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RuntimeProfile {
    fn default() -> Self {
        Self::new()
    }
}

//=====================================================
// End of file
//=====================================================
