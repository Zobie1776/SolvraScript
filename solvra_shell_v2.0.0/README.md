# Solvra Shell v2.0.0 (SolvraScript Prototype)

This prototype mirrors the behaviour of the original Rust shell (`solvra_shell_v0.1.0`) but is implemented as a collection of SolvraScript utilities. The goal is to exercise compositor auto-detection, application discovery, theming, and process launch logic without relying on the Rust runtime.

## Layout

- `scripts/` — SolvraScript entry points:
  - `compositor_detect.svs` — picks Wayland or X11 (with headless fallbacks) and prints a JSON report.
  - `app_index.svs` — indexes desktop entries and emits `[{ name, exec, icon }]`.
  - `theme.svs` — loads theme tokens or falls back to the bundled Minimal theme.
  - `spawn.svs` — detaches processes with desktop placeholder filtering.
  - `launcher.svs` — orchestrator that stitches detection, indexing, and spawning together (TTY UI for now).
- `bin/solvra_shell_v2` — bash wrapper that prepares the environment then runs `launcher.svs`.
- `tests/` — thin smoke scripts around the SolvraScript entry points.
- `assets/` — shared icons and themes for SolvraScript consumers.

## Running

```bash
./solvra_shell_v2.0.0/bin/solvra_shell_v2
```

The launcher prefers an existing Wayland compositor, falls back to X11, and attempts to start nested Weston or Xvfb sessions otherwise. When no GUI binding exists, it provides a TTY-based filtering prompt.

## Known Gaps

- Requires a working `solvrascript` CLI in `PATH`.
- Headless compositor start is best-effort; see script logs for guidance if tooling is missing.
- GUI binding is deferred to the Rust v0.1.0 shell; SolvraScript currently demonstrates orchestration through the TTY path.
