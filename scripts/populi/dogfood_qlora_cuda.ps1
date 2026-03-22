# Thin wrapper: canonical RTX 4080-class Qwen QLoRA train with --log-dir (parent can exit; tail logs).
# Requires: `target/release/vox.exe` built with gpu,populi-candle-cuda (see cursor_background_cuda_build.ps1).
#
# Usage:
#   pwsh scripts/populi/dogfood_qlora_cuda.ps1
#   Get-Content populi/runs/logs/train_*.log -Wait -Tail 25

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$vox = Join-Path $root "target\release\vox.exe"
if (-not (Test-Path $vox)) {
    throw "Missing $vox — build with: cargo vox-cuda-release (VS Developer shell on Windows)"
}

$logDir = Join-Path $root "populi\runs\logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

# Optional: $env:VOX_TRAIN_SKIP_CORPUS_MIX = "1"

& $vox populi train `
    --backend qlora `
    --tokenizer hf `
    --preset qwen_4080_16g `
    --model Qwen/Qwen2.5-Coder-3B-Instruct `
    --data-dir target/dogfood `
    --output-dir populi/runs/qwen25_qlora_dogfood `
    --device cuda `
    --qlora-require-full-proxy-stack `
    --log-dir $logDir

Write-Host "If parent exited immediately, tail: Get-Content $logDir\train_*.log -Wait -Tail 25"
