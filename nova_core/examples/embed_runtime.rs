use nova_core::bytecode::{
    assemble,
    ast::{Ast, Expr},
};
use nova_core::NovaRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Ast::from_expr(Expr::number(7.0));
    let bytecode = assemble(&ast)?;
    let runtime = NovaRuntime::new();
    let value = runtime.execute(&bytecode.into_bytes())?;
    println!("result: {:?}", value);
    Ok(())
}
