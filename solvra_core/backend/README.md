# SolvraCore CPU Backends

The backend directory hosts architecture specific implementations of the
`ArchitectureBackend` trait. Exactly one backend is selected at compile time via
Cargo features:

- `backend-x86_64` (default) — interpreter based backend targeting desktop hosts.
- `backend-armv7` — interpreter mode for legacy 32-bit ARM hardware.
- `backend-aarch64` — skeleton implementation intended for future optimisation.

To exercise a backend run the SolvraCore tests with the desired feature flag:

```bash
cargo test -p solvra_core --no-default-features --features backend-aarch64
```

When cross-compiling the runtime tool, ensure the appropriate backend feature is
enabled to match the compilation target.
