use solvrascript::interpreter::{RuntimeError, ScriptError};
use solvrascript::modules::ModuleError;
use solvrascript::parser::Parser;
use solvrascript::tokenizer::Tokenizer;

#[test]
fn parse_error_uses_e001() {
    let mut tokenizer = Tokenizer::new("fn demo(");
    let tokens = tokenizer.tokenize().expect("tokenize");
    let mut parser = Parser::new(tokens);
    let err = parser.parse().expect_err("should fail");
    let script_err: ScriptError = err.into();
    assert_eq!(script_err.code_str(), "E001");
}

#[test]
fn module_error_uses_e002() {
    let err = ModuleError::NotFound {
        module: "missing".to_string(),
    };
    let script_err: ScriptError = err.into();
    assert_eq!(script_err.code_str(), "E002");
}

#[test]
fn runtime_type_error_maps_to_e003() {
    let err = RuntimeError::TypeError("expected number".into());
    let script_err: ScriptError = err.clone().into();
    assert_eq!(script_err.code_str(), "E003");
    assert_eq!(err.code(), "E003");
}
