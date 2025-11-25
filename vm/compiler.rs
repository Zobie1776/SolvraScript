use std::collections::HashMap;

use super::core_builtins::is_core_builtin_name;
use crate::ast::{
    AssignTarget, BinaryOp, Expr, FunctionDecl, Literal, MemberKind, Parameter, Program, Stmt,
    StringPart, Type, UnaryOp, VariableDecl, Visibility,
};
use crate::symbol::Symbol;
use crate::tokenizer::Position;
use anyhow::{Result, anyhow, bail};
use solvra_core::solvrac::{self, Constant, Function};
use solvra_core::vm::compiler as vm_compiler;
use solvra_core::vm::instruction::{Instruction, Opcode};

const DYNAMIC_CALL_TARGET: u32 = u32::MAX;

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
            name: Symbol::from(param.as_str()),
            param_type: Type::Inferred,
            default_value: None,
            position: Position::new(0, 0, 0),
        })
        .collect();

    let decl = FunctionDecl {
        name: Symbol::from(name),
        params: parameters,
        return_type: Type::Inferred,
        body: body.to_vec(),
        is_async: false,
        visibility: Visibility::Private,
        position: Position::new(0, 0, 0),
    };

    compile_function_decl(&decl)
}

struct Compiler {
    constants: Vec<Constant>,
    functions: Vec<Option<Function>>,
    function_indices: HashMap<String, usize>,
    lambda_counter: usize,
    constant_cache: HashMap<ConstantKey, u32>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            constants: Vec::new(),
            functions: Vec::new(),
            function_indices: HashMap::new(),
            lambda_counter: 0,
            constant_cache: HashMap::new(),
        }
    }
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
            Literal::String(value) => Constant::String(value.to_string()),
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
        let key = ConstantKey::from(&constant);
        if let Some(index) = self.constant_cache.get(&key) {
            return *index;
        }
        let index = self.constants.len() as u32;
        self.constants.push(constant);
        self.constant_cache.insert(key, index);
        index
    }

    fn compile_lambda(
        &mut self,
        params: &[Symbol],
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
            name: Symbol::from(name),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConstantKey {
    String(String),
    Integer(i64),
    Float(u64),
    Boolean(bool),
    Null,
}

impl From<&Constant> for ConstantKey {
    fn from(value: &Constant) -> Self {
        match value {
            Constant::String(text) => ConstantKey::String(text.clone()),
            Constant::Integer(int) => ConstantKey::Integer(*int),
            Constant::Float(float) => ConstantKey::Float(float.to_bits()),
            Constant::Boolean(flag) => ConstantKey::Boolean(*flag),
            Constant::Null => ConstantKey::Null,
        }
    }
}

struct FunctionCompiler<'a> {
    program: &'a mut Compiler,
    instructions: Vec<Instruction>,
    scopes: Vec<HashMap<String, LocalBinding>>,
    next_slot: u32,
    max_slot: u32,
    param_count: u16,
    loop_stack: Vec<LoopFrame>,
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
            loop_stack: Vec::new(),
        };

        compiler.begin_scope();
        for (index, param) in decl.params.iter().enumerate() {
            compiler.declare_parameter(&param.name, index as u32)?;
        }
        compiler.next_slot = decl.params.len() as u32;
        compiler.max_slot = compiler.next_slot;

        Ok(compiler)
    }

    fn emit_instruction(&mut self, opcode: Opcode, operands: &[u32]) {
        self.instructions
            .push(Instruction::with_operands(opcode, operands));
    }

    fn emit_op(&mut self, opcode: Opcode) {
        self.emit_instruction(opcode, &[]);
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
            self.emit_instruction(Opcode::LoadConst, &[null_index]);
            self.emit_op(Opcode::Return);
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
                self.emit_op(Opcode::Pop);
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
            Stmt::For {
                variable,
                iterable,
                body,
                ..
            } => self.compile_for_stmt(variable, iterable, body),
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    let null_index = self.program.constant_index(Constant::Null);
                    self.emit_instruction(Opcode::LoadConst, &[null_index]);
                }
                self.emit_op(Opcode::Return);
                Ok(())
            }
            Stmt::Break { label, .. } => {
                if label.is_some() {
                    bail!("labeled break is not supported yet");
                }
                let jump_index = self.emit_jump(Opcode::Jump);
                self.register_break(jump_index)?;
                Ok(())
            }
            Stmt::Continue { label, .. } => {
                if label.is_some() {
                    bail!("labeled continue is not supported yet");
                }
                let jump_index = self.emit_jump(Opcode::Jump);
                self.register_continue(jump_index)?;
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
            self.emit_instruction(Opcode::LoadConst, &[null_index]);
        }
        self.emit_instruction(Opcode::StoreVar, &[slot]);
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
        self.begin_loop(loop_start);
        self.begin_scope();
        self.compile_stmt(body)?;
        self.end_scope();
        self.emit_instruction(Opcode::Jump, &[loop_start as u32]);
        self.patch_jump(exit_jump);
        let break_target = self.instructions.len();
        self.end_loop(break_target);
        Ok(())
    }

    fn compile_for_stmt(&mut self, variable: &str, iterable: &Expr, body: &Stmt) -> Result<()> {
        let elements: Vec<&Expr> = match iterable {
            Expr::List { elements, .. } => elements.iter().collect(),
            Expr::Literal { value, .. } => match value {
                Literal::Array(items) => items.iter().collect(),
                _ => bail!("for loops currently support literal list iterables only"),
            },
            _ => bail!("for loops currently support literal list iterables only"),
        };

        self.begin_scope();
        let slot = self.declare_local(variable)?;
        for element in elements {
            self.compile_expr(element)?;
            self.emit_instruction(Opcode::StoreVar, &[slot]);
            self.compile_stmt(body)?;
        }
        self.end_scope();
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Literal { value, .. } => match value {
                Literal::Array(elements) => {
                    self.emit_instruction(Opcode::MakeArray, &[elements.len() as u32]);
                    for element in elements {
                        self.compile_expr(element)?;
                        self.emit_op(Opcode::Push);
                    }
                    Ok(())
                }
                Literal::Object(fields) => {
                    for (key, expr) in fields {
                        let key_index = self.program.ensure_string_constant(key.as_str());
                        self.emit_instruction(Opcode::LoadConst, &[key_index]);
                        self.compile_expr(expr)?;
                    }
                    self.emit_instruction(Opcode::MakeObject, &[fields.len() as u32]);
                    Ok(())
                }
                _ => {
                    let index = self.program.literal_constant(value)?;
                    self.emit_instruction(Opcode::LoadConst, &[index]);
                    Ok(())
                }
            },
            Expr::Identifier { name, .. } => {
                let slot = self.resolve_local(name)?;
                self.emit_instruction(Opcode::LoadVar, &[slot]);
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
                    BinaryOp::Add => self.emit_op(Opcode::Add),
                    BinaryOp::Subtract => self.emit_op(Opcode::Sub),
                    BinaryOp::Multiply => self.emit_op(Opcode::Mul),
                    BinaryOp::Divide => self.emit_op(Opcode::Div),
                    BinaryOp::Modulo => self.emit_op(Opcode::Mod),
                    BinaryOp::Equal => self.emit_op(Opcode::Equal),
                    BinaryOp::NotEqual => {
                        self.emit_op(Opcode::Equal);
                        self.emit_op(Opcode::Not);
                    }
                    BinaryOp::Less => self.emit_op(Opcode::Less),
                    BinaryOp::Greater => self.emit_op(Opcode::Greater),
                    BinaryOp::LessEqual => self.emit_op(Opcode::LessEqual),
                    BinaryOp::GreaterEqual => self.emit_op(Opcode::GreaterEqual),
                    BinaryOp::And => self.emit_op(Opcode::And),
                    BinaryOp::Or => self.emit_op(Opcode::Or),
                    other => bail!("unsupported binary operator {other:?}"),
                }
                Ok(())
            }
            Expr::Unary {
                operator, operand, ..
            } => {
                self.compile_expr(operand)?;
                match operator {
                    UnaryOp::Minus => self.emit_op(Opcode::Neg),
                    UnaryOp::Not => self.emit_op(Opcode::Not),
                    UnaryOp::Plus => { /* no-op */ }
                    other => bail!("unsupported unary operator {other:?}"),
                }
                Ok(())
            }
            Expr::Call { callee, args, .. } => self.compile_call(callee, args, CallMode::Normal),
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => self.compile_method_call(receiver, method, args),
            Expr::Assign { target, value, .. } => self.compile_assignment(target, value),
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
                self.emit_instruction(Opcode::MakeList, &[elements.len() as u32]);
                Ok(())
            }
            Expr::StringTemplate { parts, .. } | Expr::StringInterpolation { parts, .. } => {
                if let Some(value) = flatten_literal_template(parts) {
                    let index = self.program.constant_index(Constant::String(value));
                    self.emit_instruction(Opcode::LoadConst, &[index]);
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
                self.emit_instruction(Opcode::LoadLambda, &[function_index]);
                Ok(())
            }
            Expr::Async { expr, .. } => self.compile_async(expr),
            Expr::Await { expr, .. } => {
                self.compile_expr(expr)?;
                self.emit_op(Opcode::Await);
                Ok(())
            }
            Expr::Index { object, index, .. } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit_op(Opcode::Index);
                Ok(())
            }
            Expr::Member {
                object, property, ..
            } => {
                self.compile_expr(object)?;
                let name_index = self.program.ensure_string_constant(property.as_str());
                self.emit_instruction(Opcode::LoadMember, &[name_index]);
                Ok(())
            }
            other => bail!("unsupported expression type: {other:?}"),
        }
    }

    fn compile_assignment(&mut self, target: &AssignTarget, value: &Expr) -> Result<()> {
        match target {
            AssignTarget::Variable(name) => {
                let slot = self.resolve_local(name.as_str())?;
                self.compile_expr(value)?;
                self.emit_instruction(Opcode::StoreVar, &[slot]);
                self.emit_instruction(Opcode::LoadVar, &[slot]);
                Ok(())
            }
            AssignTarget::Index { array, index } => {
                let array_name = match array.as_ref() {
                    Expr::Identifier { name, .. } => name,
                    _ => bail!("unsupported assignment target"),
                };
                let slot = self.resolve_local(array_name.as_str())?;
                self.emit_instruction(Opcode::LoadVar, &[slot]);
                self.compile_expr(index)?;
                self.compile_expr(value)?;
                self.emit_op(Opcode::SetIndex);
                self.emit_instruction(Opcode::StoreVar, &[slot]);
                self.emit_instruction(Opcode::LoadVar, &[slot]);
                Ok(())
            }
            AssignTarget::Member { object, property } => {
                self.compile_expr(object)?;
                let key_index = self.program.ensure_string_constant(property.as_str());
                self.emit_instruction(Opcode::LoadConst, &[key_index]);
                self.compile_expr(value)?;
                self.emit_op(Opcode::SetMember);
                Ok(())
            }
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

        match self.resolve_call_target(callee)? {
            ResolvedCallTarget::Function(index) => {
                let opcode = match mode {
                    CallMode::Normal => Opcode::Call,
                    CallMode::Async => Opcode::CallAsync,
                };
                self.emit_instruction(opcode, &[index as u32, args.len() as u32]);
                Ok(())
            }
            ResolvedCallTarget::Builtin(name) => {
                let name_index = self.program.ensure_string_constant(&name);
                self.emit_instruction(Opcode::CallBuiltin, &[name_index, args.len() as u32]);
                Ok(())
            }
            ResolvedCallTarget::CoreBuiltin(name) => {
                let name_index = self.program.ensure_string_constant(&name);
                self.emit_instruction(Opcode::CoreCall, &[name_index, args.len() as u32]);
                Ok(())
            }
        }
    }

    fn compile_method_call(
        &mut self,
        receiver: &Expr,
        method: &Symbol,
        args: &[Expr],
    ) -> Result<()> {
        let (temp_slot, prev_next) = self.acquire_temp_slot();
        self.compile_expr(receiver)?;
        self.emit_instruction(Opcode::StoreVar, &[temp_slot]);
        self.emit_instruction(Opcode::LoadVar, &[temp_slot]);
        let name_index = self.program.ensure_string_constant(method.as_str());
        self.emit_instruction(Opcode::LoadMember, &[name_index]);
        self.emit_instruction(Opcode::LoadVar, &[temp_slot]);
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.release_temp_slot(prev_next);
        let total_args = args.len() + 1;
        self.emit_instruction(
            Opcode::Call,
            &[DYNAMIC_CALL_TARGET, total_args as u32, name_index],
        );
        Ok(())
    }

    fn resolve_call_target(&mut self, callee: &Expr) -> Result<ResolvedCallTarget> {
        match callee {
            Expr::Identifier { name, .. } => {
                let text = name.as_str();
                if is_core_builtin_name(text) {
                    Ok(ResolvedCallTarget::CoreBuiltin(text.to_string()))
                } else if let Some(index) = self.program.resolve_function(name) {
                    Ok(ResolvedCallTarget::Function(index))
                } else {
                    Ok(ResolvedCallTarget::Builtin(text.to_string()))
                }
            }
            Expr::Member { kind, .. } if *kind == MemberKind::DoubleColon => {
                let name = self.flatten_member_name(callee)?;
                if is_core_builtin_name(&name) {
                    Ok(ResolvedCallTarget::CoreBuiltin(name))
                } else {
                    Ok(ResolvedCallTarget::Builtin(name))
                }
            }
            other => bail!("unsupported call target: {other:?}"),
        }
    }

    fn flatten_member_name(&self, expr: &Expr) -> Result<String> {
        let mut segments = Vec::new();
        self.collect_member_segments(expr, &mut segments)?;
        Ok(segments.join("::"))
    }

    fn collect_member_segments(&self, expr: &Expr, segments: &mut Vec<String>) -> Result<()> {
        match expr {
            Expr::Identifier { name, .. } => {
                segments.push(name.to_string());
                Ok(())
            }
            Expr::Member {
                object,
                property,
                kind,
                ..
            } if *kind == MemberKind::DoubleColon => {
                self.collect_member_segments(object, segments)?;
                segments.push(property.to_string());
                Ok(())
            }
            other => bail!("unsupported member access in call target: {other:?}"),
        }
    }

    fn emit_jump(&mut self, opcode: Opcode) -> usize {
        let index = self.instructions.len();
        self.instructions
            .push(Instruction::with_operands(opcode, &[0]));
        index
    }

    fn patch_jump(&mut self, index: usize) {
        let target = self.instructions.len() as u32;
        if let Some(instruction) = self.instructions.get_mut(index) {
            instruction.operand_a = target;
        }
    }

    fn patch_jump_to(&mut self, index: usize, target: usize) {
        if let Some(instruction) = self.instructions.get_mut(index) {
            instruction.operand_a = target as u32;
        }
    }

    fn begin_loop(&mut self, continue_target: usize) {
        self.loop_stack.push(LoopFrame {
            continue_target,
            breaks: Vec::new(),
        });
    }

    fn end_loop(&mut self, break_target: usize) {
        if let Some(frame) = self.loop_stack.pop() {
            for index in frame.breaks {
                self.patch_jump_to(index, break_target);
            }
        }
    }

    fn register_break(&mut self, jump_index: usize) -> Result<()> {
        let Some(frame) = self.loop_stack.last_mut() else {
            bail!("'break' used outside of loop");
        };
        frame.breaks.push(jump_index);
        Ok(())
    }

    fn register_continue(&mut self, jump_index: usize) -> Result<()> {
        let continue_target = match self.loop_stack.last() {
            Some(frame) => frame.continue_target,
            None => bail!("'continue' used outside of loop"),
        };
        self.patch_jump_to(jump_index, continue_target);
        Ok(())
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

    fn acquire_temp_slot(&mut self) -> (u32, u32) {
        let previous_next = self.next_slot;
        let slot = self.next_slot;
        self.next_slot += 1;
        self.max_slot = self.max_slot.max(self.next_slot);
        (slot, previous_next)
    }

    fn release_temp_slot(&mut self, previous_next: u32) {
        self.next_slot = previous_next;
    }
}

#[derive(Clone, Copy)]
struct LocalBinding {
    slot: u32,
}

struct LoopFrame {
    continue_target: usize,
    breaks: Vec<usize>,
}

enum ResolvedCallTarget {
    Function(usize),
    Builtin(String),
    CoreBuiltin(String),
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
