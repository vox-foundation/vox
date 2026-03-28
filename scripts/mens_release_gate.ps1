# Windows-safe delegate for `vox ci mesh-gate --profile m1m4` (alias: `mens-gate`).
# Pass -Detach for agent-friendly background gate runs (see scripts/populi/mens_gate_safe.ps1).
$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "populi\mens_gate_safe.ps1") -Profile m1m4 @args
