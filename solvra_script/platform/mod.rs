//=============================================
// solvra_script/platform/mod.rs
//=============================================
// Author: SolvraOS Contributors
// License: MIT (see LICENSE)
// Goal: Cross-platform abstraction layer for SolvraScript
// Objective: Isolate OS-specific functionality behind a unified trait
// Formatting: Zobie.format (.solvraformat)
//=============================================

use std::fmt;
use std::io;
use std::time::Duration;

//=============================================
//            Section 1: Core Types
//=============================================

/// Result type used across platform helpers.
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Error variants produced by platform operations.
#[derive(Debug, Clone)]
pub enum PlatformError {
    IoError(String),
    NotSupported(String),
    InvalidInput(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::IoError(msg) => write!(f, "I/O error: {msg}"),
            PlatformError::NotSupported(msg) => write!(f, "Not supported: {msg}"),
            PlatformError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<io::Error> for PlatformError {
    fn from(err: io::Error) -> Self {
        PlatformError::IoError(err.to_string())
    }
}

/// Structured timestamp returned by [`PlatformOps::system_timestamp`].
#[derive(Debug, Clone)]
pub struct SystemTimestamp {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub nanosecond: u32,
}

/// Command invocation specification used by process helpers.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<Vec<(String, String)>>,
    pub clear_env: bool,
    pub stdin: StdioMode,
    pub stdout: StdioMode,
    pub stderr: StdioMode,
}

/// Standard I/O redirection options for spawned processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioMode {
    Inherit,
    Null,
    Piped,
}

/// Result returned by [`PlatformOps::execute_command`].
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

//=============================================
//            Section 2: Platform Trait
//=============================================

/// Contract implemented by OS-specific backends.
pub trait PlatformOps {
    fn system_time() -> PlatformResult<f64>;
    fn system_timestamp() -> PlatformResult<SystemTimestamp>;
    fn read_file(path: &str) -> PlatformResult<String>;
    fn write_file(path: &str, data: &str) -> PlatformResult<()>;
    fn append_file(path: &str, data: &str) -> PlatformResult<()>;
    fn path_exists(path: &str) -> bool;
    fn list_directory(path: &str) -> PlatformResult<Vec<String>>;
    fn env_get(key: &str) -> Option<String>;
    unsafe fn env_set(key: &str, value: &str);
    fn home_dir() -> Option<String>;
    fn path_join(left: &str, right: &str) -> String;
    fn canonicalize_path(path: &str) -> PlatformResult<String>;
    fn sleep(duration: Duration);
    fn print(text: &str) -> PlatformResult<()>;
    fn println(text: &str) -> PlatformResult<()>;
    fn eprintln(text: &str) -> PlatformResult<()>;
    fn flush_stdout() -> PlatformResult<()>;
    fn read_line(prompt: Option<&str>) -> PlatformResult<String>;
    fn execute_command(spec: &CommandSpec) -> PlatformResult<CommandResult>;
    fn spawn_command(spec: &CommandSpec) -> PlatformResult<u32>;
}

//=============================================
//            Section 3: Backend Selection
//=============================================

#[cfg(target_os = "linux")]
mod sys_linux;
#[cfg(target_os = "linux")]
pub use sys_linux::LinuxPlatform as NativePlatform;

#[cfg(target_os = "windows")]
mod sys_windows;
#[cfg(target_os = "windows")]
pub use sys_windows::WindowsPlatform as NativePlatform;

#[cfg(target_os = "macos")]
mod sys_macos;
#[cfg(target_os = "macos")]
pub use sys_macos::MacOSPlatform as NativePlatform;

#[cfg(all(
    not(target_os = "linux"),
    not(target_os = "windows"),
    not(target_os = "macos")
))]
mod sys_solvraos;
#[cfg(all(
    not(target_os = "linux"),
    not(target_os = "windows"),
    not(target_os = "macos")
))]
pub use sys_solvraos::SolvraOSPlatform as NativePlatform;

//=============================================
//            Section 4: Public API
//=============================================

pub fn system_time() -> PlatformResult<f64> {
    NativePlatform::system_time()
}

pub fn system_timestamp() -> PlatformResult<SystemTimestamp> {
    NativePlatform::system_timestamp()
}

pub fn read_file(path: &str) -> PlatformResult<String> {
    NativePlatform::read_file(path)
}

pub fn write_file(path: &str, data: &str) -> PlatformResult<()> {
    NativePlatform::write_file(path, data)
}

pub fn append_file(path: &str, data: &str) -> PlatformResult<()> {
    NativePlatform::append_file(path, data)
}

pub fn path_exists(path: &str) -> bool {
    NativePlatform::path_exists(path)
}

pub fn list_directory(path: &str) -> PlatformResult<Vec<String>> {
    NativePlatform::list_directory(path)
}

pub fn env_get(key: &str) -> Option<String> {
    NativePlatform::env_get(key)
}

pub unsafe fn env_set(key: &str, value: &str) {
    unsafe { NativePlatform::env_set(key, value) }
}

pub fn home_dir() -> Option<String> {
    NativePlatform::home_dir()
}

pub fn path_join(left: &str, right: &str) -> String {
    NativePlatform::path_join(left, right)
}

pub fn canonicalize_path(path: &str) -> PlatformResult<String> {
    NativePlatform::canonicalize_path(path)
}

pub fn sleep(duration: Duration) {
    NativePlatform::sleep(duration)
}

pub fn print(text: &str) -> PlatformResult<()> {
    NativePlatform::print(text)
}

pub fn println(text: &str) -> PlatformResult<()> {
    NativePlatform::println(text)
}

pub fn eprintln(text: &str) -> PlatformResult<()> {
    NativePlatform::eprintln(text)
}

pub fn flush_stdout() -> PlatformResult<()> {
    NativePlatform::flush_stdout()
}

pub fn read_line(prompt: Option<&str>) -> PlatformResult<String> {
    NativePlatform::read_line(prompt)
}

pub fn execute_command(spec: &CommandSpec) -> PlatformResult<CommandResult> {
    NativePlatform::execute_command(spec)
}

pub fn spawn_command(spec: &CommandSpec) -> PlatformResult<u32> {
    NativePlatform::spawn_command(spec)
}

//=============================================
// End of solvra_script/platform/mod.rs
//=============================================
