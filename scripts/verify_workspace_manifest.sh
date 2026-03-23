#!/usr/bin/env bash
# Thin delegate — implementation: `vox ci manifest` (crates/vox-cli).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
exec cargo run -p vox-cli --quiet -- ci manifest
