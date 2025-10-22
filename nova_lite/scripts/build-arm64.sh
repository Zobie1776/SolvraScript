#!/usr/bin/env bash
set -euo pipefail

TARGET="aarch64-unknown-linux-gnu"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null; then
  echo "cargo is required" >&2
  exit 1
fi

rustup target add "${TARGET}" || true

if ! command -v aarch64-linux-gnu-gcc >/dev/null; then
  echo "Missing aarch64-linux-gnu-gcc. Install gcc-aarch64-linux-gnu or the equivalent cross toolchain." >&2
  exit 1
fi

if ! command -v qemu-aarch64 >/dev/null; then
  echo "Missing qemu-aarch64. Install QEMU user emulation for ARM64." >&2
  exit 1
fi

pushd "${PROJECT_ROOT}" >/dev/null
CARGO_TARGET_DIR="${PROJECT_ROOT}/target-arm64" \
  cargo build --release --target "${TARGET}" "$@"
popd >/dev/null

echo
cat <<INSTRUCTIONS
To emulate NovaLite on ARM64 via QEMU:
  1. Install the QEMU user static binaries package for your distribution.
  2. Ensure the built binary is located at target-arm64/${TARGET}/release/nova_lite.
  3. Run: qemu-aarch64 -L /usr/aarch64-linux-gnu target-arm64/${TARGET}/release/nova_lite
INSTRUCTIONS
