use nova_core::bytecode::{
    assemble,
    ast::{Ast, Expr},
};
use nova_core::NovaRuntime;

#[test]
fn assemble_and_execute() {
    let ast = Ast::from_expr(Expr::number(42.0));
    let bytecode = assemble(&ast).expect("assemble");
    let runtime = NovaRuntime::new();
    let value = runtime.execute(&bytecode.into_bytes()).expect("exec");
    assert_eq!(value, nova_core::Value::Float(42.0));
}
