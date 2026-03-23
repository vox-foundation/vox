#!/usr/bin/env bash
# Thin delegate — implementation: `vox ci check-codex-ssot`.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
exec cargo run -p vox-cli --quiet -- ci check-codex-ssot
