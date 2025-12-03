# Evaluation of Micro-Optimizations - Phase 2.5
**Date:** 2025-11-29

## 1. Overview

This document evaluates the safety and placement of four recommended micro-optimizations for the SolvraScript compiler and toolchain. Each optimization is classified as SAFE, REQUIRES REVIEW, or HIGH-RISK.

---

## 2. Resolver Caching Layer

- **Optimization:** Implement a multi-level cache for module resolution to reduce I/O and redundant parsing. This includes an in-memory cache for the current session and a persistent on-disk bytecode cache (`.svc` files).
- **Analysis:** The `ModuleLoader` in `solvra_script/modules.rs` already implements both of these caches.
  - The in-memory cache (`self.cache`) stores `ModuleDescriptor`s by their canonical path.
  - The on-disk cache is implemented in `compile_script_if_needed`, which creates `.svc` files in `target/solvra_modules` based on a fingerprint of the source code.
- **Placement:** The current placement inside the `ModuleLoader` is correct, as it centralizes all loading logic.
- **Safety Classification:** **SAFE**
- **Justification:** The implementation is sound and follows standard practice. The use of source code fingerprinting for the on-disk cache is a reliable way to handle cache invalidation for single files.
- **Recommendation for Improvement:**
  - **REQUIRES REVIEW:** The caching could be made more robust. Currently, if a module `A` imports module `B`, and `B` changes, `A`'s cached bytecode will become stale but will not be invalidated, because `A`'s source code hasn't changed. The fingerprinting mechanism should be extended to include the fingerprints of all transitive dependencies. This would make the build system much more reliable, but it adds significant complexity to the `ModuleLoader`.

---

## 3. Bytecode Peephole Rules

- **Optimization:** Introduce a peephole optimization pass after bytecode generation to replace common inefficient patterns, such as constant folding and dead-store elimination.
- **Analysis:** I have not been able to review the bytecode generation code itself, which is presumed to be in `solvra_core/src/bytecode/`. Therefore, this analysis is theoretical. A peephole optimizer operates on a small, sliding window of bytecode instructions.
  - **Constant Folding:** `PUSH 2; PUSH 3; ADD;` -> `PUSH 5;`
  - **Dead-Store Elimination:** `PUSH 10; STORE 'x'; PUSH 20; STORE 'x';` -> `PUSH 20; STORE 'x';`
- **Placement:** This pass should occur at the very end of the compilation pipeline, just before the `.svc` file is written to disk. It should be a simple, fast pass.
- **Safety Classification:** **SAFE**
- **Justification:** Peephole optimizations are a classic, well-understood compiler technique. When implemented correctly, they are very safe. The patterns are local and do not require complex control-flow analysis. The risk of introducing a bug is low, and the performance gain can be noticeable.
- **Recommendation for Implementation:**
  - Start with a very small, conservative set of rules. Focus on constant folding for arithmetic operations first.
  - Add extensive unit tests for the optimizer, ensuring that for every rule, the optimized bytecode produces the exact same stack state as the unoptimized version.

---

## 4. Improved Error Messages

- **Optimization:** Improve the consistency, structure, and helpfulness of compiler error messages (e.g., for parsing, type checking, or capability denial).
- **Analysis:** Good error messages are a critical feature for developer experience. The goal is to provide errors that are structured, informative, and suggest a fix. The `ModuleError` enum in `modules.rs` is a good starting point.
- **Placement:** Error handling logic is distributed:
  - `solvra_script/parser.rs` for syntax errors.
  - `solvra_script/modules.rs` for resolution errors.
  - The VM/Interpreter for runtime errors (including capability errors).
- **Safety Classification:** **SAFE**
- **Justification:** This is not a code optimization, but a developer experience optimization. There is no risk to program correctness. The only "risk" is the engineering time required to implement it thoroughly.
- **Recommendation for Implementation:**
  - Adopt a standardized error format across the entire toolchain (e.g., JSON output for IDEs).
  - For parse errors, include "did you mean?" suggestions for typos in variable or function names.
  - For capability errors, the message must state exactly which function required which capability. Example: `Runtime Error: Capability 'fs.read' denied. Required by function 'read_file' called at 'my_script.svs:10:5'`.

---

## 5. Incremental Parser Hooks

- **Optimization:** Add hooks to the parser and resolver to support future incremental compilation and analysis, primarily for the SolvraIDE.
- **Analysis:** Incremental compilation is essential for a fast IDE experience. When a developer types, the IDE needs to re-parse and re-analyze the code quickly. This requires the compiler components to be designed for it. The `ast.rs` file already includes unique IDs for each AST node, which is a key prerequisite.
- **Placement:** This requires changes throughout the frontend:
  - **Parser:** Must be able to re-parse only a single function or block of code and update the existing AST, rather than rebuilding it from scratch.
  - **ModuleLoader:** The caching mechanism needs to be able to store and retrieve finer-grained artifacts than whole modules (e.g., the AST for a single function).
- **Safety Classification:** **HIGH-RISK**
- **Justification:** True incremental compilation is notoriously difficult to implement correctly. The primary risk is incorrect cache invalidation. If the compiler fails to re-analyze a piece of code that is affected by a change, it can lead to subtle and extremely hard-to-debug bugs (e.g., incorrect type information, stale analysis). While the unique AST node IDs are a good start, the full system requires a complex dependency graph to track relationships between all symbols, functions, and modules.
- **Recommendation for Implementation:**
  - **Defer this optimization.** This is a feature for a more mature compiler. The focus for Phase 3 should be on the correctness of the JIT and the core compiler pipeline.
  - As a first step, focus on building a robust language server (LSP) that simply re-compiles the entire file on change. This is much simpler and safer.
  - Once the core compiler is stable (post-1.0), the team can revisit building a true incremental compilation engine, likely requiring a significant architectural redesign of the frontend.
