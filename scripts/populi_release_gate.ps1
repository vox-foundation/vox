# Thin delegate — implementation: `vox ci populi-gate --profile m1m4`.
$ErrorActionPreference = "Stop"
Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))
cargo run -p vox-cli --quiet -- ci populi-gate --profile m1m4
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
