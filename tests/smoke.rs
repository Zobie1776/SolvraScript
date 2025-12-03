// SolvraScript smoke tests for tokenizer, parser, and interpreter
// Covers: arithmetic, variable assignment, function definition, if/else, while loops

use solvrascript::{
    ast::{self, BinaryOp, BindingKind, Expr, ImportSource, Stmt, Type},
    parser, tokenizer,
};

fn tokenize_and_parse(source: &str) -> Result<ast::Program, parser::ParseError> {
    let mut tokenizer = tokenizer::Tokenizer::new(source);
    let tokens = tokenizer.tokenize().unwrap();
    let mut parser = parser::Parser::new(tokens);
    parser.parse()
}

fn program_body<'a>(program: &'a ast::Program) -> &'a [ast::Stmt] {
    if program.implicit_entry {
        if let Some(ast::Stmt::FunctionDecl { decl }) = program.statements.iter().find(|stmt| {
            matches!(
                stmt,
                ast::Stmt::FunctionDecl { decl } if decl.name.as_str() == "main"
            )
        }) {
            return &decl.body;
        }
    }
    &program.statements
}

#[test]
fn test_arithmetic() {
    let program = tokenize_and_parse("1 + 2 * 3 - 4 / 2;").unwrap();
    assert!(format!("{:?}", program).contains("Binary"));
}

#[test]
fn test_variable_assignment() {
    let program = tokenize_and_parse("let mut x: int = 42; x = x + 1;").unwrap();
    match &program_body(&program)[0] {
        Stmt::VariableDecl { decl } => {
            assert_eq!(decl.name.as_str(), "x");
            assert!(decl.is_mutable);
            assert!(matches!(decl.binding, BindingKind::Let));
            assert_eq!(decl.var_type, Type::Int);
        }
        other => panic!("expected variable declaration, found {other:?}"),
    }
}

#[test]
fn test_function_definition() {
    let program = tokenize_and_parse("fn add(a: int, b: int) -> int { return a + b; }").unwrap();
    match &program_body(&program)[0] {
        Stmt::FunctionDecl { decl } => {
            assert_eq!(decl.name.as_str(), "add");
            assert_eq!(decl.params.len(), 2);
            assert_eq!(decl.return_type, Type::Int);
        }
        other => panic!("expected function declaration, found {other:?}"),
    }
}

#[test]
fn test_if_else() {
    let program = tokenize_and_parse("if x > 0 { y = 1; } else { y = -1; }").unwrap();
    match &program_body(&program)[0] {
        Stmt::If { else_branch, .. } => assert!(else_branch.is_some()),
        other => panic!("expected if statement, found {other:?}"),
    }
}

#[test]
fn test_while_loop() {
    let program = tokenize_and_parse("let i = 0; while i < 10 { i = i + 1; }").unwrap();
    assert!(matches!(program_body(&program)[1], Stmt::While { .. }));
}

#[test]
fn test_const_declaration() {
    let program = tokenize_and_parse("const LIMIT: int = 10;").unwrap();
    let decl = match &program_body(&program)[0] {
        Stmt::VariableDecl { decl } => decl,
        other => panic!("expected variable declaration, found {other:?}"),
    };
    assert!(matches!(decl.binding, BindingKind::Const));
    assert!(!decl.is_mutable);
    assert_eq!(decl.var_type, Type::Int);
}

