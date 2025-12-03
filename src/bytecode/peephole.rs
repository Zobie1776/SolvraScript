use solvra_core::solvrac::{Bytecode, Constant, Function, Opcode};

pub fn optimize(bytecode: &mut Bytecode) {
    for function in &mut bytecode.functions {
        optimize_function(function, &bytecode.constants);
    }
}

fn optimize_function(function: &mut Function, constants: &[Constant]) {
    let mut optimized = Vec::with_capacity(function.instructions.len());
    let mut index = 0;
    while index < function.instructions.len() {
        if remove_add_zero(function, constants, index) {
            index += 2;
            continue;
        }
        if remove_mul_one(function, constants, index) {
            index += 2;
            continue;
        }
        if collapse_duplicate_constant(function, index) {
            index += 1;
            continue;
        }
        optimized.push(function.instructions[index].clone());
        index += 1;
    }
    function.instructions = optimized;
}

fn remove_add_zero(function: &Function, constants: &[Constant], index: usize) -> bool {
    if index + 1 >= function.instructions.len() {
        return false;
    }
    let load = &function.instructions[index];
    let add_inst = &function.instructions[index + 1];
    if load.opcode != Opcode::LoadConst || add_inst.opcode != Opcode::Add {
        return false;
    }
    let const_index = load.operand_a as usize;
    constants
        .get(const_index)
        .map(|value| is_zero(value))
        .unwrap_or(false)
}

fn remove_mul_one(function: &Function, constants: &[Constant], index: usize) -> bool {
    if index + 1 >= function.instructions.len() {
        return false;
    }
    let load = &function.instructions[index];
    let mul_inst = &function.instructions[index + 1];
    if load.opcode != Opcode::LoadConst || mul_inst.opcode != Opcode::Mul {
        return false;
    }
    let const_index = load.operand_a as usize;
    constants
        .get(const_index)
        .map(|value| is_one(value))
        .unwrap_or(false)
}

fn collapse_duplicate_constant(function: &Function, index: usize) -> bool {
    if index + 2 >= function.instructions.len() {
        return false;
    }
    let first = &function.instructions[index];
    let second = &function.instructions[index + 1];
    let follower = &function.instructions[index + 2];
    if first.opcode != Opcode::LoadConst || second.opcode != Opcode::LoadConst {
        return false;
    }
    if first.operand_a != second.operand_a {
        return false;
    }
    safe_duplicate_successor(follower.opcode)
}

fn safe_duplicate_successor(opcode: Opcode) -> bool {
    matches!(opcode, Opcode::Pop | Opcode::Return | Opcode::CoreReturn)
}

fn is_zero(constant: &Constant) -> bool {
    match constant {
        Constant::Integer(value) => *value == 0,
        Constant::Float(value) => *value == 0.0,
        _ => false,
    }
}

fn is_one(constant: &Constant) -> bool {
    match constant {
        Constant::Integer(value) => *value == 1,
        Constant::Float(value) => *value == 1.0,
        _ => false,
    }
}
