//=============================================
// solvra_core/src/backend/ir.rs
//=============================================
// Author: SolvraCore Team
// License: MIT
// Goal: Define SolvraCore SSA-based intermediate representation
// Objective: Provide data structures and builders used by lowering, optimisations, and codegen
//=============================================

use std::fmt;

//=============================================
// SECTION 1: Identifiers & Core Types
//=============================================

/// Identifier for functions stored within an [`Module`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId(usize);

/// Identifier for basic blocks (SSA blocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(usize);

/// Identifier for SSA values (parameters, constants, instruction results).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(usize);

/// Identifier for instructions. In SSA, every instruction that yields
/// a value has a matching [`ValueId`]. Void instructions have no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstructionId(usize);

impl InstructionId {
    /// Constructs an instruction identifier mapped from a value id.
    pub fn from_value(value: ValueId) -> Self {
        InstructionId(value.0)
    }
}

impl From<InstructionId> for ValueId {
    fn from(instr: InstructionId) -> Self {
        ValueId(instr.0)
    }
}

/// Identifier for function parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParameterId(usize);

/// Primitive types supported by the IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
    Void,
    Bool,
    I32,
    I64,
    F32,
    F64,
    Ptr,
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::Void => write!(f, "void"),
            IrType::Bool => write!(f, "bool"),
            IrType::I32 => write!(f, "i32"),
            IrType::I64 => write!(f, "i64"),
            IrType::F32 => write!(f, "f32"),
            IrType::F64 => write!(f, "f64"),
            IrType::Ptr => write!(f, "ptr"),
        }
    }
}

/// Immediate constant payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

/// Classification of an SSA value.
#[derive(Debug, Clone)]
enum ValueKind {
    Parameter(ParameterId),
    Instruction(InstructionId),
    Constant(ConstantValue),
}

/// Stored SSA value metadata.
#[derive(Debug, Clone)]
struct ValueData {
    ty: IrType,
    kind: ValueKind,
    name: Option<String>,
}

//=============================================
// SECTION 2: Instructions & Blocks
//=============================================

/// Operation codes supported by the Solvra IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opcode {
    /// SSA Phi node (first operands correspond to predecessor/value pairs).
    Phi,
    /// Integer/float arithmetic.
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    /// Comparisons.
    CmpEq,
    CmpNe,
    CmpLt,
    CmpLe,
    CmpGt,
    CmpGe,
    /// Memory operations.
    Load,
    Store,
    /// Control flow: unconditional branch.
    Branch,
    /// Control flow: conditional branch.
    CondBranch,
    /// Return from function.
    Return,
    /// Call another function.
    Call,
}

/// Instruction metadata.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub id: InstructionId,
    pub opcode: Opcode,
    pub ty: Option<IrType>,
    pub operands: Vec<ValueId>,
    pub block: BlockId,
    pub debug_name: Option<String>,
}

impl Instruction {
    fn new(
        id: InstructionId,
        opcode: Opcode,
        ty: Option<IrType>,
        operands: Vec<ValueId>,
        block: BlockId,
    ) -> Self {
        Self {
            id,
            opcode,
            ty,
            operands,
            block,
            debug_name: None,
        }
    }
}

/// SSA basic block storing instructions and control-flow edges.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub name: Option<String>,
    pub instructions: Vec<InstructionId>,
    pub terminator: Option<InstructionId>,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    fn new(id: BlockId, name: Option<String>) -> Self {
        Self {
            id,
            name,
            instructions: Vec::new(),
            terminator: None,
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }
}

//=============================================
// SECTION 3: Function & Module Containers
//=============================================

/// Function signature capturing parameter and return types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub params: Vec<IrType>,
    pub result: IrType,
}

impl FunctionSignature {
    pub fn new(params: Vec<IrType>, result: IrType) -> Self {
        Self { params, result }
    }
}

/// Function definition stored inside a module.
#[derive(Debug, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub signature: FunctionSignature,
    parameters: Vec<ValueId>,
    blocks: Vec<BasicBlock>,
    values: Vec<ValueData>,
    instructions: Vec<Instruction>,
}

impl Function {
    fn new(id: FunctionId, name: impl Into<String>, signature: FunctionSignature) -> Self {
        let mut function = Self {
            id,
            name: name.into(),
            signature,
            parameters: Vec::new(),
            blocks: Vec::new(),
            values: Vec::new(),
            instructions: Vec::new(),
        };
        function.initialise_parameters();
        function
    }

    fn initialise_parameters(&mut self) {
        let mut ids = Vec::with_capacity(self.signature.params.len());
        for (index, ty) in self.signature.params.clone().into_iter().enumerate() {
            let value_id = self.alloc_value(ValueKind::Parameter(ParameterId(index)), ty, None);
            ids.push(value_id);
        }
        self.parameters = ids;
    }

    fn alloc_value(&mut self, kind: ValueKind, ty: IrType, name: Option<String>) -> ValueId {
        let id = ValueId(self.values.len());
        self.values.push(ValueData { ty, kind, name });
        id
    }

    fn alloc_instruction(
        &mut self,
        block: BlockId,
        opcode: Opcode,
        ty: Option<IrType>,
        operands: Vec<ValueId>,
        name: Option<String>,
    ) -> InstructionId {
        let value_id = if let Some(result_ty) = &ty {
            self.alloc_value(
                ValueKind::Instruction(InstructionId(self.instructions.len())),
                result_ty.clone(),
                name.clone(),
            )
        } else {
            ValueId(self.instructions.len())
        };
        let id = InstructionId(value_id.0);
        let mut instr = Instruction::new(id, opcode, ty, operands, block);
        instr.debug_name = name;
        self.instructions.push(instr);
        id
    }

