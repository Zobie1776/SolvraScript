use solvra_core::bytecode::{
    assemble,
    ast::{Ast, Expr},
};
use solvra_core::SolvraRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Ast::from_expr(Expr::number(7.0));
    let bytecode = assemble(&ast)?;
    let runtime = SolvraRuntime::new();
    let value = runtime.execute(&bytecode.into_bytes())?;
    println!("result: {:?}", value);
    Ok(())
}
