//===================================================
// SolvraCore Runtime
//===================================================
// Author: Zobie
// License:
// Goal:
// Objective:
//===================================================

use solvra_core::bytecode::assemble;
use solvra_core::bytecode::ast::{Ast, BinaryOp, Expr, Function, Span, Stmt};
use solvra_core::{SolvraError, SolvraRuntime, Value};

fn synthetic() -> Span {
    Span::synthetic()
}

#[test]
fn assemble_and_execute() {
    let ast = Ast::from_expr(Expr::binary(
        BinaryOp::Add,
        Expr::number(1.0),
        Expr::number(2.0),
    ));
    let bytecode = assemble(&ast).expect("assemble");
    let runtime = SolvraRuntime::new();
    let value = runtime
        .execute(&bytecode.into_bytes())
        .expect("execution succeeds");
    assert_eq!(value, Value::Float(3.0));
}

#[test]
fn executes_recursive_function() {
    let span = synthetic();
    let factorial = Function::new(
        "fact",
        vec!["n".into()],
        vec![Stmt::If {
            condition: Expr::binary(
                BinaryOp::LessEqual,
                Expr::identifier("n"),
                Expr::number(1.0),
            ),
            then_branch: vec![Stmt::Return(Some(Expr::number(1.0)), span.clone())],
            else_branch: vec![Stmt::Return(
                Some(Expr::binary(
                    BinaryOp::Multiply,
                    Expr::identifier("n"),
                    Expr::call(
                        Expr::identifier("fact"),
                        vec![Expr::binary(
                            BinaryOp::Subtract,
                            Expr::identifier("n"),
                            Expr::number(1.0),
                        )],
                    ),
                )),
                span.clone(),
            )],
            span: span.clone(),
        }],
        span.clone(),
    );
    let ast = Ast::new(
        vec![factorial],
        vec![Stmt::Return(
            Some(Expr::call(
                Expr::identifier("fact"),
                vec![Expr::number(5.0)],
            )),
            span.clone(),
        )],
    );
    let bytecode = assemble(&ast).expect("assemble factorial");
    let runtime = SolvraRuntime::new();
    let value = runtime
        .execute(&bytecode.into_bytes())
        .expect("exec factorial");
    assert_eq!(value, Value::Float(120.0));
}

#[test]
fn handles_try_catch() {
    let span = synthetic();
    let ast = Ast::new(
        Vec::new(),
        vec![Stmt::Try {
            try_block: vec![Stmt::Throw {
                expr: Expr::string("boom"),
                span: span.clone(),
            }],
            catch_name: "err".into(),
            catch_block: vec![Stmt::Return(Some(Expr::identifier("err")), span.clone())],
            finally_block: Vec::new(),
            span: span.clone(),
        }],
    );
    let bytecode = assemble(&ast).expect("assemble try");
    let runtime = SolvraRuntime::new();
    let value = runtime.execute(&bytecode.into_bytes()).expect("exec try");
    assert_eq!(value, Value::String("boom".into()));
}

#[test]
fn builtins_file_io() {
    let span = synthetic();
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("solvra_runtime.txt");
    let path_str = path.to_string_lossy().to_string();
    let ast = Ast::new(
        Vec::new(),
        vec![
            Stmt::Expr(
                Expr::call(
                    Expr::identifier("write_file"),
                    vec![Expr::string(&path_str), Expr::string("solvra")],
                ),
                span.clone(),
            ),
            Stmt::Return(
                Some(Expr::call(
                    Expr::identifier("read_file"),
                    vec![Expr::string(&path_str)],
                )),
                span.clone(),
            ),
        ],
    );
    let bytecode = assemble(&ast).expect("assemble io");
    let runtime = SolvraRuntime::new();
    let value = runtime.execute(&bytecode.into_bytes()).expect("exec io");
    assert_eq!(value, Value::String("solvra".into()));
}

#[test]
fn reports_stack_trace_on_error() {
    let span = synthetic();
    let crash = Function::new(
        "crash",
        vec![],
        vec![Stmt::Return(
            Some(Expr::binary(
                BinaryOp::Divide,
                Expr::number(1.0),
                Expr::number(0.0),
            )),
            span.clone(),
        )],
        span.clone(),
    );
    let ast = Ast::new(
        vec![crash],
        vec![Stmt::Return(
            Some(Expr::call(Expr::identifier("crash"), Vec::new())),
            span.clone(),
        )],
    );
    let bytecode = assemble(&ast).expect("assemble crash");
    let runtime = SolvraRuntime::new();
    let err = runtime
        .execute(&bytecode.into_bytes())
        .expect_err("runtime error");
    match err {
        SolvraError::RuntimeException { stack, .. } => {
            assert!(!stack.is_empty(), "stack trace should not be empty");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
