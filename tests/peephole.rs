use solvra_core::solvrac::{
    Bytecode as SolvracBytecode, Constant, Function as SolvracFunction,
    Instruction as SolvracInstruction, Opcode as SolvracOpcode,
};
use solvra_core::vm::bytecode::VmBytecode;
use solvra_core::vm::instruction::Opcode;
use solvrascript::bytecode::peephole;
use solvrascript::parser::Parser;
use solvrascript::tokenizer::Tokenizer;
use solvrascript::vm::compiler as vm_compiler;

fn compile_vm(source: &str) -> VmBytecode {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize().expect("tokenize program");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse program");
    let bytes = vm_compiler::compile_program(&program).expect("compile program");
    VmBytecode::decode(&bytes[..]).expect("decode bytecode")
}

#[test]
fn peephole_removes_add_zero_sequences() {
    let vm = compile_vm(
        r#"
        fn add_identity(x) {
            return x + 0;
        }
    "#,
    );
    let function = vm
        .functions
        .iter()
        .find(|func| func.name == "add_identity")
        .expect("function to exist");
    assert!(
        !function
            .instructions
            .iter()
            .any(|inst| inst.opcode == Opcode::Add),
        "Add opcode should be removed"
    );
}

#[test]
fn peephole_removes_mul_one_sequences() {
    let vm = compile_vm(
        r#"
        fn mul_identity(x) {
            return x * 1;
        }
    "#,
    );
    let function = vm
        .functions
        .iter()
        .find(|func| func.name == "mul_identity")
        .expect("function to exist");
    assert!(
        !function
            .instructions
            .iter()
            .any(|inst| inst.opcode == Opcode::Mul),
        "Mul opcode should be removed"
    );
}

#[test]
fn peephole_collapses_redundant_constant_loads() {
    let mut bytecode = SolvracBytecode::new(
        vec![Constant::Integer(7)],
        vec![SolvracFunction::new(
            "dup",
            0,
            vec![
                SolvracInstruction::with_operands(SolvracOpcode::LoadConst, &[0]),
                SolvracInstruction::with_operands(SolvracOpcode::LoadConst, &[0]),
                SolvracInstruction::with_operands(SolvracOpcode::Pop, &[]),
                SolvracInstruction::with_operands(SolvracOpcode::Return, &[]),
            ],
        )],
    );
    peephole::optimize(&mut bytecode);
    let function = &bytecode.functions[0];
    let load_count = function
        .instructions
        .iter()
        .filter(|inst| inst.opcode == SolvracOpcode::LoadConst)
        .count();
    assert_eq!(load_count, 1);
}
