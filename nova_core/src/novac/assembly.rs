use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use super::format::{Bytecode, Constant, Function, Instruction, NovacError, Opcode, VERSION};

#[derive(Debug, Default)]
struct FunctionBuilder {
    name: String,
    params: u16,
    body: Vec<Line>,
}

#[derive(Debug)]
enum Line {
    Label(String, usize),
    Instruction(String, Vec<String>, usize),
}

/// Assemble NovaCore textual assembly into bytecode.
pub fn assemble(source: &str) -> Result<Bytecode, NovacError> {
    let mut version = VERSION;
    let mut constants = Vec::new();
    let mut function_builders = Vec::new();
    let mut in_constants = false;
    let mut current_function: Option<FunctionBuilder> = None;

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line).trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Some(builder) = current_function.as_mut() {
            if line == ".end" {
                let finished = current_function.take().unwrap();
                function_builders.push(finished);
                continue;
            }

            if let Some(label) = line.strip_suffix(':') {
                builder
                    .body
                    .push(Line::Label(label.trim().to_string(), line_no));
            } else {
                let mut parts = line.split_whitespace();
                let opcode = parts
                    .next()
                    .ok_or_else(|| {
                        NovacError::Message(format!("missing opcode at line {line_no}"))
                    })?
                    .to_string();
                let operands = parts.map(|p| p.to_string()).collect();
                builder
                    .body
                    .push(Line::Instruction(opcode, operands, line_no));
            }
            continue;
        }

        if in_constants {
            if line == ".end" {
                in_constants = false;
                continue;
            }
            let mut parts = line.split_whitespace();
            let kind = parts.next().ok_or_else(|| {
                NovacError::Message(format!("invalid constant declaration at line {line_no}"))
            })?;
            match kind {
                "string" => {
                    let value = parts.collect::<Vec<_>>().join(" ");
                    let literal = parse_string_literal(&value, line_no)?;
                    constants.push(Constant::String(literal));
                }
                "int" => {
                    let value = parts.next().ok_or_else(|| {
                        NovacError::Message(format!("missing integer constant at line {line_no}"))
                    })?;
                    if parts.next().is_some() {
                        return Err(NovacError::Message(format!(
                            "unexpected tokens after integer constant at line {line_no}"
                        )));
                    }
                    let number = value.parse::<i64>().map_err(|_| {
                        NovacError::Message(format!(
                            "invalid integer literal {value} at line {line_no}"
                        ))
                    })?;
                    constants.push(Constant::Integer(number));
                }
                "float" => {
                    let value = parts.next().ok_or_else(|| {
                        NovacError::Message(format!("missing float constant at line {line_no}"))
                    })?;
                    if parts.next().is_some() {
                        return Err(NovacError::Message(format!(
                            "unexpected tokens after float constant at line {line_no}"
                        )));
                    }
                    let number = value.parse::<f64>().map_err(|_| {
                        NovacError::Message(format!(
                            "invalid float literal {value} at line {line_no}"
                        ))
                    })?;
                    constants.push(Constant::Float(number));
                }
                other => {
                    return Err(NovacError::Message(format!(
                        "unknown constant type {other} at line {line_no}"
                    )));
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(".version") {
            let parts: Vec<_> = rest.trim().split_whitespace().collect();
            if parts.len() != 1 {
                return Err(NovacError::Message(format!(
                    "expected .version <number> at line {line_no}"
                )));
            }
            version = parse_u8(parts[0], line_no)?;
            if version != VERSION {
                return Err(NovacError::UnsupportedVersion(version));
            }
            continue;
        }

        if line == ".constants" {
            if in_constants {
                return Err(NovacError::Message(
                    "nested .constants sections are not allowed".to_string(),
                ));
            }
            in_constants = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix(".function") {
            let parts: Vec<_> = rest.trim().split_whitespace().collect();
            if parts.len() != 2 {
                return Err(NovacError::Message(format!(
                    "expected .function <name> <params> at line {line_no}"
                )));
            }
            let name = parts[0].to_string();
            let params = parse_u16(parts[1], line_no)?;
            current_function = Some(FunctionBuilder {
                name,
                params,
                body: Vec::new(),
            });
            continue;
        }

        return Err(NovacError::Message(format!(
            "unexpected directive {line} at line {line_no}"
        )));
    }

    if in_constants {
        return Err(NovacError::Message(
            "unterminated .constants block".to_string(),
        ));
    }

    if current_function.is_some() {
        return Err(NovacError::Message(
            "unterminated function definition".to_string(),
        ));
    }

    let mut name_map = HashMap::new();
    for (index, builder) in function_builders.iter().enumerate() {
        if name_map
            .insert(builder.name.clone(), index as u32)
            .is_some()
        {
            return Err(NovacError::DuplicateFunction(builder.name.clone()));
        }
    }

    let mut functions = Vec::new();
    for builder in &function_builders {
        let function = finalize_function(builder, &name_map)?;
        functions.push(function);
    }

    Ok(Bytecode {
        version,
        constants,
        functions,
    })
}

fn finalize_function(
    builder: &FunctionBuilder,
    function_map: &HashMap<String, u32>,
) -> Result<Function, NovacError> {
    let mut label_map = HashMap::new();
    let mut instruction_count = 0u32;
    for entry in &builder.body {
        match entry {
            Line::Label(name, line) => {
                if label_map.contains_key(name) {
                    return Err(NovacError::DuplicateLabel(format!("{name} (line {line})")));
                }
                label_map.insert(name.clone(), instruction_count);
            }
            Line::Instruction(_, _, _) => {
                instruction_count += 1;
            }
        }
    }

    let mut instructions = Vec::new();
    for entry in &builder.body {
        if let Line::Instruction(op, operands, line) = entry {
            let opcode = parse_opcode(op)?;
            let expected = opcode.operand_count();
            if operands.len() != expected {
                return Err(NovacError::OperandMismatch(
                    opcode.name(),
                    expected,
                    operands.len(),
                ));
            }

            let mut parsed_operands = Vec::new();
            match opcode {
                Opcode::LoadConst | Opcode::LoadVar | Opcode::StoreVar => {
                    parsed_operands.push(parse_u32(&operands[0], *line)?);
                }
                Opcode::Call => {
                    let target = parse_function_operand(&operands[0], *line, function_map)?;
                    parsed_operands.push(target);
                    parsed_operands.push(parse_u32(&operands[1], *line)?);
                }
                Opcode::Jump | Opcode::JumpIfFalse => {
                    parsed_operands.push(parse_label_operand(&operands[0], *line, &label_map)?);
                }
                Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Return
                | Opcode::CmpLt
                | Opcode::CmpEq => {}
            }
            instructions.push(Instruction::new(opcode, parsed_operands));
        }
    }

    Ok(Function::new(
        builder.name.clone(),
        builder.params,
        instructions,
    ))
}

fn parse_function_operand(
    operand: &str,
    line: usize,
    map: &HashMap<String, u32>,
) -> Result<u32, NovacError> {
    if let Ok(value) = operand.parse::<u32>() {
        return Ok(value);
    }
    map.get(operand)
        .copied()
        .ok_or_else(|| NovacError::UndefinedFunction(format!("{operand} at line {line}")))
}

fn parse_label_operand(
    operand: &str,
    line: usize,
    map: &HashMap<String, u32>,
) -> Result<u32, NovacError> {
    if let Ok(value) = operand.parse::<u32>() {
        return Ok(value);
    }
    map.get(operand)
        .copied()
        .ok_or_else(|| NovacError::UndefinedLabel(format!("{operand} at line {line}")))
}

fn parse_opcode(value: &str) -> Result<Opcode, NovacError> {
    match value.to_ascii_uppercase().as_str() {
        "LOAD_CONST" => Ok(Opcode::LoadConst),
        "LOAD_VAR" => Ok(Opcode::LoadVar),
        "STORE_VAR" => Ok(Opcode::StoreVar),
        "ADD" => Ok(Opcode::Add),
        "SUB" => Ok(Opcode::Sub),
        "MUL" => Ok(Opcode::Mul),
        "DIV" => Ok(Opcode::Div),
        "CALL" => Ok(Opcode::Call),
        "RETURN" => Ok(Opcode::Return),
        "JUMP" => Ok(Opcode::Jump),
        "JUMP_IF_FALSE" => Ok(Opcode::JumpIfFalse),
        "CMP_LT" => Ok(Opcode::CmpLt),
        "CMP_EQ" => Ok(Opcode::CmpEq),
        other => Err(NovacError::Message(format!("unknown opcode {other}"))),
    }
}

fn parse_string_literal(value: &str, line: usize) -> Result<String, NovacError> {
    let trimmed = value.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') || trimmed.len() < 2 {
        return Err(NovacError::Message(format!(
            "invalid string literal {value} at line {line}"
        )));
    }

    let mut result = String::new();
    let mut chars = trimmed[1..trimmed.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escape = chars.next().ok_or_else(|| {
                NovacError::Message(format!(
                    "unterminated escape sequence in string literal at line {line}"
                ))
            })?;
            match escape {
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                other => {
                    return Err(NovacError::Message(format!(
                        "unsupported escape \\{other} at line {line}"
                    )));
                }
            }
        } else {
            result.push(ch);
        }
    }
    Ok(result)
}

