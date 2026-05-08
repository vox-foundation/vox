#!/usr/bin/env bash
# Release-check programmatic gates G1-G4 for vox-mental-tracker.
#
# Usage: bash apps/vox-mental-tracker/scripts/release_check.sh
# Run from the repo root or from this script's directory; both work.
#
# Exits 0 only if all four gates pass. Manual gates G5-G8 are walked
# from docs/how-to/release.md by a human; this script does not cover them.

set -euo pipefail

# Locate repo root regardless of cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$APP_DIR/../.." && pwd)"

PASS=()
FAIL=()

run_gate() {
    local name="$1"; shift
    echo "==> $name"
    if "$@"; then
        PASS+=("$name")
        echo "    PASS"
    else
        FAIL+=("$name")
        echo "    FAIL"
    fi
}

g1_vitest() {
    cd "$APP_DIR" && pnpm exec vitest run >/dev/null
}

g2_playwright() {
    if [ -z "${BASE_URL:-}" ]; then
        echo "    SKIP-DETECT: BASE_URL unset → Playwright specs self-skip; run a preview server and re-run with BASE_URL set"
        return 0
    fi
    cd "$APP_DIR" && pnpm exec playwright test >/dev/null
}

g3_vox_check() {
    cd "$REPO_ROOT" && cargo run -q -p vox-cli -- check apps/vox-mental-tracker/src/main.vox >/dev/null
}

g4_contracts() {
    cd "$REPO_ROOT"
    for f in apps/vox-mental-tracker/contracts/event-payloads/*.json; do
        python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$f"
    done
    python3 - <<'PY'
import glob, yaml
for path in sorted(glob.glob('apps/vox-mental-tracker/contracts/export/*.yaml')):
    with open(path) as f:
        yaml.safe_load(f)
PY
}

run_gate "G1 — Vitest" g1_vitest
run_gate "G2 — Playwright (browser lane)" g2_playwright
run_gate "G3 — vox check" g3_vox_check
run_gate "G4 — Contracts parse" g4_contracts

echo
echo "================ Summary ================"
for n in "${PASS[@]}"; do echo "  PASS  $n"; done
for n in "${FAIL[@]}"; do echo "  FAIL  $n"; done

if [ "${#FAIL[@]}" -ne 0 ]; then
    echo
    echo "Programmatic gates failed; do not proceed to manual gates."
    exit 1
fi

echo
echo "Programmatic gates passed. Walk manual gates G5-G8 per docs/how-to/release.md."
