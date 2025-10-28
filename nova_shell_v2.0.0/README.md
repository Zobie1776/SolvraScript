# Nova Shell v2.0.0 (NovaScript Prototype)

This prototype mirrors the behaviour of the original Rust shell (`nova_shell_v0.1.0`) but is implemented as a collection of NovaScript utilities. The goal is to exercise compositor auto-detection, application discovery, theming, and process launch logic without relying on the Rust runtime.

## Layout

- `scripts/` — NovaScript entry points:
  - `compositor_detect.ns` — picks Wayland or X11 (with headless fallbacks) and prints a JSON report.
  - `app_index.ns` — indexes desktop entries and emits `[{ name, exec, icon }]`.
  - `theme.ns` — loads theme tokens or falls back to the bundled Minimal theme.
  - `spawn.ns` — detaches processes with desktop placeholder filtering.
  - `launcher.ns` — orchestrator that stitches detection, indexing, and spawning together (TTY UI for now).
- `bin/nova_shell_v2` — bash wrapper that prepares the environment then runs `launcher.ns`.
- `tests/` — thin smoke scripts around the NovaScript entry points.
- `assets/` — shared icons and themes for NovaScript consumers.

## Running

```bash
./nova_shell_v2.0.0/bin/nova_shell_v2
```

The launcher prefers an existing Wayland compositor, falls back to X11, and attempts to start nested Weston or Xvfb sessions otherwise. When no GUI binding exists, it provides a TTY-based filtering prompt.

## Known Gaps

- Requires a working `novascript` CLI in `PATH`.
- Headless compositor start is best-effort; see script logs for guidance if tooling is missing.
- GUI binding is deferred to the Rust v0.1.0 shell; NovaScript currently demonstrates orchestration through the TTY path.
