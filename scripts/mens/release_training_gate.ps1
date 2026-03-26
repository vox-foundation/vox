# Legacy path: forwards to canonical `scripts/populi/release_training_gate.ps1`.
$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "..\populi\release_training_gate.ps1") @args
