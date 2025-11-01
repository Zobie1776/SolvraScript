# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SolvraOS is a modular, AI-native operating system built from scratch with a custom scripting language (SolvraScript), VM runtime (SolvraCore), GUI shell, IDE, and CLI. The project is under active development, currently in Phase 6 of the runtime implementation.

## Build & Test Commands

### Building the Project

```bash
# Build all workspace members
cargo build

# Build with release optimizations
cargo build --release

# Build specific package
cargo build -p solvrascript
cargo build -p solvra_core
cargo build -p solvra_cli
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test -p solvrascript
cargo test -p solvra_core

# Run specific test suites (Phase 6 validation)
cargo test -p solvrascript async          # Async runtime tests
cargo test -p solvrascript memory         # Memory tracking tests
cargo test -p solvrascript async_timeout  # Timeout enforcement tests

# Run full test coverage for SolvraScript
cargo test -p solvrascript --tests
```

### Running SolvraScript

```bash
# Execute a SolvraScript file
cargo run -p solvrascript -- path/to/script.svs

# With async timeout (in milliseconds)
cargo run -p solvrascript -- --async-timeout-ms 1000 path/to/script.svs

# Compile SolvraScript to bytecode
cargo run -p solvrascript --bin solvra_compile -- input.svs output.svc

# Disassemble bytecode
cargo run -p solvrascript --bin solvra_disasm -- compiled.svc
```

### Running the CLI Shell

```bash
cd solvra_cli
cargo run
```

## Code Formatting Standard (Zobie.format)

All new files should follow this template structure:

```rust
//=====================================================
// File
//=====================================================
// Author:
// License:
// Goal:
// Objective:
//=====================================================

//=====================================================
// Import & Modules
//=====================================================


//=====================================================
//  Section x.x (title of the section)
//=====================================================
code    // short precise comments of the code lines

/*---------------------------------------------------------------------------

large comments

*/---------------------------------------------------------------------------


//---------------------------------------------------------------------------
//  End comments and what the above code does
// @TODO or @ZNOTES for the section above
//---------------------------------------------------------------------------

//=====================================================
// End of file
//=====================================================
```

### Formatting Rules

- Use `//=====` dividers for major sections
- File header must include: File name, Author, License, Goal, Objective
- Section headers use format: `Section x.x (descriptive title)`
- Inline comments use `//` for brief line explanations
- Block comments use `/*---` and `*/---` delimiters for extended explanations
- End-of-section comments summarize what code does
- Use `@TODO` for planned work, `@ZNOTES` for important notes
- Always include "End of file" marker

## Architecture & Key Concepts

### Core Components

1. **SolvraScript** (`solvra_script/`) — Custom scripting language with async support
   - Tokenizer → Parser → AST → Interpreter/VM pipeline
   - Module system with stdlib (`io`, `vector`, `string`)
   - Built-in async/await with deterministic timeout enforcement
   - Integration with SolvraCore via `CoreBridge`

2. **SolvraCore** (`solvra_core/`) — Low-level runtime and bytecode execution engine
   - Memory contract system for deterministic allocation (`MemoryContract`)
   - AOT/LLVM compilation pipeline for `.svc` modules
   - Work-stealing task executor for concurrency
   - Hardware Abstraction Layer (HAL) for drivers
   - Backend support: x86_64, ARMv7, AArch64

3. **SolvraCLI** (`solvra_cli/`) — Command-line shell with SolvraScript embedding
   - Pipeline execution and command parsing
   - Plugin system via `libloading` (optional feature)
   - Integration with SolvraAppStore

4. **SolvraIDE** (`solvra_ide/`) — Integrated development environment
   - Tauri-based desktop app
   - TypeScript/React frontend
   - Rust backend for LSP integration

### Critical Architectural Patterns

**Memory Model (SolvraCore)**
- Runtime instances created via `SolvraRuntime::from_bootstrap` (CRT0-style)
- Cross-crate allocations use `MemoryContract` with manual `allocate_arc`/`release`
- Returns stable `MemoryHandle`s for deterministic lifetime management
- `core_memory_stats()` builtin exposes telemetry

**SolvraScript ↔ SolvraCore Bridge**
- `CoreBridge` in SolvraScript connects to SolvraCore's executor
- Compiled `.svc` modules share allocator with script objects
- Module handles stored in memory contract, released explicitly
- `core_module_execute(handle)`, `core_module_release(handle)` builtins

**Async Runtime (Phase 6)**
- Deterministic timeout enforcement via `RuntimeOptions::with_async_timeout(ms)`
- Raises `RuntimeException::Timeout` with merged stack traces
- Memory tracker hooks (`MemoryTracker`) for instrumentation
- Telemetry API via `RuntimeOptions::with_telemetry_hook`

### Module System

SolvraScript supports two import styles:

```solvrascript
// Standard library modules
import <vector>;
import { append, length } from <vector>;

// User modules (relative paths)
import "lib/math.svs";
import "tools/format.svs" as fmt;
```

