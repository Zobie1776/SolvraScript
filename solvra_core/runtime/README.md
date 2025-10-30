# Solvra Runtime Tool

The Solvra runtime tool exposes the `solvra_core::SolvraRuntime` interpreter as a
standalone executable. It supports one-off execution of `.svc` bytecode files
as well as an interactive REPL for iterating on Solvra assembly.

The REPL understands a handful of commands prefixed with `:`. Use `:help` to
display the full list, including `:backend` which prints the currently active
CPU backend.

## Building

From the repository root, build the tool with Cargo:

```bash
cargo build -p solvra_core --bin solvra-runtime
```

To cross-compile for an ARM64 target, install the appropriate target and build:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build -p solvra_core --bin solvra-runtime --target aarch64-unknown-linux-gnu
```

## Running

Launch the REPL with no arguments:

```bash
cargo run -p solvra_core --bin solvra-runtime
```

Execute a `.svc` bytecode file once and exit:

```bash
cargo run -p solvra_core --bin solvra-runtime -- path/to/program.svc
```

Enter the REPL after executing a program:

```bash
cargo run -p solvra_core --bin solvra-runtime -- --exec path/to/program.svc --repl
```

Use `--help` for a full list of supported flags at runtime.
