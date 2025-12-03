//=============================================
// solvra_script/vm/tests/control_flow_tests.rs
//=============================================
// Purpose: Validate VM support for break/continue in compiled loops.
//=============================================

use std::sync::Arc;

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use crate::vm::compiler as vm_compiler;
use crate::vm::runtime::{RuntimeOptions, run_bytecode};
use solvra_core::Value;
use solvra_core::vm::bytecode::VmBytecode;

fn compile_program(source: &str) -> Arc<VmBytecode> {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize script");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse program");
    let bytecode = vm_compiler::compile_program(&program).expect("compile program");
    let vm_program = VmBytecode::decode(&bytecode[..]).expect("decode vm bytecode");
    Arc::new(vm_program)
}

#[test]
fn while_loop_breaks_and_continues() {
    let program = compile_program(
        r#"
fn main() {
    let mut sum = 0;
    let mut n = 0;
    while true {
        n = n + 1;
        if n == 3 {
            continue;
        }
        if n == 5 {
            break;
        }
        sum = sum + n;
    }
    return sum;
}
"#,
    );

    let value = run_bytecode(program, RuntimeOptions::default()).expect("run program");
    assert_eq!(value, Value::Integer(7));
}

#[test]
fn break_outside_loop_is_compile_error() {
    let source = r#"
fn main() {
    break;
}
"#;
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize script");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse program");
    let result = vm_compiler::compile_program(&program);
    assert!(
        result.is_err(),
        "expected compile error for break outside loop"
    );
}

//=============================================
// End of file
//=============================================
