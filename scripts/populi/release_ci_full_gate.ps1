# Delegate for `vox ci mesh-gate --profile ci_full` (alias: `mens-gate`; same isolation as training gate).
# Use -Detach from Cursor/agents; tail `target/mens-gate-logs/mens_gate_ci_full_*.log`.
$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "mens_gate_safe.ps1") -Profile ci_full @args
