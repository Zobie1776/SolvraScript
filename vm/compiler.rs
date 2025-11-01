use std::collections::HashMap;

use crate::ast::{
    BinaryOp, Expr, FunctionDecl, Literal, Parameter, Program, Stmt, StringPart, Type, UnaryOp,
    VariableDecl, Visibility,
};
use crate::tokenizer::Position;
use anyhow::{Result, anyhow, bail};
use solvra_core::solvrac::{self, Constant, Function, Instruction, Opcode};
use solvra_core::vm::compiler as vm_compiler;

pub fn compile_program(program: &Program) -> Result<Vec<u8>> {
    let mut compiler = Compiler::default();
    compiler.index_functions(program)?;
    compiler.compile_program(program)?;
    let bytecode = compiler.into_bytecode()?;
    vm_to_bytes(bytecode)
}

pub fn compile_function(stmt: &Stmt) -> Result<Vec<u8>> {
    match stmt {
        Stmt::FunctionDecl { decl } => compile_function_decl(decl),
        _ => bail!("expected function declaration"),
    }
}

fn compile_function_decl(decl: &FunctionDecl) -> Result<Vec<u8>> {
    let mut compiler = Compiler::default();
    compiler.register_function(&decl.name);
    let function = compiler.build_function(decl)?;
    compiler.store_function(&decl.name, function)?;
    let bytecode = compiler.into_bytecode()?;
    vm_to_bytes(bytecode)
}

pub fn compile_function_from_parts(
    name: &str,
    params: &[String],
    body: &[Stmt],
) -> Result<Vec<u8>> {
    let parameters: Vec<Parameter> = params
        .iter()
        .map(|param| Parameter {
            name: param.clone(),
            param_type: Type::Inferred,
            default_value: None,
            position: Position::new(0, 0, 0),
        })
        .collect();

    let decl = FunctionDecl {
        name: name.to_string(),
        params: parameters,
        return_type: Type::Inferred,
        body: body.to_vec(),
        is_async: false,
        visibility: Visibility::Private,
        position: Position::new(0, 0, 0),
    };

    compile_function_decl(&decl)
}

#[derive(Default)]
struct Compiler {
    constants: Vec<Constant>,
    functions: Vec<Option<Function>>,
    function_indices: HashMap<String, usize>,
    lambda_counter: usize,
}

impl Compiler {
    fn index_functions(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.statements {
            if let Stmt::FunctionDecl { decl } = stmt {
                self.register_function(&decl.name);
            }
        }
        if self.functions.is_empty() {
            bail!("no functions found to compile");
        }
        Ok(())
    }

    fn compile_program(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.statements {
            if let Stmt::FunctionDecl { decl } = stmt {
                let function = self.build_function(decl)?;
                self.store_function(&decl.name, function)?;
            }
        }
        Ok(())
    }

    fn register_function(&mut self, name: &str) {
        if self.function_indices.contains_key(name) {
            return;
        }
        let index = self.functions.len();
        self.function_indices.insert(name.to_string(), index);
        self.functions.push(None);
    }

    fn store_function(&mut self, name: &str, function: Function) -> Result<()> {
        let index = *self
            .function_indices
            .get(name)
            .ok_or_else(|| anyhow!("missing function slot for '{name}'"))?;
        if self.functions[index].is_some() {
            bail!("function '{name}' already compiled");
        }
        self.functions[index] = Some(function);
        Ok(())
    }

