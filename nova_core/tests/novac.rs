use nova_core::novac::{self, Bytecode, Constant, Function, Instruction, Opcode};

#[test]
fn assemble_and_roundtrip() {
    let source = r#"
.version 1

.constants
string "Hello"
int 42
float 3.14
.end

.function main 0
    LOAD_CONST 0
    LOAD_CONST 1
    ADD
    LOAD_CONST 2
    CMP_LT
    JUMP_IF_FALSE end
    RETURN
end:
    LOAD_CONST 1
    RETURN
.end

.function helper 1
    LOAD_VAR 0
    RETURN
.end
"#;

    let bytecode = novac::assemble(source).expect("assembly should succeed");
    let bytes = bytecode.encode().expect("encoding should succeed");
    let decoded = Bytecode::decode(&bytes).expect("decoding should succeed");
    assert_eq!(bytecode, decoded);

    let assembly = novac::disassemble(&decoded).expect("disassembly should succeed");
    let reassembled = novac::assemble(&assembly).expect("reassembly should succeed");
    assert_eq!(decoded, reassembled);

    assert_eq!(decoded.constants.len(), 3);
    assert!(matches!(decoded.constants[0], Constant::String(_)));
}

#[test]
fn disassembly_labels_are_emitted() {
    let bytecode = Bytecode::new(
        vec![Constant::Integer(1)],
        vec![Function::new(
            "main",
            0,
            vec![
                Instruction::new(Opcode::Jump, vec![1]),
                Instruction::new(Opcode::Return, vec![]),
            ],
        )],
    );

    let assembly = novac::disassemble(&bytecode).expect("disassembly should succeed");
    assert!(assembly.contains("L1:"));
}
