use std::collections::HashMap;

use super::{
    ast::{Ast, BinaryOp, Expr, ExprKind, Span, Stmt, UnaryOp},
    ir::{CallTarget, IrFunction, IrInstruction, IrOpcode, IrProgram, IrSpan, LabelId},
    spec::{
        Constant, DebugSymbol, FunctionDescriptor, Instruction, NovaBytecode, NovaBytecodeError,
        Opcode,
    },
};

/// Configuration toggles for the assembler.
#[derive(Debug, Clone, Copy)]
pub struct AssemblyConfig {
    /// Enables optimisation passes on the intermediate representation.
    pub optimise: bool,
}

impl Default for AssemblyConfig {
    fn default() -> Self {
        Self { optimise: true }
    }
}

/// Public entry point used by consumers.
pub fn assemble(ast: &Ast) -> Result<NovaBytecode, NovaBytecodeError> {
    assemble_with(ast, AssemblyConfig::default())
}

/// Assembles AST into bytecode with the given configuration.
pub fn assemble_with(ast: &Ast, config: AssemblyConfig) -> Result<NovaBytecode, NovaBytecodeError> {
    let mut assembler = Assembler::new(ast, config);
    assembler.compile()
}

struct Assembler<'a> {
    ast: &'a Ast,
    config: AssemblyConfig,
    builtins: Builtins,
    function_map: HashMap<String, usize>,
}

impl<'a> Assembler<'a> {
    fn new(ast: &'a Ast, config: AssemblyConfig) -> Self {
        Self {
            ast,
            config,
            builtins: Builtins::new(),
            function_map: HashMap::new(),
        }
    }

    fn compile(&mut self) -> Result<NovaBytecode, NovaBytecodeError> {
        self.register_function("__entry", 0)?;
        for function in &self.ast.functions {
            self.register_function(&function.name, function.params.len())?;
        }

        let mut ir_functions = Vec::with_capacity(self.ast.functions.len() + 1);
        let entry_ir = self.compile_function("__entry", &[], &self.ast.body, &Span::synthetic())?;
        ir_functions.push(entry_ir);
        for function in &self.ast.functions {
            let ir = self.compile_function(
                &function.name,
                &function.params,
                &function.body,
                &function.span,
            )?;
            ir_functions.push(ir);
        }

        let mut program = IrProgram::new(ir_functions, 0);
        if self.config.optimise {
            optimise_program(&mut program);
        }

        self.lower(program)
    }

    fn register_function(&mut self, name: &str, arity: usize) -> Result<(), NovaBytecodeError> {
        if self.function_map.contains_key(name) {
            return Err(NovaBytecodeError::Assembly(format!(
                "duplicate function declaration: {name}"
            )));
        }
        let index = self.function_map.len();
        self.function_map.insert(name.to_string(), index);
        if arity > u16::MAX as usize {
            return Err(NovaBytecodeError::Assembly(format!(
                "function {name} has too many parameters"
            )));
        }
        Ok(())
    }

    fn compile_function(
        &self,
        name: &str,
        params: &[String],
        body: &[Stmt],
        span: &Span,
    ) -> Result<IrFunction, NovaBytecodeError> {
        let mut state =
            FunctionState::new(name.to_string(), params, &self.function_map, &self.builtins)?;
        state.push_scope();
        for (index, param) in params.iter().enumerate() {
            state.declare_parameter(param, index as u16)?;
        }
        state.compile_block(body)?;
        state.ensure_terminated(span);
        state.finish()
    }

    fn lower(&self, program: IrProgram) -> Result<NovaBytecode, NovaBytecodeError> {
        let mut constants = Vec::new();
        let mut debug_table = DebugTable::default();
        let mut functions = Vec::with_capacity(program.functions.len());

        for function in program.functions {
            let descriptor = lower_function(function, &mut constants, &mut debug_table)?;
            functions.push(descriptor);
        }

        Ok(NovaBytecode::new(
            constants,
            functions,
            debug_table.into_symbols(),
            program.entry,
        ))
    }
}

struct FunctionState<'a> {
    name: String,
    arity: u16,
    locals: u16,
    instructions: Vec<IrInstruction>,
    scopes: Vec<HashMap<String, u16>>,
    next_label: usize,
    function_map: &'a HashMap<String, usize>,
    builtins: &'a Builtins,
}

