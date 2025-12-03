# SolvraScript
/* The command-line entrypoint and standard library of the Solvra scripting layer. */

SolvraScript follows a disciplined artifact pipeline that keeps the runtime deterministic while allowing tiered performance:

1. **`.svs`** — Source authoring format. Write your scripts here.
2. **`solvra_compile`** converts `.svs` into **`.svc` portable bytecode** that the Tier-0 interpreter consumes; this is the deterministic artifact that is shared across Hive nodes and caches.
3. **`.svc`** modules load through `solvra_core` (Tier-0) and can be rematerialized into higher tiers dynamically.
4. **`solvra_aot`** compiles `.svs`/`.svc` into **`.saot`** native binaries for Tier-3 execution when zero runtime compilation and native throughput are required; `.saot` files are CPU/ABI specific and are not distributed like `.svc`.

```bash
# Compile to portable bytecode
cargo run -p solvra_core --bin solvra_compile -- script.svs script.svc

# Run the `.svc` bytecode with the Tier-0 interpreter
cargo run -p solvra_core -- script.svc --tier=0

# Optionally produce an AOT binary for native execution
cargo run -p solvra_core --bin solvra_aot -- compile script.svs -o script.saot -O2
cargo run -p solvra_core -- script.saot
```

This pipeline keeps the interpreter deterministic (`.svc`), while the optional `.saot` binaries deliver maximum performance when local execution demands native code.
