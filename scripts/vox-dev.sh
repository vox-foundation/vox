#!/usr/bin/env bash
# Run `vox` from the workspace clone: default `cargo run -p vox-cli`, or PATH `vox` when VOX_USE_PATH=1.
#
# Env:
#   VOX_REPO_ROOT     - Force workspace root (root Cargo.toml must contain [workspace]).
#   VOX_USE_PATH=1    - Use `vox` on PATH when available (may be stale vs this clone).
#   VOX_DEV_FEATURES  - Comma-separated extra features for vox-cli (overrides coderabbit auto-detect).
#   VOX_DEV_QUIET=1   - Pass --quiet to cargo run.
#
# Auto: if argv contains the token `coderabbit` and VOX_DEV_FEATURES is unset, adds --features coderabbit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_FROM_SCRIPT="$(cd "$SCRIPT_DIR/.." && pwd)"

is_workspace_root() {
  local d="$1"
  [[ -f "$d/Cargo.toml" ]] && grep -q '^\[workspace\]' "$d/Cargo.toml"
}

find_repo_root() {
  if [[ -n "${VOX_REPO_ROOT:-}" ]]; then
    local r="${VOX_REPO_ROOT%/}"
    if is_workspace_root "$r"; then
      (cd "$r" && pwd)
      return 0
    fi
    echo "vox-dev.sh: VOX_REPO_ROOT is set but is not a Cargo workspace root: $r" >&2
    return 1
  fi

  local starts=("$PWD" "$REPO_FROM_SCRIPT")
  local dir cur parent
  for start in "${starts[@]}"; do
    cur="$start"
    while [[ -n "$cur" ]]; do
      if is_workspace_root "$cur"; then
        (cd "$cur" && pwd)
        return 0
      fi
      parent="$(dirname "$cur")"
      [[ "$parent" == "$cur" ]] && break
      cur="$parent"
    done
  done

  echo "vox-dev.sh: could not find workspace Cargo.toml with [workspace]. Set VOX_REPO_ROOT or cd into the repo." >&2
  return 1
}

need_coderabbit=0
for a in "$@"; do
  if [[ "$a" == coderabbit ]]; then
    need_coderabbit=1
    break
  fi
done

ROOT="$(find_repo_root)"

if [[ "${VOX_USE_PATH:-}" == "1" ]] && command -v vox >/dev/null 2>&1; then
  (cd "$ROOT" && exec vox "$@")
fi

FEATURES=()
if [[ -n "${VOX_DEV_FEATURES:-}" ]]; then
  f="${VOX_DEV_FEATURES//[[:space:]]/}"
  if [[ -n "$f" ]]; then
    FEATURES=(--features "$f")
  fi
elif [[ "$need_coderabbit" -eq 1 ]]; then
  FEATURES=(--features coderabbit)
fi

QUIET=()
if [[ "${VOX_DEV_QUIET:-}" == "1" ]]; then
  QUIET=(--quiet)
fi

cd "$ROOT"
exec cargo run -p vox-cli "${QUIET[@]}" "${FEATURES[@]}" -- "$@"
