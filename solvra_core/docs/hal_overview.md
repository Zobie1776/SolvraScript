# SolvraCore Hardware Abstraction Layer

The SolvraCore HAL (`sys::hal`) provides a device-agnostic facade for the runtime and backends.  It
introduces:

- **Device descriptors** capturing the peripheral kind (keyboard, mouse, game controller, speakers,
  microphones, displays, storage buses, and sensors) alongside register counts and advertised
  capabilities.
- **Handle-based access** for enumerating devices, reading/writing registers, and emitting
  interrupts without leaking backend-specific details.
- **Telemetry integration** by reusing `RuntimeHooks`, ensuring SolvraShell / SolvraIDE receive
  consistent driver registration events.
- **Software-backed implementation** (`SoftwareHal`) that leverages the existing in-memory
  `DriverRegistry`, enabling sandboxed execution while preserving the final hardware-facing APIs.

Built-in virtual devices include keyboard, audio output, SD-card storage, and a temperature sensor
stub.  Each device exposes a deterministic register layout so higher layers can begin wiring input
and telemetry before real drivers are available.

### Sandbox Security Policy

The default `SandboxSecurityPolicy` enforces safe execution within development environments by:

- Limiting each device to a capped register count (defaults to 256).
- Maintaining an allowlist of registered device handles.
- Blocking write operations to sensor devices to prevent accidental tampering.
- Reusing runtime hooks to surface denied operations for auditing.

Policies are pluggable, allowing future Solvra Security Plan components to provide hardened
implementations for production deployments.

The HAL is designed to support multiple architectures (x86_64, ARM, embedded ROM targets) and will
serve as the convergence point for future hardware accelerators, GPU bindings, and sandboxing
constructs described in the Solvra Security Plan.
