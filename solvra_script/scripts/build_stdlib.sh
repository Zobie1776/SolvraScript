#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STD_SRC="$ROOT/stdlib"
OUT_DIR="${1:-$ROOT/target/stdlib}"

mkdir -p "$OUT_DIR"

echo "[solvra_script] Compiling standard library modules (placeholder)"
for module in "$STD_SRC"/*.svs; do
    name="$(basename "$module" .svs)"
    target="$OUT_DIR/$name.svc"
    echo "  - $name.svs -> $target"
    # TODO: replace the copy with SolvraCore compiler invocation when available.
    cp "$module" "$target"
done

echo "[solvra_script] Build complete. Replace copy step with 'solvrac --emit svc' once the bytecode compiler is ready."
