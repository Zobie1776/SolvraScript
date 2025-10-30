# SolvraCore Backend Architecture

SolvraCore now exposes a modular backend interface that makes it possible to
support multiple CPU architectures from a single codebase.  This document
summarises the design, describes each backend module, and explains how to build
and test the crate for specific targets.

## Module Layout

```
src/
├── backend/
│   ├── mod.rs       # Backend trait, target selection helpers
│   ├── x86_64/      # Sative x86_64 backend backed by the bytecode interpreter
│   ├── arm/         # ARMv7 backend (interpreter based for now)
│   └── aarch64/     # ARM64 backend (interpreter based for now)
├── bytecode/        # Architecture agnostic IR, assembler, and VM
└── ...
```

The `backend` module defines the [`ArchitectureBackend`](../src/backend/mod.rs)
trait which every backend implements.  Backends are compiled conditionally using
Cargo feature flags so only the relevant code is included in a build.

## Feature Flags

Three backend features are provided:

* `backend-x86_64` (default) – enables the traditional interpreter backend.
* `backend-armv7` – builds the ARMv7 backend module.
* `backend-aarch64` – builds the 64-bit ARM backend module.

Only one backend feature may be enabled at a time.  Attempting to compile with
multiple backend features results in a compile-time error, ensuring the runtime
has a single unambiguous target.

## Selecting a Backend

By default the crate builds with the x86_64 backend:

```bash
cargo build -p solvra_core
```

To target ARMv7:

```bash
cargo build -p solvra_core --no-default-features --features backend-armv7
```

To target AArch64:

```bash
cargo build -p solvra_core --no-default-features --features backend-aarch64
```

When cross-compiling, pair the feature flag with Rust's `--target` option and a
configured toolchain.  For example, after installing the `armv7-unknown-linux-gnueabihf`
toolchain you can run:

```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo build -p solvra_core --no-default-features --features backend-armv7 --target armv7-unknown-linux-gnueabihf
```

The interpreter-based backends currently emit Solvra bytecode and execute it using
the shared VM.  This keeps functionality identical across platforms while
providing clear extension points for future native code generators.

## Runtime API

The [`SolvraRuntime`](../src/lib.rs) type exposes helper methods to introspect the
selected backend:

* `SolvraRuntime::backend()` returns the backend implementation.
* `SolvraRuntime::target_arch()` returns the active [`TargetArch`](../src/backend/mod.rs) value.

Both helpers are useful for host applications that need to adapt behaviour based
on the runtime architecture (for instance to select appropriate native modules).

## Adding a New Backend

1. Create a new sub-module under `src/backend/` (e.g. `riscv64/`).
2. Implement `ArchitectureBackend` for the backend type.
3. Add a new Cargo feature flag in `Cargo.toml` enabling the module.
4. Update `backend::active_backend` to select the new backend when its feature is
   active.
5. Add tests under `solvra_core/tests/` gated behind the new feature flag to ensure
   the backend can execute sample programs.
6. Update CI to exercise the backend with `cargo test` and, if applicable,
   `cargo clippy`.

Following this process keeps SolvraCore extensible and ensures every backend ships
with validation coverage.
