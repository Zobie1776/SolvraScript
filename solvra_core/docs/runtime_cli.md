# SolvraCore Runtime Guide

SolvraRuntime offers an embeddable interpreter with debugger, logging, telemetry,
and driver integration hooks.  This guide shows how to execute `.svc` programs
from the command line and how host tools such as SolvraShell or SolvraIDE can attach
observers.

## Executing Bytecode

Use the convenience helper added to `SolvraRuntime` to load and execute a `.svc`
file directly:

```rust
use solvra_core::SolvraRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = SolvraRuntime::new();
    let value = runtime.execute_file("samples/hello_world.svc")?;
    println!("Program returned: {value:?}");
    Ok(())
}
```

Run the snippet with:

```bash
cargo run --example embed_runtime
```

You can replace `samples/hello_world.svc` with any program produced by the
assembler or SolvraScript compiler.  The runtime automatically records telemetry
(`TelemetryEvent::ShellLoaded`) and emits debugger events for start, success, and
failure conditions.

## Registering Hooks

SolvraShell and SolvraIDE can subscribe to runtime events via `RuntimeHooks`:

```rust
use solvra_core::{DebuggerEvent, RuntimeHooks, RuntimeLog, TelemetryEvent};

let runtime = SolvraRuntime::new();
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
immediately visible to SolvraCore modules.  SolvraShell can bridge device events by
calling `runtime.signal_interrupt("device", irq, payload)` while SolvraIDE can
surface telemetry updates triggered by register writes and interrupts.

A mock driver written in SolvraCore is available in
`samples/virtual_device_driver.svc` and demonstrates the native bindings:
`driver_register`, `driver_write_u32`, `driver_next_interrupt`, and
`driver_raise_interrupt`.

## Batch Execution Scripts

To run the runtime along multiple targets from the terminal, rely on the build
scripts introduced under `solvra_core/build/`:

```bash
# Build and test the runtime for host + ARM64 targets
./build/run-all.sh

# Execute a program after building host artifacts
./build/cargo-desktop.sh
cargo run --example embed_runtime --release
```

These utilities give SolvraShell and SolvraIDE a consistent way to orchestrate the
runtime from external processes.
