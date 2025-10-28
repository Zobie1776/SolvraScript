# Configuration Guide

- Primary config file: `~/.config/nova-shell/config.toml`
- Themes live under `~/.config/nova-shell/themes/*.toml`
- Runtime profile overrides can be toggled via `novactl theme <name>` or `novactl layout <strategy>`.
- The compositor exposes `config_reload` over JSON-RPC to watch for hot reloads.