    fn block_mut(&mut self, block: BlockId) -> &mut BasicBlock {
        &mut self.blocks[block.0]
    }

    fn value_ty(&self, value: ValueId) -> &IrType {
        &self.values[value.0].ty
    }

    fn block(&self, block: BlockId) -> &BasicBlock {
        &self.blocks[block.0]
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    fn add_block(&mut self, name: Option<String>) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(BasicBlock::new(id, name));
        id
    }
}

/// IR module containing functions.
#[derive(Debug, Default)]
pub struct Module {
    functions: Vec<Function>,
}

impl Module {
    /// Create an empty module.
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    /// Add a function to the module, returning its identifier.
    pub fn add_function(
        &mut self,
        name: impl Into<String>,
        signature: FunctionSignature,
    ) -> FunctionId {
        let id = FunctionId(self.functions.len());
        self.functions.push(Function::new(id, name, signature));
        id
    }

    /// Borrow a function immutably.
    pub fn function(&self, id: FunctionId) -> &Function {
        &self.functions[id.0]
    }

    /// Borrow a function mutably.
    pub fn function_mut(&mut self, id: FunctionId) -> &mut Function {
        &mut self.functions[id.0]
    }

    /// Return an iterator over functions.
    pub fn functions(&self) -> impl Iterator<Item = &Function> {
        self.functions.iter()
    }
}

//=============================================
// SECTION 4: Function Builder
//=============================================

/// Builder used to construct SSA functions block-by-block.
pub struct FunctionBuilder<'m> {
    module: &'m mut Module,
    func_id: FunctionId,
    current_block: Option<BlockId>,
}

impl<'m> FunctionBuilder<'m> {
    /// Create a new builder for the given function.
    pub fn new(module: &'m mut Module, func_id: FunctionId) -> Self {
        Self {
            module,
            func_id,
            current_block: None,
        }
    }

    /// Access the underlying function.
    fn function(&self) -> &Function {
        self.module.function(self.func_id)
    }

    /// Mutably access the underlying function.
    fn function_mut(&mut self) -> &mut Function {
        self.module.function_mut(self.func_id)
    }

    /// Append a new basic block to the function.
    pub fn append_block(&mut self, name: impl Into<Option<String>>) -> BlockId {
        let block_id = self.function_mut().add_block(name.into());
        if self.current_block.is_none() {
            self.current_block = Some(block_id);
        }
        block_id
    }

    /// Position the builder to append instructions to the specified block.
    pub fn position_at_end(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    /// Emit a non-terminator instruction that produces a value.
    pub fn emit_value(
        &mut self,
        opcode: Opcode,
        operands: Vec<ValueId>,
        ty: IrType,
        name: impl Into<Option<String>>,
    ) -> ValueId {
        let block = self.current_block.expect("builder not positioned");
        let func = self.function_mut();
        let instr_id =
            func.alloc_instruction(block, opcode, Some(ty.clone()), operands, name.into());
        let block_ref = func.block_mut(block);
        block_ref.instructions.push(instr_id);
        ValueId::from(instr_id)
    }

    /// Emit a terminator instruction (does not yield a value).
    pub fn emit_terminator(
        &mut self,
        opcode: Opcode,
        operands: Vec<ValueId>,
        name: impl Into<Option<String>>,
    ) -> InstructionId {
        let block = self.current_block.expect("builder not positioned");
        let func = self.function_mut();
        let instr_id = func.alloc_instruction(block, opcode, None, operands, name.into());
        let block_ref = func.block_mut(block);
        block_ref.terminator = Some(instr_id);
        instr_id
    }

    /// Return the identifiers of all parameters.
    pub fn parameters(&self) -> &[ValueId] {
        &self.function().parameters
    }

    /// Retrieve the type of a value.
    pub fn value_type(&self, value: ValueId) -> IrType {
        self.function().value_ty(value).clone()
    }

    /// Create an immutable constant inside the function and return its value id.
    pub fn make_constant(
        &mut self,
        value: ConstantValue,
        ty: IrType,
        name: impl Into<Option<String>>,
    ) -> ValueId {
        self.function_mut()
            .alloc_value(ValueKind::Constant(value), ty, name.into())
    }

    /// Borrow a basic block.
    pub fn block(&self, block: BlockId) -> &BasicBlock {
        self.function().block(block)
    }

    /// Borrow instruction metadata for analysis passes.
    pub fn instructions(&self) -> &[Instruction] {
        self.function().instructions()
    }

    /// Number of blocks currently present in the function.
    pub fn block_count(&self) -> usize {
        self.function().blocks.len()
    }
}

//=============================================
// SECTION 5: Tests
//=============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_creates_basic_function() {
        let mut module = Module::new();
        let sig = FunctionSignature::new(vec![IrType::I32], IrType::I32);
        let func_id = module.add_function("add_one", sig);
        let mut builder = FunctionBuilder::new(&mut module, func_id);

        let entry = builder.append_block(Some("entry".into()));
        builder.position_at_end(entry);
        let param = builder.parameters()[0];
        let const_one =
            builder.make_constant(ConstantValue::I32(1), IrType::I32, Some("one".into()));
        let sum = builder.emit_value(
            Opcode::Add,
            vec![param, const_one],
            IrType::I32,
            Some("sum".into()),
        );
        builder.emit_terminator(Opcode::Return, vec![sum], None);

        assert_eq!(builder.block_count(), 1);
        assert_eq!(builder.instructions().len(), 2); // add + return
        assert_eq!(builder.block(entry).instructions.len(), 1);
    }
}
