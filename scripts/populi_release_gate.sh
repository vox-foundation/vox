#!/usr/bin/env bash
# Thin delegate — implementation: `vox ci populi-gate --profile m1m4` (see scripts/populi/gates.yaml).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
exec cargo run -p vox-cli --quiet -- ci populi-gate --profile m1m4
