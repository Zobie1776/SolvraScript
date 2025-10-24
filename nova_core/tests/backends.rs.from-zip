use nova_core::backend::{active_target, TargetArch};
use nova_core::bytecode::assemble;
use nova_core::bytecode::ast::{Ast, Expr};
use nova_core::{NovaRuntime, Value};

fn sample_program() -> Vec<u8> {
    let ast = Ast::from_expr(Expr::number(42.0));
    assemble(&ast).expect("assemble sample").into_bytes()
}

#[cfg(feature = "backend-x86_64")]
#[test]
fn x86_backend_executes_program() {
    let runtime = NovaRuntime::new();
    assert_eq!(runtime.target_arch(), TargetArch::X86_64);
    let value = runtime.execute(&sample_program()).expect("execute");
    assert_value_is_42(value);
}

#[cfg(feature = "backend-armv7")]
#[test]
fn armv7_backend_executes_program() {
    let runtime = NovaRuntime::new();
    assert_eq!(runtime.target_arch(), TargetArch::Armv7);
    let value = runtime.execute(&sample_program()).expect("execute");
    assert_value_is_42(value);
}

#[cfg(feature = "backend-aarch64")]
#[test]
fn aarch64_backend_executes_program() {
    let runtime = NovaRuntime::new();
    assert_eq!(runtime.target_arch(), TargetArch::AArch64);
    let value = runtime.execute(&sample_program()).expect("execute");
    assert_value_is_42(value);
}

#[test]
fn target_helper_matches_runtime() {
    let runtime = NovaRuntime::new();
    assert_eq!(runtime.target_arch(), active_target());
}

fn assert_value_is_42(value: Value) {
    match value {
        Value::Integer(n) => assert_eq!(n, 42),
        Value::Float(n) => assert_eq!(n, 42.0),
        other => panic!("unexpected value: {other:?}"),
    }
}
