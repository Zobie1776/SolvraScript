# SolvraScript Standard Library Audit - Phase 2.5
**Date:** 2025-11-29

## 1. Overview and Core Assumptions

This audit analyzes the existing SolvraScript standard library modules based on the file structure in `solvra_script/std` and the API specifications found in `solvra_script/stdlib/specs`.

**CRITICAL ASSUMPTION:** The actual FFI implementation of the Host Bridge (`__host_*` functions in SolvraCore) is **not present** in the reviewed files. This audit assumes the `.svs` modules in `std` are intended to be implemented as specified in `host_bridge_map.md` and `security_model.md`. The highest priority for the project must be to implement and verify this bridge.

**General Recommendations:**
- **Docstring Standard:** Every function in the standard library must have a docstring. The recommended format is:
  ```solvrascript
  /**
   * A brief, one-sentence description of the function.
   *
   * A more detailed explanation of what the function does, its parameters,
   * and its return value.
   *
   * @param name The description for a parameter.
   * @returns A description of the return value.
   * @capability The capability required, if any (e.g., `fs.read`).
   * @deterministic Describes determinism (e.g., 'Yes', 'No - uses system time').
   */
  ```
- **Linter/IDE Hint Pattern:** The linter should parse `@capability` and `@deterministic` annotations.
  - If a function with `@capability` is used, the linter must trace back to ensure the capability has been granted in the script's manifest or via a dynamic `request_capability()` call.
  - The IDE should display a warning icon next to any function call marked as non-deterministic.

---

## 2. Audit of Existing `std` Modules

### Module: `<core>`
*(Files: `iter.svs`, `option.svs`, `result.svs`, `string.svs`, `vector.svs`, etc.)*

This module contains the absolute core language features.

- **API Clarity:** Excellent. The concepts of `option`, `result`, `vector`, and `string` are fundamental and their APIs (based on the `module_index.md`) appear clean and conventional.
- **Missing Metadata:**
  - Docstrings are likely missing across the board. Every function needs one.
  - `@deterministic Yes` should be added to every function in this module. These are pure computational primitives.
- **Linter/IDE Hints:** The linter should be able to leverage `option` and `result` types to provide warnings about unhandled `None` or `Err` values, similar to how Rust's compiler warns about unused `Result`s.
- **Missing Helpers:**
  - **`string`:** `trim_start`, `trim_end`, `split_whitespace`, `to_uppercase`, `to_lowercase`.
  - **`vector`:** `sort`, `reverse`, `find`, `filter`, `map`. While a full `iter` module is better, some basic helpers on `vector` itself are convenient.
- **Safety/Determinism:**
  - **Concern:** None. This entire module should be implemented in pure SolvraScript and have no host dependencies, making it perfectly deterministic and safe.

---

### Module: `<io>`
*(Files: `io.svs`)*

- **API Clarity:** Good. `read_line`, `print`, `eprint` are clear.
- **Missing Metadata:**
  - `print`/`eprint`: Docstrings should specify that these are unbuffered and intended for debugging.
  - `read_line`: Needs `@capability io.stdin`. It also needs `@deterministic No - depends on user input`.
- **Linter/IDE Hints:** The IDE should warn that `read_line` will block execution until the user provides input.
- **Missing Helpers:** A `read_all()` function to read from stdin until EOF would be useful for piping data.
- **Safety/Determinism:**
  - **Concern:** `io` is inherently non-deterministic. The host implementation of the underlying `__host_io_*` functions must be secure. It must not be possible for a script to block the entire VM thread by waiting on stdin indefinitely; a timeout mechanism, even if not exposed in the high-level API, is essential at the host level.

---

### Module: `<math>`
*(Files: `mod.svs`)*

- **API Clarity:** Good. Standard math functions (`sin`, `cos`, `sqrt`, `log`, etc.).
- **Missing Metadata:** Needs docstrings for all functions specifying behavior for edge cases (e.g., `log(0)`, `sqrt(-1)`). Must be annotated `@deterministic Yes`.
- **Linter/IDE Hints:** The linter could offer hints for floating point comparisons (e.g., suggest using an epsilon).
- **Missing Helpers:** `deg_to_rad`, `rad_to_deg`, `clamp`.
- **Safety/Determinism:**
  - **Concern:** Floating point math can have minor implementation differences across platforms. To guarantee perfect determinism for the `.svc` bytecode, the host functions (`__host_math_*`) must be implemented using a cross-platform library that guarantees identical results (e.g., by using a software floating point implementation like `softfloat-sys` if hardware differences are a concern). This is a **subtle but critical detail** for the determinism guarantee.

