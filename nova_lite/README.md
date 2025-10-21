# NovaLite

NovaLite is a lightweight, touch-first companion to NovaOS that targets mobile and tablet form factors.
It focuses on ergonomic gesture-driven interaction, tight NovaAI integration, and the essentials required
for coding on the go with NovaIDE and Nova App Store access.

## Features

- Async event loop with touch-aware UI manager.
- Modular egui-based interface with responsive layout primitives.
- Mobile NovaIDE panels for browsing files and editing code.
- NovaAI client bridge with pluggable authentication.
- App store browser to discover, install, and manage NovaOS applications.
- Cross-compilation tooling for ARM64 Linux and QEMU emulation scripts.

## Project Layout

```
nova_lite/
├── src/
│   ├── main.rs               # Async bootstrap + event loop
│   ├── ui/                   # UI manager, layout primitives, gesture handling
│   ├── ide/                  # Mobile NovaIDE stubs
│   ├── api/                  # NovaAI async client bridge + auth hooks
│   └── app_store/            # App store browser + management workflow
├── scripts/build-arm64.sh    # Helper script for cross-compiling to ARM64
├── .cargo/config.toml        # Linker + QEMU runner configuration
└── README.md                 # This file
```

## Building Locally

NovaLite is part of the NovaOS Rust workspace:

```bash
cargo run -p nova_lite
```

The crate depends on `nova_core`, `nova_script`, and `nova_ai`, all of which live in the same workspace.
If you are hacking on NovaLite in isolation, export the workspace root as `NOVAOS_ROOT` to help the
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

The script stores build output under `nova_lite/target-arm64` and prints QEMU instructions after a
successful build. By default it expects dynamic glibc-based builds; adjust the `.cargo/config.toml`
file for musl-based builds if you prefer static binaries.

## Emulating NovaLite on QEMU

Once built, run NovaLite under QEMU user-mode emulation:

```bash
qemu-aarch64 -L /usr/aarch64-linux-gnu target-arm64/aarch64-unknown-linux-gnu/release/nova_lite
```

This configuration is suitable for quick smoke tests of the UI event loop and service integrations.
For full device emulation, pair the binary with a lightweight Wayland compositor or embed it inside
an Android-compatible container.

## Future Enhancements

- Integrate real rendering backends (wgpu, eframe) optimized for touch devices.
- Expand app store APIs to use decentralized package feeds.
- Bridge to the NovaIDE language services for on-device completions and debugging.
- Support offline-first caching layers for NovaAI prompts and responses.
