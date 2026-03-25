# Thin delegate — implementation: `vox ci mens-gate --profile training`.
$ErrorActionPreference = "Stop"
Set-Location (Resolve-Path (Join-Path $PSScriptRoot "..\.."))
cargo run -p vox-cli --quiet -- ci mens-gate --profile training
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
