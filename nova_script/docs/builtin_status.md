# NovaScript Built-in Function Inventory

This document captures the current state of NovaScript's built-in runtime library prior to the major upgrade described in the prompt.

## Built-ins currently available

| Name | Status | Notes |
| ---- | ------ | ----- |
| `prt` / `print` / `println` | ✅ Implemented | Escape-aware output; `prt` is the canonical form, `println` appends newline. |
| `endl` | ✅ Implemented | Emits a single newline with an immediate flush for console-style formatting. |
| `div` / `division` | ✅ Implemented | Variadic-free numeric division with divide-by-zero protection. |
| `sbt` / `subtract` | ✅ Implemented | Numeric subtraction returning `int` when both inputs are integers. |
| `bool` / `boolean` | ✅ Implemented | Truthiness coercion into NovaScript booleans. |
| `input` | ✅ Implemented | Optional prompt, trims trailing newlines. |
| `to_string` | ✅ Implemented | Uses runtime `Display` for conversion. |
| `parse_int`, `parse_float` | ✅ Implemented | Graceful error handling with base support for integers. |
| `len` | ✅ Implemented | Counts Unicode scalar values for strings. |
| `type` | ✅ Implemented | Returns NovaScript type names. |
| `random` | ✅ Implemented | Supports floats and integer ranges. |
| `time`, `now`, `sleep` | ✅ Implemented | Epoch seconds, structured timestamps, and millisecond sleeps. |
| `push`, `pop`, `insert`, `remove` | ✅ Implemented | Pure array transformations returning updated structures. |
| `sin`, `cos`, `tan`, `sqrt`, `log`, `pow`, `abs` | ✅ Implemented | Math helpers built on Rust's `f64` primitives. |
| `open_file`, `read_file`, `write_file`, `close_file` | ✅ Implemented | Handle-based resource management with path support. |
| `http_get`, `http_post` | ✅ Implemented | Powered by the `ureq` HTTP client with JSON decoding. |
| `env_get`, `env_set`, `exit` | ✅ Implemented | Environment and process control helpers. |
| `on_event`, `trigger_event` | ✅ Implemented | In-process event bus invoking NovaScript callbacks. |

## Still under consideration

The following helpers were identified as potentially useful but remain unimplemented in this pass:

- Boolean parsing (`parse_bool`)
- Numeric clamps and extrema (`clamp`, `min`, `max`)

These items can be tackled in future iterations if the scripting workloads demand them.
