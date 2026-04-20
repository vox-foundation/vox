#!/usr/bin/env bash
# vox-dev.sh (Thin Launcher)
# Forward all arguments to vox-cli via cargo run.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" && exec cargo run -q -p vox-cli -- "$@"
