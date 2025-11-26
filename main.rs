//=====================================================
// File: main.rs
//=====================================================
// Author: ZobieLabs
// License: Duality Public License (DPL v1.0)
// Goal: SolvraScript CLI entry point
// Objective: Command-line interface for executing .svs source files and .svc bytecode,
//            with support for AST printing, telemetry, and runtime options
//=====================================================

// Added by Claude for Zobie.format compliance
mod ast;
mod core_bridge;
mod interpreter;
mod ir;
mod modules;
mod parser;
mod platform;
mod stdlib_registry;
mod symbol;
mod tier1;
mod tier2;
mod tokenizer;
mod vm;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use bincode;
use clap::{Args as ClapArgs, Parser, Subcommand};
use ir::interpreter::{IrInterpreter, RuntimeValue};
use ir::lowering::lower_program;
use ir::verify::verify_function;
use parser::{ParseError, Parser as AstParser};
use serde_json::json;
use solvra_core::jit::tier0_codegen::Tier0Compiler;
use solvra_core::vm::bytecode::VmBytecode;
use solvra_core::{SolvraError, StackFrame, Value};
use tokenizer::Tokenizer;
use vm::TelemetryCollector;
use tier2::Tier2Options;
use vm::compiler;
use vm::runtime::{MemoryTracker, RuntimeOptions, SolvraProgram, run_bytecode};

#[derive(Parser, Debug)]
#[command(name = "solvrascript", about = "SolvraScript CLI")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Execute a .svs source file or .svc bytecode.
    Run(RunArgs),
    /// Compile a .svs source file into .svc bytecode.
    Compile(CompileArgs),
}

#[derive(ClapArgs, Debug, Clone)]
pub struct CompileArgs {
    /// Input .svs source file.
    pub input: PathBuf,
    /// Output .svc bytecode file.
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct RunArgs {
    /// Path to the script or bytecode to execute.
    pub script: PathBuf,

    /// Print parsed AST before execution.
    #[arg(long = "print-ast")]
    pub print_ast: bool,

    /// Execute using the IR interpreter instead of the VM.
    #[arg(long = "enable-ir")]
    pub enable_ir: bool,

    /// Emit Tier-0 IR listing and exit.
    #[arg(long = "emit-tier0")]
    pub emit_tier0: bool,

    /// Emit Tier-1 MIR listing and exit.
    #[arg(long = "emit-mir")]
    pub emit_mir: bool,

    /// Emit Tier-1 MIR listing only if verification succeeds.
    #[arg(long = "emit-mir-verified")]
    pub emit_mir_verified: bool,

    /// Run Tier-1 register allocation and dump the assignments.
    #[arg(long = "emit-regalloc")]
    pub emit_regalloc: bool,

    /// Enable Tier-0 JIT dispatch.
    #[arg(long = "jit-tier0")]
    pub jit_tier0: bool,

    /// Enable Tier-1 JIT dispatch.
    #[arg(long = "jit-tier1")]
    pub jit_tier1: bool,

    /// Print JIT statistics after execution.
    #[arg(long = "jit-stats")]
    pub jit_stats: bool,

    /// Print Tier-1 deopt debug information.
    #[arg(long = "jit-deopt-debug")]
    pub jit_deopt_debug: bool,

    /// Print fused Tier-1 IC debug information after execution.
    #[arg(long = "jit-tier1-fused-debug")]
    pub jit_tier1_fused_debug: bool,

    /// Print Tier-1 OSR landing pad metadata after execution.
    #[arg(long = "jit-osr-debug")]
    pub jit_osr_debug: bool,

    /// Print transfer-plan reconstruction debug information.
    #[arg(long = "jit-transfer-debug")]
    pub jit_transfer_debug: bool,

    /// Validate OSR landing pads before use and report results.
    #[arg(long = "jit-osr-validate")]
    pub jit_osr_validate: bool,

    /// Enable Tier-2 optimizing JIT.
    #[arg(long = "jit-tier2")]
    pub jit_tier2: bool,

    /// Print Tier-2 OSR/metadata debug information.
    #[arg(long = "jit-osr-tier2-debug")]
    pub jit_osr_tier2_debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Compile(cmd) => compile_svs_to_svc(&cmd.input, &cmd.output),
        Command::Run(cmd) => run_entry(cmd),
    }
}

