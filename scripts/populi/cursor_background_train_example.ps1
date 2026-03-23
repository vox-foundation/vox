# Example: start QLoRA training detached with log (parent returns immediately — OK for Cursor timeouts).
# Requires: release `vox` with gpu,populi-candle-cuda; VS env already used for the build.
# Child processes inherit the current environment (`VOX_*`, `RUST_LOG`, etc.). On PowerShell 7+ you can pass
# an isolated env with `Start-Process ... -Environment @{ VOX_TRAIN_SKIP_CORPUS_MIX = '1' }` if needed.
# Adjust MODEL, OUT, LOGDIR, and flags as needed.
#
# Usage:
#   pwsh scripts/populi/cursor_background_train_example.ps1
#   Get-Content populi/runs/logs/train_*.log -Wait -Tail 20

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$vox = Join-Path $root "target\release\vox.exe"
if (-not (Test-Path $vox)) {
    throw "Missing $vox — run CUDA release build first (see cursor_background_cuda_build.ps1)"
}

$logDir = Join-Path $root "populi\runs\logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

# $env:VOX_TRAIN_SKIP_CORPUS_MIX = "1"  # uncomment to skip mix (pinned train.jsonl / faster IDE runs)
# $env:VOX_QLORA_DEBUG_NORMS = "1"      # optional: stderr norms from qlora-rs patch

$p = Start-Process -FilePath $vox -WorkingDirectory $root -WindowStyle Hidden -PassThru -ArgumentList @(
    "populi", "train",
    "--backend", "qlora",
    "--tokenizer", "hf",
    "--preset", "qwen_4080_16g",
    "--model", "Qwen/Qwen2.5-Coder-3B-Instruct",
    "--data-dir", "target/dogfood",
    "--output-dir", "populi/runs/qwen25_qlora_bg",
    "--device", "cuda",
    "--epochs", "1",
    "--background",
    "--log-dir", $logDir
)

Write-Host "Started vox populi train PID $($p.Id). Logs under $logDir (train_<timestamp>.log)"
Write-Host "Monitor: Get-ChildItem $logDir\train_*.log | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | ForEach-Object { Get-Content `$_.FullName -Wait -Tail 25 }"
