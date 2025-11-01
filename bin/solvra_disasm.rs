use anyhow::{Result, anyhow};
use solvrascript::vm::{
    bytecode::{VmBytecode, VmConstant},
    instruction::Opcode,
};
use std::{env, fs};

fn main() -> Result<()> {
    let input = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("no input file provided"))?;
    let data = fs::read(&input)?;
    let bytecode = VmBytecode::decode(&data[..])?;

    let function_names: Vec<_> = bytecode
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect();

    for (index, function) in bytecode.functions.iter().enumerate() {
        println!(
            "function {}: {} (arity {}, locals {})",
            index, function.name, function.arity, function.locals
        );

        let mut offset = 0usize;
        for instruction in &function.instructions {
            println!(
                "  {:04}: {}",
                offset,
                format_instruction(instruction, &bytecode.constants, &function_names)
            );
            offset += 1 + operand_count(instruction);
        }
        println!();
    }

    Ok(())
}

fn operand_count(instruction: &solvrascript::vm::instruction::Instruction) -> usize {
    match instruction.opcode {
        Opcode::Call | Opcode::CallBuiltin | Opcode::CallAsync => 2,
        Opcode::LoadConst
        | Opcode::LoadVar
        | Opcode::StoreVar
        | Opcode::Jump
        | Opcode::JumpIfFalse
        | Opcode::MakeList
        | Opcode::LoadLambda => 1,
        _ => 0,
    }
}

fn format_instruction(
    instruction: &solvrascript::vm::instruction::Instruction,
    constants: &[VmConstant],
    function_names: &[String],
) -> String {
    match instruction.opcode {
        Opcode::LoadConst => {
            let index = instruction.operand_a as usize;
            let value = constants
                .get(index)
                .map(constant_to_string)
                .unwrap_or_else(|| "?".into());
            format!("LoadConst {} ({})", instruction.operand_a, value)
        }
        Opcode::LoadVar => format!("LoadVar {}", instruction.operand_a),
        Opcode::StoreVar => format!("StoreVar {}", instruction.operand_a),
        Opcode::Add => "Add".to_string(),
        Opcode::Sub => "Sub".to_string(),
        Opcode::Mul => "Mul".to_string(),
        Opcode::Div => "Div".to_string(),
        Opcode::Mod => "Mod".to_string(),
        Opcode::Neg => "Neg".to_string(),
        Opcode::Not => "Not".to_string(),
        Opcode::Pop => "Pop".to_string(),
        Opcode::Jump => format!("Jump {}", instruction.operand_a),
        Opcode::JumpIfFalse => format!("JumpIfFalse {}", instruction.operand_a),
        Opcode::MakeList => format!("MakeList {}", instruction.operand_a),
        Opcode::LoadLambda => format!("LoadLambda {}", instruction.operand_a),
        Opcode::Equal => "Equal".to_string(),
        Opcode::NotEqual => "NotEqual".to_string(),
        Opcode::Less => "Less".to_string(),
        Opcode::LessEqual => "LessEqual".to_string(),
        Opcode::Greater => "Greater".to_string(),
        Opcode::GreaterEqual => "GreaterEqual".to_string(),
        Opcode::And => "And".to_string(),
        Opcode::Or => "Or".to_string(),
        Opcode::Call => {
            let callee = instruction.operand_a as usize;
            let name = function_names
                .get(callee)
                .map(|name| name.as_str())
                .unwrap_or("<unknown>");
            format!("Call {} ({} args)", name, instruction.operand_b)
        }
        Opcode::CallBuiltin => {
            let index = instruction.operand_a as usize;
            let name = constants
                .get(index)
                .and_then(|constant| match constant {
                    VmConstant::String(value) => Some(value.as_str()),
                    _ => None,
                })
                .unwrap_or("<builtin>");
            format!("CallBuiltin {} ({} args)", name, instruction.operand_b)
        }
        Opcode::CallAsync => {
            let callee = instruction.operand_a as usize;
            let name = function_names
                .get(callee)
                .map(|name| name.as_str())
                .unwrap_or("<unknown>");
            format!("CallAsync {} ({} args)", name, instruction.operand_b)
        }
        Opcode::Await => "Await".to_string(),
        Opcode::Return => "Return".to_string(),
        Opcode::Nop => "Nop".to_string(),
    }
}

fn constant_to_string(constant: &VmConstant) -> String {
    match constant {
        VmConstant::Null => "null".to_string(),
        VmConstant::Bool(value) => value.to_string(),
        VmConstant::Int(value) => value.to_string(),
        VmConstant::Float(value) => value.to_string(),
        VmConstant::String(value) => value.clone(),
    }
}
