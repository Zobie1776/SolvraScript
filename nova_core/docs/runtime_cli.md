# NovaCore Runtime Guide

NovaRuntime offers an embeddable interpreter with debugger, logging, telemetry,
and driver integration hooks.  This guide shows how to execute `.nvc` programs
from the command line and how host tools such as NovaShell or NovaIDE can attach
observers.

## Executing Bytecode

Use the convenience helper added to `NovaRuntime` to load and execute a `.nvc`
file directly:

```rust
use nova_core::NovaRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = NovaRuntime::new();
    let value = runtime.execute_file("samples/hello_world.nvc")?;
    println!("Program returned: {value:?}");
    Ok(())
}
```

Run the snippet with:

```bash
cargo run --example embed_runtime
```

You can replace `samples/hello_world.nvc` with any program produced by the
assembler or NovaScript compiler.  The runtime automatically records telemetry
(`TelemetryEvent::ShellLoaded`) and emits debugger events for start, success, and
failure conditions.

## Registering Hooks

NovaShell and NovaIDE can subscribe to runtime events via `RuntimeHooks`:

```rust
use nova_core::{DebuggerEvent, RuntimeHooks, RuntimeLog, TelemetryEvent};

let runtime = NovaRuntime::new();
let hooks = runtime.hooks();

hooks.set_debugger(|event: &DebuggerEvent| {
    println!("debugger: {event:?}");
});

hooks.set_logger(|log: &RuntimeLog| {
    println!("log[{source}]: {message}", source = log.source, message = log.message);
});

hooks.set_telemetry(|event: &TelemetryEvent| {
    println!("telemetry: {event:?}");
});
```

The callbacks run on the same thread as the runtime, ensuring deterministic
ordering with respect to bytecode execution.

## Integrating Drivers

Drivers registered from host code via `runtime.register_driver(...)` are
immediately visible to NovaCore modules.  NovaShell can bridge device events by
calling `runtime.signal_interrupt("device", irq, payload)` while NovaIDE can
surface telemetry updates triggered by register writes and interrupts.

A mock driver written in NovaCore is available in
`samples/virtual_device_driver.nvc` and demonstrates the native bindings:
`driver_register`, `driver_write_u32`, `driver_next_interrupt`, and
`driver_raise_interrupt`.

## Batch Execution Scripts

To run the runtime along multiple targets from the terminal, rely on the build
scripts introduced under `nova_core/build/`:

```bash
# Build and test the runtime for host + ARM64 targets
./build/run-all.sh

# Execute a program after building host artifacts
./build/cargo-desktop.sh
cargo run --example embed_runtime --release
```

These utilities give NovaShell and NovaIDE a consistent way to orchestrate the
runtime from external processes.
