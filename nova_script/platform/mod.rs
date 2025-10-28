//=============================================
// nova_script/platform/mod.rs
//=============================================
// Author: NovaOS Contributors
// License: MIT (see LICENSE)
// Goal: Cross-platform abstraction layer for NovaScript
// Objective: Isolate OS-specific functionality behind unified trait
// Formatting: Zobie.format (.novaformat)
//=============================================

//=============================================
//            Section 1: Platform Trait
//=============================================

use std::io;
use std::time::Duration;

/// Result type for platform operations
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Platform-specific error types
#[derive(Debug, Clone)]
pub enum PlatformError {
IoError(String),
NotSupported(String),
InvalidInput(String),
}

impl std::fmt::Display for PlatformError {
fn fmt(&self, f: &mut std::fmt::Formatter<’_>) -> std::fmt::Result {
match self {
PlatformError::IoError(msg) => write!(f, “I/O error: {}”, msg),
PlatformError::NotSupported(msg) => write!(f, “Not supported: {}”, msg),
PlatformError::InvalidInput(msg) => write!(f, “Invalid input: {}”, msg),
}
}
}

impl std::error::Error for PlatformError {}

impl From<io::Error> for PlatformError {
fn from(err: io::Error) -> Self {
PlatformError::IoError(err.to_string())
}
}

/// Core platform operations trait
pub trait PlatformOps {
/// Get current system time in seconds since UNIX epoch
fn system_time() -> PlatformResult<f64>;

```
/// Get current UTC timestamp with structured fields
fn system_timestamp() -> PlatformResult<SystemTimestamp>;

/// Read entire file as string
fn read_file(path: &str) -> PlatformResult<String>;

/// Write string to file (truncate mode)
fn write_file(path: &str, data: &str) -> PlatformResult<()>;

/// Append string to file
fn append_file(path: &str, data: &str) -> PlatformResult<()>;

/// Check if file or directory exists
fn path_exists(path: &str) -> bool;

/// List directory contents
fn list_directory(path: &str) -> PlatformResult<Vec<String>>;

/// Get environment variable
fn env_get(key: &str) -> Option<String>;

/// Set environment variable (unsafe operation)
unsafe fn env_set(key: &str, value: &str);

/// Get user home directory
fn home_dir() -> Option<String>;

/// Join path components
fn path_join(left: &str, right: &str) -> String;

/// Canonicalize path
fn canonicalize_path(path: &str) -> PlatformResult<String>;

/// Sleep for specified duration
fn sleep(duration: Duration);

/// Print to stdout without newline
fn print(text: &str) -> PlatformResult<()>;

/// Print to stdout with newline
fn println(text: &str) -> PlatformResult<()>;

/// Print to stderr with newline
fn eprintln(text: &str) -> PlatformResult<()>;

/// Flush stdout
fn flush_stdout() -> PlatformResult<()>;

/// Read line from stdin
fn read_line(prompt: Option<&str>) -> PlatformResult<String>;

/// Execute command and wait for completion
fn execute_command(spec: &CommandSpec) -> PlatformResult<CommandResult>;

/// Spawn command without waiting
fn spawn_command(spec: &CommandSpec) -> PlatformResult<u32>;
```

}

/// Structured timestamp representation
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

/// Command execution specification
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

/// Standard I/O redirection modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioMode {
Inherit,
Null,
Piped,
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
pub success: bool,
pub exit_code: Option<i32>,
pub stdout: String,
pub stderr: String,
}

//=============================================
//            Section 2: Platform Selection
//=============================================

// Conditional compilation for each platform
#[cfg(target_os = “linux”)]
mod sys_linux;
#[cfg(target_os = “linux”)]
pub use sys_linux::LinuxPlatform as NativePlatform;

#[cfg(target_os = “windows”)]
mod sys_windows;
#[cfg(target_os = “windows”)]
pub use sys_windows::WindowsPlatform as NativePlatform;

#[cfg(target_os = “macos”)]
mod sys_macos;
#[cfg(target_os = “macos”)]
pub use sys_macos::MacOSPlatform as NativePlatform;

#[cfg(all(not(target_os = “linux”), not(target_os = “windows”), not(target_os = “macos”)))]
mod sys_novaos;
#[cfg(all(not(target_os = “linux”), not(target_os = “windows”), not(target_os = “macos”)))]
pub use sys_novaos::NovaOSPlatform as NativePlatform;

//=============================================
//            Section 3: Public API
//=============================================

/// Get current system time in seconds since UNIX epoch
pub fn system_time() -> PlatformResult<f64> {
NativePlatform::system_time()
}

/// Get current UTC timestamp with structured fields
pub fn system_timestamp() -> PlatformResult<SystemTimestamp> {
NativePlatform::system_timestamp()
}

/// Read entire file as string
pub fn read_file(path: &str) -> PlatformResult<String> {
NativePlatform::read_file(path)
}

/// Write string to file (truncate mode)
pub fn write_file(path: &str, data: &str) -> PlatformResult<()> {
NativePlatform::write_file(path, data)
}

/// Append string to file
pub fn append_file(path: &str, data: &str) -> PlatformResult<()> {
NativePlatform::append_file(path, data)
}

/// Check if file or directory exists
pub fn path_exists(path: &str) -> bool {
NativePlatform::path_exists(path)
}

/// List directory contents
pub fn list_directory(path: &str) -> PlatformResult<Vec<String>> {
NativePlatform::list_directory(path)
}

/// Get environment variable
pub fn env_get(key: &str) -> Option<String> {
NativePlatform::env_get(key)
}

/// Set environment variable (unsafe operation)
pub unsafe fn env_set(key: &str, value: &str) {
NativePlatform::env_set(key, value)
}

/// Get user home directory
pub fn home_dir() -> Option<String> {
NativePlatform::home_dir()
}

/// Join path components
pub fn path_join(left: &str, right: &str) -> String {
NativePlatform::path_join(left, right)
}

/// Canonicalize path
pub fn canonicalize_path(path: &str) -> PlatformResult<String> {
NativePlatform::canonicalize_path(path)
}

/// Sleep for specified duration
pub fn sleep(duration: Duration) {
NativePlatform::sleep(duration)
}

/// Print to stdout without newline
pub fn print(text: &str) -> PlatformResult<()> {
NativePlatform::print(text)
}

/// Print to stdout with newline
pub fn println(text: &str) -> PlatformResult<()> {
NativePlatform::println(text)
}

/// Print to stderr with newline
pub fn eprintln(text: &str) -> PlatformResult<()> {
NativePlatform::eprintln(text)
}

/// Flush stdout
pub fn flush_stdout() -> PlatformResult<()> {
NativePlatform::flush_stdout()
}

/// Read line from stdin
pub fn read_line(prompt: Option<&str>) -> PlatformResult<String> {
NativePlatform::read_line(prompt)
}

/// Execute command and wait for completion
pub fn execute_command(spec: &CommandSpec) -> PlatformResult<CommandResult> {
NativePlatform::execute_command(spec)
}

/// Spawn command without waiting
pub fn spawn_command(spec: &CommandSpec) -> PlatformResult<u32> {
NativePlatform::spawn_command(spec)
}

//=============================================
// End Of nova_script/platform/mod.rs
//=============================================