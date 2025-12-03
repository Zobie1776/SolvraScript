use solvrascript::{
    ast::{self, Expr, Stmt},
    parser::Parser,
    resolver::{self, Diagnostics},
    tokenizer::Tokenizer,
};

fn parse_program(source: &str) -> ast::Program {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize");
    let mut parser = Parser::new(tokens);
    parser.parse().expect("parse")
}

#[test]
fn resolver_maps_identifier_to_definition() {
    let program = parse_program("fn foo(y: int) { return y; }");
    let mut diagnostics = Diagnostics::new();
    let resolutions = resolver::resolve_module(&program, &mut diagnostics);
    assert!(!diagnostics.has_errors());

    let identifier_id = match &program.statements[0] {
        Stmt::FunctionDecl { decl } => match &decl.body[0] {
            Stmt::Return {
                value: Some(expr), ..
            } => match expr {
                Expr::Identifier { node_id, .. } => *node_id,
                other => panic!("expected identifier in return, found {other:?}"),
            },
            other => panic!("expected return statement, found {other:?}"),
        },
        other => panic!("expected function declaration, found {other:?}"),
    };
    assert!(resolutions.map.contains_key(&identifier_id));
}

#[test]
fn resolver_records_unresolved_identifiers() {
    let program = parse_program("fn foo() { return missing; }");
    let mut diagnostics = Diagnostics::new();
    let resolutions = resolver::resolve_module(&program, &mut diagnostics);
    assert!(resolutions.map.is_empty());
    assert!(diagnostics.has_errors());
}

#[test]
fn resolver_tracks_let_binding_and_usage() {
    let program = parse_program("let value = 3; value;");
    let mut diagnostics = Diagnostics::new();
    let resolutions = resolver::resolve_module(&program, &mut diagnostics);
    assert!(!diagnostics.has_errors());

    let main_body = match &program.statements[0] {
        Stmt::FunctionDecl { decl } => &decl.body,
        other => panic!("expected implicit main function, found {other:?}"),
    };
    let use_id = match &main_body[1] {
        Stmt::Expression {
            expr: Expr::Identifier { node_id, .. },
            ..
        } => *node_id,
        Stmt::Return {
            value: Some(Expr::Identifier { node_id, .. }),
            ..
        } => *node_id,
        other => panic!("expected identifier expression, found {other:?}"),
    };
    assert!(resolutions.map.contains_key(&use_id));
}

#[test]
fn resolver_tracks_for_loop_variable() {
    let program = parse_program("for i in [1, 2] { i; }");
    let mut diagnostics = Diagnostics::new();
    let resolutions = resolver::resolve_module(&program, &mut diagnostics);
    assert!(!diagnostics.has_errors());

    let main_body = match &program.statements[0] {
        Stmt::FunctionDecl { decl } => &decl.body,
        other => panic!("expected implicit main function, found {other:?}"),
    };
    let loop_body = match &main_body[0] {
        Stmt::For { body, .. } => body,
        other => panic!("expected for loop, found {other:?}"),
    };
    let use_id = match loop_body.as_ref() {
        Stmt::Block { statements, .. } => match &statements[0] {
            Stmt::Expression {
                expr: Expr::Identifier { node_id, .. },
                ..
            } => node_id,
            other => panic!("expected identifier inside loop body, found {other:?}"),
        },
        other => panic!("expected block body in loop, found {other:?}"),
    };
    assert!(resolutions.map.contains_key(&use_id));
}