fn run_entry(args: RunArgs) -> Result<()> {
    let options = RuntimeOptions {
        jit_tier0: args.jit_tier0,
        jit_tier1: args.jit_tier1,
        jit_deopt_debug: args.jit_deopt_debug,
        jit_transfer_debug: args.jit_transfer_debug,
        jit_tier2: args.jit_tier2,
        jit_osr_tier2_debug: args.jit_osr_tier2_debug,
        ..Default::default()
    };

    if args
        .script
        .extension()
        .map(|ext| ext == "svc")
        .unwrap_or(false)
    {
        run_svc_file(&args.script, options)
    } else {
        let program = parse_source(&args.script)?;
        run_source_program(
            &args.script,
            &program,
            options,
            args.print_ast,
            args.enable_ir,
            args.emit_tier0,
            args.emit_mir,
            args.emit_mir_verified,
            args.emit_regalloc,
            args.jit_tier0,
            args.jit_tier1,
            args.jit_stats,
            args.jit_deopt_debug,
            args.jit_tier1_fused_debug,
            args.jit_osr_debug,
            args.jit_transfer_debug,
            args.jit_osr_validate,
            None,
            None,
        )
    }
}

fn run_source_program(
    path: &Path,
    program: &ast::Program,
    mut options: RuntimeOptions,
    print_ast: bool,
    enable_ir: bool,
    emit_tier0: bool,
    emit_mir: bool,
    emit_mir_verified: bool,
    emit_regalloc: bool,
    jit_tier0: bool,
    jit_tier1: bool,
    jit_stats: bool,
    jit_deopt_debug: bool,
    jit_tier1_fused_debug: bool,
    jit_osr_debug: bool,
    jit_transfer_debug: bool,
    jit_osr_validate: bool,
    telemetry: Option<TelemetryCollector>,
    memory_tracker: Option<MemoryTracker>,
) -> Result<()> {
    if print_ast {
        println!("{:#?}", program);
    }

    if emit_tier0 {
        return run_tier0_pipeline(&program);
    }

    if emit_mir || emit_mir_verified || emit_regalloc {
        return run_tier1_debug_pipeline(&program, emit_mir, emit_mir_verified, emit_regalloc);
    }

    if enable_ir {
        return run_ir_pipeline(&program);
    }

    run_vm_pipeline(
        &program,
        options,
        jit_tier0,
        jit_tier1,
        jit_stats,
        jit_deopt_debug,
        jit_tier1_fused_debug,
        jit_osr_debug,
        jit_transfer_debug,
        jit_osr_validate,
        telemetry,
        memory_tracker,
    )
}

fn run_vm_pipeline(
    program: &ast::Program,
    mut options: RuntimeOptions,
    jit_tier0: bool,
    jit_tier1: bool,
    jit_stats: bool,
    jit_deopt_debug: bool,
    jit_tier1_fused_debug: bool,
    jit_osr_debug: bool,
    jit_transfer_debug: bool,
    jit_osr_validate: bool,
    telemetry: Option<TelemetryCollector>,
    memory_tracker: Option<MemoryTracker>,
) -> Result<()> {
    options.jit_tier0 = jit_tier0;
    options.jit_tier1 = jit_tier1;
    options.jit_stats = jit_stats;
    options.jit_deopt_debug = jit_deopt_debug;
    options.jit_tier1_fused_debug = jit_tier1_fused_debug;
    options.jit_osr_debug = jit_osr_debug;
    options.jit_transfer_debug = jit_transfer_debug;
    options.jit_osr_validate = jit_osr_validate;

    if jit_tier0 || jit_stats || jit_tier1 {
        let module = lower_program(program).map_err(|err| anyhow!("IR lowering failed: {err}"))?;
        let module_arc = Arc::new(module);
        if jit_tier0 || jit_stats {
            options.jit_ir_module = Some(Arc::clone(&module_arc));
        }
        if jit_tier1 {
            let lowered = tier1::lower_ir_to_mir(module_arc.as_ref());
            let mir_arc = Arc::new(lowered.module);
            let osr_arc = Arc::new(lowered.osr_registry);
            options.tier1_mir_module = Some(Arc::clone(&mir_arc));
            options.tier1_osr_registry = Some(osr_arc);
            if options.jit_ir_module.is_none() {
                options.jit_ir_module = Some(Arc::clone(&module_arc));
            }
            if options.tier1_mir_module.is_none() {
                options.tier1_mir_module = Some(mir_arc);
            }
        }
        if options.jit_ir_module.is_none() {
            options.jit_ir_module = Some(module_arc);
        }
    }

    let bytecode =
        compiler::compile_program(program).map_err(|err| anyhow!("compiler error: {err}"))?;
    let vm_program =
        VmBytecode::decode(&bytecode[..]).map_err(|err| anyhow!("bytecode decode error: {err}"))?;
    let value = execute_vm(Arc::new(vm_program), options)?;
    emit_runtime_value(&value);
    emit_runtime_metrics(telemetry, memory_tracker)?;
    Ok(())
}

