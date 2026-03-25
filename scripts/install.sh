#!/usr/bin/env bash
# Thin wrapper around `vox-bootstrap`:
# 1) Prefer local `cargo run -p vox-bootstrap` in a repo checkout (debuggable SSOT path).
# 2) Else use `vox-bootstrap` from PATH if present.
# 3) Else download a standalone `vox-bootstrap` release asset, verify checksum, execute it.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

API_LATEST_URL="https://api.github.com/repos/vox-foundation/vox/releases/latest"
RELEASE_BASE_URL="https://github.com/vox-foundation/vox/releases/download"
FORCE_BINARY="${VOX_USE_BOOTSTRAP_BINARY:-0}"
PASS_ARGS=("$@")

in_repo_checkout() {
  [[ -f "$ROOT/Cargo.toml" && -f "$ROOT/crates/vox-bootstrap/Cargo.toml" ]]
}

extract_version_arg() {
  local i=0
  while [[ $i -lt ${#PASS_ARGS[@]} ]]; do
    if [[ "${PASS_ARGS[$i]}" == "--version" ]]; then
      local next=$((i + 1))
      if [[ $next -lt ${#PASS_ARGS[@]} ]]; then
        echo "${PASS_ARGS[$next]}"
        return 0
      fi
    fi
    i=$((i + 1))
  done
  echo ""
}

normalize_tag() {
  local v="$1"
  if [[ -z "$v" ]]; then
    echo ""
    return 0
  fi
  if [[ "$v" == v* ]]; then
    echo "$v"
  else
    echo "v$v"
  fi
}

resolve_latest_tag() {
  curl --proto '=https' --tlsv1.2 -fsSL "$API_LATEST_URL" \
    | sed -nE 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/p' \
    | head -n 1
}

host_target_triple() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)
      [[ "$arch" == "x86_64" ]] && echo "x86_64-unknown-linux-gnu" && return 0
      ;;
    Darwin)
      [[ "$arch" == "x86_64" ]] && echo "x86_64-apple-darwin" && return 0
      [[ "$arch" == "arm64" || "$arch" == "aarch64" ]] && echo "aarch64-apple-darwin" && return 0
      ;;
    MINGW*|MSYS*|CYGWIN*)
      [[ "$arch" == "x86_64" || "$arch" == "amd64" ]] && echo "x86_64-pc-windows-msvc" && return 0
      ;;
  esac
  return 1
}

sha256_for_file() {
  local p="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$p" | awk '{print $1}'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$p" | awk '{print $1}'
    return 0
  fi
  if command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$p" | awk '{print $NF}'
    return 0
  fi
  echo "no sha256 tool found (need sha256sum, shasum, or openssl)" >&2
  return 1
}

verify_checksum() {
  local file="$1" checksums="$2" expected_name="$3"
  local expected
  expected="$(awk -v n="$expected_name" '$2==n {print tolower($1)}' "$checksums" | head -n 1)"
  if [[ -z "$expected" ]]; then
    echo "checksum entry not found for $expected_name" >&2
    return 1
  fi
  local actual
  actual="$(sha256_for_file "$file" | tr '[:upper:]' '[:lower:]')"
  if [[ "$actual" != "$expected" ]]; then
    echo "checksum mismatch for $expected_name (expected $expected, got $actual)" >&2
    return 1
  fi
}

run_standalone_bootstrap() {
  local req_version tag triple ext asset tempdir checksums_url asset_url asset_path checksums_path
  req_version="$(extract_version_arg)"
  tag="$(normalize_tag "$req_version")"
  if [[ -z "$tag" ]]; then
    tag="$(resolve_latest_tag)"
  fi
  if [[ -z "$tag" ]]; then
    echo "failed to resolve release tag from GitHub API" >&2
    return 1
  fi
  triple="$(host_target_triple)" || {
    echo "unsupported host platform for standalone bootstrap installer" >&2
    return 1
  }
  if [[ "$triple" == *windows* ]]; then
    ext="zip"
  else
    ext="tar.gz"
  fi
  asset="vox-bootstrap-${tag}-${triple}.${ext}"
  tempdir="$(mktemp -d)"
  trap 'rm -rf "$tempdir"' EXIT
  asset_path="$tempdir/$asset"
  checksums_path="$tempdir/checksums.txt"
  asset_url="${RELEASE_BASE_URL}/${tag}/${asset}"
  checksums_url="${RELEASE_BASE_URL}/${tag}/checksums.txt"

  echo "  downloading standalone bootstrap asset: $asset"
  curl --proto '=https' --tlsv1.2 -fsSL "$asset_url" -o "$asset_path"
  curl --proto '=https' --tlsv1.2 -fsSL "$checksums_url" -o "$checksums_path"
  verify_checksum "$asset_path" "$checksums_path" "$asset"

  if [[ "$ext" == "zip" ]]; then
    if command -v unzip >/dev/null 2>&1; then
      unzip -q "$asset_path" -d "$tempdir"
    else
      echo "unzip not found for zip extraction" >&2
      return 1
    fi
    "$tempdir/vox-bootstrap.exe" "${PASS_ARGS[@]}"
  else
    tar -xzf "$asset_path" -C "$tempdir"
    chmod +x "$tempdir/vox-bootstrap"
    "$tempdir/vox-bootstrap" "${PASS_ARGS[@]}"
  fi
}

if [[ "$FORCE_BINARY" != "1" ]] && in_repo_checkout && command -v cargo >/dev/null 2>&1; then
  cargo run --locked -p vox-bootstrap -- "${PASS_ARGS[@]}"
  exit $?
fi

if [[ "$FORCE_BINARY" != "1" ]] && command -v vox-bootstrap >/dev/null 2>&1; then
  vox-bootstrap "${PASS_ARGS[@]}"
  exit $?
fi

run_standalone_bootstrap
