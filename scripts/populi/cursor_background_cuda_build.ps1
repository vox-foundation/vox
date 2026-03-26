# Thin delegate: CUDA release build with tee to mens/runs/logs (same as `cargo vox-cuda-release` + log).
# Requires: VS 2022 / MSVC x64 + nvcc on PATH for `mens-candle-cuda`.
# Usage (repo root):
#   pwsh scripts/populi/cursor_background_cuda_build.ps1
#   Get-Content mens/runs/logs/cuda_build_*.log -Wait -Tail 30
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

if (Get-Command vox -ErrorAction SilentlyContinue) {
    & vox ci cuda-release-build
    exit $LASTEXITCODE
}

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = "cargo" }
& $cargo run -p vox-cli -- ci cuda-release-build
exit $LASTEXITCODE
