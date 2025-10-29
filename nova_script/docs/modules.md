## NovaScript Module & Import System

NovaScript now supports modular development with a flexible import system that understands both user-authored scripts and precompiled NovaCore (`.nvc`) artifacts.

### Import Syntax

```novascript
import <vector>;                // Standard library module
import "lib/math.ns";           // Relative script module
import { append, length } from <vector>;
import "tools/format.ns" as fmt;
```

- `<module>` resolves modules from the standard library search paths (defaults to `nova_script/stdlib`). Compiled `.nvc` modules are preferred when available, falling back to `.ns`.
- `"path.ns"` resolves script modules from the importing file’s directory, the process working directory, or additional search paths you register on the `ModuleLoader`.
- Named imports inject individual symbols directly into the caller’s scope. Namespace imports (`import <vector>;`) expose a module object whose members are accessed via property notation (`vector.append(data, value)`).

### Module Loader Overview

`ModuleLoader` centralises module discovery, parsing, caching, and dependency resolution:

1. **Discovery** – Locates modules across user and standard-library search paths.
2. **Parsing** – Tokenises and parses `.ns` sources into ASTs once, caching the result.
3. **Recursion** – Walks a module’s import graph to prepare dependencies before execution.
4. **Evaluation** – The interpreter executes modules in isolated environments, capturing any new globals as the exported namespace.
5. **Caching** – Exports are cached to avoid re-execution unless explicitly cleared.

The loader detects cyclic imports and reports friendly diagnostics. When a compiled `.nvc` artefact is discovered it is bound to a shared NovaCore runtime through the internal CoreBridge, exposing a stable `module.handle` plus helper builtins for execution and teardown.

### Standard Library Modules

Initial standard modules live under `nova_script/stdlib`:

- `vector` – Dynamic array helpers (`make`, `append`, `length`, `pop_last`, `first`).
- `string` – Basic string manipulation (`concat`, `length`, `repeat`).
- `io` – Console I/O wrappers (`write`, `writeln`, `read`).

Import them with:

```novascript
import <vector>;

let mut values = vector.make();
values = vector.append(values, 42);
println("len:", vector.length(values));
```

### Authoring Script Modules

Any `.ns` file can act as a module. Define top-level functions or variables; when the module runs, any globals created beyond the built-ins become part of the export surface.

```
// examples/modules/math_utils.ns
fn square(value) {
    return value * value;
}
```

Usage:

```novascript
import "examples/modules/math_utils.ns" as math;
println(math.square(9));
```

### Tooling & Build Support

A helper script (`scripts/build_stdlib.sh`) demonstrates how to package standard modules into `.nvc` artifacts. The interpreter recognises these artefacts automatically; the exported namespace now complements the script exports with a `module` metadata object while global builtins (`core_module_execute`, `core_module_release`, `core_value_release`) manage execution and lifecycle.

```bash
./scripts/build_stdlib.sh             # writes to target/stdlib
./scripts/build_stdlib.sh ./dist/lib  # custom output directory
```

### NovaCore Integration

- Execute a compiled module by calling `core_module_execute(module.module.handle)`. The return value mirrors NovaCore scalars directly; opaque runtime objects appear as handles that can later be freed with `core_value_release(handle)`.
- Inspect the deterministic allocator that backs both runtimes via `core_memory_stats()`, which mirrors `MemoryContract::stats()` from NovaCore.
- Dispose of compiled modules when no longer needed through `core_module_release(module.module.handle)` to reclaim deterministic heap space.
- Queue asynchronous interpreter tasks with `core_bridge.execute_async(|| { /* ... */ })` (or higher-level helpers) and drive them to completion from the host by invoking `NovaRuntime::run_loop()`.

### Best Practices

- Prefer namespace imports for clarity when modules expose multiple utilities.
- Use named imports for isolated helpers and minimise global namespace pollution.
- Keep module files focused; re-export dependencies explicitly to control public APIs.
- When shipping reusable modules, document expected exports and usage patterns.

### Next Steps

- Hook NovaCore’s bytecode compiler to produce `.nvc` modules from `.ns` sources.
- Extend the loader to validate explicit `export` declarations once the language grammar supports them.
- Add optional hot-reload/invalidation hooks for development workflows.
