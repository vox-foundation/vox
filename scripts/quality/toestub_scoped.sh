#!/usr/bin/env bash
# Thin delegate — implementation: `vox ci toestub-scoped` (default scan root: crates/vox-repository).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
exec cargo run -p vox-cli --quiet -- ci toestub-scoped "${1:-crates/vox-repository}"