fn parse_u8(value: &str, line: usize) -> Result<u8, NovacError> {
    value
        .parse::<u8>()
        .map_err(|_| NovacError::Message(format!("invalid u8 literal {value} at line {line}")))
}

fn parse_u16(value: &str, line: usize) -> Result<u16, NovacError> {
    value
        .parse::<u16>()
        .map_err(|_| NovacError::Message(format!("invalid u16 literal {value} at line {line}")))
}

fn parse_u32(value: &str, line: usize) -> Result<u32, NovacError> {
    value
        .parse::<u32>()
        .map_err(|_| NovacError::Message(format!("invalid u32 literal {value} at line {line}")))
}

fn strip_comment(line: &str) -> &str {
    if let Some(pos) = line.find('#') {
        &line[..pos]
    } else {
        line
    }
}

/// Disassemble binary bytecode back into textual assembly.
pub fn disassemble(bytecode: &Bytecode) -> Result<String, NovacError> {
    if bytecode.version != VERSION {
        return Err(NovacError::UnsupportedVersion(bytecode.version));
    }

    let mut output = String::new();
    writeln!(&mut output, ".version {}", bytecode.version).unwrap();
    output.push('\n');
    writeln!(&mut output, ".constants").unwrap();
    for constant in &bytecode.constants {
        match constant {
            Constant::String(value) => {
                writeln!(&mut output, "    string {}", encode_string(value)).unwrap();
            }
            Constant::Integer(value) => {
                writeln!(&mut output, "    int {}", value).unwrap();
            }
            Constant::Float(value) => {
                writeln!(&mut output, "    float {}", value).unwrap();
            }
        }
    }
    writeln!(&mut output, ".end").unwrap();
    output.push('\n');

    let function_names: Vec<_> = bytecode.functions.iter().map(|f| f.name.clone()).collect();

    for function in &bytecode.functions {
        writeln!(
            &mut output,
            ".function {} {}",
            function.name, function.parameters
        )
        .unwrap();

        let mut label_targets = HashSet::new();
        for instruction in function.instructions.iter() {
            match instruction.opcode {
                Opcode::Jump | Opcode::JumpIfFalse => {
                    if let Some(target) = instruction.operands.first() {
                        label_targets.insert(*target as usize);
                    }
                }
                _ => {}
            }
        }

        let mut labels = BTreeMap::new();
        for target in label_targets {
            labels.insert(target, format!("L{target}"));
        }

        for (index, instruction) in function.instructions.iter().enumerate() {
            if let Some(label) = labels.get(&index) {
                writeln!(&mut output, "{label}:").unwrap();
            }
            write!(&mut output, "    {}", instruction.opcode.name()).unwrap();
            match instruction.opcode {
                Opcode::LoadConst | Opcode::LoadVar | Opcode::StoreVar => {
                    if let Some(value) = instruction.operands.first() {
                        write!(&mut output, " {}", value).unwrap();
                    }
                }
                Opcode::Call => {
                    let target = instruction.operands.get(0).copied().unwrap_or(0);
                    let args = instruction.operands.get(1).copied().unwrap_or(0);
                    let display = function_names
                        .get(target as usize)
                        .map(|name| name.as_str())
                        .unwrap_or("<unknown>");
                    write!(&mut output, " {} {}", display, args).unwrap();
                }
                Opcode::Jump | Opcode::JumpIfFalse => {
                    if let Some(value) = instruction.operands.first() {
                        if let Some(label) = labels.get(&(*value as usize)) {
                            write!(&mut output, " {}", label).unwrap();
                        } else {
                            write!(&mut output, " {}", value).unwrap();
                        }
                    }
                }
                Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Return
                | Opcode::CmpLt
                | Opcode::CmpEq => {}
            }
            output.push('\n');
        }
        writeln!(&mut output, ".end").unwrap();
        output.push('\n');
    }

    Ok(output)
}

fn encode_string(value: &str) -> String {
    let mut result = String::new();
    result.push('"');
    for ch in value.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result.push('"');
    result
}
