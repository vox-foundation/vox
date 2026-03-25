# Continuous Training Orchestrator
# This script manages the full life cycle of Mens dogfooding and training dynamically.

param (
    [int]$LoopDelaySeconds = 300
)

$RepoRoot_ = (Resolve-Path "$PSScriptRoot\..").Path
$RunName = "qwen25_qlora"
$TelemetryPath = "$RepoRoot_\mens\runs\$RunName\telemetry.jsonl"

Write-Host "Pre-Compiling vox.exe natively (features: gpu) to prevent cargo OS locking..." -ForegroundColor Yellow
& "$RepoRoot_\build_vox.bat"

if ($LASTEXITCODE -ne 0) {
    Write-Host "FATAL: Cargo build failed!" -ForegroundColor Red
    exit 1
}

while ($true) {
    Write-Host "`n[$(Get-Date -f 'yyyy-MM-dd HH:mm:ss')] Starting new continuous training loop iteration" -ForegroundColor Cyan

    # 1. Re-generate synthetic traces (including search traces!)
    Write-Host "Generating synthetic tool traces..."
    Set-Location $RepoRoot_
    & "$RepoRoot_\target\release\vox.exe" mens corpus generate --output mens/data/synthetic.jsonl
    
    # 2. Extract recent Arca DB chat and A2A replays
    Write-Host "Replaying recent ARCA database interactions..."
    & "$RepoRoot_\target\release\vox.exe" mens corpus replay --output mens/data/arca_replay.jsonl --limit 5000

    # 3. Mix the training corpus (which natively weights searches!)
    Write-Host "Mixing full dynamic corpus dataset..."
    & "$RepoRoot_\target\release\vox.exe" mens corpus mix --config mens/config/mix.yaml

    # 4. Ensure output directories exist
    New-Item -ItemType Directory -Force -Path "$RepoRoot_\target\dogfood" | Out-Null
    New-Item -ItemType Directory -Force -Path "$RepoRoot_\mens\runs\$RunName" | Out-Null

    # 5. Launch Native GPU Training Pipeline (Blocks until complete)
    Write-Host "Launching QLoRA training run $RunName"
    & "$RepoRoot_\launch_train.bat"

    # Post-training: display telemetry
    Write-Host "Training iteration completed or failed. Checking telemetry..."
    if (Test-Path $TelemetryPath) {
        $LastLine = Get-Content $TelemetryPath | Select-Object -Last 1
        Write-Host "Last telemetry event: $LastLine" -ForegroundColor Gray
    }

    Write-Host "Sleeping for $LoopDelaySeconds seconds before next dataset sweep..."
    Start-Sleep -Seconds $LoopDelaySeconds
}
