use thiserror::Error;

/// Unified error type for SolvraIDE core services.
#[derive(Debug, Error)]
pub enum SolvraIdeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("workspace error: {0}")]
    Workspace(String),
    #[error("task failed: {0}")]
    Task(String),
    #[error("lsp error: {0}")]
    Language(String),
    #[error("debugger error: {0}")]
    Debugger(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SolvraIdeError {
    pub fn workspace<T: Into<String>>(msg: T) -> Self {
        Self::Workspace(msg.into())
    }

    pub fn task<T: Into<String>>(msg: T) -> Self {
        Self::Task(msg.into())
    }

    pub fn language<T: Into<String>>(msg: T) -> Self {
        Self::Language(msg.into())
    }

    pub fn debugger<T: Into<String>>(msg: T) -> Self {
        Self::Debugger(msg.into())
    }
}
