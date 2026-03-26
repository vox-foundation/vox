#!/usr/bin/env bash
# Thin delegate to `vox ci mens-gate --isolated-runner` (temp vox copy; avoids file locks).
# Use --detach for agent sessions with wall-clock limits (re-execs this script in the background).
set -euo pipefail

PROFILE="training"
LOG_FILE=""
DETACH=false

usage() {
  echo "Usage: $0 [--profile training|ci_full|m1m4] [--log-file PATH] [--detach]" >&2
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      [[ $# -ge 2 ]] || usage
      PROFILE="$2"
      shift 2
      ;;
    --log-file)
      [[ $# -ge 2 ]] || usage
      LOG_FILE="$2"
      shift 2
      ;;
    --detach)
      DETACH=true
      shift
      ;;
    -h | --help)
      usage
      ;;
    *)
      usage
      ;;
  esac
done

case "${PROFILE}" in
  training | ci_full | m1m4) ;;
  *)
    echo "Invalid --profile ${PROFILE} (expected training|ci_full|m1m4)" >&2
    exit 2
    ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

if [[ "${DETACH}" == true ]]; then
  if [[ -z "${LOG_FILE}" ]]; then
    LOG_DIR="${REPO_ROOT}/target/mens-gate-logs"
    mkdir -p "${LOG_DIR}"
    LOG_FILE="${LOG_DIR}/mens_gate_${PROFILE}_$(date +%Y%m%d_%H%M%S).log"
  fi
  mkdir -p "$(dirname "${LOG_FILE}")"
  nohup env bash "$0" --profile "${PROFILE}" --log-file "${LOG_FILE}" </dev/null >/dev/null 2>&1 &
  echo "Detached mens-gate (profile=${PROFILE}). Tail log:"
  echo "  tail -f \"${LOG_FILE}\""
  exit 0
fi

cd "${REPO_ROOT}"

run_isolated_gate() {
  local -a args=(ci mens-gate --profile "${PROFILE}" --isolated-runner)
  if [[ -n "${LOG_FILE}" ]]; then
    args+=(--gate-log-file "${LOG_FILE}")
  fi
  if command -v vox >/dev/null 2>&1; then
    vox "${args[@]}"
  else
    cargo run -p vox-cli -- "${args[@]}"
  fi
}

run_isolated_gate
