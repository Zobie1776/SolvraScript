# NovaCore Bytecode → IR Lowering Plan

This design note describes the initial lowering pipeline from Nova bytecode (`NovaBytecode`) into the SSA-based IR defined in `src/backend/ir.rs`.

## Goals
- Translate existing bytecode instructions into SSA form with explicit control-flow graph (CFG).
- Prepare IR for subsequent optimisation, register allocation, and codegen stages.
- Preserve debug metadata (line/column) and constant pool references.

## Scope
The initial lowering pass will target the interpreter-compatible subset of Nova bytecode (arithmetic, comparisons, branching, function calls, and simple memory ops). The pass will evolve to cover driver IO and advanced features as new opcodes are added.

## Strategy
1. **Module Builder** – introduce `LoweringContext` responsible for producing an `ir::Module` and maintaining maps between bytecode indices and SSA values.
2. **Function Translation** – for each bytecode function:
   - Create an IR function with matching parameters and return type.
   - Pre-scan instructions to build block boundaries (leaders at entry, branch targets, instructions following branch/return).
   - Allocate basic blocks in the IR module, mapping bytecode offsets to `BlockId`s.
   - Translate instructions sequentially, emitting SSA values via `FunctionBuilder`.
   - Generate phi nodes for merge points where stack values differ across predecessors.
3. **Value Mapping** – Use a stack-based representation to model the Nova operand stack. Each bytecode instruction consumes/produces SSA `ValueId`s.
4. **Debug Info** – attach debug names to values and instructions using metadata from `NovaBytecode`.
5. **Error Handling** – return structured errors (enum) when encountering unsupported opcodes or malformed bytecode.

## Deliverables
- New file `src/backend/lowering.rs` implementing the lowering context and public API `lower_bytecode(&NovaBytecode) -> Result<ir::Module>`.
- Unit tests covering simple functions (straight-line arithmetic, branching, function call).
- Integration hook in `backend/mod.rs` exposing the lowering helper and documenting usage.

Subsequent iterations will add advanced control flow (loops, try/catch), memory operations, and driver IO translation.
