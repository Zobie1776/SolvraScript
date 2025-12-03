//=====================================================
// File: devtools/repl.rs
//=====================================================
// Author: Codex Agent
// License: Duality Public License (DPL v1.0)
// Goal: Provide REPL-friendly helpers for the legacy interpreter
// Objective: Keep tree-walking evaluation available for debugging without
//            impacting the primary VM execution pipeline
//=====================================================

use crate::ast::{Expr, Stmt};
use crate::interpreter::{Interpreter, RuntimeError, Value};

/// Evaluate an expression using the legacy interpreter for debugging sessions.
pub fn eval_expression(interpreter: &mut Interpreter, expr: &Expr) -> Result<Value, RuntimeError> {
    interpreter.eval_expression(expr)
}

/// Evaluate a statement using the interpreter, returning its resulting value.
pub fn eval_statement(
    interpreter: &mut Interpreter,
    stmt: &Stmt,
) -> Result<Option<Value>, RuntimeError> {
    interpreter.eval_statement(stmt)
}

//=====================================================
// End of file
//=====================================================
