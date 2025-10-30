# SolvraLite

SolvraLite is a lightweight, touch-first companion to SolvraOS that targets mobile and tablet form factors.
It focuses on ergonomic gesture-driven interaction, tight SolvraAI integration, and the essentials required
for coding on the go with SolvraIDE and Solvra App Store access.

## Features

- Async event loop with touch-aware UI manager.
- Modular egui-based interface with responsive layout primitives.
- Mobile SolvraIDE panels for browsing files and editing code.
- SolvraAI client bridge with pluggable authentication.
- App store browser to discover, install, and manage SolvraOS applications.
- Cross-compilation tooling for ARM64 Linux and QEMU emulation scripts.

## Project Layout

```
solvra_lite/
├── src/
│   ├── main.rs               # Async bootstrap + event loop
│   ├── ui/                   # UI manager, layout primitives, gesture handling
│   ├── ide/                  # Mobile SolvraIDE stubs
│   ├── api/                  # SolvraAI async client bridge + auth hooks
│   └── app_store/            # App store browser + management workflow
├── scripts/build-arm64.sh    # Helper script for cross-compiling to ARM64
├── .cargo/config.toml        # Linker + QEMU runner configuration
└── README.md                 # This file
```

## Building Locally

SolvraLite is part of the SolvraOS Rust workspace:

```bash
cargo run -p solvra_lite
```

The crate depends on `solvra_core`, `solvrascript`, and `solvra_ai`, all of which live in the same workspace.
If you are hacking on SolvraLite in isolation, export the workspace root as `NOVAOS_ROOT` to help the
configuration scripts locate assets and ensure the dependencies are built first.

## Cross-Compiling to ARM64

Install the necessary Rust targets and cross toolchain:

```bash
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu qemu-user-static
```

Then invoke the helper script:

```bash
./scripts/build-arm64.sh
```

The script stores build output under `solvra_lite/target-arm64` and prints QEMU instructions after a
successful build. By default it expects dynamic glibc-based builds; adjust the `.cargo/config.toml`
file for musl-based builds if you prefer static binaries.

## Emulating SolvraLite on QEMU

Once built, run SolvraLite under QEMU user-mode emulation:

```bash
qemu-aarch64 -L /usr/aarch64-linux-gnu target-arm64/aarch64-unknown-linux-gnu/release/solvra_lite
```

This configuration is suitable for quick smoke tests of the UI event loop and service integrations.
For full device emulation, pair the binary with a lightweight Wayland compositor or embed it inside
an Android-compatible container.

## Future Enhancements

- Integrate real rendering backends (wgpu, eframe) optimized for touch devices.
- Expand app store APIs to use decentralized package feeds.
- Bridge to the SolvraIDE language services for on-device completions and debugging.
- Support offline-first caching layers for SolvraAI prompts and responses.
