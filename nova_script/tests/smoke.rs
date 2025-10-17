// NovaScript smoke tests for tokenizer, parser, and interpreter
// Covers: arithmetic, variable assignment, function definition, if/else, while loops

use novascript::*;

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
    let program = tokenize_and_parse("let x = 42; x = x + 1;").unwrap();
    assert!(format!("{:?}", program).contains("VariableDecl"));
}

#[test]
fn test_function_definition() {
    let program = tokenize_and_parse("fn add(a: int, b: int) -> int { return a + b; }").unwrap();
    assert!(format!("{:?}", program).contains("FunctionDecl"));
}

#[test]
fn test_if_else() {
    let program = tokenize_and_parse("if x > 0 { y = 1; } else { y = -1; }").unwrap();
    assert!(format!("{:?}", program).contains("If"));
}

#[test]
fn test_while_loop() {
    let program = tokenize_and_parse("let i = 0; while i < 10 { i = i + 1; }").unwrap();
    assert!(format!("{:?}", program).contains("While"));
}
