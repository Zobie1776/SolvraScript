//=============================================
// nova_script/platform/sys_novaos.rs
//=============================================
// Author: NovaOS Contributors
// License: MIT (see LICENSE)
// Goal: NovaOS-native platform implementation
// Objective: Direct NovaOS syscall integration
// Formatting: Zobie.format (.novaformat)
//=============================================

use super::{
CommandResult, CommandSpec, PlatformError, PlatformOps, PlatformResult, StdioMode,
SystemTimestamp,
};
use std::time::Duration;

pub struct NovaOSPlatform;

impl PlatformOps for NovaOSPlatform {
fn system_time() -> PlatformResult<f64> {
// TODO: Replace with NovaOS syscall once available
// For now, fallback to standard implementation
#[cfg(feature = “std”)]
{
use std::time::{SystemTime, UNIX_EPOCH};
let now = SystemTime::now();
let duration = now
.duration_since(UNIX_EPOCH)
.map_err(|e| PlatformError::IoError(e.to_string()))?;
Ok(duration.as_secs_f64())
}
#[cfg(not(feature = “std”))]
{
// NovaOS native syscall: syscall(SYS_TIME, 0)
Err(PlatformError::NotSupported(
“NovaOS time syscall not yet implemented”.to_string(),
))
}
}

```
fn system_timestamp() -> PlatformResult<SystemTimestamp> {
    // TODO: Replace with NovaOS syscall for structured time
    #[cfg(feature = "std")]
    {
        use chrono::{Datelike, Timelike, Utc};
        let now = Utc::now();
        Ok(SystemTimestamp {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
            second: now.second(),
            nanosecond: now.nanosecond(),
        })
    }
    #[cfg(not(feature = "std"))]
    {
        Err(PlatformError::NotSupported(
            "NovaOS timestamp syscall not yet implemented".to_string(),
        ))
    }
}

fn read_file(path: &str) -> PlatformResult<String> {
    // TODO: Replace with NovaOS VFS syscall
    #[cfg(feature = "std")]
    {
        use std::fs;
        fs::read_to_string(path).map_err(PlatformError::from)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = path;
        Err(PlatformError::NotSupported(
            "NovaOS file read syscall not yet implemented".to_string(),
        ))
    }
}

fn write_file(path: &str, data: &str) -> PlatformResult<()> {
    // TODO: Replace with NovaOS VFS syscall
    #[cfg(feature = "std")]
    {
        use std::fs;
        fs::write(path, data).map_err(PlatformError::from)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = (path, data);
        Err(PlatformError::NotSupported(
            "NovaOS file write syscall not yet implemented".to_string(),
        ))
    }
}

fn append_file(path: &str, data: &str) -> PlatformResult<()> {
    // TODO: Replace with NovaOS VFS syscall
    #[cfg(feature = "std")]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        file.write_all(data.as_bytes())?;
        file.flush()?;
        Ok(())
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = (path, data);
        Err(PlatformError::NotSupported(
            "NovaOS file append syscall not yet implemented".to_string(),
        ))
    }
}

fn path_exists(path: &str) -> bool {
    // TODO: Replace with NovaOS VFS syscall
    #[cfg(feature = "std")]
    {
        use std::path::Path;
        Path::new(path).exists()
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = path;
        false
    }
}

fn list_directory(path: &str) -> PlatformResult<Vec<String>> {
    // TODO: Replace with NovaOS VFS syscall
    #[cfg(feature = "std")]
    {
        use std::fs;
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                entries.push(name.to_string());
            }
        }
        Ok(entries)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = path;
        Err(PlatformError::NotSupported(
            "NovaOS directory listing syscall not yet implemented".to_string(),
        ))
    }
}

fn env_get(key: &str) -> Option<String> {
    // TODO: Replace with NovaOS environment syscall
    #[cfg(feature = "std")]
    {
        std::env::var(key).ok()
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = key;
        None
    }
}

unsafe fn env_set(key: &str, value: &str) {
    // TODO: Replace with NovaOS environment syscall
    #[cfg(feature = "std")]
    {
        std::env::set_var(key, value);
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = (key, value);
    }
}

fn home_dir() -> Option<String> {
    // TODO: Replace with NovaOS user info syscall
    #[cfg(feature = "std")]
    {
        dirs::home_dir().map(|p| p.to_string_lossy().to_string())
    }
    #[cfg(not(feature = "std"))]
    {
        None
    }
}

fn path_join(left: &str, right: &str) -> String {
    // NovaOS uses forward slashes for paths
    format!("{}/{}", left.trim_end_matches('/'), right.trim_start_matches('/'))
}

fn canonicalize_path(path: &str) -> PlatformResult<String> {
    // TODO: Replace with NovaOS VFS canonicalization
    #[cfg(feature = "std")]
    {
        use std::fs;
        fs::canonicalize(path)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(PlatformError::from)
    }
    #[cfg(not(feature = "std"))]
    {
        // Simple path normalization for NovaOS
        Ok(path.to_string())
    }
}

fn sleep(duration: Duration) {
    // TODO: Replace with NovaOS sleep syscall
    #[cfg(feature = "std")]
    {
        std::thread::sleep(duration);
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = duration;
        // NovaOS syscall: syscall(SYS_SLEEP, duration_ms)
    }
}

fn print(text: &str) -> PlatformResult<()> {
    // TODO: Replace with NovaOS console syscall
    #[cfg(feature = "std")]
    {
        print!("{}", text);
        Ok(())
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = text;
        // NovaOS syscall: syscall(SYS_WRITE, STDOUT, text.as_ptr(), text.len())
        Ok(())
    }
}

fn println(text: &str) -> PlatformResult<()> {
    // TODO: Replace with NovaOS console syscall
    #[cfg(feature = "std")]
    {
        println!("{}", text);
        Ok(())
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = text;
        // NovaOS syscall with newline
        Ok(())
    }
}

fn eprintln(text: &str) -> PlatformResult<()> {
    // TODO: Replace with NovaOS console syscall
    #[cfg(feature = "std")]
    {
        eprintln!("{}", text);
        Ok(())
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = text;
        // NovaOS syscall: syscall(SYS_WRITE, STDERR, text.as_ptr(), text.len())
        Ok(())
    }
}

fn flush_stdout() -> PlatformResult<()> {
    // TODO: Replace with NovaOS console flush syscall
    #[cfg(feature = "std")]
    {
        use std::io::{self, Write};
        io::stdout().flush().map_err(PlatformError::from)
    }
    #[cfg(not(feature = "std"))]
    {
        Ok(())
    }
}

fn read_line(prompt: Option<&str>) -> PlatformResult<String> {
    // TODO: Replace with NovaOS console input syscall
    #[cfg(feature = "std")]
    {
        use std::io::{self, BufRead, Write};
        if let Some(p) = prompt {
            print!("{}", p);
            io::stdout().flush()?;
        }
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        while line.ends_with(['\n', '\r']) {
            line.pop();
        }
        Ok(line)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = prompt;
        Err(PlatformError::NotSupported(
            "NovaOS console input syscall not yet implemented".to_string(),
        ))
    }
}

fn execute_command(spec: &CommandSpec) -> PlatformResult<CommandResult> {
    // TODO: Replace with NovaOS process execution syscall
    #[cfg(feature = "std")]
    {
        use std::process::{Command, Stdio};
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args);

        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }

        if spec.clear_env {
            cmd.env_clear();
        }

        if let Some(env) = &spec.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        cmd.stdin(match spec.stdin {
            StdioMode::Inherit => Stdio::inherit(),
            StdioMode::Null => Stdio::null(),
            StdioMode::Piped => Stdio::piped(),
        });

        let output = cmd.output()?;
        
        Ok(CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = spec;
        Err(PlatformError::NotSupported(
            "NovaOS process execution syscall not yet implemented".to_string(),
        ))
    }
}

fn spawn_command(spec: &CommandSpec) -> PlatformResult<u32> {
    // TODO: Replace with NovaOS process spawn syscall
    #[cfg(feature = "std")]
    {
        use std::process::{Command, Stdio};
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args);

        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }

        if spec.clear_env {
            cmd.env_clear();
        }

        if let Some(env) = &spec.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        cmd.stdin(match spec.stdin {
            StdioMode::Inherit => Stdio::inherit(),
            StdioMode::Null => Stdio::null(),
            StdioMode::Piped => Stdio::piped(),
        });

        cmd.stdout(match spec.stdout {
            StdioMode::Inherit => Stdio::inherit(),
            StdioMode::Null => Stdio::null(),
            StdioMode::Piped => Stdio::piped(),
        });

        cmd.stderr(match spec.stderr {
            StdioMode::Inherit => Stdio::inherit(),
            StdioMode::Null => Stdio::null(),
            StdioMode::Piped => Stdio::piped(),
        });

        let child = cmd.spawn()?;
        Ok(child.id())
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = spec;
        Err(PlatformError::NotSupported(
            "NovaOS process spawn syscall not yet implemented".to_string(),
        ))
    }
}
```

}

//=============================================
// End Of nova_script/platform/sys_novaos.rs
//=============================================
// Notes:
// - All NovaOS syscalls are marked with TODO comments
// - Fallback to std implementation when available
// - NovaCore integration will replace these placeholders
//=============================================