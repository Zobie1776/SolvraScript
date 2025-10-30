# Repository Guidelines

## Project Structure & Module Organization
SolvraOS is a Rust workspace (`Cargo.toml`) with accompanying TypeScript packages (`pnpm-workspace.yaml`). Core runtimes live in `solvra_core/` (VM, runtime, integration tests in `tests/`), language tooling in `solvra_script/` (SolvraScript compiler and REPL), and automation features in `solvra_ai/`. CLI and thin client layers are provided by `solvra_cli/` and `solvra_lite/`, while UI crates collect under `solvra_ide/crates/*`. Generated artifacts land in `target/`; keep it out of version control.

## Build, Test, and Development Commands
- `cargo build --workspace` — compile every crate with the workspace resolver.
- `cargo test --workspace` — execute unit and integration suites.
- `cargo run -p solvra_cli -- --help` — smoke-check the CLI entrypoint.
- `pnpm install` — install Node/Svelte dependencies before linting or UI tests.
- `pnpm lint` / `pnpm typecheck` / `pnpm test` — run ESLint, TypeScript, and Vitest across packages.

## Coding Style & Naming Conventions
Rust code targets the toolchain pinned in `rust-toolchain.toml`; run `cargo fmt --all` and `cargo clippy --all-targets --all-features` before committing. Use 4-space indentation, snake_case files, and module trees that mirror directories (e.g., `solvra_core/src/backends/mod.rs`). Frontend code follows `eslint.config.mjs`; keep Svelte components PascalCase under `solvra_ide/` and shared utilities camelCase.

## Testing Guidelines
Place Rust integration tests in `tests/*.rs` named after their subsystem (`runtime.rs`, `backends.rs`). Reuse fixtures from the crate’s `samples/` directory instead of hardcoding inputs. Svelte/Vitest specs sit beside components with the `.spec.ts` suffix. Run `cargo test --workspace` and `pnpm test` before every PR.

## Commit & Pull Request Guidelines
Commits lean toward Conventional Commits (`chore:`, `feat:`, `fix:`) and short, present-tense summaries; follow the pattern when possible. PRs should outline the problem and solution, link related issues or roadmap items, attach test evidence, and call out any backward-incompatible changes or migrations.

## Environment & Security Notes
Use the workspace Rust toolchain (auto-selected via `rust-toolchain.toml`) and execute commands from the repo root to keep dependency resolution consistent. Store secrets in local, gitignored `.env` files. When adding JavaScript dependencies, prefer `pnpm add --filter <package>` to scope them.
