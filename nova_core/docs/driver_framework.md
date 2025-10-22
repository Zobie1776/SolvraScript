# NovaCore Driver Framework Overview

The driver framework unifies host-native peripherals and NovaCore modules.  It
provides shared register storage, interrupt queues, and telemetry hooks that
allow NovaShell, NovaIDE, and other host tools to introspect device behaviour.

## Architecture

* `DriverRegistry` maintains a map of registered devices, each with a fixed-size
  register bank (`Vec<u32>`) and a FIFO interrupt queue.
* `DriverDescriptor` lets host applications create devices with pre-filled
  registers prior to executing NovaCore bytecode.
* `Interrupt` represents queued hardware events; it carries an IRQ number and an
  optional payload (`u32`).
* `RuntimeHooks` emit telemetry (`DriverRegistered`, `RegisterWrite`,
  `InterruptRaised`), log messages, and debugger events for runtime consumers.

## Host API Surface

```rust
use nova_core::{DriverDescriptor, NovaRuntime};

let runtime = NovaRuntime::new();
runtime.register_driver(DriverDescriptor::new("gpu", vec![0; 16]))?;
runtime.signal_interrupt("gpu", 7, Some(0xDEAD))?;
```

Host code can also access the shared registry directly:

```rust
let registry = runtime.driver_registry();
let temperature = registry.read_register("sensor", 0)?;
```

Every public API returns `NovaResult<T>` so errors integrate with existing
runtime diagnostics (`NovaError::Native`).

## NovaCore Native Bindings

Driver bindings are exposed as native functions callable from NovaCore modules
and `.novac` programs:

| Function | Parameters | Description |
| --- | --- | --- |
| `driver_register(name, register_count)` | `string`, `number` | Registers a zero-initialised virtual device |
| `driver_write_u32(name, register, value)` | `string`, `number`, `number` | Writes a 32-bit value to a register |
| `driver_read_u32(name, register)` | `string`, `number` | Reads a 32-bit register value |
| `driver_raise_interrupt(name, irq, payload)` | `string`, `number`, `number/null` | Queues an interrupt for the device |
| `driver_next_interrupt(name)` | `string` | Pops the next pending interrupt (returns `[irq, payload]` or `null`) |

These bindings are available immediately after `NovaRuntime::new()` because the
VM initialises its native registry with driver functions.  The sample program in
`samples/virtual_device_driver.novac` demonstrates how to register a device,
write to a register, and poll for interrupts.

## Telemetry and Logging

The registry emits telemetry through `RuntimeHooks` for every significant
operation:

* `DriverRegistered { name, registers }`
* `RegisterWrite { name, register, value }`
* `InterruptRaised { name, irq, payload }`

NovaShell and NovaIDE can attach listeners via `runtime.hooks().set_telemetry(...)`
to display device dashboards, while `set_logger(...)` surfaces structured log
messages such as `driver[virtual_temp] = 1`.

## Testing Strategy

Unit tests in `nova_core/tests/drivers.rs` assemble NovaCore AST programs that
exercise the driver bindings end-to-end.  They confirm that:

* Virtual devices registered from bytecode persist after execution.
* Host-queued interrupts become visible to NovaCore modules via
  `driver_next_interrupt`.
* Interrupts raised from bytecode are observable from host code by querying the
  registry.

These tests leverage the same APIs available to NovaShell/NovaIDE and guard the
behaviour expected by integration tooling.
