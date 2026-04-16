# Thin delegate — implementation: `vox ci check-codex-ssot`.
# PowerShell parity for scripts/check_codex_ssot.sh.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot
cargo run -p vox-cli --quiet -- ci check-codex-ssot
