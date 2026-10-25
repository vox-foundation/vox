#!/bin/bash
# Observable live deployment & SHA-256 parity runner for vox (voxlang.org) via @ngi/platform-ssot
set -euo pipefail
cd "$(dirname "$0")/.."
exec node --experimental-strip-types ../ngi.video/scripts/sync-prod-observable.mjs --repo=vox "$@"
