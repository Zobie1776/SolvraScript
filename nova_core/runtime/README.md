# Nova Runtime Tool

The Nova runtime tool exposes the `nova_core::NovaRuntime` interpreter as a
standalone executable. It supports one-off execution of `.novac` bytecode files
as well as an interactive REPL for iterating on Nova assembly.

The REPL understands a handful of commands prefixed with `:`. Use `:help` to
display the full list, including `:backend` which prints the currently active
CPU backend.

## Building

From the repository root, build the tool with Cargo:

```bash
cargo build -p nova_core --bin nova-runtime
```

To cross-compile for an ARM64 target, install the appropriate target and build:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build -p nova_core --bin nova-runtime --target aarch64-unknown-linux-gnu
```

## Running

Launch the REPL with no arguments:

```bash
cargo run -p nova_core --bin nova-runtime
```

Execute a `.novac` bytecode file once and exit:

```bash
cargo run -p nova_core --bin nova-runtime -- path/to/program.novac
```

Enter the REPL after executing a program:

```bash
cargo run -p nova_core --bin nova-runtime -- --exec path/to/program.novac --repl
```

Use `--help` for a full list of supported flags at runtime.
