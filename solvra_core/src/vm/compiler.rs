use super::bytecode::{VmBytecode, VmConstant, VmFunction};
use super::instruction::{Instruction, Opcode};
use crate::solvrac;

pub fn from_solvrac(bytecode: &solvrac::Bytecode) -> VmBytecode {
    let constants = bytecode
        .constants
        .iter()
        .map(convert_constant)
        .collect::<Vec<_>>();

    let functions = bytecode
        .functions
        .iter()
        .map(|function| {
            let instructions = function
                .instructions
                .iter()
                .map(convert_instruction)
                .collect::<Vec<_>>();
            let locals = calculate_local_count(function);
            VmFunction {
                name: function.name.clone(),
                arity: function.parameters,
                locals,
                instructions,
            }
        })
        .collect();

    VmBytecode {
        version: 1,
        constants,
        functions,
        entry: 0,
    }
}

fn convert_instruction(instruction: &solvrac::Instruction) -> Instruction {
    let operand_a = *instruction.operands.first().unwrap_or(&0);
    let operand_b = *instruction.operands.get(1).unwrap_or(&0);
    Instruction::new(map_opcode(instruction.opcode), operand_a, operand_b, None)
}

fn calculate_local_count(function: &solvrac::Function) -> u16 {
    let mut max_slot = function.parameters as u32;
    for instruction in &function.instructions {
        if matches!(
            instruction.opcode,
            solvrac::Opcode::LoadVar | solvrac::Opcode::StoreVar
        ) {
            if let Some(slot) = instruction.operands.first() {
                max_slot = max_slot.max(slot.saturating_add(1));
            }
        }
    }
    max_slot.min(u32::from(u16::MAX)) as u16
}

fn map_opcode(op: solvrac::Opcode) -> Opcode {
    match op {
        solvrac::Opcode::Nop => Opcode::Nop,
        solvrac::Opcode::LoadConst => Opcode::LoadConst,
        solvrac::Opcode::LoadVar => Opcode::LoadVar,
        solvrac::Opcode::StoreVar => Opcode::StoreVar,
        solvrac::Opcode::Add => Opcode::Add,
        solvrac::Opcode::Sub => Opcode::Sub,
        solvrac::Opcode::Mul => Opcode::Mul,
        solvrac::Opcode::Div => Opcode::Div,
        solvrac::Opcode::Mod => Opcode::Mod,
        solvrac::Opcode::Neg => Opcode::Neg,
        solvrac::Opcode::Not => Opcode::Not,
        solvrac::Opcode::Pop => Opcode::Pop,
        solvrac::Opcode::Jump => Opcode::Jump,
        solvrac::Opcode::JumpIfFalse => Opcode::JumpIfFalse,
        solvrac::Opcode::MakeList => Opcode::MakeList,
        solvrac::Opcode::LoadLambda => Opcode::LoadLambda,
        solvrac::Opcode::CallBuiltin => Opcode::CallBuiltin,
        solvrac::Opcode::Call => Opcode::Call,
        solvrac::Opcode::CallAsync => Opcode::CallAsync,
        solvrac::Opcode::Await => Opcode::Await,
        solvrac::Opcode::Return => Opcode::Return,
        solvrac::Opcode::CmpLt => Opcode::Less,
        solvrac::Opcode::CmpEq => Opcode::Equal,
        solvrac::Opcode::CmpGt => Opcode::Greater,
        solvrac::Opcode::CmpLe => Opcode::LessEqual,
        solvrac::Opcode::CmpGe => Opcode::GreaterEqual,
        solvrac::Opcode::And => Opcode::And,
        solvrac::Opcode::Or => Opcode::Or,
    }
}

fn convert_constant(constant: &solvrac::Constant) -> VmConstant {
    match constant {
        solvrac::Constant::String(value) => VmConstant::String(value.clone()),
        solvrac::Constant::Integer(value) => VmConstant::Int(*value),
        solvrac::Constant::Float(value) => VmConstant::Float(*value),
        solvrac::Constant::Boolean(value) => VmConstant::Bool(*value),
        solvrac::Constant::Null => VmConstant::Null,
    }
}
