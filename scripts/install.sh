#!/usr/bin/env bash
# Thin wrapper: ensure rustup, then run **`vox-bootstrap`** (Rust SSOT in `crates/vox-bootstrap`).
#
# Usage: same flags as `vox-bootstrap`:
#   ./scripts/install.sh
#   ./scripts/install.sh --dev
#   ./scripts/install.sh --install-clang --apply
#   ./scripts/install.sh plan
#   ./scripts/install.sh plan --human
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

run_bootstrap() {
  cargo run --locked -p vox-bootstrap -- "$@"
}

if command -v cargo >/dev/null 2>&1; then
  run_bootstrap "$@"
  exit $?
fi

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "${HOME}/.cargo/env"
fi

if command -v cargo >/dev/null 2>&1; then
  run_bootstrap "$@"
  exit $?
fi

echo "  cargo not found — installing rustup (https://rustup.rs) …"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# shellcheck source=/dev/null
source "${HOME}/.cargo/env"
run_bootstrap "$@"
