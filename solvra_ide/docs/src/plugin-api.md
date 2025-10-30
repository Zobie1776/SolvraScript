# Plug-in API

SolvraIDE loads extensions from the `extensions/` directory. Each extension must provide an `extension.toml` manifest describing the plug-in name, entrypoint and capability set.

```toml
name = "SolvraIDE Git"
version = "0.1.0"
entrypoint = "dist/plugin.wasm"
[permissions]
fs = true
network = ["https://api.github.com"]
```

At runtime the desktop shell uses Wasmer to sandbox WebAssembly extensions. Plug-ins can register commands, contribute status bar items and expose views by publishing JSON-RPC messages over the host bridge. Refer to the included Git, Terminal and Theme examples for reference implementations.
