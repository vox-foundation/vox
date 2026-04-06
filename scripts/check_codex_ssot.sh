#!/usr/bin/env bash
# Thin delegate — implementation: `vox ci check-codex-ssot`.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p vox-cli --quiet -- ci check-codex-ssot
