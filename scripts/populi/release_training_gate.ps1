# Windows-safe delegate for `vox ci mens-gate --profile training`.
# Pass -Detach to avoid Cursor/agent wall-clock timeouts (log under target/mens-gate-logs).
# Pass -LogFile path to tee output (optional with -Detach; auto-picked if -Detach alone).
$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "mens_gate_safe.ps1") -Profile training @args
