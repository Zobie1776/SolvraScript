#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/cargo-desktop.sh"
"${SCRIPT_DIR}/cargo-arm64.sh" "$@"

cd "${SCRIPT_DIR}/.."

cargo test --all-targets
