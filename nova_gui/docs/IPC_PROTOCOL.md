# IPC Protocol

Nova GUI uses JSON-RPC 2.0 over Unix domain sockets (`/run/user/<uid>/nova-gui.sock`). Core methods:

- `focus_next`, `focus_prev`
- `layout_set { layout }`
- `config_reload`
- `theme_set { theme }`
- `power_suspend`

All responses mirror JSON-RPC semantics with either `result` payloads or standard error objects.
