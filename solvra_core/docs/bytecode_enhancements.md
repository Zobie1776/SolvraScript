# SolvraCore Bytecode Enhancements Roadmap

1. **Instruction Set Review** – catalogue existing opcodes, tag gaps for device IO, and add
   meta-information for SolvraAI-aware diagnostics.
2. **Register Allocation Hooks** – extend assembler to emit register pressure metadata for hardware
   backends.
3. **HAL Syscalls** – define bytecode ops for device register access and interrupts, mapping to the
   SoftwareHal during interpretation.
4. **Deterministic Sandbox Mode** – add cost counters per instruction class and expose to SolvraLite.
5. **Testing Strategy** – craft `.svc` fixtures (see `tests/svc_samples`) and interpreter-driven
   regression tests once compilation pipeline is online.