    fn into_bytecode(self) -> Result<solvrac::Bytecode> {
        let functions = self
            .functions
            .into_iter()
            .enumerate()
            .map(|(idx, maybe)| {
                maybe.ok_or_else(|| anyhow!("function at index {idx} was never compiled"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(solvra_core::solvrac::Bytecode::new(
            self.constants,
            functions,
        ))
    }

    fn build_function(&mut self, decl: &FunctionDecl) -> Result<Function> {
        let mut builder = FunctionCompiler::new(self, decl)?;
        builder.compile_statements(&decl.body)?;
        builder.finish(&decl.name)
    }

    fn literal_constant(&mut self, literal: &Literal) -> Result<u32> {
        let constant = match literal {
            Literal::Integer(value) => Constant::Integer(*value),
            Literal::Float(value) => Constant::Float(*value),
            Literal::String(value) => Constant::String(value.clone()),
            Literal::Boolean(value) => Constant::Boolean(*value),
            Literal::Null => Constant::Null,
            other => bail!("literal type not supported in VM compiler: {other:?}"),
        };
        Ok(self.constant_index(constant))
    }

    fn ensure_string_constant(&mut self, value: &str) -> u32 {
        let constant = Constant::String(value.to_string());
        self.constant_index(constant)
    }

    fn constant_index(&mut self, constant: Constant) -> u32 {
        if let Some(index) = self.constants.iter().position(|c| c == &constant) {
            index as u32
        } else {
            let index = self.constants.len();
            self.constants.push(constant);
            index as u32
        }
    }

    fn compile_lambda(
        &mut self,
        params: &[String],
        body: &Expr,
        position: &Position,
    ) -> Result<u32> {
        let lambda_index = self.lambda_counter;
        self.lambda_counter += 1;
        let name = format!("__lambda{}", lambda_index);
        let parameters = params
            .iter()
            .map(|param| Parameter {
                name: param.clone(),
                param_type: Type::Inferred,
                default_value: None,
                position: position.clone(),
            })
            .collect::<Vec<_>>();
        let lambda_body = vec![Stmt::Return {
            value: Some(body.clone()),
            position: position.clone(),
        }];
        let decl = FunctionDecl {
            name,
            params: parameters,
            return_type: Type::Inferred,
            body: lambda_body,
            is_async: false,
            visibility: Visibility::Private,
            position: position.clone(),
        };
        let mut builder = FunctionCompiler::new(self, &decl)?;
        builder.compile_statements(&decl.body)?;
        let function = builder.finish(&decl.name)?;
        let index = self.functions.len();
        self.functions.push(Some(function));
        Ok(index as u32)
    }

    fn resolve_function(&self, name: &str) -> Option<usize> {
        self.function_indices.get(name).copied()
    }
}

struct FunctionCompiler<'a> {
    program: &'a mut Compiler,
    instructions: Vec<Instruction>,
    scopes: Vec<HashMap<String, LocalBinding>>,
    next_slot: u32,
    max_slot: u32,
    param_count: u16,
}

impl<'a> FunctionCompiler<'a> {
    fn new(program: &'a mut Compiler, decl: &FunctionDecl) -> Result<Self> {
        if decl.params.len() > u16::MAX as usize {
            bail!("function '{}' has too many parameters", decl.name);
        }

        let mut compiler = Self {
            program,
            instructions: Vec::new(),
            scopes: Vec::new(),
            next_slot: 0,
            max_slot: 0,
            param_count: decl.params.len() as u16,
        };

        compiler.begin_scope();
        for (index, param) in decl.params.iter().enumerate() {
            compiler.declare_parameter(&param.name, index as u32)?;
        }
        compiler.next_slot = decl.params.len() as u32;
        compiler.max_slot = compiler.next_slot;

        Ok(compiler)
    }

    fn compile_statements(&mut self, statements: &[Stmt]) -> Result<()> {
        for stmt in statements {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    fn finish(mut self, name: &str) -> Result<Function> {
        if !matches!(self.instructions.last(), Some(inst) if inst.opcode == Opcode::Return) {
            let null_index = self.program.constant_index(Constant::Null);
            self.instructions
                .push(Instruction::new(Opcode::LoadConst, vec![null_index]));
            self.instructions
                .push(Instruction::new(Opcode::Return, Vec::new()));
        }
        self.end_scope();
        Ok(Function::new(
            name.to_string(),
            self.param_count,
            self.instructions,
        ))
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.compile_expr(expr)?;
                self.instructions
                    .push(Instruction::new(Opcode::Pop, Vec::new()));
                Ok(())
            }
            Stmt::VariableDecl { decl } => self.compile_variable_decl(decl),
            Stmt::Block { statements, .. } => {
                self.begin_scope();
                self.compile_statements(statements)?;
                self.end_scope();
                Ok(())
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.compile_if_stmt(condition, then_branch, else_branch.as_deref()),
            Stmt::While {
                condition, body, ..
            } => self.compile_while_stmt(condition, body),
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    let null_index = self.program.constant_index(Constant::Null);
                    self.instructions
                        .push(Instruction::new(Opcode::LoadConst, vec![null_index]));
                }
                self.instructions
                    .push(Instruction::new(Opcode::Return, Vec::new()));
                Ok(())
            }
            other => bail!("unsupported statement in function body: {other:?}"),
        }
    }

    fn compile_variable_decl(&mut self, decl: &VariableDecl) -> Result<()> {
        let slot = self.declare_local(&decl.name)?;
        if let Some(initializer) = &decl.initializer {
            self.compile_expr(initializer)?;
        } else {
            let null_index = self.program.constant_index(Constant::Null);
            self.instructions
                .push(Instruction::new(Opcode::LoadConst, vec![null_index]));
        }
        self.instructions
            .push(Instruction::new(Opcode::StoreVar, vec![slot]));
        Ok(())
    }

    fn compile_if_stmt(
        &mut self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) -> Result<()> {
        self.compile_expr(condition)?;
        let jump_if_false = self.emit_jump(Opcode::JumpIfFalse);
        self.begin_scope();
        self.compile_stmt(then_branch)?;
        self.end_scope();
        let jump_end = self.emit_jump(Opcode::Jump);
        self.patch_jump(jump_if_false);
        if let Some(else_branch) = else_branch {
            self.begin_scope();
            self.compile_stmt(else_branch)?;
            self.end_scope();
        }
        self.patch_jump(jump_end);
        Ok(())
    }

    fn compile_while_stmt(&mut self, condition: &Expr, body: &Stmt) -> Result<()> {
        let loop_start = self.instructions.len();
        self.compile_expr(condition)?;
        let exit_jump = self.emit_jump(Opcode::JumpIfFalse);
        self.begin_scope();
        self.compile_stmt(body)?;
        self.end_scope();
        self.instructions
            .push(Instruction::new(Opcode::Jump, vec![loop_start as u32]));
        self.patch_jump(exit_jump);
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Literal { value, .. } => {
                let index = self.program.literal_constant(value)?;
                self.instructions
                    .push(Instruction::new(Opcode::LoadConst, vec![index]));
                Ok(())
            }
            Expr::Identifier { name, .. } => {
                let slot = self.resolve_local(name)?;
                self.instructions
                    .push(Instruction::new(Opcode::LoadVar, vec![slot]));
                Ok(())
            }
            Expr::Binary {
                left,
                operator,
                right,
                ..
            } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match operator {
                    BinaryOp::Add => self
                        .instructions
                        .push(Instruction::new(Opcode::Add, Vec::new())),
                    BinaryOp::Subtract => self
                        .instructions
                        .push(Instruction::new(Opcode::Sub, Vec::new())),
                    BinaryOp::Multiply => self
                        .instructions
                        .push(Instruction::new(Opcode::Mul, Vec::new())),
                    BinaryOp::Divide => self
                        .instructions
                        .push(Instruction::new(Opcode::Div, Vec::new())),
                    BinaryOp::Modulo => self
                        .instructions
                        .push(Instruction::new(Opcode::Mod, Vec::new())),
                    BinaryOp::Equal => self
                        .instructions
                        .push(Instruction::new(Opcode::CmpEq, Vec::new())),
                    BinaryOp::NotEqual => {
                        self.instructions
                            .push(Instruction::new(Opcode::CmpEq, Vec::new()));
                        self.instructions
                            .push(Instruction::new(Opcode::Not, Vec::new()));
                    }
                    BinaryOp::Less => self
                        .instructions
                        .push(Instruction::new(Opcode::CmpLt, Vec::new())),
                    BinaryOp::Greater => self
                        .instructions
                        .push(Instruction::new(Opcode::CmpGt, Vec::new())),
                    BinaryOp::LessEqual => self
                        .instructions
                        .push(Instruction::new(Opcode::CmpLe, Vec::new())),
                    BinaryOp::GreaterEqual => self
                        .instructions
                        .push(Instruction::new(Opcode::CmpGe, Vec::new())),
                    BinaryOp::And => self
                        .instructions
                        .push(Instruction::new(Opcode::And, Vec::new())),
                    BinaryOp::Or => self
                        .instructions
                        .push(Instruction::new(Opcode::Or, Vec::new())),
                    other => bail!("unsupported binary operator {other:?}"),
                }
                Ok(())
            }
            Expr::Unary {
                operator, operand, ..
            } => {
                self.compile_expr(operand)?;
                match operator {
                    UnaryOp::Minus => {
                        self.instructions
                            .push(Instruction::new(Opcode::Neg, Vec::new()));
                    }
                    UnaryOp::Not => {
                        self.instructions
                            .push(Instruction::new(Opcode::Not, Vec::new()));
                    }
                    UnaryOp::Plus => { /* no-op */ }
                    other => bail!("unsupported unary operator {other:?}"),
                }
                Ok(())
            }
            Expr::Call { callee, args, .. } => self.compile_call(callee, args, CallMode::Normal),
            Expr::Assignment { target, value, .. } => self.compile_assignment(target, value),
            Expr::If {
                condition,
                then_expr,
                else_expr,
                ..
            } => self.compile_if_expr(condition, then_expr, else_expr),
            Expr::List { elements, .. } => {
                for element in elements {
                    self.compile_expr(element)?;
                }
                self.instructions.push(Instruction::new(
                    Opcode::MakeList,
                    vec![elements.len() as u32],
                ));
                Ok(())
            }
            Expr::StringTemplate { parts, .. } | Expr::StringInterpolation { parts, .. } => {
                if let Some(value) = flatten_literal_template(parts) {
                    let index = self.program.constant_index(Constant::String(value));
                    self.instructions
                        .push(Instruction::new(Opcode::LoadConst, vec![index]));
                    Ok(())
                } else {
                    bail!("string templates with embedded expressions are not supported")
                }
            }
            Expr::Lambda {
                params,
                body,
                position,
            } => {
                let function_index = self.program.compile_lambda(params, body, position)?;
                self.instructions
                    .push(Instruction::new(Opcode::LoadLambda, vec![function_index]));
                Ok(())
            }
            Expr::Async { expr, .. } => self.compile_async(expr),
            Expr::Await { expr, .. } => {
                self.compile_expr(expr)?;
                self.instructions
                    .push(Instruction::new(Opcode::Await, Vec::new()));
                Ok(())
            }
            other => bail!("unsupported expression type: {other:?}"),
        }
    }

