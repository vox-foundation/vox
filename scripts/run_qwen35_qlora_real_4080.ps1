# Build (optional CUDA) + background VoxMens Candle QLoRA (SSOT Qwen3.5-4B) for RTX 4080 Super.
# Run from repo root in Developer PowerShell for VS (so nvcc finds cl.exe) if using CUDA.
param(
    [switch]$SkipBuild,
    [switch]$CpuOnlyBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$Cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $Cargo)) { $Cargo = "cargo" }

if (-not $SkipBuild) {
    if ($CpuOnlyBuild) {
        Write-Host "Building vox-cli (GPU profile, CPU Candle — no CUDA kernels)..."
        & $Cargo build -p vox-cli --release --features gpu
    } else {
        Write-Host "cargo vox-cuda-release (alias: gpu,mens-candle-cuda; needs MSVC+nvcc)..."
        & $Cargo vox-cuda-release
    }
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# Respect workspace `target-dir` (e.g. target-agent-test) via cargo metadata.
$metaJson = & $Cargo metadata --format-version 1 --no-deps 2>$null
if (-not $metaJson) { Write-Error "cargo metadata failed" }
$targetDir = ($metaJson | ConvertFrom-Json).target_directory
$Vox = Join-Path $targetDir "release\vox.exe"
if (-not (Test-Path $Vox)) {
    Write-Error "Missing $Vox — build first (same shell / target-dir as cargo)."
}

New-Item -ItemType Directory -Force -Path "$Root\target\dogfood" | Out-Null
New-Item -ItemType Directory -Force -Path "$Root\mens\runs\logs" | Out-Null
New-Item -ItemType Directory -Force -Path "$Root\mens\runs\qwen35_real" | Out-Null

Write-Host "Launching background training (log under mens/runs/logs)..."
& $Vox @(
    "mens", "train",
    "--preset", "qwen_4080_16g",
    "--data-dir", "target/dogfood",
    "--output-dir", "mens/runs/qwen35_real",
    "--device", "cuda",
    "--background",
    "--vram-limit-fraction", "0.72",
    "--qlora-max-skip-rate", "0.20",
    "--log-dir", "mens/runs/logs"
)
exit $LASTEXITCODE
