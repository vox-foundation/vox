# Thin wrapper: canonical RTX 4080-class Qwen QLoRA train with --log-dir (parent can exit; tail logs).
# Requires: `target/release/vox.exe` built with gpu,mens-candle-cuda (see cursor_background_cuda_build.ps1).
#
# Usage:
#   pwsh scripts/populi/dogfood_qlora_cuda.ps1
#   Get-Content mens/runs/logs/train_*.log -Wait -Tail 25

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$vox = Join-Path $root "target\release\vox.exe"
if (-not (Test-Path $vox)) {
    throw "Missing $vox — build with: cargo vox-cuda-release (VS Developer shell on Windows)"
}

$logDir = Join-Path $root "mens\runs\logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

# Optional: $env:VOX_TRAIN_SKIP_CORPUS_MIX = "1"

& $vox mens train `
    --backend qlora `
    --tokenizer hf `
    --preset qwen_4080_16g `
    --model Qwen/Qwen3.5-4B `
    --data-dir target/dogfood `
    --output-dir mens/runs/qwen35_qlora_dogfood `
    --device cuda `
    --qlora-require-full-proxy-stack `
    --data-mode strict `
    --background `
    --log-dir $logDir

Write-Host "If parent exited immediately, tail: Get-Content $logDir\train_*.log -Wait -Tail 25"
