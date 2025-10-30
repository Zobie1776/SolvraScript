# Register Allocation Strategy

This document outlines the baseline register allocation architecture for SolvraCore. The SSA IR produced by `ir.rs` and `lowering.rs` will be lowered to target machine instructions via architecture backends. A common register allocator is required to map virtual registers to physical registers while respecting target constraints.

## Goals
- Provide an initial linear-scan allocator shared across x86_64, Armv7, and AArch64 backends.
- Keep the design extensible so graph-coloring or hybrid allocators can be introduced later.
- Integrate with the IR so that live intervals and spills are expressed in a backend-neutral manner.

## Non-Goals (for the first iteration)
- Advanced optimisations such as rematerialisation, coalescing heuristics, or register priority tuning.
- Callee-saved/restored register management (these will be stubbed per backend initially).

## Architecture
- `RegisterClass`: Represents groups of physical registers (e.g., general-purpose, floating-point). Each backend will define its own sets.
- `PhysicalRegister`: Metadata describing registers available in the target architecture.
- `Allocation`: Maps IR values to physical registers or spill slots.
- `LinearScanAllocator`: Baseline allocator performing live interval construction and linear scan assignment.

## Live Intervals & Liveness
Before allocation, liveness analysis is required. For the initial implementation we will:
1. Derive basic block ordering using reverse post-order.
2. Compute live-in/live-out sets using iterative dataflow.
3. Build live intervals per SSA value, capturing start/end positions along a linearised instruction sequence.

## Deliverables
1. `src/backend/register_alloc.rs` implementing data structures and the linear scan algorithm scaffold.
2. Trait `RegisterAllocator` exposing `allocate(function: &ir::Function, target: &dyn RegisterLayout) -> AllocationResult`.
3. Unit tests covering simple virtual register assignment and spill behaviour.
4. Integration hooks in backend modules (e.g., `x86_64`) will be added later once code emission consumes allocation results.

## Incremental Plan
- Implement data structures (`RegisterClass`, `PhysicalRegister`, `AllocationResult`).
- Add `LinearScanAllocator` with placeholders for liveness/interval computation.
- Provide a smoke test using a hand-crafted `ir::Function` to ensure allocations return sane results.
- Later iterations will flesh out liveness, spilling, and backend-specific register sets.
