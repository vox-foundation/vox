# Delegates to `vox ci toestub-self-apply` (release build + full-repo TOESTUB scan).
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$vox = Join-Path $root "target\release\vox.exe"
if (-not (Test-Path $vox)) {
    $vox = Join-Path $root "target\debug\vox.exe"
}
if (Test-Path $vox) {
    & $vox ci toestub-self-apply
    exit $LASTEXITCODE
}

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = "cargo" }
& $cargo run -p vox-cli -- ci toestub-self-apply
exit $LASTEXITCODE
