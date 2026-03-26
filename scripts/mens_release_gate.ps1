# Windows-safe delegate for `vox ci mens-gate --profile m1m4`.
# Pass -Detach for agent-friendly background gate runs (see scripts/populi/mens_gate_safe.ps1).
$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "populi\mens_gate_safe.ps1") -Profile m1m4 @args
