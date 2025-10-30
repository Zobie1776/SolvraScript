use criterion::{criterion_group, criterion_main, Criterion};
use solvra_core::bytecode::{
    assemble,
    ast::{Ast, BinaryOp, Expr},
};
use solvra_core::SolvraRuntime;

fn fib_expr(n: u32) -> Expr {
    if n <= 1 {
        Expr::number(1.0)
    } else {
        Expr::binary(BinaryOp::Add, fib_expr(n - 1), fib_expr(n - 2))
    }
}

fn bench_interpreter(c: &mut Criterion) {
    let ast = Ast::from_expr(fib_expr(5));
    let bytecode = assemble(&ast).expect("assemble");
    let bytes = bytecode.into_bytes();
    let runtime = SolvraRuntime::new();
    c.bench_function("fib_interpret", |b| {
        b.iter(|| {
            let _ = runtime.execute(&bytes).unwrap();
        })
    });
}

criterion_group!(benches, bench_interpreter);
criterion_main!(benches);
