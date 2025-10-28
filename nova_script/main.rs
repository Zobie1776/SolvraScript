//=============================================
// nova_script/main.rs
//=============================================
// Author: NovaOS Contributors
// License: MIT (see LICENSE)
// Goal: NovaScript CLI entrypoint for running .ns scripts
// Objective: Provide parsing, optional diagnostics, and execution with dry-run support
// Formatting: Zobie.format (.novaformat)
//=============================================

mod ast;
mod interpreter;
mod modules;
mod parser;
mod tokenizer;

use anyhow::{anyhow, Context, Result};
use ast::{Program, Stmt};
use clap::Parser as ClapParser;
use interpreter::{value_to_json, Interpreter, RuntimeError, Value};
use parser::{ParseError, Parser};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tokenizer::Tokenizer;

//=============================================
//            Section 1: CLI Definition
//=============================================

#[derive(Debug, ClapParser)]
#[command(
    name = "novascript",
    about = "Runs NovaScript files or evaluates inline expressions.",
    version
)]
struct Args {
    /// Path to the NovaScript file to execute.
    script: PathBuf,

    /// Skip side effects such as spawning processes.
    #[arg(long)]
    dry_run: bool,

    /// Evaluate an expression after the script finishes and print the JSON result.
    #[arg(long)]
    json: Option<String>,

    /// Pretty-print the parsed AST.
    #[arg(long)]
    print_ast: bool,

    /// Dump a pseudo IR listing of the program.
    #[arg(long)]
    print_ir: bool,
}

//=============================================
//            Section 2: Entry Point
//=============================================

fn main() -> Result<()> {
    let args = Args::parse();
    run_script(&args)
}

fn run_script(args: &Args) -> Result<()> {
    let source = fs::read_to_string(&args.script)
        .with_context(|| format!("Failed to read {}", args.script.display()))?;

    let mut tokenizer = Tokenizer::new(&source);
    let tokens = tokenizer
        .tokenize()
        .map_err(|err| anyhow!("Tokenizer error: {err}"))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|error| map_parse_error(&args.script, error))?;

    if args.print_ast {
        println!("{:#?}", program);
    }

    if args.print_ir {
        print_ir(&program);
    }

    let mut interpreter = Interpreter::new();
    interpreter.set_dry_run(args.dry_run);
    if let Some(parent) = args.script.parent() {
        interpreter.add_module_search_path(parent);
    }

    let result = interpreter
        .eval_program_with_origin(&program, Some(&args.script))
        .map_err(|error| match error {
            RuntimeError::Exit(code) => {
                process::exit(code);
            }
            other => anyhow!(other),
        })?;

    if let Some(expr) = args.json.as_deref() {
        evaluate_expression_as_json(&mut interpreter, expr)?;
        return Ok(());
    }

    if let Some(value) = result {
        println!("{value}");
    }

    Ok(())
}

//=============================================
//            Section 3: Helpers
//=============================================

fn evaluate_expression_as_json(interpreter: &mut Interpreter, expr: &str) -> Result<()> {
    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer
        .tokenize()
        .map_err(|err| anyhow!("Tokenizer error in --json expression: {err}"))?;
    let mut parser = Parser::new(tokens);
    let expression = parser
        .parse_expression_only()
        .map_err(|error| anyhow!("Expression parse error: {error:?}"))?;
    let value = interpreter
        .eval_expression(&expression)
        .map_err(|error| match error {
            RuntimeError::Exit(code) => {
                process::exit(code);
            }
            other => anyhow!(other),
        })?;
    println!("{}", value_to_json(&value));
    Ok(())
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
        ParseError::InvalidAssignmentTarget { position } => anyhow!(
            "{}:{}:{}: invalid assignment target",
            path.display(),
            position.line,
            position.column
        ),
        ParseError::DuplicateBinding { name, position } => anyhow!(
            "{}:{}:{}: duplicate binding '{}'",
            path.display(),
            position.line,
            position.column,
            name
        ),
        other => anyhow!("{other:?}"),
    }
}

fn print_ir(program: &Program) {
    println!("; NovaScript pseudo-IR ({} statements)", program.statements.len());
    for (index, statement) in program.statements.iter().enumerate() {
        println!("{:04} | {}", index, describe_statement(statement));
    }
}

fn describe_statement(statement: &Stmt) -> String {
    use Stmt::*;
    match statement {
        ImportDecl { decl } => format!("import {:?}", decl.source),
        VariableDecl { decl } => format!("let {}", decl.name),
        FunctionDecl { decl } => format!("fn {}(..)", decl.name),
        ExpressionStmt { .. } => "expr".into(),
        Return { .. } => "return".into(),
        If { .. } => "if".into(),
        While { .. } => "while".into(),
        For { .. } => "for".into(),
        Block { .. } => "block".into(),
        Try { .. } => "try".into(),
        Throw { .. } => "throw".into(),
        Break => "break".into(),
        Continue => "continue".into(),
        Namespace { decl } => format!("namespace {}", decl.name),
        _ => format!("{statement:?}"),
    }
}
