// SolvraScript smoke tests for tokenizer, parser, and interpreter
// Covers: arithmetic, variable assignment, function definition, if/else, while loops

use solvrascript::{
    ast::{self, BindingKind, ImportSource, Stmt, Type},
    parser, tokenizer,
};

fn tokenize_and_parse(source: &str) -> Result<ast::Program, parser::ParseError> {
    let mut tokenizer = tokenizer::Tokenizer::new(source);
    let tokens = tokenizer.tokenize().unwrap();
    let mut parser = parser::Parser::new(tokens);
    parser.parse()
}

#[test]
fn test_arithmetic() {
    let program = tokenize_and_parse("1 + 2 * 3 - 4 / 2;").unwrap();
    assert!(format!("{:?}", program).contains("Binary"));
}

#[test]
fn test_variable_assignment() {
    let program = tokenize_and_parse("let mut x: int = 42; x = x + 1;").unwrap();
    match &program.statements[0] {
        Stmt::VariableDecl { decl } => {
            assert_eq!(decl.name, "x");
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
    match &program.statements[0] {
        Stmt::FunctionDecl { decl } => {
            assert_eq!(decl.name, "add");
            assert_eq!(decl.params.len(), 2);
            assert_eq!(decl.return_type, Type::Int);
        }
        other => panic!("expected function declaration, found {other:?}"),
    }
}

#[test]
fn test_if_else() {
    let program = tokenize_and_parse("if x > 0 { y = 1; } else { y = -1; }").unwrap();
    match &program.statements[0] {
        Stmt::If { else_branch, .. } => assert!(else_branch.is_some()),
        other => panic!("expected if statement, found {other:?}"),
    }
}

#[test]
fn test_while_loop() {
    let program = tokenize_and_parse("let i = 0; while i < 10 { i = i + 1; }").unwrap();
    assert!(matches!(program.statements[1], Stmt::While { .. }));
}

#[test]
fn test_const_declaration() {
    let program = tokenize_and_parse("const LIMIT: int = 10;").unwrap();
    let decl = match &program.statements[0] {
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
    let imports = program.find_imports();
    assert_eq!(imports.len(), 1);
    match &imports[0].source {
        ImportSource::ScriptPath(path) => assert_eq!(path, "modules/sample.svs"),
        other => panic!("expected script import, found {other:?}"),
    }
}

#[test]
fn test_import_std_module_syntax() {
    let program = tokenize_and_parse("import <vector>;").unwrap();
    let imports = program.find_imports();
    assert_eq!(imports.len(), 1);
    match &imports[0].source {
        ImportSource::StandardModule(name) => assert_eq!(name, "vector"),
        other => panic!("expected std module import, found {other:?}"),
    }
}

#[test]
fn test_array_type_annotation() {
    let program = tokenize_and_parse("let numbers: [int] = [1, 2, 3];").unwrap();
    let decl = match &program.statements[0] {
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
fn test_else_if_chain() {
    let source = "if x { y = 1; } else if y { y = 2; } else { y = 3; }";
    let program = tokenize_and_parse(source).unwrap();
    match &program.statements[0] {
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
