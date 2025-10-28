# IPC Protocol

Nova Shell uses JSON-RPC 2.0 over Unix domain sockets. The compositor exposes the following baseline methods:

| Method        | Params Example                 | Description                              |
|---------------|--------------------------------|------------------------------------------|
| `focus_next`  | `{}`                           | Move focus to the next workspace         |
| `focus_prev`  | `{}`                           | Move focus to the previous workspace     |
| `layout_set`  | `{ "layout": "bsp" }`        | Switch to the requested layout strategy  |
| `config_reload` | `{}`                        | Reload configuration and themes          |
| `theme_set`   | `{ "theme": "CyberGrid" }`   | Apply a theme across compositor + UIs    |

Requests and responses follow standard JSON-RPC envelopes with integer ids.
