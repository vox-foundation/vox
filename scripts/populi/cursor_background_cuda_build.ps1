# Run a long CUDA release build with logging (survives Cursor/agent tool timeouts).
# Usage (from repo root, VS 2022 Developer PowerShell recommended):
#   pwsh scripts/mens/cursor_background_cuda_build.ps1
#   Get-Content mens/runs/logs/cuda_build_*.log -Wait -Tail 30
#
# Optional: set $env:CARGO_BUILD_JOBS = "8"

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

$logDir = Join-Path $root "mens/runs/logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$log = Join-Path $logDir "cuda_build_$stamp.log"

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { throw "cargo not found: $cargo" }

Write-Host "Logging to $log (tee). Build runs in foreground of this script; start it in a separate terminal if the IDE times out."
Write-Host "  Or: Start-Process pwsh -ArgumentList '-NoProfile','-File','$PSCommandPath' -WorkingDirectory '$root'"

& $cargo build -p vox-cli --bin vox --release --features gpu,mens-candle-cuda *>&1 | Tee-Object -FilePath $log
