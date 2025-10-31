mod ast;
mod core_bridge;
mod interpreter;
mod modules;
mod parser;
mod platform;
mod tokenizer;
mod vm;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use clap::Parser as ClapParser;
use parser::{ParseError, Parser};
use solvra_core::vm::bytecode::VmBytecode;
use solvra_core::{SolvraError, StackFrame, Value};
use tokenizer::Tokenizer;
use vm::compiler;
use vm::runtime::{RuntimeOptions, SolvraProgram, run_bytecode};

#[derive(Debug, ClapParser)]
#[command(
    name = "solvrascript",
    about = "Executes SolvraScript source (.svs) or bytecode (.svc) files.",
    version
)]
struct Args {
    /// Path to a SolvraScript source (.svs) or bytecode (.svc) file.
    script: PathBuf,

    /// Enable opcode-level tracing (equivalent to setting SOLVRA_TRACE=1).
    #[arg(long)]
    trace: bool,

    /// Pretty-print the parsed AST before execution (source files only).
    #[arg(long)]
    print_ast: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let trace_enabled = args.trace || trace_from_env();

    match file_kind(&args.script) {
        Some(FileKind::Source) => run_source_file(&args.script, trace_enabled, args.print_ast),
        Some(FileKind::Bytecode) => run_bytecode_file(&args.script, trace_enabled),
        None => Err(anyhow!(
            "unsupported input extension for {}",
            args.script.display()
        )),
    }
}

fn run_source_file(path: &Path, trace: bool, print_ast: bool) -> Result<()> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    let mut tokenizer = Tokenizer::new(&source);
    let tokens = tokenizer
        .tokenize()
        .map_err(|err| anyhow!("Tokenizer error: {err}"))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|error| map_parse_error(path, error))?;

    if print_ast {
        println!("{:#?}", program);
    }

    let bytecode =
        compiler::compile_program(&program).map_err(|err| anyhow!("compiler error: {err}"))?;
    let vm_program =
        VmBytecode::decode(&bytecode[..]).map_err(|err| anyhow!("bytecode decode error: {err}"))?;
    execute_vm(Arc::new(vm_program), trace)
}

fn run_bytecode_file(path: &Path, trace: bool) -> Result<()> {
    let data = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let vm_program =
        VmBytecode::decode(&data[..]).map_err(|err| anyhow!("failed to decode bytecode: {err}"))?;
    execute_vm(Arc::new(vm_program), trace)
}

fn execute_vm(program: SolvraProgram, trace: bool) -> Result<()> {
    match run_bytecode(program, RuntimeOptions::with_trace(trace)) {
        Ok(value) => {
            if !matches!(value, Value::Null) {
                println!("{}", value_to_string(&value));
            }
            Ok(())
        }
        Err(SolvraError::RuntimeException { message, stack }) => {
            eprintln!("runtime error: {message}");
            for frame in stack.iter().rev() {
                eprintln!("    at {}", format_stack_frame(frame));
            }
            Err(anyhow!("runtime error: {message}"))
        }
        Err(err) => Err(anyhow!("runtime error: {err}")),
    }
}

fn trace_from_env() -> bool {
    env::var("SOLVRA_TRACE")
        .ok()
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            !(lower.is_empty() || lower == "0" || lower == "false" || lower == "off")
        })
        .unwrap_or(false)
}

enum FileKind {
    Source,
    Bytecode,
}

fn file_kind(path: &Path) -> Option<FileKind> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("svs") => Some(FileKind::Source),
        Some("svc") => Some(FileKind::Bytecode),
        _ => None,
    }
}

fn map_parse_error(path: &Path, error: ParseError) -> anyhow::Error {
    match error {
        ParseError::UnexpectedToken {
            expected,
            found,
            position,
        } => anyhow!(
            "{}:{}:{}: expected {}, found {:?}",
            path.display(),
            position.line,
            position.column,
            expected,
            found
        ),
        ParseError::UnexpectedEndOfInput { expected, position } => anyhow!(
            "{}:{}:{}: unexpected end of input (expected {})",
            path.display(),
            position.line,
            position.column,
            expected
        ),
        other => anyhow!("{other:?}"),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Integer(int) => int.to_string(),
        Value::Float(float) => float.to_string(),
        Value::String(text) => text.clone(),
        Value::Object(_) => "<object>".into(),
    }
}

fn format_stack_frame(frame: &StackFrame) -> String {
    if let Some(location) = &frame.location {
        format!("{} ({}:{})", frame.function, location.file, location.line)
    } else {
        frame.function.clone()
    }
}
