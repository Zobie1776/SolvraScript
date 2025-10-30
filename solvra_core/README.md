# SolvraCore

SolvraCore provides the low-level runtime, bytecode toolchain, and platform services that back the broader SolvraOS stack. The crate now formalises the hybrid C/C++ × Rust × Python architecture by exposing explicit bootstrap hooks, deterministic memory contracts, and a reusable concurrency executor that other crates (notably `solvra_script`) can depend on.

## Memory Model

- Runtime instances are created through `SolvraRuntime::from_bootstrap`, which mirrors a CRT0-style startup: you select the reserved stack size, heap capacity, and worker count via `RuntimeBootstrap` before execution begins.
- Heap allocations that must cross crate boundaries flow through `memory::MemoryContract`. The contract presents manual `allocate_arc` / `release` semantics, returning stable `MemoryHandle`s so higher layers can manage lifetimes deterministically.
- `MemoryContract::stats` surfaces allocation counts and usage, forming the basis for runtime telemetry and the new SolvraScript `core_memory_stats()` builtin.

## Compilation Pipeline

- `SolvraRuntime::execute_module` exposes the AOT pipeline directly. Callers hand the runtime an already-loaded `Module` and receive a fully interpreted result after LLVM/AOT compilation through the active backend.
- Module loaders still register bytecode through `ModuleLoader`, but the new API keeps the compiler linkage explicit so SolvraScript can stage compiled `.svc` artefacts alongside script modules.

## Execution & Integration

- `SolvraRuntime::bootstrap()` returns a builder tuned for deterministic stack and heap sizing, aligning with the C runtime expectations outlined in the architectural guidelines.
- Consumers can retrieve the shared `MemoryContract` (`SolvraRuntime::memory_contract()`) and `DriverRegistry` to implement foreign language bridges without poking into internal state.
- SolvraScript now leans on these APIs via an internal `CoreBridge`, ensuring compiled SolvraCore modules and SolvraScript objects share the same allocator and lifetime tracking.
- Host applications can drive cooperative work with `SolvraRuntime::run_loop()`, which polls the shared executor until all scheduled jobs report `LoopState::Idle`.

## Concurrency Model

- `concurrency::TaskExecutor` wraps the existing work-stealing scheduler with a result-aware handle (`TaskHandle`). Runtime clients can spawn deterministic background jobs and poll or block until completion, mirroring Rust's `JoinHandle` semantics without requiring async syntax.
- `SolvraRuntime::executor()` returns a cloneable executor seeded with the bootstrap worker count, making it trivial to wire async-style helpers in higher layers.

## Dynamic Module Loading

- The runtime continues to support mixed `.svc` and dynamically loaded modules. The new `CoreBridge` (in SolvraScript) feeds modules into `load_module_file`, stores the resulting `MemoryHandle` inside the shared contract, and invokes `execute_module` when a script requests execution.
- Memory handles can be safely released via the contract, preventing leaks when SolvraScript unloads modules or drops SolvraCore object handles.
- Use `CoreBridge::schedule_script` to queue interpreter work onto SolvraCore's executor and then call `SolvraRuntime::run_loop()` to synchronously drain the queue when embedding.

## Safety Notes

- Manual releases of `MemoryHandle`s should always go through the contract to avoid double-frees. The contract guards against capacity overrun by returning `MemoryError::CapacityExceeded` before allocations occur.
- The CRT0-style bootstrap zeroes the reserved stack pages, ensuring deterministic initial memory state even before the Rust runtime fully initialises.

## Phase 3 – Unified Runtime + VM Fusion

- `SolvraRuntime::run_loop()` now drains the work-stealing executor until no queued tasks remain. Each iteration reports the number of in-flight jobs via `tracing::info!`, making it easy to instrument long-running workloads.
- `CoreBridge::schedule_script(..)` parses and executes SolvraScript sources on SolvraCore's executor threads. Scripts scheduled this way share the same deterministic runtime contract as compiled `.svc` modules.
- Host applications can enqueue interpreter jobs and bytecode modules side by side, then call `run_loop()` once to drive both execution paths under a single cooperative event loop.
