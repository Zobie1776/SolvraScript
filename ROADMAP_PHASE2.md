# SolvraOS Phase 2 Roadmap (GUI, Shell, and Ecosystem)

## 1. SolvraGUI & Shell

### Objectives
- Composable window manager with tiling + floating layouts.
- Event loop bridging HAL input devices (keyboard, mouse, controllers) into SolvraScript callbacks.
- GPU acceleration hooks exposing frame buffers and shaders via SolvraCore HAL.
- Theming system defined in SolvraScript `.svs` modules compiled to `.svc` for performance.

### Milestones
1. **Window Server Core** – surface compositor API, task scheduler, and surface registry.
2. **Input Pipeline** – map HAL events to shell actions and user-land SolvraScript handlers.
3. **UI Toolkit** – reusable widgets (panels, buttons, lists) authored in SolvraScript.
4. **Integration Tests** – SolvraScript-driven scenarios validating window focus, input routing, and redraw timing.

## 2. SolvraLite Runtime Profile

### Objectives
- Lightweight runtime for embedded boards (Raspberry Pi, Arduino-class MCUs).
- Subset of SolvraScript (no floating GC, reduced stdlib) compiled via SolvraCore.
- Deterministic scheduling with configurable memory ceilings.

### Milestones
1. Define feature flags for SolvraCore to strip heavy subsystems.
2. Produce reference board configuration (ARM / bare-metal) with HAL bindings.
3. Port essential modules (`io`, `vector`, `string`) to SolvraLite.
4. Ship sample firmware (`blinky.svs`, sensor reader) and CI hardware tests (simulated when hardware unavailable).

## 3. SolvraCLI & SolvraIDE Integration

### Objectives
- Command-line automation shell with scripting (SolvraScript) and native SolvraCore extensions.
- SolvraIDE with syntax highlighting, LSP support, and integration with SolvraAI for refactors.

### Milestones
1. **CLI Core** – command parser, pipeline execution, SolvraScript embedding.
2. **IDE Language Server** – extend existing tokenizer/parser support for imports, provide completions.
3. **Tooling Integration** – diagnostics, formatting, debugger hooks using `RuntimeHooks`.
4. **AI Bridge** – SolvraAI-assisted code completion and diagnostics.

## 4. SolvraAI Services

### Objectives
- Secure system-level AI services with sandbox-aware policies.
- Adaptive optimisation guidance (profiling) and proactive diagnostics.

### Milestones
1. Define SolvraAI API surface (request/response types in `.svs` / `.svc`).
2. Implement security gate aligning with Solvra Security Plan (operator consent, resource quotas).
3. Integrate with SolvraIDE/CLI for on-demand assistance.

## 5. Device Driver Matrix & Solvra App Store

### Driver Matrix
- Keyboard, mouse, controllers – extend HAL stubs into full SolvraCore drivers.
- Audio (speakers/microphones) – streaming APIs and sample rate management.
- Storage (NVMe, SATA, SD) – block device abstraction + filesystem hooks.
- Sensors (temperature, motion, light, humidity) – asynchronous sampling with telemetry.

### App Store Workflow
1. Package format (`.nvpkg`) bundling `.svs` sources and compiled `.svc` artifacts.
2. Permission manifest aligning with HAL capabilities.
3. CLI tools for signing, publishing, and updating packages.
4. Store backend (registry service) with sandboxed install/update routines.

## 6. Testing & CI
- Resume full `cargo test` + SolvraScript module tests once network access restored.
- Add hardware-in-loop simulation for HAL drivers when feasible.
- Enforce security policy coverage (sensor write guards, register limits) in automated suites.

This roadmap guides Phase 2 implementation following the newly established SolvraCore HAL foundation. Each milestone will be accompanied by SolvraScript/SolvraCore code, tests, and documentation updates.
