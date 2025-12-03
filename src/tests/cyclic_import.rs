use crate::interpreter::Interpreter;

#[test]
fn test_math_module_import_does_not_hang() {
    let mut interpreter = Interpreter::with_std();
    let code = r#"
        import "stdx/math" as math;
        let x = math::PI;
    "#;
    let result = interpreter.eval_program_with_origin(
        &crate::parser::Parser::new(crate::tokenizer::Tokenizer::new(code).tokenize().unwrap()).parse().unwrap(),
        None::<&std::path::Path>,
    );
    assert!(result.is_ok(), "Importing stdx/math should not fail, but failed with: {:?}", result.err());
}

#[test]fn test_datetime_module_import_does_not_hang() {
    let mut interpreter = Interpreter::with_std();
    let code = r#"
        import "stdx/datetime" as dt;
        let x = dt::now();
    "#;
    let result = interpreter.eval_program_with_origin(
        &crate::parser::Parser::new(crate::tokenizer::Tokenizer::new(code).tokenize().unwrap()).parse().unwrap(),
        None::<&std::path::Path>,
    );
    assert!(result.is_ok(), "Importing stdx/datetime should not fail, but failed with: {:?}", result.err());
}
