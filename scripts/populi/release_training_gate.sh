#!/usr/bin/env bash
# Thin delegate — implementation: `vox ci mens-gate --profile training`.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
exec cargo run -p vox-cli --quiet -- ci mens-gate --profile training