    fn compile_assignment(&mut self, target: &Expr, value: &Expr) -> Result<()> {
        match target {
            Expr::Identifier { name, .. } => {
                let slot = self.resolve_local(name)?;
                self.compile_expr(value)?;
                self.instructions
                    .push(Instruction::new(Opcode::StoreVar, vec![slot]));
                self.instructions
                    .push(Instruction::new(Opcode::LoadVar, vec![slot]));
                Ok(())
            }
            _ => bail!("unsupported assignment target"),
        }
    }

    fn compile_if_expr(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<()> {
        self.compile_expr(condition)?;
        let jump_if_false = self.emit_jump(Opcode::JumpIfFalse);
        self.compile_expr(then_expr)?;
        let jump_end = self.emit_jump(Opcode::Jump);
        self.patch_jump(jump_if_false);
        self.compile_expr(else_expr)?;
        self.patch_jump(jump_end);
        Ok(())
    }

    fn compile_async(&mut self, expr: &Expr) -> Result<()> {
        if let Expr::Call { callee, args, .. } = expr {
            self.compile_call(callee, args, CallMode::Async)
        } else {
            bail!("async expressions currently support call targets only")
        }
    }

    fn compile_call(&mut self, callee: &Expr, args: &[Expr], mode: CallMode) -> Result<()> {
        for arg in args {
            self.compile_expr(arg)?;
        }

        match callee {
            Expr::Identifier { name, .. } => {
                if let Some(index) = self.program.resolve_function(name) {
                    let opcode = match mode {
                        CallMode::Normal => Opcode::Call,
                        CallMode::Async => Opcode::CallAsync,
                    };
                    self.instructions.push(Instruction::new(
                        opcode,
                        vec![index as u32, args.len() as u32],
                    ));
                    Ok(())
                } else {
                    let name_index = self.program.ensure_string_constant(name);
                    self.instructions.push(Instruction::new(
                        Opcode::CallBuiltin,
                        vec![name_index, args.len() as u32],
                    ));
                    Ok(())
                }
            }
            _ => bail!("unsupported call target"),
        }
    }

    fn emit_jump(&mut self, opcode: Opcode) -> usize {
        let index = self.instructions.len();
        self.instructions.push(Instruction::new(opcode, vec![0]));
        index
    }

    fn patch_jump(&mut self, index: usize) {
        let target = self.instructions.len() as u32;
        if let Some(instruction) = self.instructions.get_mut(index) {
            if instruction.operands.is_empty() {
                instruction.operands.push(target);
            } else {
                instruction.operands[0] = target;
            }
        }
    }

    fn resolve_local(&self, name: &str) -> Result<u32> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Ok(binding.slot);
            }
        }
        bail!("unknown variable '{name}'")
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_parameter(&mut self, name: &str, slot: u32) -> Result<()> {
        let scope = self
            .scopes
            .last_mut()
            .ok_or_else(|| anyhow!("no active scope for parameter declaration"))?;
        if scope.contains_key(name) {
            bail!("parameter '{name}' already declared in this scope");
        }
        scope.insert(name.to_string(), LocalBinding { slot });
        Ok(())
    }

    fn declare_local(&mut self, name: &str) -> Result<u32> {
        let scope = self
            .scopes
            .last_mut()
            .ok_or_else(|| anyhow!("no active scope for variable declaration"))?;
        if scope.contains_key(name) {
            bail!("variable '{name}' already declared in this scope");
        }
        let slot = self.next_slot;
        scope.insert(name.to_string(), LocalBinding { slot });
        self.next_slot += 1;
        self.max_slot = self.max_slot.max(self.next_slot);
        Ok(slot)
    }
}

#[derive(Clone, Copy)]
struct LocalBinding {
    slot: u32,
}

#[derive(Clone, Copy)]
enum CallMode {
    Normal,
    Async,
}

fn vm_to_bytes(bytecode: solvrac::Bytecode) -> Result<Vec<u8>> {
    let vm_bytecode = vm_compiler::from_solvrac(&bytecode);
    vm_bytecode
        .serialize()
        .map_err(|err| anyhow!(err.to_string()))
}

fn flatten_literal_template(parts: &[StringPart]) -> Option<String> {
    let mut result = String::new();
    for part in parts {
        match part {
            StringPart::Literal(value) => result.push_str(value),
            StringPart::Expression(_) => return None,
        }
    }
    Some(result)
}
