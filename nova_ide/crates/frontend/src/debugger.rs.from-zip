use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DebugTargetKind {
    NovaScript,
    NovaCore,
    Rust,
    C,
    Cpp,
    CSharp,
    Java,
    JavaScript,
    TypeScript,
    Html,
    Python,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugTarget {
    pub kind: DebugTargetKind,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub file: PathBuf,
    pub line: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StackFrame {
    pub function: String,
    pub file: PathBuf,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VariableEntry {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugStateSnapshot {
    pub stack: Vec<StackFrame>,
    pub locals: Vec<VariableEntry>,
    pub watches: Vec<VariableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugEvent {
    Paused(DebugStateSnapshot),
    Continued,
    Terminated,
    Output(String),
}

#[derive(Debug)]
pub struct DebugSession {
    pub id: u64,
    pub target: DebugTarget,
    pub breakpoints: Vec<Breakpoint>,
    pub last_state: Option<DebugStateSnapshot>,
}

#[derive(Debug, Default)]
pub struct DebuggerHub {
    sessions: HashMap<u64, DebugSession>,
    next_session_id: u64,
}

impl DebuggerHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach(&mut self, target: DebugTarget) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        let session = DebugSession {
            id,
            target,
            breakpoints: Vec::new(),
            last_state: None,
        };
        self.sessions.insert(id, session);
        id
    }

    pub fn detach(&mut self, session_id: u64) {
        self.sessions.remove(&session_id);
    }

    pub fn toggle_breakpoint(&mut self, session_id: u64, file: PathBuf, line: u32) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            if let Some(bp) = session
                .breakpoints
                .iter_mut()
                .find(|bp| bp.file == file && bp.line == line)
            {
                bp.enabled = !bp.enabled;
            } else {
                session.breakpoints.push(Breakpoint {
                    file,
                    line,
                    enabled: true,
                });
            }
        }
    }

    pub fn set_breakpoint_state(
        &mut self,
        session_id: u64,
        file: &PathBuf,
        line: u32,
        enabled: bool,
    ) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            if let Some(bp) = session
                .breakpoints
                .iter_mut()
                .find(|bp| &bp.file == file && bp.line == line)
            {
                bp.enabled = enabled;
            }
        }
    }

    pub fn sessions(&self) -> impl Iterator<Item = &DebugSession> {
        self.sessions.values()
    }

    pub fn session_mut(&mut self, id: u64) -> Option<&mut DebugSession> {
        self.sessions.get_mut(&id)
    }

    pub fn update_state(&mut self, session_id: u64, snapshot: DebugStateSnapshot) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.last_state = Some(snapshot);
        }
    }
}
