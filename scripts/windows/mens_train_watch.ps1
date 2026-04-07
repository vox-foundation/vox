# Start `vox mens train` in a separate process (survives Cursor terminal teardown) and tail telemetry here.
# Defaults match `vox mens watch-telemetry` log paths under target/dogfood.
param(
    [string[]]$TrainArgs = @("--device", "cuda", "--backend", "qlora", "--tokenizer", "hf"),
    [string]$TelemetryPath = "target/dogfood/telemetry.jsonl",
    [string]$ErrLogPath = "target/dogfood/train.err.log",
    [ValidateRange(500, 600000)]
    [int]$IntervalMs = 3000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path -LiteralPath $cargo)) {
    throw "cargo not found at $cargo"
}

$trainArgList = @("run", "-p", "vox-cli", "--", "mens", "train") + $TrainArgs
Write-Host "[mens_train_watch] repo=$repoRoot"
Write-Host "[mens_train_watch] spawning: $cargo $($trainArgList -join ' ')"

$proc = Start-Process -FilePath $cargo -ArgumentList $trainArgList -WorkingDirectory $repoRoot -PassThru
Write-Host "[mens_train_watch] train PID=$($proc.Id); Ctrl+C stops watch only (train keeps running in its window)."

$watchArgList = @(
    "run", "-p", "vox-cli", "--", "mens", "watch-telemetry",
    "--telemetry", $TelemetryPath,
    "--err-log", $ErrLogPath,
    "--interval-ms", "$IntervalMs"
)
& $cargo @watchArgList