impl<'a> FunctionState<'a> {
    fn new(
        name: String,
        params: &[String],
        function_map: &'a HashMap<String, usize>,
        builtins: &'a Builtins,
    ) -> Result<Self, NovaBytecodeError> {
        if params.len() > u16::MAX as usize {
            return Err(NovaBytecodeError::Assembly(format!(
                "function {name} has too many parameters"
            )));
        }
        Ok(Self {
            name,
            arity: params.len() as u16,
            locals: params.len() as u16,
            instructions: Vec::new(),
            scopes: Vec::new(),
            next_label: 0,
            function_map,
            builtins,
        })
    }

    fn finish(self) -> Result<IrFunction, NovaBytecodeError> {
        Ok(IrFunction::new(
            self.name,
            self.arity,
            self.locals,
            self.instructions,
        ))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_parameter(&mut self, name: &str, index: u16) -> Result<(), NovaBytecodeError> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), index);
        }
        Ok(())
    }

    fn declare_local(&mut self, name: &str, span: &Span) -> Result<u16, NovaBytecodeError> {
        if self.locals == u16::MAX {
            return Err(NovaBytecodeError::Assembly(format!(
                "function {} exceeds the maximum number of locals",
                self.name
            )));
        }
        let index = self.locals;
        self.locals += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), index);
        } else {
            return Err(NovaBytecodeError::Assembly(format!(
                "missing scope while declaring variable {} at {}:{}",
                name,
                span.line(),
                span.column()
            )));
        }
        Ok(index)
    }

    fn resolve_local(&self, name: &str) -> Option<u16> {
        for scope in self.scopes.iter().rev() {
            if let Some(index) = scope.get(name) {
                return Some(*index);
            }
        }
        None
    }

    fn emit(&mut self, opcode: IrOpcode, span: &Span) {
        self.instructions
            .push(IrInstruction::new(opcode, IrSpan::from_span(span)));
    }

    fn new_label(&mut self) -> LabelId {
        let label = LabelId(self.next_label);
        self.next_label += 1;
        label
    }

    fn emit_label(&mut self, label: LabelId, span: &Span) {
        self.emit(IrOpcode::Label(label), span);
    }

    fn compile_block(&mut self, statements: &[Stmt]) -> Result<(), NovaBytecodeError> {
        for stmt in statements {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), NovaBytecodeError> {
        match stmt {
            Stmt::Let { name, expr, span } => {
                self.compile_expr(expr)?;
                let index = self.declare_local(name, span)?;
                self.emit(IrOpcode::StoreLocal(index), span);
            }
            Stmt::Assign { name, expr, span } => {
                let Some(index) = self.resolve_local(name) else {
                    return Err(NovaBytecodeError::Assembly(format!(
                        "assignment to unknown variable {name}"
                    )));
                };
                self.compile_expr(expr)?;
                self.emit(IrOpcode::StoreLocal(index), span);
            }
            Stmt::Expr(expr, span) => {
                self.compile_expr(expr)?;
                self.emit(IrOpcode::Pop, span);
            }
            Stmt::Return(expr, span) => {
                if let Some(expr) = expr {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(IrOpcode::PushConst(Constant::Null), span);
                }
                self.emit(IrOpcode::Return, span);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let else_label = self.new_label();
                let end_label = self.new_label();
                self.compile_expr(condition)?;
                self.emit(IrOpcode::JumpIfFalse(else_label), &condition.span);
                self.push_scope();
                self.compile_block(then_branch)?;
                self.pop_scope();
                self.emit(IrOpcode::Jump(end_label), span);
                self.emit_label(else_label, span);
                self.push_scope();
                self.compile_block(else_branch)?;
                self.pop_scope();
                self.emit_label(end_label, span);
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                let start_label = self.new_label();
                let end_label = self.new_label();
                self.emit_label(start_label, span);
                self.compile_expr(condition)?;
                self.emit(IrOpcode::JumpIfFalse(end_label), &condition.span);
                self.push_scope();
                self.compile_block(body)?;
                self.pop_scope();
                self.emit(IrOpcode::Jump(start_label), span);
                self.emit_label(end_label, span);
            }
            Stmt::Try {
                try_block,
                catch_name,
                catch_block,
                finally_block,
                span,
            } => {
                let handler_label = self.new_label();
                let finally_label = self.new_label();
                self.emit(
                    IrOpcode::PushCatch {
                        handler: handler_label,
                    },
                    span,
                );
                self.push_scope();
                self.compile_block(try_block)?;
                self.pop_scope();
                self.emit(IrOpcode::PopCatch, span);
                self.emit(IrOpcode::Jump(finally_label), span);
                self.emit_label(handler_label, span);
                self.push_scope();
                let index = self.declare_local(catch_name, span)?;
                self.emit(IrOpcode::StoreLocal(index), span);
                self.compile_block(catch_block)?;
                self.pop_scope();
                self.emit_label(finally_label, span);
                self.push_scope();
                self.compile_block(finally_block)?;
                self.pop_scope();
            }
            Stmt::Throw { expr, span } => {
                self.compile_expr(expr)?;
                self.emit(IrOpcode::Throw, span);
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), NovaBytecodeError> {
        match &expr.kind {
            ExprKind::Number(value) => {
                self.emit(IrOpcode::PushConst(Constant::Float(*value)), &expr.span);
            }
            ExprKind::Boolean(value) => {
                self.emit(IrOpcode::PushConst(Constant::Boolean(*value)), &expr.span);
            }
            ExprKind::String(value) => {
                self.emit(
                    IrOpcode::PushConst(Constant::String(value.clone())),
                    &expr.span,
                );
            }
            ExprKind::Identifier(name) => {
                if let Some(index) = self.resolve_local(name) {
                    self.emit(IrOpcode::LoadLocal(index), &expr.span);
                } else {
                    return Err(NovaBytecodeError::Assembly(format!(
                        "use of unknown identifier {name}"
                    )));
                }
            }
            ExprKind::Binary { left, op, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                let opcode = match op {
                    BinaryOp::Add => IrOpcode::Add,
                    BinaryOp::Subtract => IrOpcode::Subtract,
                    BinaryOp::Multiply => IrOpcode::Multiply,
                    BinaryOp::Divide => IrOpcode::Divide,
                    BinaryOp::Modulo => IrOpcode::Modulo,
                    BinaryOp::Equals => IrOpcode::Equals,
                    BinaryOp::NotEquals => IrOpcode::NotEquals,
                    BinaryOp::Less => IrOpcode::Less,
                    BinaryOp::LessEqual => IrOpcode::LessEqual,
                    BinaryOp::Greater => IrOpcode::Greater,
                    BinaryOp::GreaterEqual => IrOpcode::GreaterEqual,
                    BinaryOp::And => IrOpcode::LogicalAnd,
                    BinaryOp::Or => IrOpcode::LogicalOr,
                };
                self.emit(opcode, &expr.span);
            }
            ExprKind::Unary { op, expr: inner } => {
                self.compile_expr(inner)?;
                let opcode = match op {
                    UnaryOp::Negate => IrOpcode::Negate,
                    UnaryOp::Not => IrOpcode::LogicalNot,
                };
                self.emit(opcode, &expr.span);
            }
            ExprKind::Call { callee, args } => match &callee.kind {
                ExprKind::Identifier(name) => {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let args_len = u16::try_from(args.len()).map_err(|_| {
                        NovaBytecodeError::Assembly("too many call arguments".into())
                    })?;
                    if let Some(index) = self.function_map.get(name) {
                        self.emit(
                            IrOpcode::Call {
                                target: CallTarget::Function(*index),
                                args: args_len,
                            },
                            &expr.span,
                        );
                    } else if let Some(index) = self.builtins.lookup(name) {
                        self.emit(
                            IrOpcode::Call {
                                target: CallTarget::Native(index),
                                args: args_len,
                            },
                            &expr.span,
                        );
                    } else {
                        return Err(NovaBytecodeError::Assembly(format!(
                            "call to unknown function {name}"
                        )));
                    }
                }
                _ => {
                    return Err(NovaBytecodeError::Assembly(
                        "only identifier calls are supported".into(),
                    ));
                }
            },
            ExprKind::List(elements) => {
                for element in elements {
                    self.compile_expr(element)?;
                }
                let count = u16::try_from(elements.len()).map_err(|_| {
                    NovaBytecodeError::Assembly("list literal exceeds limit".into())
                })?;
                self.emit(IrOpcode::BuildList(count), &expr.span);
            }
            ExprKind::Index { target, index } => {
                self.compile_expr(target)?;
                self.compile_expr(index)?;
                self.emit(IrOpcode::Index, &expr.span);
            }
            ExprKind::Map(_) | ExprKind::Lambda { .. } => {
                return Err(NovaBytecodeError::Assembly(
                    "map literals and lambdas are not yet supported".into(),
                ));
            }
        }
        Ok(())
    }

    fn ensure_terminated(&mut self, span: &Span) {
        let has_return = self
            .instructions
            .iter()
            .rev()
            .find(|inst| !matches!(inst.opcode, IrOpcode::Label(_)))
            .map(|inst| matches!(inst.opcode, IrOpcode::Return))
            .unwrap_or(false);
        if !has_return {
            self.emit(IrOpcode::PushConst(Constant::Null), span);
            self.emit(IrOpcode::Return, span);
        }
    }
}

#[derive(Debug, Default)]
struct DebugTable {
    symbols: Vec<DebugSymbol>,
    map: HashMap<DebugKey, u32>,
}

impl DebugTable {
    fn index(&mut self, span: &IrSpan) -> u32 {
        let key = DebugKey {
            file: span.file().clone(),
            line: span.line(),
            column: span.column(),
        };
        if let Some(index) = self.map.get(&key) {
            *index
        } else {
            let index = self.symbols.len() as u32;
            self.symbols.push(DebugSymbol {
                file: key.file.to_string(),
                line: key.line,
                column: key.column,
            });
            self.map.insert(key, index);
            index
        }
    }

    fn into_symbols(self) -> Vec<DebugSymbol> {
        self.symbols
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DebugKey {
    file: std::sync::Arc<str>,
    line: u32,
    column: u32,
}

fn lower_function(
    function: IrFunction,
    constants: &mut Vec<Constant>,
    debug_table: &mut DebugTable,
) -> Result<FunctionDescriptor, NovaBytecodeError> {
    use IrOpcode::*;

    let mut label_positions = HashMap::new();
    let mut ip = 0u32;
    for inst in &function.instructions {
        if let Label(label) = inst.opcode {
            label_positions.insert(label.0, ip);
        } else {
            ip += 1;
        }
    }

    let mut instructions = Vec::with_capacity(ip as usize);
    for inst in &function.instructions {
        let debug_index = Some(debug_table.index(&inst.span));
        match &inst.opcode {
            Label(_) => continue,
            PushConst(constant) => {
                constants.push(constant.clone());
                let index = (constants.len() - 1) as u32;
                instructions.push(Instruction::new(Opcode::LoadConst, index, 0, debug_index));
            }
            LoadLocal(index) => instructions.push(Instruction::new(
                Opcode::LoadLocal,
                u32::from(*index),
                0,
                debug_index,
            )),
            StoreLocal(index) => instructions.push(Instruction::new(
                Opcode::StoreLocal,
                u32::from(*index),
                0,
                debug_index,
            )),
            LoadGlobal(index) => {
                instructions.push(Instruction::new(Opcode::LoadGlobal, *index, 0, debug_index))
            }
            StoreGlobal(index) => instructions.push(Instruction::new(
                Opcode::StoreGlobal,
                *index,
                0,
                debug_index,
            )),
            Jump(label) => {
                let Some(target) = label_positions.get(&label.0) else {
                    return Err(NovaBytecodeError::Assembly("dangling jump label".into()));
                };
                instructions.push(Instruction::new(Opcode::Jump, *target, 0, debug_index));
            }
            JumpIfFalse(label) => {
                let Some(target) = label_positions.get(&label.0) else {
                    return Err(NovaBytecodeError::Assembly("dangling jump label".into()));
                };
                instructions.push(Instruction::new(
                    Opcode::JumpIfFalse,
                    *target,
                    0,
                    debug_index,
                ));
            }
            JumpIfTrue(label) => {
                let Some(target) = label_positions.get(&label.0) else {
                    return Err(NovaBytecodeError::Assembly("dangling jump label".into()));
                };
                instructions.push(Instruction::new(
                    Opcode::JumpIfTrue,
                    *target,
                    0,
                    debug_index,
                ));
            }
            Add => instructions.push(Instruction::new(Opcode::Add, 0, 0, debug_index)),
            Subtract => instructions.push(Instruction::new(Opcode::Subtract, 0, 0, debug_index)),
            Multiply => instructions.push(Instruction::new(Opcode::Multiply, 0, 0, debug_index)),
            Divide => instructions.push(Instruction::new(Opcode::Divide, 0, 0, debug_index)),
            Modulo => instructions.push(Instruction::new(Opcode::Modulo, 0, 0, debug_index)),
            Negate => instructions.push(Instruction::new(Opcode::Negate, 0, 0, debug_index)),
            Equals => instructions.push(Instruction::new(Opcode::Equals, 0, 0, debug_index)),
            NotEquals => instructions.push(Instruction::new(Opcode::NotEquals, 0, 0, debug_index)),
            Less => instructions.push(Instruction::new(Opcode::Less, 0, 0, debug_index)),
            LessEqual => instructions.push(Instruction::new(Opcode::LessEqual, 0, 0, debug_index)),
            Greater => instructions.push(Instruction::new(Opcode::Greater, 0, 0, debug_index)),
            GreaterEqual => {
                instructions.push(Instruction::new(Opcode::GreaterEqual, 0, 0, debug_index))
            }
            LogicalAnd => {
                instructions.push(Instruction::new(Opcode::LogicalAnd, 0, 0, debug_index))
            }
            LogicalOr => instructions.push(Instruction::new(Opcode::LogicalOr, 0, 0, debug_index)),
            LogicalNot => {
                instructions.push(Instruction::new(Opcode::LogicalNot, 0, 0, debug_index))
            }
            Call { target, args } => match target {
                CallTarget::Function(index) => instructions.push(Instruction::new(
                    Opcode::Call,
                    u32::try_from(*index).map_err(|_| {
                        NovaBytecodeError::Assembly("function index overflow".into())
                    })?,
                    u32::from(*args),
                    debug_index,
                )),
                CallTarget::Native(index) => instructions.push(Instruction::new(
                    Opcode::CallNative,
                    u32::try_from(*index)
                        .map_err(|_| NovaBytecodeError::Assembly("native index overflow".into()))?,
                    u32::from(*args),
                    debug_index,
                )),
            },
            Return => instructions.push(Instruction::new(Opcode::Return, 0, 0, debug_index)),
            Pop => instructions.push(Instruction::new(Opcode::Pop, 0, 0, debug_index)),
            BuildList(count) => instructions.push(Instruction::new(
                Opcode::BuildList,
                u32::from(*count),
                0,
                debug_index,
            )),
            Index => instructions.push(Instruction::new(Opcode::Index, 0, 0, debug_index)),
            StoreIndex => {
                instructions.push(Instruction::new(Opcode::StoreIndex, 0, 0, debug_index))
            }
            PushCatch { handler } => {
                let Some(target) = label_positions.get(&handler.0) else {
                    return Err(NovaBytecodeError::Assembly("dangling catch label".into()));
                };
                instructions.push(Instruction::new(Opcode::PushCatch, *target, 0, debug_index));
            }
            PopCatch => instructions.push(Instruction::new(Opcode::PopCatch, 0, 0, debug_index)),
            Throw => instructions.push(Instruction::new(Opcode::Throw, 0, 0, debug_index)),
        }
    }

    instructions.push(Instruction::new(Opcode::Halt, 0, 0, None));

    Ok(FunctionDescriptor::new(
        function.name,
        function.arity,
        function.locals,
        instructions,
    ))
}

fn optimise_program(program: &mut IrProgram) {
    for function in &mut program.functions {
        constant_fold(function);
    }
}

fn constant_fold(function: &mut IrFunction) {
    let mut i = 0;
    while i + 2 < function.instructions.len() {
        let (Some(left), Some(right), Some(op)) = (
            function.instructions.get(i),
            function.instructions.get(i + 1),
            function.instructions.get(i + 2),
        ) else {
            break;
        };
        if let (IrOpcode::PushConst(a), IrOpcode::PushConst(b), operator) =
            (&left.opcode, &right.opcode, &op.opcode)
        {
            if let Some(constant) = fold_constants(a, b, operator) {
                let span = op.span.clone();
                function.instructions.splice(
                    i..=i + 2,
                    [IrInstruction::new(IrOpcode::PushConst(constant), span)],
                );
                i = i.saturating_sub(1);
                continue;
            }
        }
        i += 1;
    }
}

fn fold_constants(a: &Constant, b: &Constant, op: &IrOpcode) -> Option<Constant> {
    use Constant::*;
    match op {
        IrOpcode::Add => match (a, b) {
            (Integer(lhs), Integer(rhs)) => Some(Integer(lhs + rhs)),
            (Float(lhs), Float(rhs)) => Some(Float(lhs + rhs)),
            (String(lhs), String(rhs)) => Some(String(format!("{lhs}{rhs}"))),
            _ => None,
        },
        IrOpcode::Subtract => match (a, b) {
            (Integer(lhs), Integer(rhs)) => Some(Integer(lhs - rhs)),
            (Float(lhs), Float(rhs)) => Some(Float(lhs - rhs)),
            _ => None,
        },
        IrOpcode::Multiply => match (a, b) {
            (Integer(lhs), Integer(rhs)) => Some(Integer(lhs * rhs)),
            (Float(lhs), Float(rhs)) => Some(Float(lhs * rhs)),
            _ => None,
        },
        IrOpcode::Divide => match (a, b) {
            (Integer(lhs), Integer(rhs)) if *rhs != 0 => Some(Float(*lhs as f64 / *rhs as f64)),
            (Float(lhs), Float(rhs)) if *rhs != 0.0 => Some(Float(lhs / rhs)),
            _ => None,
        },
        IrOpcode::Modulo => match (a, b) {
            (Integer(lhs), Integer(rhs)) if *rhs != 0 => Some(Integer(lhs % rhs)),
            _ => None,
        },
        IrOpcode::Equals => Some(Constant::Boolean(a == b)),
        IrOpcode::NotEquals => Some(Constant::Boolean(a != b)),
        IrOpcode::Less => fold_compare(a, b, |lhs, rhs| lhs < rhs),
        IrOpcode::LessEqual => fold_compare(a, b, |lhs, rhs| lhs <= rhs),
        IrOpcode::Greater => fold_compare(a, b, |lhs, rhs| lhs > rhs),
        IrOpcode::GreaterEqual => fold_compare(a, b, |lhs, rhs| lhs >= rhs),
        IrOpcode::LogicalAnd => match (a, b) {
            (Boolean(lhs), Boolean(rhs)) => Some(Boolean(*lhs && *rhs)),
            _ => None,
        },
        IrOpcode::LogicalOr => match (a, b) {
            (Boolean(lhs), Boolean(rhs)) => Some(Boolean(*lhs || *rhs)),
            _ => None,
        },
        _ => None,
    }
}

fn fold_compare<F>(a: &Constant, b: &Constant, cmp: F) -> Option<Constant>
where
    F: Fn(f64, f64) -> bool,
{
    match (a, b) {
        (Constant::Integer(lhs), Constant::Integer(rhs)) => {
            Some(Constant::Boolean(cmp(*lhs as f64, *rhs as f64)))
        }
        (Constant::Float(lhs), Constant::Float(rhs)) => Some(Constant::Boolean(cmp(*lhs, *rhs))),
        _ => None,
    }
}

#[derive(Debug)]
struct Builtins {
    map: HashMap<&'static str, usize>,
}

impl Builtins {
    fn new() -> Self {
        let names = [
            "print",
            "println",
            "read_file",
            "write_file",
            "read_bytes",
            "time_now",
            "time_sleep",
            "rand_int",
            "rand_float",
            "net_udp_bind",
            "net_udp_send",
        ];
        let map = names
            .into_iter()
            .enumerate()
            .map(|(idx, name)| (name, idx))
            .collect();
        Self { map }
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.map.get(name).copied()
    }
}
