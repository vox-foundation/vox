#!/usr/bin/env bash
# Delegates to canonical `vox ci toestub-self-apply` (release build + full-repo scan).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if command -v vox >/dev/null 2>&1; then
  exec vox ci toestub-self-apply
fi
exec cargo run -p vox-cli -- ci toestub-self-apply
