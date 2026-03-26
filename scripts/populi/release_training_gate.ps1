# Windows-safe delegate for `vox ci mens-gate --profile training`.
# Uses an isolated target dir and temp-copied vox.exe to avoid file-lock collisions.
$ErrorActionPreference = "Stop"
Set-Location (Resolve-Path (Join-Path $PSScriptRoot "..\.."))

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = "cargo" }
$targetDir = Join-Path (Get-Location) "target\mens-gate-safe"

& $cargo build -p vox-cli --target-dir $targetDir --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$vox = Join-Path $targetDir "debug\vox.exe"
if (-not (Test-Path $vox)) {
  throw "vox gate binary not found: $vox"
}

$tmpVox = Join-Path ([System.IO.Path]::GetTempPath()) ("vox-gate-" + [Guid]::NewGuid().ToString("N") + ".exe")
Copy-Item $vox $tmpVox -Force
try {
  & $tmpVox ci mens-gate --profile training
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
  Remove-Item $tmpVox -ErrorAction SilentlyContinue
}
