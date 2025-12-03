# Standard Library (std/stdlib/stdx) Recommendations - 2025-11-29

This document provides architectural and optimization recommendations for the SolvraScript standard library, focusing on the Host Bridge model and the library layers.

---

## 1. Host Bridge & FFI

- **Recommendation:** Implement batching for chatty FFI calls. For operations that happen in a tight loop (e.g., writing many small chunks to a file), the high-level `stdlib` wrapper should accumulate the operations in a buffer and make a single, larger call to the native Host Bridge function.
- **Justification:** Crossing the FFI boundary from SolvraScript to native Rust code has a non-trivial overhead. For "chatty" APIs, this overhead can dominate the actual work being done. Batching is a classic technique to reduce this overhead.
- **Expected Gain:** Significant performance improvement for I/O-bound scripts and other code that makes heavy use of bridged calls.
- **Risk:** Medium. The `stdlib` wrapper becomes more complex, as it needs to manage the buffer. Care must be taken to ensure the buffer is always flushed at the appropriate time (e.g., on file close, or when the buffer is full).

- **Recommendation:** Use structured, serializable types for all data passed across the Host Bridge. Avoid passing raw pointers or complex, nested objects directly. Define simple `struct`s for requests and responses.
- **Justification:** This practice makes the FFI boundary much more robust and secure. It prevents a large class of bugs related to memory layout and object lifetimes. It also forces a clear definition of the contract between the script and the host, which is excellent for debuggability.
- **Expected Gain:** Improved stability, security, and long-term maintainability of the standard library.
- **Risk:** Low. There may be a small performance cost for serialization/deserialization, but this is usually negligible compared to the cost of an FFI call itself, and the safety benefits are immense.

---

## 2. Library Architecture

- **Recommendation:** Create a formal, documented process for graduating a module from `stdx` to `stdlib`.
- **Justification:** The `stdx` library is an excellent idea for experimentation, but there needs to be a clear path for successful experiments to become part of the stable API. This process should include criteria like "has a complete test suite," "has been benchmarked," and "API has been reviewed by at least two senior developers."
- **Expected Gain:** Ensures that the `stdlib` remains stable and high-quality, while still allowing for rapid innovation in `stdx`.
- **Risk:** Low. This is a process-related improvement.

- **Recommendation:** Develop a comprehensive test suite for the standard library that specifically targets the Host Bridge. These should not just be unit tests for the `.svs` code; they should be integration tests that verify the *interaction* between the `.svs` wrapper and the native Rust code.
- **Justification:** A bug in the standard library could be in the high-level script, the low-level native code, or in the boundary between them. The test suite must be able to pinpoint where the failure occurred. Testing the boundary is critical for security and correctness.
- **Expected Gain:** A more robust and reliable standard library. This will catch subtle bugs, particularly related to the capability-based security model.
- **Risk:** Low. Writing tests is always a good investment.

---

## 3. What NOT to Change

- **Do NOT Expose Native Functions Directly:** The Host Bridge abstraction is the cornerstone of the standard library's security model. Never provide a mechanism for SolvraScript to call a native function without going through a vetted `stdlib` wrapper. The capability-based security model depends on this indirection.
- **Do NOT Mix Concerns Between `std` and `stdlib`:** Maintain the strict separation between `std` (low-level primitives for use by the compiler and `stdlib`) and `stdlib` (high-level, user-facing APIs). User-facing code should never need to import from `std`. This separation keeps the user-facing API surface clean and stable.
- **Do NOT Write Complex Logic in the Native FFI Layer:** The native functions in `solvra_core/ffi` should be small, simple, and focused. They should do one thing well (e.g., read a block of bytes from a file descriptor). The complex logic (e.g., parsing, data manipulation) should be handled in the high-level `.svs` wrapper or in other Rust modules called by the FFI layer. This keeps the security-critical FFI boundary as small and auditable as possible.
