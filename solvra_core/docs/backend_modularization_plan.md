# SolvraCore Backend Modularization Plan

This document tracks the step-by-step refactor required to turn the single
backend implementation into a modular architecture that supports multiple CPU
targets.  The plan is intentionally detailed so that future contributors can
trace the rationale for the new module layout and backend trait design.

## Goals

* Extract architecture-neutral components (parser, AST, VM bytecode, runtime
  orchestration) so they can be reused by any backend.
* Introduce an explicit backend trait that defines the contract a backend must
  satisfy to integrate with SolvraCore.
* Create dedicated backend modules for x86_64, ARMv7, and AArch64.  The initial
  ARM implementations can defer to the interpreter while keeping room for
  architecture-specific code generation in the future.
* Wire the runtime so the active backend is selected at compile time via Cargo
  features.

## Module Layout

The refactor introduces a new `backend` namespace under `solvra_core/src`:

```
solvra_core/src/backend/
├── mod.rs          // Backend trait, shared types, feature selection helpers
├── manager.rs      // Lightweight registry/selector used by the runtime
├── x86_64/mod.rs   // Existing interpreter-based backend extracted from the
│                   // current implementation
├── arm/mod.rs      // ARMv7 backend (initially interpreter-based)
└── aarch64/mod.rs  // AArch64 backend (initially interpreter-based)
```

Common bytecode and runtime logic that is architecture-agnostic will remain in
`bytecode/` and other existing modules.  The x86_64 backend will wrap the
existing interpreter and expose it through the backend trait, ensuring behaviour
remains unchanged when the default feature is enabled.

## Backend Trait Design

All backends implement a new `ArchitectureBackend` trait:

```rust
pub trait ArchitectureBackend {
    /// Human-friendly name of the backend ("x86_64", "armv7", ...).
    fn name(&self) -> &'static str;

    /// Returns the target triple used for code generation.
    fn target(&self) -> TargetArch;

    /// Lowers Solvra bytecode or IR into a backend-specific artifact (native
    /// machine code, a specialised bytecode, etc.).
    fn compile(&self, program: &BackendInput) -> Result<BackendArtifact>;

    /// Executes a previously compiled artifact.
    fn execute(
        &self,
        artifact: BackendArtifact,
        config: RuntimeConfig,
        modules: Arc<RwLock<ModuleLoader>>,
    ) -> SolvraResult<Value>;

    /// Optional hook for backend-specific optimisation passes.  Default is a
    /// no-op so interpreter-based backends can remain simple.
    fn optimise(&self, _program: &mut BackendInput) -> Result<()> {
        Ok(())
    }
}
```

* `TargetArch` is a lightweight enum describing supported architectures and can
  be reused by tooling.
* `BackendInput` will initially wrap `SolvraBytecode`.  The abstraction keeps the
  door open for richer IR or SSA-based lowering later without changing the
  trait.
* `BackendArtifact` is an enum capturing the outputs we care about today:
  bytecode (for interpreter backends) and opaque binary blobs (for future native
  code emission).

## Runtime Integration

* `SolvraRuntime` gains a `backend: Arc<dyn ArchitectureBackend>` field.
* A helper `backend::active()` function picks the correct backend based on
  enabled Cargo features.  Mutually exclusive features are enforced at compile
  time.
* The runtime uses the backend to compile and execute bytecode.  When targeting
  interpreter-based backends we simply pass through the bytecode.

## Feature Flags

New Cargo features control backend compilation:

* `backend-x86_64` (default) – enables the x86 backend module.
* `backend-armv7` – enables the ARMv7 backend module.
* `backend-aarch64` – enables the ARM64 backend module.

A `backend-default` feature flag may optionally alias the platform-preferred
backend, but initially we rely on `backend-x86_64` as the default for backwards
compatibility.  Compilation will emit a clear error if more than one backend is
selected simultaneously.

## Testing Strategy

* Unit tests asserting that the backend selector resolves to the expected
  architecture for each feature combination.
* Integration tests running sample SolvraScript programs through each backend.
  Interpreter-based backends will reuse the same execution path so the tests can
  run on any host.
* CI updates invoke `cargo test --no-default-features --features backend-armv7`
  and the equivalent for AArch64 to ensure all backends build.

## Documentation

A new `docs/backend.md` (or similar) document will explain:

* How the backend trait works and expectations for new backends.
* How to enable a specific backend via Cargo features and cross-compilation.
* Where to place architecture-specific files.

This plan will guide the subsequent refactor steps.
