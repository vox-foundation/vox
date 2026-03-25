# Thin delegate — implementation: `vox ci mens-gate --profile m1m4`.
$ErrorActionPreference = "Stop"
Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))
cargo run -p vox-cli --quiet -- ci mens-gate --profile m1m4
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
