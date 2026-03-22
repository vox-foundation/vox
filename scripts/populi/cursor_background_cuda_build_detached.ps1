# Return immediately while a CUDA release build runs in a separate PowerShell process.
# PowerShell cannot use the same file for RedirectStandardOutput and RedirectStandardError on Start-Process;
# spawning `cursor_background_cuda_build.ps1` in a child pwsh avoids that and matches IDE/agent timeouts.
#
# Usage (repo root or any cwd — script resolves repo via $PSScriptRoot):
#   pwsh scripts/populi/cursor_background_cuda_build_detached.ps1
#   Get-Content populi/runs/logs/cuda_build_*.log -Wait -Tail 30

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$buildScript = Join-Path $PSScriptRoot "cursor_background_cuda_build.ps1"

Start-Process -FilePath "pwsh" -WorkingDirectory $root -ArgumentList @(
    "-NoProfile",
    "-File",
    $buildScript
) | Out-Null

Write-Host "Spawned background CUDA build (see populi/runs/logs/cuda_build_*.log). This shell returned immediately."
