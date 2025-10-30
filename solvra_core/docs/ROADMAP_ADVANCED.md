# SolvraCore Advanced Roadmap

This document outlines the staged plan for evolving SolvraCore into the feature-complete runtime described in the latest engineering brief. Each stage is scoped so that we can deliver, review, and test one file (or tightly scoped module) at a time while keeping the codebase buildable.

## Stage 0 — Foundations & Tracking
1. **Roadmap (this file).** Establish scope, deliverables, and sequencing.
2. **Design Memos.** For major subsystems (IR/JIT, allocator, concurrency, HAL), create focused design documents under `docs/` before touching code.
3. **CI Requirements.** Capture new testing/build expectations in `docs/CI_PLAN.md` so every feature includes validation guidance.

## Stage 1 — Compiler Backend Enablement
1. `src/backend/ir.rs`: Define a Solvra SSA/IR representation, instruction set, and builder APIs.
2. `src/backend/lowering.rs`: Lower existing Solvra bytecode into the IR, handling control-flow graphs and phi nodes.
3. `src/backend/register_alloc.rs`: Implement register allocator skeleton (linear scan baseline) plus trait hooks for future algorithms.
4. `src/backend/scheduler.rs`: Add instruction scheduling utilities respecting pipeline constraints.
5. `src/backend/codegen/{x86_64,armv7,aarch64}.rs`: Scaffold native code emission traits that consume IR and register-allocation output.
6. `src/backend/jit.rs`: Introduce JIT execution pipeline (lazy compilation cache, hot-path counters).
7. `src/backend/verification.rs`: Build bytecode and IR verifiers to guard against malformed input.

## Stage 2 — Runtime & Debug Enhancements
1. `runtime/debug/symbols.rs`: Capture debug symbol metadata (local variables, inline stacks, source positions).
2. `runtime/debug/tracing.rs`: Provide tracing hooks for instruction-level profiling.
3. `runtime/errors/stack_unwind.rs`: Expand exception stack unwinding for both interpreter and backend executors.
4. `runtime/cost_meter.rs`: Track deterministic instruction budgets and sandbox resource usage.
5. `runtime/scheduler/async.rs`: Add cooperative multitasking primitives, timers, and async event integration.

## Stage 3 — Memory Management
1. `runtime/memory/arena.rs`: Implement bump allocators for short-lived objects/bytecode heaps.
2. `runtime/memory/gc.rs`: Design tracing GC with optional reference counting fallback.
3. `runtime/memory/layout.rs`: Optimise object layout and cache locality using profiling data.
4. Replace `Arc<RwLock<...>>` patterns with lock-free or sharded alternatives (`runtime/sync/`).

## Stage 4 — HAL & Device Integration
1. `sys/hal/device_registry.rs`: Expand device registry with dynamic discovery and capability metadata.
2. `sys/hal/drivers/*`: Incrementally add drivers (GPIO, storage bus, audio, sensors, USB) using platform-appropriate bindings (`libc`, `winapi`, `libusb`, ALSA/PulseAudio).
3. `sys/hal/dma.rs`: Introduce Direct Memory Access abstraction with safety checks.
4. `sys/hal/security.rs`: Sandbox policies for driver access, mandatory isolation, and audit logging.

## Stage 5 — IPC & System Calls
1. `runtime/ipc/jsonrpc.rs`: Harden IPC server, support concurrency, and extend command set (power, theme, layout).
2. `runtime/syscalls/mod.rs`: Provide SolvraScript-visible system call API bridging to HAL.
3. Update SolvraScript standard library modules to exercise new operations (tests under `solvra_script/tests`).

## Stage 6 — Build, Testing, CI
1. `docs/CI_PLAN.md`: Document new pipelines (cross-compilation, perf benchmarks, driver tests).
2. `.github/workflows/ci.yml`: Add matrix builds (x86_64, ARMv7, AArch64) with `cargo clippy --all-targets --all-features`, `cargo test --all`, and benchmark smoke tests.
3. `benches/`: Introduce criterion benchmarks for interpreter vs backend throughput, allocator performance, and HAL drivers.

## Stage 7 — Documentation & Examples
1. `docs/IR_SPEC.md`: Formal specification of the Solvra IR.
2. `docs/JIT_OVERVIEW.md`: Explain JIT pipeline, hot-path detection, and safeguards.
3. `docs/MEMORY_MODEL.md`: Describe allocator strategy, GC, and tuning knobs.
4. `examples/`: Add advanced SolvraScript samples (system calls, concurrency, device interaction) covering the `.svs` test checklist.

## Execution Guidelines
- **One File at a Time:** For each item above, land a design note (if required), implement the code with Zobie-format section headers, and add targeted documentation/tests before moving on.
- **Feature Flags:** Introduce incremental functionality behind `cfg(feature = "...")` gates (e.g., `backend`, `jit`, `plugins`, `offline`) to keep default builds stable.
- **Testing:** Pair each file change with focused unit/integration tests, ensuring they are runnable offline where possible.
- **Documentation:** After finishing a file, add or update the relevant doc section (`docs/` or module-level rustdoc) so new contributors can follow along.

This roadmap will evolve as each milestone lands; append dated notes summarizing completed work and next steps.
