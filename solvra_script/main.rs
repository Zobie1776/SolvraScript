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
use serde_json::json;
use solvra_core::vm::bytecode::VmBytecode;
use solvra_core::{SolvraError, StackFrame, Value};
use tokenizer::Tokenizer;
use vm::TelemetryCollector;
use vm::compiler;
use vm::runtime::{MemoryTracker, RuntimeOptions, SolvraProgram, run_bytecode};

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

    /// Async timeout in milliseconds (equivalent to SOLVRA_ASYNC_TIMEOUT_MS).
    #[arg(long, value_name = "MS")]
    async_timeout_ms: Option<u64>,

    /// Pretty-print the parsed AST before execution (source files only).
    #[arg(long)]
    print_ast: bool,

    /// Emit runtime telemetry as JSON to stdout after execution completes.
    #[arg(long)]
    telemetry: bool,

    /// Emit memory statistics JSON after execution completes.
    #[arg(long = "memory-stats")]
    memory_stats: bool,

    /// Enable module hot reload semantics for imports.
    #[arg(long)]
    hot_reload: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    modules::set_global_hot_reload(args.hot_reload);
    let trace_enabled = args.trace || trace_from_env();
    let mut options = RuntimeOptions::with_trace(trace_enabled);
    if let Some(timeout_ms) = args.async_timeout_ms.or_else(async_timeout_from_env) {
        options = options.with_async_timeout(timeout_ms);
    }

    let telemetry_collector = if args.telemetry {
        Some(TelemetryCollector::new())
    } else {
        None
    };
    if let Some(collector) = &telemetry_collector {
        options = options.with_telemetry_collector(collector.clone());
    }

    let memory_tracker = if args.memory_stats {
        Some(MemoryTracker::new())
    } else {
        None
    };
    if let Some(tracker) = &memory_tracker {
        options = options.with_memory_tracker(tracker.clone());
    }

    match file_kind(&args.script) {
        Some(FileKind::Source) => run_source_file(
            &args.script,
            options.clone(),
            args.print_ast,
            telemetry_collector.clone(),
            memory_tracker.clone(),
        ),
        Some(FileKind::Bytecode) => {
            run_bytecode_file(&args.script, options, telemetry_collector, memory_tracker)
        }
        None => Err(anyhow!(
            "unsupported input extension for {}",
            args.script.display()
        )),
    }
}

fn run_source_file(
    path: &Path,
    options: RuntimeOptions,
    print_ast: bool,
    telemetry: Option<TelemetryCollector>,
    memory_tracker: Option<MemoryTracker>,
) -> Result<()> {
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
    let value = execute_vm(Arc::new(vm_program), options)?;
    emit_runtime_value(&value);
    emit_runtime_metrics(telemetry, memory_tracker)?;
    Ok(())
}

fn run_bytecode_file(
    path: &Path,
    options: RuntimeOptions,
    telemetry: Option<TelemetryCollector>,
    memory_tracker: Option<MemoryTracker>,
) -> Result<()> {
    let data = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let vm_program =
        VmBytecode::decode(&data[..]).map_err(|err| anyhow!("failed to decode bytecode: {err}"))?;
    let value = execute_vm(Arc::new(vm_program), options)?;
    emit_runtime_value(&value);
    emit_runtime_metrics(telemetry, memory_tracker)?;
    Ok(())
}

fn execute_vm(program: SolvraProgram, options: RuntimeOptions) -> Result<Value> {
    match run_bytecode(program, options) {
        Ok(value) => Ok(value),
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

fn emit_runtime_value(value: &Value) {
    if !matches!(value, Value::Null) {
        println!("{}", value_to_string(value));
    }
}

fn emit_runtime_metrics(
    telemetry: Option<TelemetryCollector>,
    memory_tracker: Option<MemoryTracker>,
) -> Result<()> {
    if let Some(collector) = telemetry {
        let events = collector.snapshot();
        let json = serde_json::to_string(&json!({ "events": events }))
            .map_err(|err| anyhow!("failed to serialise telemetry events: {err}"))?;
        println!("{json}");
    }

    if let Some(tracker) = memory_tracker {
        let stats = tracker.snapshot();
        let json = serde_json::to_string(&json!({ "memory_stats": stats }))
            .map_err(|err| anyhow!("failed to serialise memory stats: {err}"))?;
        println!("{json}");
    }

    Ok(())
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

fn async_timeout_from_env() -> Option<u64> {
    env::var("SOLVRA_ASYNC_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|timeout| *timeout > 0)
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