#[test]
fn test_import_string_module_syntax() {
    let program = tokenize_and_parse("import \"modules/sample.svs\";").unwrap();
    let imports: Vec<_> = program_body(&program)
        .iter()
        .filter_map(|stmt| {
            if let Stmt::ImportDecl { decl } = stmt {
                Some(decl)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(imports.len(), 1);
    match &imports[0].source {
        ImportSource::ScriptPath(path) => assert_eq!(path, "modules/sample.svs"),
        other => panic!("expected script import, found {other:?}"),
    }
}

#[test]
fn test_import_std_module_syntax() {
    let program = tokenize_and_parse("import <vector>;").unwrap();
    let imports: Vec<_> = program_body(&program)
        .iter()
        .filter_map(|stmt| {
            if let Stmt::ImportDecl { decl } = stmt {
                Some(decl)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(imports.len(), 1);
    match &imports[0].source {
        ImportSource::StandardModule(name) => assert_eq!(name, "vector"),
        other => panic!("expected std module import, found {other:?}"),
    }
}

#[test]
fn test_array_type_annotation() {
    let program = tokenize_and_parse("let numbers: [int] = [1, 2, 3];").unwrap();
    let decl = match &program_body(&program)[0] {
        Stmt::VariableDecl { decl } => decl,
        other => panic!("expected variable declaration, found {other:?}"),
    };
    match &decl.var_type {
        Type::Array(inner) => assert!(matches!(**inner, Type::Int)),
        other => panic!("expected array type, found {other:?}"),
    }
}

#[test]
fn test_const_requires_initializer() {
    let error = tokenize_and_parse("const FLAG: bool;").unwrap_err();
    assert!(matches!(
        error,
        parser::ParseError::InvalidSyntax { message, .. } if message.contains("initializer")
    ));
}

#[test]
fn test_slice_expression_parsing() {
    let program = tokenize_and_parse("let x = arr[1:5:2];").unwrap();
    let initializer = match &program_body(&program)[0] {
        Stmt::VariableDecl { decl } => decl
            .initializer
            .as_ref()
            .expect("expected initializer on variable"),
        other => panic!("expected variable declaration, found {other:?}"),
    };
    match initializer {
        Expr::Slice {
            start, end, step, ..
        } => {
            assert!(start.is_some());
            assert!(end.is_some());
            assert!(step.is_some());
        }
        other => panic!("expected slice expression, found {other:?}"),
    }
}

#[test]
fn test_compound_assignment_desugars() {
    let program = tokenize_and_parse("let mut x = 1; x += 2;").unwrap();
    let assign_expr = match &program_body(&program)[1] {
        Stmt::Expression { expr, .. } => expr,
        Stmt::Return {
            value: Some(expr), ..
        } => expr,
        other => panic!("expected expression statement, found {other:?}"),
    };
    match assign_expr {
        Expr::Assign { value, .. } => match &**value {
            Expr::Binary { operator, .. } => assert_eq!(*operator, BinaryOp::Add),
            other => panic!("expected binary expression inside assignment, found {other:?}"),
        },
        other => panic!("expected assignment expression, found {other:?}"),
    }
}

#[test]
fn test_elif_chain_parses_as_nested_if() {
    let program = tokenize_and_parse("if a { b; } elif c { d; } else { e; }").unwrap();
    let else_branch = match &program_body(&program)[0] {
        Stmt::If { else_branch, .. } => else_branch,
        other => panic!("expected if statement, found {other:?}"),
    };
    let nested = else_branch
        .as_ref()
        .expect("expected elif branch to be present");
    match nested.as_ref() {
        Stmt::If { else_branch, .. } => {
            assert!(else_branch.is_some());
        }
        other => panic!("expected nested if in elif branch, found {other:?}"),
    }
}

#[test]
fn test_for_loop_node_id_populated() {
    let program = tokenize_and_parse("for i in items { i; }").unwrap();
    match &program_body(&program)[0] {
        Stmt::For { node_id, .. } => assert!(*node_id != 0),
        other => panic!("expected for loop, found {other:?}"),
    }
}

#[test]
fn test_combined_language_features() {
    let source = "\
        let mut total = 0; \
        let nums = [1, 2, 3, 4]; \
        for n in nums[1:3] { total += n; } \
        if not (total is 0) and total > 0 or false { total -= 1; } \
        elif total == 0 { total = 1; } \
        else { total = 2; }";
    let program = tokenize_and_parse(source).unwrap();
    let body = program_body(&program);

    let for_stmt = match &body[2] {
        Stmt::For {
            iterable,
            node_id,
            body,
            ..
        } => {
            assert!(*node_id != 0);
            assert!(matches!(iterable, Expr::Slice { .. }));
            body
        }
        other => panic!("expected for loop, found {other:?}"),
    };

    // compound assignment inside loop lowers to an assignment with a binary add.
    match for_stmt.as_ref() {
        Stmt::Block { statements, .. } => match &statements[0] {
            Stmt::Expression {
                expr: Expr::Assign { value, .. },
                ..
            } => {
                assert!(matches!(
                    value.as_ref(),
                    Expr::Binary {
                        operator: BinaryOp::Add,
                        ..
                    }
                ));
            }
            other => panic!("expected assignment expression in loop body, found {other:?}"),
        },
        other => panic!("expected block body in for loop, found {other:?}"),
    }

    // elif chain should become nested if nodes.
    match &body[3] {
        Stmt::If {
            condition,
            else_branch,
            ..
        } => {
            // boolean expression uses not/and/or/is operators
            assert!(matches!(
                condition,
                Expr::Binary {
                    operator: BinaryOp::Or,
                    ..
                }
            ));
            let nested = else_branch.as_ref().expect("elif branch present");
            assert!(matches!(nested.as_ref(), Stmt::If { .. }));
        }
        other => panic!("expected if statement, found {other:?}"),
    }
}

#[test]
fn test_string_literals_with_colons_and_pipes() {
    tokenize_and_parse(r#"print("key:value");"#).unwrap();
    tokenize_and_parse(r#"print("hello | world");"#).unwrap();
    tokenize_and_parse(r#"print("x/y/z");"#).unwrap();
}

#[test]
fn test_double_colon_namespace_access() {
    let program = tokenize_and_parse("let cfg = toml::load();").unwrap();
    assert!(matches!(
        program_body(&program)[0],
        Stmt::VariableDecl { .. }
    ));
}

#[test]
fn test_else_if_chain() {
    let source = "if x { y = 1; } else if y { y = 2; } else { y = 3; }";
    let program = tokenize_and_parse(source).unwrap();
    match &program_body(&program)[0] {
        Stmt::If { else_branch, .. } => {
            let else_branch = else_branch.as_ref().expect("expected else branch");
            match &**else_branch {
                Stmt::If { .. } => {} // else-if is represented as nested if
                other => panic!("expected nested if for else-if, found {other:?}"),
            }
        }
        other => panic!("expected if statement, found {other:?}"),
    }
}
