# Getting Started

SolvraIDE v0.1 ships as a Tauri desktop application with a SvelteKit front-end. Use the following steps to bootstrap a workspace:

1. Install Rust (stable) and Node.js 20 with pnpm 8.
2. From the repository root run `pnpm install` and `cargo build` to fetch dependencies.
3. Launch the desktop shell with `pnpm --filter @solvra-ide/desktop dev` in one terminal and `cargo tauri dev` in another for live reload.
4. Open a SolvraScript project and run the default build/run tasks from the command palette or the task runner panel.

The project explorer honours `.gitignore` and `.svside/workspace.toml` globs, enabling quick navigation of large codebases.
