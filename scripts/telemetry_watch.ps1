# Thin delegate to `vox mens watch-telemetry` (default ~3s poll; Ctrl+C to stop).
# Canonical: `vox mens watch` from repo root. Legacy 500ms PowerShell implementation removed.
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

if (Get-Command vox -ErrorAction SilentlyContinue) {
    & vox mens watch-telemetry
    exit $LASTEXITCODE
}

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = "cargo" }
& $cargo run -p vox-cli -- mens watch-telemetry
exit $LASTEXITCODE
