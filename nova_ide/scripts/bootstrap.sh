#!/usr/bin/env bash
set -euo pipefail

# Synchronise Rust and Node dependencies for NovaIDE.

if ! command -v pnpm >/dev/null; then
  echo "pnpm is required" >&2
  exit 1
fi

pnpm install
cargo fetch
