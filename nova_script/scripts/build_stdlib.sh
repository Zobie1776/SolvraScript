#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STD_SRC="$ROOT/stdlib"
OUT_DIR="${1:-$ROOT/target/stdlib}"

mkdir -p "$OUT_DIR"

echo "[nova_script] Compiling standard library modules (placeholder)"
for module in "$STD_SRC"/*.ns; do
    name="$(basename "$module" .ns)"
    target="$OUT_DIR/$name.nvc"
    echo "  - $name.ns -> $target"
    # TODO: replace the copy with NovaCore compiler invocation when available.
    cp "$module" "$target"
done

echo "[nova_script] Build complete. Replace copy step with 'novac --emit nvc' once the bytecode compiler is ready."
