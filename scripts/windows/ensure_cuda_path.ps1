#Requires -Version 5.1
<#
.SYNOPSIS
  Idempotently prepend NVIDIA CUDA toolkit bin dirs to the **User** PATH and set User CUDA_PATH.

.DESCRIPTION
  Use when `nvcc` works in a full desktop shell but Cursor / CI / minimal agents cannot find it.
  Run once per user account (no admin required for User scope).

.EXAMPLE
  pwsh -File scripts/windows/ensure_cuda_path.ps1
  pwsh -File scripts/windows/ensure_cuda_path.ps1 -CudaRoot 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1'
#>
param(
    [string]$CudaRoot = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1'
)

$ErrorActionPreference = 'Stop'
$bin = Join-Path $CudaRoot 'bin'
$binX64 = Join-Path $CudaRoot 'bin\x64'
if (-not (Test-Path (Join-Path $bin 'nvcc.exe'))) {
    Write-Error "nvcc.exe not found under $bin — adjust -CudaRoot or install CUDA Toolkit."
}

$toAdd = @($bin, $binX64) | Where-Object { Test-Path $_ }
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not $userPath) { $userPath = '' }
$parts = $userPath -split ';' | Where-Object { $_ -ne '' }
$missing = $toAdd | Where-Object { $parts -notcontains $_ }
if ($missing.Count -eq 0) {
    Write-Host "User PATH already contains CUDA bin entries for $CudaRoot"
} else {
    $newPath = ($toAdd + $parts) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    Write-Host "Updated User PATH (prepended): $($missing -join '; ')"
}

[Environment]::SetEnvironmentVariable('CUDA_PATH', $CudaRoot, 'User')
Write-Host "Set User CUDA_PATH=$CudaRoot"
Write-Host "Open a **new** terminal (or restart Cursor) so processes pick up the change."
