use super::RuntimeError;
use crate::modules::ModuleError;
use crate::parser::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Syntax,
    ModuleResolution,
    TypeMismatch,
    InvalidOperation,
    RuntimePanic,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Syntax => "E001",
            ErrorCode::ModuleResolution => "E002",
            ErrorCode::TypeMismatch => "E003",
            ErrorCode::InvalidOperation => "E004",
            ErrorCode::RuntimePanic => "E005",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptError {
    pub code: ErrorCode,
    pub message: String,
}

impl ScriptError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }
}

impl From<ParseError> for ScriptError {
    fn from(value: ParseError) -> Self {
        ScriptError::new(ErrorCode::Syntax, value.to_string())
    }
}

impl From<ModuleError> for ScriptError {
    fn from(value: ModuleError) -> Self {
        ScriptError::new(ErrorCode::ModuleResolution, value.to_string())
    }
}

impl From<RuntimeError> for ScriptError {
    fn from(value: RuntimeError) -> Self {
        ScriptError::new(runtime_error_code(&value), value.to_string())
    }
}

pub fn runtime_error_code(error: &RuntimeError) -> ErrorCode {
    match error {
        RuntimeError::TypeError(_) => ErrorCode::TypeMismatch,
        RuntimeError::ArgumentError(_)
        | RuntimeError::IndexError(_)
        | RuntimeError::DivisionByZero
        | RuntimeError::VariableNotFound(_)
        | RuntimeError::NotImplemented(_) => ErrorCode::InvalidOperation,
        RuntimeError::StackOverflow
        | RuntimeError::IoError(_)
        | RuntimeError::NetworkError(_)
        | RuntimeError::Exit(_)
        | RuntimeError::Custom(_) => ErrorCode::RuntimePanic,
        RuntimeError::Return(_) | RuntimeError::Break | RuntimeError::Continue => {
            ErrorCode::RuntimePanic
        }
    }
}
