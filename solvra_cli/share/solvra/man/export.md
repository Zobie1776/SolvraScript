# export

## Synopsis
`export NAME=VALUE ...`

## Description
Set environment variables for the current SolvraCLI session. Each `NAME=VALUE` pair updates the process environment immediately.

## Examples
- `export RUST_LOG=debug` — enable verbose logging for subsequent commands.
- `export PATH=$PATH:/opt/solvra/bin` — append a directory to `PATH`.
