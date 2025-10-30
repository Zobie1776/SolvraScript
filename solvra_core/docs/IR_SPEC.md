# SolvraCore SSA Intermediate Representation

This document introduces the initial SolvraCore SSA IR implemented in `src/backend/ir.rs`.

## Design Goals
- Provide a backend-neutral representation suitable for optimisation and code generation.
- Preserve SSA form: every instruction that yields a value has a single static assignment.
- Offer builders to create functions block-by-block while enforcing structural invariants.

## Core Concepts
- **Module**: container for functions; currently in-memory only.
- **Function**: holds signature (parameter/result types), SSA values, basic blocks, and instructions.
- **Basic Block**: sequence of instructions with a single terminator and explicit predecessor/successor lists.
- **Values**: represented by `ValueId`; may be parameters, constants, or instruction results.
- **Instructions**: identified by `InstructionId`; each carries an opcode, operands, optional result type, and parent block.

## Type System
- Primitive scalar types: `bool`, `i32`, `i64`, `f32`, `f64`, `ptr`, and `void`.
- Additional composite/vector types will be added once needed by bytecode lowering.

## Opcodes (Initial Set)
- Arithmetic: `Add`, `Sub`, `Mul`, `Div`, `Rem`.
- Comparisons: `CmpEq`, `CmpNe`, `CmpLt`, `CmpLe`, `CmpGt`, `CmpGe`.
- Memory: `Load`, `Store`.
- Control Flow: `Phi`, `Branch`, `CondBranch`, `Return`, `Call`.

## Builder Workflow
1. Create a `Module` and add functions with `Module::add_function`.
2. Use `FunctionBuilder` to append basic blocks, emit instructions, constants, and terminators.
3. Builder maintains the current insertion point; explicit `position_at_end` calls adjust it.
4. Helper queries expose block counts, instruction metadata, and parameter handles.

## Next Steps
- Lower Solvra bytecode into this IR (`lowering.rs`).
- Expand opcode coverage for Solvra bytecode operations.
- Attach debug metadata (source locations, variable names) to values and instructions.
- Integrate register allocation and scheduling stages consuming this IR.

## Example
```rust
let mut module = Module::new();
let sig = FunctionSignature::new(vec![IrType::I32], IrType::I32);
let func_id = module.add_function("add_one", sig);
let mut builder = FunctionBuilder::new(&mut module, func_id);
let entry = builder.append_block(Some("entry".into()));
builder.position_at_end(entry);
let param = builder.parameters()[0];
let one = builder.make_constant(ConstantValue::I32(1), IrType::I32, None);
let result = builder.emit_value(Opcode::Add, vec![param, one], IrType::I32, None);
builder.emit_terminator(Opcode::Return, vec![result], None);
```
This mirrors the unit test contained in `ir.rs`.
