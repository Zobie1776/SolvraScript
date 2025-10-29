use novascript::{
    interpreter::{Interpreter, Value},
    parser::Parser,
    tokenizer::Tokenizer,
};

fn eval_source(source: &str) -> Value {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize source");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse program");
    let mut interpreter = Interpreter::new();
    interpreter
        .eval_program(&program)
        .expect("execution")
        .unwrap_or(Value::Null)
}

#[test]
fn import_vector_module_executes() {
    let source = r#"
        import <vector>;
        let mut numbers = vector.make();
        numbers = vector.append(numbers, 10);
        numbers = vector.append(numbers, 32);
        vector.length(numbers)
    "#;
    let result = eval_source(source);
    assert_eq!(result, Value::Int(2));
}

#[test]
fn import_script_module_executes() {
    let source = r#"
        import "tests/modules/sample_module.ns" as sample;
        sample.double(21)
    "#;
    let result = eval_source(source);
    assert_eq!(result, Value::Int(42));
}

#[test]
fn core_memory_stats_builtin_exposes_contract() {
    let source = r#"
        let stats = core_memory_stats();
        stats.used_bytes == 0 && stats.capacity_bytes > 0 && stats.allocations == 0
    "#;
    let result = eval_source(source);
    assert_eq!(result, Value::Bool(true));
}
