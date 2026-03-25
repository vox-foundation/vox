param(
    [string]$RunDir = "mens/runs/latest",
    [string]$PolicyPath = "mens/config/eval-gates.yaml",
    [string]$ModelPath = "mens/runs/latest/candle_qlora_adapter.safetensors",
    [string]$BenchPath = "mens/data/heldout_bench",
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [string]$Name,
        [string]$Command
    )
    Write-Host "[cycle] $Name"
    Write-Host "  cmd: $Command"
    if (-not $DryRun) {
        Invoke-Expression $Command
    }
}

Write-Host "Running 4080 experiment cycle pipeline"
Write-Host "  run_dir: $RunDir"
Write-Host "  policy:  $PolicyPath"
Write-Host "  bench:   $BenchPath"
Write-Host "  dry_run: $DryRun"

# Cycle 1: stability-first checks after training completes.
Invoke-Step -Name "Cycle1/checkpoint-integrity" -Command "& `"$env:USERPROFILE\.cargo\bin\cargo.exe`" run -p vox-cli -- mens eval-gate --run-dir `"$RunDir`" --policy `"$PolicyPath`""

# Cycle 2: quality measurement with pass@k.
Invoke-Step -Name "Cycle2/eval-local-passk" -Command "& `"$env:USERPROFILE\.cargo\bin\cargo.exe`" run -p vox-cli -- mens eval-local --model `"$ModelPath`" --bench `"$BenchPath`" --samples 4 --seed-base 1337 -o `"$RunDir/eval_local_report.json`""
Invoke-Step -Name "Cycle2/gate-passk" -Command "& `"$env:USERPROFILE\.cargo\bin\cargo.exe`" run -p vox-cli -- mens eval-gate --run-dir `"$RunDir`" --policy `"$PolicyPath`""

# Cycle 3: orchestrator multi-use flow compile sanity.
Invoke-Step -Name "Cycle3/orchestrator-compile-check" -Command "& `"$env:USERPROFILE\.cargo\bin\cargo.exe`" check -p vox-orchestrator"

Write-Host "Cycle pipeline complete."
