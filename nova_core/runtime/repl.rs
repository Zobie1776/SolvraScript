use std::io::{self, BufRead, Write};

use crate::backend;
use crate::nvc;
use crate::runtime::{NovaError, NovaRuntime};
use crate::NovaResult;

/// Interactive REPL helper bound to a [`NovaRuntime`].
pub struct RuntimeRepl {
    runtime: NovaRuntime,
}

impl RuntimeRepl {
    /// Creates a new REPL wrapper around the provided runtime.
    pub fn new(runtime: NovaRuntime) -> Self {
        Self { runtime }
    }

    /// Launches the REPL using standard input and output streams.
    pub fn run(&mut self) -> NovaResult<()> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        self.run_with(&mut reader, &mut writer)
    }

    /// Executes the REPL using the supplied reader and writer, useful for testing.
    pub fn run_with<R: BufRead, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> NovaResult<()> {
        writeln!(writer, "Nova runtime REPL. Type :help for commands.").map_err(map_io_error)?;
        let mut line = String::new();
        loop {
            writer.write_all(b"nova> ").map_err(map_io_error)?;
            writer.flush().map_err(map_io_error)?;
            line.clear();
            let read = reader.read_line(&mut line).map_err(map_io_error)?;
            if read == 0 {
                writeln!(writer).map_err(map_io_error)?;
                break;
            }
            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            if input.starts_with(':') {
                if !self.handle_command(input, reader, writer)? {
                    break;
                }
                continue;
            }

            self.execute_assembly(input, writer)?;
        }
        Ok(())
    }

    fn handle_command<R: BufRead, W: Write>(
        &mut self,
        input: &str,
        reader: &mut R,
        writer: &mut W,
    ) -> NovaResult<bool> {
        let mut parts = input.split_whitespace();
        let command = parts.next().unwrap_or("");
        match command {
            ":help" => {
                writeln!(writer, "Available commands:").map_err(map_io_error)?;
                writeln!(writer, "  :help           Show this help message")
                    .map_err(map_io_error)?;
                writeln!(writer, "  :quit/:exit     Exit the REPL").map_err(map_io_error)?;
                writeln!(writer, "  :load <path>    Load a .nvc module into memory")
                    .map_err(map_io_error)?;
                writeln!(
                    writer,
                    "  :exec <path>    Execute a .nvc binary file immediately"
                )
                .map_err(map_io_error)?;
                writeln!(
                    writer,
                    "  :backend        Display the active CPU backend"
                )
                .map_err(map_io_error)?;
                writeln!(
                    writer,
                    "  :asm            Enter multi-line Nova assembly (finish with :end)"
                )
                .map_err(map_io_error)?;
                writeln!(
                    writer,
                    "  <assembly>      Single line Nova assembly executed immediately"
                )
                .map_err(map_io_error)?;
                Ok(true)
            }
            ":quit" | ":exit" => {
                writeln!(writer, "Exiting runtime REPL.").map_err(map_io_error)?;
                Ok(false)
            }
            ":load" => {
                if let Some(path) = parts.next() {
                    match self.runtime.load_module_file(path) {
                        Ok(module) => {
                            let bytecode = module.bytecode();
                            let instruction_count = bytecode
                                .functions()
                                .iter()
                                .map(|f| f.instructions.len())
                                .sum::<usize>();
                            writeln!(
                                writer,
                                "Loaded module '{}' ({} instructions)",
                                module.name(),
                                instruction_count
                            )
                            .map_err(map_io_error)?;
                        }
                        Err(err) => {
                            writeln!(writer, "Load failed: {err}").map_err(map_io_error)?;
                        }
                    }
                } else {
                    writeln!(writer, ":load requires a path argument").map_err(map_io_error)?;
                }
                Ok(true)
            }
            ":exec" => {
                if let Some(path) = parts.next() {
                    match self.runtime.execute_file(path) {
                        Ok(value) => {
                            writeln!(writer, "= {:?}", value).map_err(map_io_error)?;
                        }
                        Err(err) => {
                            writeln!(writer, "Execution failed: {err}").map_err(map_io_error)?;
                        }
                    }
                } else {
                    writeln!(writer, ":exec requires a path argument").map_err(map_io_error)?;
                }
                Ok(true)
            }
            ":backend" => {
                let backend = backend::active_backend();
                writeln!(
                    writer,
                    "Active backend: {} ({})",
                    backend.name(),
                    backend.target().as_str()
                )
                .map_err(map_io_error)?;
                Ok(true)
            }
            ":asm" => {
                writeln!(
                    writer,
                    "Enter Nova assembly. Finish input with a single line containing :end."
                )
                .map_err(map_io_error)?;
                let mut source = String::new();
                let mut buffer = String::new();
                loop {
                    writer.write_all(b"....> ").map_err(map_io_error)?;
                    writer.flush().map_err(map_io_error)?;
                    buffer.clear();
                    let read = reader.read_line(&mut buffer).map_err(map_io_error)?;
                    if read == 0 {
                        break;
                    }
                    let trimmed = buffer.trim_end();
                    if trimmed == ":end" {
                        break;
                    }
                    source.push_str(&buffer);
                }
                if source.trim().is_empty() {
                    writeln!(writer, "No assembly provided.").map_err(map_io_error)?;
                } else {
                    self.execute_assembly(&source, writer)?;
                }
                Ok(true)
            }
            _ => {
                writeln!(
                    writer,
                    "Unknown command {command}. Type :help for a list of commands."
                )
                .map_err(map_io_error)?;
                Ok(true)
            }
        }
    }

    fn execute_assembly<W: Write>(&self, source: &str, writer: &mut W) -> NovaResult<()> {
        match novac::assemble(source) {
            Ok(bytecode) => match bytecode.encode() {
                Ok(bytes) => match self.runtime.execute(&bytes) {
                    Ok(value) => {
                        writeln!(writer, "= {:?}", value).map_err(map_io_error)?;
                        Ok(())
                    }
                    Err(err) => {
                        writeln!(writer, "Execution failed: {err}").map_err(map_io_error)?;
                        Ok(())
                    }
                },
                Err(err) => {
                    writeln!(writer, "Encoding failed: {err}").map_err(map_io_error)?;
                    Ok(())
                }
            },
            Err(err) => {
                writeln!(writer, "Assembly failed: {err}").map_err(map_io_error)?;
                Ok(())
            }
        }
    }

    /// Returns the value of the underlying runtime for programmatic use.
    pub fn runtime(&self) -> &NovaRuntime {
        &self.runtime
    }
}

fn map_io_error(err: io::Error) -> NovaError {
    NovaError::Native(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn repl_handles_exit_command() {
        let runtime = NovaRuntime::new();
        let mut repl = RuntimeRepl::new(runtime);
        let input = b":help\n:backend\n:quit\n";
        let mut reader = Cursor::new(&input[..]);
        let mut output = Vec::new();
        repl.run_with(&mut reader, &mut output).expect("repl run");
        let out_str = String::from_utf8(output).expect("utf8");
        assert!(out_str.contains("Available commands"));
        assert!(out_str.contains("Active backend:"));
        assert!(out_str.contains("Exiting runtime REPL"));
    }
}