fn run_svc_file(path: &Path, options: RuntimeOptions) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let bytecode: VmBytecode =
        bincode::deserialize(&bytes).map_err(|err| anyhow!("svc decode error: {err}"))?;
    let value = execute_vm(Arc::new(bytecode), options)?;
    emit_runtime_value(&value);
    Ok(())
}

fn compile_svs_to_svc(input: &Path, output: &Path) -> Result<()> {
    let program = parse_source(input)?;
    let bytecode =
        compiler::compile_program(&program).map_err(|err| anyhow!("compiler error: {err}"))?;
    let vm_program =
        VmBytecode::decode(&bytecode[..]).map_err(|err| anyhow!("bytecode decode error: {err}"))?;
    let encoded =
        bincode::serialize(&vm_program).map_err(|err| anyhow!("svc encode error: {err}"))?;
    fs::write(output, encoded)
        .with_context(|| format!("failed to write {}", output.display()))?;
    Ok(())
}

fn run_ir_pipeline(program: &ast::Program) -> Result<()> {
    let module = lower_program(program).map_err(|err| anyhow!("IR lowering failed: {err}"))?;
    for function in module.functions() {
        verify_function(function)
            .map_err(|err| anyhow!("IR verification failed for {}: {err}", function.name))?;
    }
    let interpreter = IrInterpreter::new(&module);
    let value = interpreter
        .run_entry("main", &[])
        .map_err(|err| anyhow!("IR interpreter error: {err}"))?;
    if !matches!(value, RuntimeValue::Null) {
        println!("{value}");
    }
    Ok(())
}

fn run_tier0_pipeline(program: &ast::Program) -> Result<()> {
    let module = lower_program(program).map_err(|err| anyhow!("IR lowering failed: {err}"))?;
    let compiler = Tier0Compiler::new();
    for function in module.functions() {
        verify_function(function)
            .map_err(|err| anyhow!("IR verification failed for {}: {err}", function.name))?;
        let artifact = compiler.compile(function);
        println!("// Tier-0 IR: {}", function.name);
        println!("{}", artifact.listing.trim_end());
        println!();
    }
    Ok(())
}

fn run_tier1_debug_pipeline(
    program: &ast::Program,
    emit_mir: bool,
    emit_verified: bool,
    emit_regalloc: bool,
) -> Result<()> {
    let module = lower_program(program).map_err(|err| anyhow!("IR lowering failed: {err}"))?;
    let lowered = tier1::lower_ir_to_mir(&module);
    let mir_module = lowered.module;
    if emit_verified {
        tier1::verify_lowered_module(&mir_module)
            .map_err(|err| anyhow!("MIR verification failed: {err}"))?;
        tier1::dump_mir(&mir_module);
    } else if emit_mir {
        tier1::dump_mir(&mir_module);
    }
    if emit_regalloc {
        let _ = tier1::dump_regalloc(&mir_module);
    }
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

fn parse_source(path: &Path) -> Result<ast::Program> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut tokenizer = Tokenizer::new(&source);
    let tokens = tokenizer
        .tokenize()
        .map_err(|err| anyhow!("Tokenizer error: {err}"))?;
    let mut parser = AstParser::new(tokens);
    parser
        .parse()
        .map_err(|error| map_parse_error(path, error))
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
        Value::Array(elements) => format!("[array:{}]", elements.len()),
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

//=====================================================
// End of file
//=====================================================
// Added by Claude for Zobie.format compliance
