//! Persistent history tracking for the SolvraCLI shell.

use anyhow::Context;
use directories::ProjectDirs;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const HISTORY_FILE: &str = "history";
const MAX_HISTORY: usize = 1000;

/// Manages command history persistence and de-duplication.
#[derive(Debug)]
pub struct HistoryManager {
    path: PathBuf,
    entries: VecDeque<String>,
}

impl HistoryManager {
    /// Load history from disk creating the data directory when required.
    pub fn load() -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("dev", "Solvra", "solvra")
            .ok_or_else(|| anyhow::anyhow!("unable to determine data directory"))?;
        let data_dir = dirs.data_local_dir();
        fs::create_dir_all(data_dir).context("creating Solvra data directory")?;
        let path = data_dir.join(HISTORY_FILE);
        let mut entries = VecDeque::new();
        if path.exists() {
            let file = OpenOptions::new().read(true).open(&path)?;
            for line in BufReader::new(file)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if entries.back() != Some(&line) {
                    entries.push_back(line);
                }
            }
            if entries.len() > MAX_HISTORY {
                let overflow = entries.len() - MAX_HISTORY;
                for _ in 0..overflow {
                    entries.pop_front();
                }
            }
        }
        Ok(Self { path, entries })
    }

    /// Construct a new empty manager backed by the provided file path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            path,
            entries: VecDeque::new(),
        }
    }

    /// Append an entry to the history while avoiding duplicates.
    pub fn add(&mut self, entry: &str) {
        if entry.trim().is_empty() {
            return;
        }
        if self.entries.back().is_some_and(|last| last == entry) {
            return;
        }
        if self.entries.len() == MAX_HISTORY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry.to_string());
    }

    /// Persist history entries to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .with_context(|| format!("opening history file {}", self.path.display()))?;
        for line in &self.entries {
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }

    /// Borrow history entries.
    pub fn entries(&self) -> impl Iterator<Item = &String> {
        self.entries.iter()
    }

    /// Retrieve the most recent entry if present.
    pub fn last(&self) -> Option<&String> {
        self.entries.back()
    }

    /// Path backing the history storage.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_consecutive_entries() {
        let mut history = HistoryManager::with_path(PathBuf::from("/tmp/history_test"));
        history.add("echo one");
        history.add("echo one");
        history.add("echo two");
        let collected: Vec<_> = history.entries().cloned().collect();
        assert_eq!(collected, vec!["echo one", "echo two"]);
    }
}
