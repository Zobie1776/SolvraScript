# NovaCore

NovaCore provides the low-level runtime, bytecode toolchain, and platform services that back the broader NovaOS stack. The crate now formalises the hybrid C/C++ × Rust × Python architecture by exposing explicit bootstrap hooks, deterministic memory contracts, and a reusable concurrency executor that other crates (notably `nova_script`) can depend on.

## Memory Model

- Runtime instances are created through `NovaRuntime::from_bootstrap`, which mirrors a CRT0-style startup: you select the reserved stack size, heap capacity, and worker count via `RuntimeBootstrap` before execution begins.
- Heap allocations that must cross crate boundaries flow through `memory::MemoryContract`. The contract presents manual `allocate_arc` / `release` semantics, returning stable `MemoryHandle`s so higher layers can manage lifetimes deterministically.
- `MemoryContract::stats` surfaces allocation counts and usage, forming the basis for runtime telemetry and the new NovaScript `core_memory_stats()` builtin.

## Compilation Pipeline

- `NovaRuntime::execute_module` exposes the AOT pipeline directly. Callers hand the runtime an already-loaded `Module` and receive a fully interpreted result after LLVM/AOT compilation through the active backend.
- Module loaders still register bytecode through `ModuleLoader`, but the new API keeps the compiler linkage explicit so NovaScript can stage compiled `.nvc` artefacts alongside script modules.

## Execution & Integration

- `NovaRuntime::bootstrap()` returns a builder tuned for deterministic stack and heap sizing, aligning with the C runtime expectations outlined in the architectural guidelines.
- Consumers can retrieve the shared `MemoryContract` (`NovaRuntime::memory_contract()`) and `DriverRegistry` to implement foreign language bridges without poking into internal state.
- NovaScript now leans on these APIs via an internal `CoreBridge`, ensuring compiled NovaCore modules and NovaScript objects share the same allocator and lifetime tracking.
- Host applications can drive cooperative work with `NovaRuntime::run_loop()`, which polls the shared executor until all scheduled jobs report `LoopState::Idle`.

## Concurrency Model

- `concurrency::TaskExecutor` wraps the existing work-stealing scheduler with a result-aware handle (`TaskHandle`). Runtime clients can spawn deterministic background jobs and poll or block until completion, mirroring Rust's `JoinHandle` semantics without requiring async syntax.
- `NovaRuntime::executor()` returns a cloneable executor seeded with the bootstrap worker count, making it trivial to wire async-style helpers in higher layers.

## Dynamic Module Loading

- The runtime continues to support mixed `.nvc` and dynamically loaded modules. The new `CoreBridge` (in NovaScript) feeds modules into `load_module_file`, stores the resulting `MemoryHandle` inside the shared contract, and invokes `execute_module` when a script requests execution.
- Memory handles can be safely released via the contract, preventing leaks when NovaScript unloads modules or drops NovaCore object handles.
- Use `CoreBridge::execute_async` to queue interpreter work onto NovaCore's executor and then call `NovaRuntime::run_loop()` to synchronously drain the queue when embedding.

## Safety Notes

- Manual releases of `MemoryHandle`s should always go through the contract to avoid double-frees. The contract guards against capacity overrun by returning `MemoryError::CapacityExceeded` before allocations occur.
- The CRT0-style bootstrap zeroes the reserved stack pages, ensuring deterministic initial memory state even before the Rust runtime fully initialises.
