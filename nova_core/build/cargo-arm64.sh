#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${SCRIPT_DIR}/.."
TARGET="${1:-aarch64-unknown-linux-gnu}"

cd "${ROOT_DIR}"

cargo build --all-targets --target "${TARGET}" "${@:2}"
