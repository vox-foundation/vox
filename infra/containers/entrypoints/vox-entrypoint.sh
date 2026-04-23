#!/usr/bin/env sh
# Optional mens sidecar: when VOX_MESH_MESH_SIDECAR=1, start `vox populi serve` in the background
# then exec the remaining arguments (e.g. `vox mcp`).
set -e
if [ "${VOX_MESH_MESH_SIDECAR:-0}" = "1" ] || [ "${VOX_MESH_MESH_SIDECAR:-0}" = "true" ]; then
  BIND="${VOX_MESH_SIDECAR_BIND:-0.0.0.0:9847}"
  vox populi serve --bind "$BIND" &
  export VOX_MESH_ENABLED="${VOX_MESH_ENABLED:-1}"
  export VOX_MESH_CONTROL_ADDR="${VOX_MESH_CONTROL_ADDR:-http://127.0.0.1:9847}"
fi
exec "$@"