Standard library location: `solvra_script/stdlib/`

### Workspace Structure

The repository uses Cargo workspace with these members:
- `solvra_ai` — AI assistant integration
- `solvra_app_store` — Package management
- `solvra_cli` — Command-line shell
- `solvra_core` — Core runtime & VM
- `solvra_script` — Scripting language
- `solvra_lite` — Embedded/lightweight runtime
- `solvra_ide/crates/*` — IDE components

Note: Shell components (`solvra_shell_v0.1.0`) are commented out in workspace.

## Development Workflow

### Running Single Tests

```bash
# Run a specific test by name
cargo test -p solvrascript test_async_parallel_awaits

# Run tests with output visible
cargo test -p solvrascript -- --nocapture

# Run single test file
cargo test -p solvrascript --test async_tests
```

### Linting

The workspace enforces `warnings = "deny"` at the workspace level. All warnings must be fixed before code will compile.

### Edition & Toolchain

- Rust edition: 2021 (most crates), 2024 (SolvraScript)
- Minimum Rust version: 1.78
- Toolchain config: `rust-toolchain.toml`

## Current Development Phase

**Phase 6.3** — Runtime telemetry, memory tracking, and async timeout detection

Recent milestones (from `docs/phase6_validation.md`):
- Phase 6.1: Runtime parity foundation
- Phase 6.2: Async integrity & scheduler validation
- Phase 6.3: Memory tracking, timeout enforcement, telemetry hooks
- Phase 6.3A: Async timeout validation complete (2025-11-01)

All Phase 6 tests passing as of latest validation.

## Key Files for Understanding the System

- `solvra_script/lib.rs` — SolvraScript crate root
- `solvra_script/vm/runtime.rs` — VM runtime with async handling
- `solvra_script/interpreter.rs` — Main interpreter loop
- `solvra_script/parser.rs` — AST parser
- `solvra_script/tokenizer.rs` — Lexer
- `solvra_script/core_bridge.rs` — SolvraCore integration
- `solvra_core/src/` — Core runtime implementation
- `solvra_script/docs/language_reference.md` — Language syntax & builtins
- `docs/phase6_validation.md` — Latest runtime validation report

## Common Patterns

### Testing Async Runtime

```rust
use solvrascript::vm::{Runtime, RuntimeOptions};

let mut rt = Runtime::new(RuntimeOptions::default()
    .with_async_timeout(100)  // 100ms timeout
    .with_memory_tracker(tracker));

let result = rt.execute(script_source);
assert!(matches!(result, Err(RuntimeException::Timeout { .. })));
```

### SolvraScript Builtin Extensions

When adding new builtins, update:
1. `solvra_script/interpreter.rs` — Add to `call_builtin_function`
2. `solvra_script/docs/language_reference.md` — Document the function
3. `solvra_script/docs/builtin_status.md` — Track implementation status

### Bytecode Pipeline

```bash
# Source → Bytecode → Disassembly validation
cargo run -p solvrascript --bin solvra_compile -- input.svs output.svc
cargo run -p solvrascript --bin solvra_disasm -- output.svc
cargo run -p solvrascript -- output.svc  # Execute compiled
```

## SolvraScript Language Notes

- SolvraScript uses **double-quoted strings only** (`"text"`), single quotes rejected
- No implicit spacing in string concatenation: use `"Hello " + name` not `"Hello" + name`
- Variables immutable by default, use `let mut` for mutability
- Type annotations use postfix syntax: `let id: string = "value"`
- Escape sequences: `\n`, `\t`, `\r`, `\0`, `\\`, `\"`
- Template strings with backticks for multi-line: `` `text` ``

## Notes

- Git status shows current work on async timeout detection (Phase 6.3)
- LICENSE: Apache License 2.0
- Author: Zachariah Obie

## Documentation & Curriculum Architect

When working on educational materials for SolvraScript and SolvraCore:

**Your Role**: Documentation & Curriculum Architect translating SolvraScript and SolvraCore internals into ADHD-friendly learning materials inside the `Solvra_Curriculum` repo.

**Curriculum Repository**: https://github.com/Zobie1776/Solvra_Curriculum

### Tasks

- **Convert Phase 7-8 features into teachable `.svs` examples**
- **Annotate every lesson using SolvraFormat, with one-line and long comments**
- **Write `README.md` overviews for each Phase directory explaining concepts in plain language**
- **Add diagrams or ASCII flows showing data between SolvraScript ↔ SolvraCore**
- **Maintain documentation for built-ins** (`core_memory_events()`, `core_timeout_stats()`, etc.)

### Deliverables

- `phase7_5` and `phase8_0` curriculum folders
- `docs/solvra_language_reference.md`
- `tutorials/getting_started_with_solvrascript.md`

### Goal

Make SolvraScript a language people can **learn in 15 minutes and master in a weekend**.