---

### Module: `<time>`
*(Files: `mod.svs`)*

- **API Clarity:** Good.
- **Missing Metadata:**
  - `now_ms()`: Must have `@capability time.monotonic` and `@deterministic No - uses monotonic clock`. Docstring must clarify this is for measuring durations, not for wall-clock time.
  - `now_unix()`: Must have `@capability time.system` and `@deterministic No - uses system wall clock`. Docstring must warn that this is non-deterministic and should not be used for game loops or simulations.
  - `sleep(ms)`: Must have `@capability time.sleep` and `@deterministic No - timing is not guaranteed`.
- **Linter/IDE Hints:** The IDE should heavily flag any use of `now_unix()` in code that appears to be part of a simulation or update loop.
- **Missing Helpers:** `format_unix_time(timestamp, format_string)`, `parse_iso8601(iso_string)`.
- **Safety/Determinism:**
  - **Concern:** This module is a primary source of non-determinism. The distinction between the monotonic clock (for durations) and the system wall clock (for timestamps) is critical and must be strictly enforced through the capability model.

---

### Module: `<fs>`
*(Files: `mod.svs`)*

- **API Clarity:** Good. `read`, `write`, `exists`, `is_dir`, `list_dir`.
- **Missing Metadata:** Every function needs a `@capability` tag.
  - `read`, `exists`, `is_dir`, `list_dir`: `@capability fs.read`.
  - `write`, `create_dir`, `delete`: `@capability fs.write`.
  - All should be marked `@deterministic No`.
- **Linter/IDE Hints:** Any path provided as a literal string should be checked by the IDE to see if it falls within the paths granted in the script's manifest.
- **Missing Helpers:** `read_text`, `write_text` (for UTF-8 handling), `copy`, `move`, `rename`.
- **Safety/Determinism:**
  - **Concern:** This is the most dangerous module. The host implementation in SolvraCore must be incredibly robust.
    - **Path Traversal:** The host must rigorously prevent `../` attacks to escape the script's sandbox. All paths must be canonicalized and checked against the granted permissions *before* any filesystem operation.
    - **Symbolic Links:** The policy on symbolic links needs to be defined. A safe default is to not follow them. If they are followed, the resolved path must also be checked against the capability sandbox.
    - **Resource Leaks:** The host must ensure that file handles are always closed, even if the SolvraScript program panics.

---

### Module: `<json>`
*(Files: `mod.svs`)*

- **API Clarity:** Good. `parse`, `stringify`.
- **Missing Metadata:** Docstrings should specify behavior on invalid JSON and non-serializable types (e.g., functions). Should be marked `@deterministic Yes`.
- **Linter/IDE Hints:** None. This is a straightforward computational module.
- **Missing Helpers:** `parse_stream`, `stringify_pretty`.
- **Safety/Determinism:**
  - **Concern:** Assumed to be implemented in pure SolvraScript. If it uses a host function (`__host_json_*`), that host function must be audited to ensure it doesn't panic on malformed input and that it is fully deterministic. A pure `.svs` implementation is safer.

---

### Module: `<system>` (mapped to `<sys>`)
*(Files: `mod.svs`)*

- **API Clarity:** The API is not defined in the specs, but we can infer its purpose. Likely functions: `env`, `args`, `exit`, `platform`.
- **Missing Metadata:**
  - `env(var_name)`: `@capability process.env.read`. `@deterministic No`.
  - `args()`: `@capability process.args`. `@deterministic No`.
  - `exit(code)`: `@capability process.exit`. `@deterministic No`.
- **Linter/IDE Hints:** `exit()` is a "sharp edge" and the IDE should flag it as a hard process termination.
- **Missing Helpers:** `pid()`, `cwd()`, `set_cwd(path)`.
- **Safety/Determinism:**
  - **Concern:** This module directly exposes the environment the script is running in, which is a major source of non-determinism. The `process.env.read` capability is a good idea, but it should be possible to restrict it further (e.g., only allow reading variables with a specific prefix).

---
## 3. Conclusion of Audit

The standard library's *design*, as specified in the `specs` documents, is excellent. The separation of concerns, the Host Bridge architecture, and the capability-based security model are all state-of-the-art.

The primary risk is the **implementation gap**. The specifications are writing checks that the implementation has not yet cashed. The immediate and sole focus should be on building the SolvraCore side of the Host Bridge and rigorously testing its adherence to the security model. Until that is done, the entire standard library is a theoretical construct.
