use crate::error::SolvraIdeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DebuggerEvent {
    BreakpointHit { file: PathBuf, line: u32 },
    StepCompleted { file: PathBuf, line: u32 },
    Output { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DebuggerState {
    pub breakpoints: HashMap<PathBuf, Vec<u32>>,
    pub current_file: Option<PathBuf>,
    pub current_line: Option<u32>,
}

#[derive(Debug, Default)]
pub struct DebugSession {
    state: DebuggerState,
}

impl DebugSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_breakpoint(&mut self, file: PathBuf, line: u32) {
        let entry = self.state.breakpoints.entry(file).or_default();
        if let Some(index) = entry.iter().position(|value| *value == line) {
            entry.remove(index);
        } else {
            entry.push(line);
            entry.sort_unstable();
        }
    }

    pub fn has_breakpoint(&self, file: impl AsRef<Path>, line: u32) -> bool {
        self.state
            .breakpoints
            .get(file.as_ref())
            .map(|lines| lines.contains(&line))
            .unwrap_or(false)
    }

    pub fn step(&mut self, file: PathBuf, line: u32) -> Result<DebuggerEvent, SolvraIdeError> {
        self.state.current_file = Some(file.clone());
        self.state.current_line = Some(line);
        Ok(DebuggerEvent::StepCompleted { file, line })
    }

    pub fn hit_breakpoint(
        &mut self,
        file: PathBuf,
        line: u32,
    ) -> Result<DebuggerEvent, SolvraIdeError> {
        if !self.has_breakpoint(&file, line) {
            return Err(SolvraIdeError::debugger("breakpoint missing"));
        }
        self.state.current_file = Some(file.clone());
        self.state.current_line = Some(line);
        Ok(DebuggerEvent::BreakpointHit { file, line })
    }

    pub fn emit_output(&self, message: impl Into<String>) -> DebuggerEvent {
        DebuggerEvent::Output {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_breakpoint() {
        let mut session = DebugSession::new();
        let file = PathBuf::from("main.svs");
        session.toggle_breakpoint(file.clone(), 5);
        assert!(session.has_breakpoint(&file, 5));
        session.toggle_breakpoint(file.clone(), 5);
        assert!(!session.has_breakpoint(&file, 5));
    }

    #[test]
    fn reports_breakpoint_hit() {
        let mut session = DebugSession::new();
        let file = PathBuf::from("main.svs");
        session.toggle_breakpoint(file.clone(), 2);
        match session.hit_breakpoint(file.clone(), 2).unwrap() {
            DebuggerEvent::BreakpointHit { line, .. } => assert_eq!(line, 2),
            _ => panic!("unexpected event"),
        }
    }
}
