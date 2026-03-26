#!/usr/bin/env bash
# Legacy path: forwards to canonical scripts/populi/release_training_gate.sh.
set -euo pipefail
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")/../populi" && pwd)/release_training_gate.sh" "$@"
