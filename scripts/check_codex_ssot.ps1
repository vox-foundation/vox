# Thin delegate — implementation: `vox ci check-codex-ssot`.
$ErrorActionPreference = "Stop"
Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))
cargo run -p vox-cli --quiet -- ci check-codex-ssot
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
