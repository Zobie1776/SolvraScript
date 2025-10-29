use nova_core::bytecode::assemble;
use nova_core::bytecode::ast::{Ast, BinaryOp, Expr, Span, Stmt};
#[allow(unused_imports)]
use nova_core::sys::drivers::DriverDescriptor;
use nova_core::{NovaRuntime, Value};

fn synthetic() -> Span {
    Span::synthetic()
}

#[test]
fn driver_native_functions_work() {
    let runtime = NovaRuntime::new();
    let span = synthetic();
    let ast = Ast::new(
        Vec::new(),
        vec![
            Stmt::Expr(
                Expr::call(
                    Expr::identifier("driver_register"),
                    vec![Expr::string("virtual_device"), Expr::number(2.0)],
                ),
                span.clone(),
            ),
            Stmt::Expr(
                Expr::call(
                    Expr::identifier("driver_write_u32"),
                    vec![
                        Expr::string("virtual_device"),
                        Expr::number(0.0),
                        Expr::number(41.0),
                    ],
                ),
                span.clone(),
            ),
            Stmt::Expr(
                Expr::call(
                    Expr::identifier("driver_raise_interrupt"),
                    vec![
                        Expr::string("virtual_device"),
                        Expr::number(1.0),
                        Expr::number(1.0),
                    ],
                ),
                span.clone(),
            ),
            Stmt::Return(
                Some(Expr::call(
                    Expr::identifier("driver_read_u32"),
                    vec![Expr::string("virtual_device"), Expr::number(0.0)],
                )),
                span.clone(),
            ),
        ],
    );
    let bytecode = assemble(&ast).expect("assemble driver program");
    let value = runtime
        .execute(&bytecode.into_bytes())
        .expect("execute driver program");
    assert_eq!(value, Value::Integer(41));

    let interrupt = runtime
        .driver_registry()
        .next_interrupt("virtual_device")
        .expect("driver exists");
    assert!(
        interrupt.is_some(),
        "interrupt raised by program should be queued"
    );
}

#[ignore = "legacy driver API pending refactor"]
#[test]
fn host_signalled_interrupts_are_visible() {
    let runtime = NovaRuntime::new();
    // TODO: restore when runtime loop ready
    // runtime
    //     .register_driver(DriverDescriptor::new("host_device", vec![0, 0]))
    //     .expect("register host driver");
    // runtime
    //     .signal_interrupt("host_device", 5, Some(3))
    //     .expect("queue host interrupt");

    let span = synthetic();
    let ast = Ast::new(
        Vec::new(),
        vec![
            Stmt::Let {
                name: "intr".into(),
                expr: Expr::call(
                    Expr::identifier("driver_next_interrupt"),
                    vec![Expr::string("host_device")],
                ),
                span: span.clone(),
            },
            Stmt::Return(
                Some(Expr::binary(
                    BinaryOp::Add,
                    Expr::index(Expr::identifier("intr"), Expr::number(0.0)),
                    Expr::index(Expr::identifier("intr"), Expr::number(1.0)),
                )),
                span.clone(),
            ),
        ],
    );
    let bytecode = assemble(&ast).expect("assemble interrupt program");
    let value = runtime
        .execute(&bytecode.into_bytes())
        .expect("execute interrupt program");
    match value {
        Value::Integer(n) => assert_eq!(n, 8),
        Value::Float(n) => assert_eq!(n, 8.0),
        other => panic!("unexpected interrupt sum: {other:?}"),
    }
}
