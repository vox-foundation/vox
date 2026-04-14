<#
.SYNOPSIS
    MENS full automation pipeline — single-command training runner.

.DESCRIPTION
    Runs the complete MENS training cycle without any manual steps:
      1. Corpus mix  (domain-aware, uses validated_mixed.jsonl as primary)
      2. Pre-flight eval gate
      3. GPU training (foreground with live telemetry tail, or -Background)
      4. Post-training eval gate + timing summary

.PARAMETER Domain
    Domain profile to train. One of: vox-lang, rust-expert, agents.
    Defaults to vox-lang (fastest, best first feedback loop).

.PARAMETER Epochs
    Number of training epochs. Defaults to 3.

.PARAMETER Binary
    Path to the vox CLI binary. If omitted, tries 'vox' on PATH, then
    target\release\vox.exe, then builds via 'cargo run -p vox-cli'.

.PARAMETER Background
    If set, launch training as a detached background process with log tailing.

.PARAMETER SkipMix
    If set, skip the corpus mix step (useful when data is already fresh).

.PARAMETER SkipEval
    If set, skip the eval gate (use only for rapid iteration).

.PARAMETER Preset
    Training preset name. Defaults to 'qwen_4080_16g' for RTX 4080 SUPER.

.PARAMETER Device
    Training device. Defaults to 'cuda'.

.EXAMPLE
    .\scripts\mens-full-pipeline.ps1
    .\scripts\mens-full-pipeline.ps1 -Domain vox-lang -Epochs 3
    .\scripts\mens-full-pipeline.ps1 -Domain rust-expert -Background
    .\scripts\mens-full-pipeline.ps1 -Domain vox-lang -SkipMix -Epochs 1
#>

[CmdletBinding()]
param(
    [ValidateSet('vox-lang', 'rust-expert', 'agents')]
    [string]$Domain = 'vox-lang',

    [int]$Epochs = 3,

    [string]$Binary = '',

    [switch]$Background,

    [switch]$SkipMix,

    [switch]$SkipEval,

    [string]$Preset = 'qwen_4080_16g',

    [string]$Device = 'cuda'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ── Colours ──────────────────────────────────────────────────────────────────
function Write-Step([string]$msg) { Write-Host "  ✦ $msg" -ForegroundColor Cyan }
function Write-Ok([string]$msg)   { Write-Host "  ✓ $msg" -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host "  ⚠ $msg" -ForegroundColor Yellow }
function Write-Fail([string]$msg) { Write-Host "  ✗ $msg" -ForegroundColor Red }

# ── Resolve repo root ─────────────────────────────────────────────────────────
$RepoRoot = (git rev-parse --show-toplevel 2>$null)
if (-not $RepoRoot) {
    $RepoRoot = $PSScriptRoot | Split-Path -Parent
}
$RepoRoot = Resolve-Path $RepoRoot

Push-Location $RepoRoot

# ── Resolve vox binary ────────────────────────────────────────────────────────
if (-not $Binary) {
    if (Test-Path "target\debug\vox.exe") {
        $Binary = "target\debug\vox.exe"
    } elseif (Test-Path "target\release\vox.exe") {
        $Binary = "target\release\vox.exe"
    } else {
        # Try PATH as last resort
        $voxCmd = Get-Command vox -ErrorAction SilentlyContinue
        if ($voxCmd) {
            $Binary = $voxCmd.Source
        } else {
            Write-Warn "vox binary not found on PATH or in target\. Using 'cargo run -p vox-cli --' as fallback."
            $Binary = $null
        }
    }
}

function Invoke-Vox {
    param([string[]]$CommandArgs)
    if ($Binary) {
        & $Binary @CommandArgs
        if ($LASTEXITCODE -ne 0) {
            throw "vox exited with code ${LASTEXITCODE}: vox $($CommandArgs -join ' ')"
        }
    } else {
        cargo run -p vox-cli -- @CommandArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo run vox-cli exited with code $LASTEXITCODE"
        }
    }
}

# ── Banner ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║     VoxMens — Automated Training Pipeline        ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Domain:   $Domain" -ForegroundColor White
Write-Host "  Epochs:   $Epochs" -ForegroundColor White
Write-Host "  Device:   $Device" -ForegroundColor White
Write-Host "  Preset:   $Preset" -ForegroundColor White
Write-Host "  Background: $Background" -ForegroundColor White
Write-Host ""

$PipelineStart = Get-Date

# ── Step 1: Corpus mix ────────────────────────────────────────────────────────
if (-not $SkipMix) {
    Write-Step "Step 1/4: Running corpus mix for domain '$Domain'..."

    # Determine which mix config to use for the domain
    $MixConfigMap = @{
        'vox-lang'    = 'mens\config\mix-vox-lang.yaml'
        'rust-expert' = 'mens\config\mix-rust.yaml'
        'agents'      = 'mens\config\mix-agents.yaml'
    }
    $DomainMixConfig = $MixConfigMap[$Domain]

    # Always run the primary mix first (produces validated_mixed.jsonl)
    Write-Step "  Running primary mix (mix.yaml → target/dogfood/train_mixed.jsonl)..."
    Invoke-Vox 'mens', 'corpus', 'mix', '--config', 'mens\config\mix.yaml'
    Write-Ok "Primary mix complete."

    # Run domain-specific mix if the config exists
    if (Test-Path $DomainMixConfig) {
        Write-Step "  Running domain mix ($DomainMixConfig)..."
        Invoke-Vox 'mens', 'corpus', 'mix', '--config', $DomainMixConfig
        Write-Ok "Domain mix complete."
    } else {
        Write-Warn "Domain mix config not found: $DomainMixConfig (using primary mix output)"
    }

    # Report corpus sizes
    $CorpusFiles = @(
        'target\dogfood\validated_mixed.jsonl',
        'target\dogfood\train_mixed.jsonl',
        'target\dogfood\train.jsonl'
    )
    foreach ($f in $CorpusFiles) {
        if (Test-Path $f) {
            $size = (Get-Item $f).Length
            $lines = (Get-Content $f | Measure-Object -Line).Lines
            Write-Ok "  $f — $([math]::Round($size/1MB, 1))MB, $lines lines"
            
            # Record diversity metrics for the domain if this is the primary mixed corpus
            if ($f -eq 'target\dogfood\validated_mixed.jsonl' -or $f -eq 'target\dogfood\train_mixed.jsonl') {
                Write-Step "  Recording diversity metrics (Domain: $Domain)..."
                # We use -Binary directly or & if we want to ignore the 'alarm' exit code
                # Invoke-Vox throws on non-zero, but diversity-check returns 1 on alarm.
                # We want the pipeline to continue even if diversity is low (it's just a warning here).
                if ($Binary) {
                    & $Binary 'mens' 'corpus' 'diversity-check' '--input' $f '--domain' $Domain
                } else {
                    cargo run -q -p vox-cli -- 'mens' 'corpus' 'diversity-check' '--input' $f '--domain' $Domain
                }
                Write-Ok "  Diversity metrics recording attempt finished."
            }
        }
    }
} else {
    Write-Warn "Step 1/4: Skipping corpus mix (--SkipMix set)."
}

# ── Step 2: Pre-flight eval gate ──────────────────────────────────────────────
if (-not $SkipEval) {
    Write-Step "Step 2/4: Running pre-flight eval gate..."

    # Check that the primary corpus exists and meets minimum size
    $PrimaryCorpus = 'target\dogfood\validated_mixed.jsonl'
    
    # Check lines in validated corpus
    $validatedCount = 0
    if (Test-Path $PrimaryCorpus) {
        $validatedCount = (Get-Content $PrimaryCorpus | Measure-Object -Line).Lines
    }

    if ($validatedCount -lt 1000) {
        Write-Warn "Validated corpus only has $validatedCount lines. Checking train_mixed.jsonl fallback..."
        $FallbackCorpus = 'target\dogfood\train_mixed.jsonl'
        if (Test-Path $FallbackCorpus) {
            $fallbackCount = (Get-Content $FallbackCorpus | Measure-Object -Line).Lines
            if ($fallbackCount -ge 1000) {
                $PrimaryCorpusId = $FallbackCorpus
                $corpusLines = $fallbackCount
                Write-Ok "Using fallback corpus: $FallbackCorpus ($fallbackCount lines)"
            } else {
                throw "No corpus available with at least 1000 lines. (Validated: $validatedCount, Mixed: $fallbackCount)"
            }
        } else {
            throw "No corpus available. Run without -SkipMix first."
        }
    } else {
        $PrimaryCorpusId = $PrimaryCorpus
        $corpusLines = $validatedCount
    }

    Write-Ok "Corpus OK: $corpusLines lines in $PrimaryCorpusId"

    # Run latest eval_gate if a previous run exists
    if (Test-Path 'mens\runs\latest') {
        try {
            Invoke-Vox 'mens', 'eval-gate', '--run-dir', 'mens\runs\latest', '--policy', 'warn'
            Write-Ok "Eval gate: passed."
        } catch {
            Write-Warn "Eval gate: $($_.Exception.Message) (non-blocking — training will proceed)"
        }
    } else {
        Write-Warn "No previous run found for eval gate (first run — skipping)."
    }
} else {
    Write-Warn "Step 2/4: Skipping pre-flight eval (--SkipEval set)."
}

# ── Step 3: Training ──────────────────────────────────────────────────────────
Write-Step "Step 3/4: Launching training..."
Write-Host ""

$TrainArgs = @(
    'mens', 'train',
    '--backend', 'qlora',
    '--tokenizer', 'hf',
    '--device', $Device,
    '--preset', $Preset,
    '--domain', $Domain,
    '--epochs', $Epochs.ToString(),
    '--data-dir', 'target\dogfood',
    '--output-dir', 'mens\runs\latest',
    '--seed', '42',
    '--qlora-ce-last-k', '64',
    '--validation-split-ratio', '0.05',
    '--force-restart'
)

if ($Background) {
    $LogDir = "mens\runs\logs"
    $null = New-Item -ItemType Directory -Force -Path $LogDir
    $LogFile = Join-Path $LogDir "train_$(Get-Date -Format 'yyyyMMddTHHmmss').log"

    # Detach via Start-Process
    $BinaryOrCargo = if ($Binary) { $Binary } else { 'cargo' }
    $BinaryArgs    = if ($Binary) { $TrainArgs } else { @('run', '-p', 'vox-cli', '--') + $TrainArgs }

    Write-Step "Spawning background process → log: $LogFile"
    $proc = Start-Process -FilePath $BinaryOrCargo -ArgumentList $BinaryArgs `
        -RedirectStandardOutput $LogFile `
        -RedirectStandardError "$LogFile.err" `
        -PassThru -WindowStyle Hidden

    Write-Ok "Training process spawned (PID=$($proc.Id))."
    Write-Host ""
    Write-Host "  Monitor live progress with:" -ForegroundColor White
    Write-Host "    Get-Content -Wait $LogFile" -ForegroundColor Yellow
    Write-Host "    vox mens watch-telemetry --telemetry mens\runs\latest\telemetry.jsonl" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  Check manifest after run:" -ForegroundColor White
    Write-Host "    Get-Content mens\runs\latest\training_manifest.json" -ForegroundColor Yellow
} else {
    Write-Step "Running training in foreground (Ctrl+C for graceful pause)..."
    Write-Host ""

    # Tail telemetry in a background job while training runs in foreground
    $TelPath = Join-Path $RepoRoot 'mens\runs\latest\telemetry.jsonl'
    $tailJob = $null
    if (Test-Path (Split-Path $TelPath -Parent)) {
        $tailJob = Start-Job -ScriptBlock {
            param($tp)
            $lastSize = 0
            while ($true) {
                Start-Sleep -Milliseconds 3000
                if (Test-Path $tp) {
                    $current = (Get-Item $tp).Length
                    if ($current -gt $lastSize) {
                        $lines = Get-Content $tp | Select-Object -Last 3
                        foreach ($l in $lines) {
                            try {
                                $j = $l | ConvertFrom-Json -ErrorAction SilentlyContinue
                                if ($j.event -eq 'train_step') {
                                    $eta  = if ($j.eta_seconds) { "eta ~$([math]::Round($j.eta_seconds/60))min" } else { "calibrating..." }
                                    $loss = if ($j.loss)  { "loss=$([math]::Round($j.loss,4))" } else { "" }
                                    $pct  = if ($j.progress_pct) { "$([math]::Round($j.progress_pct,1))%" } else { "" }
                                    Write-Host "  [tel] E$($j.epoch) opt_step=$($j.optimizer_step) $loss $pct $eta" -ForegroundColor DarkCyan
                                }
                            } catch {}
                        }
                        $lastSize = $current
                    }
                }
            }
        } -ArgumentList $TelPath
    }

    try {
        Invoke-Vox @TrainArgs
        Write-Ok "Training complete."
    } finally {
        if ($tailJob) {
            Stop-Job $tailJob -ErrorAction SilentlyContinue
            Remove-Job $tailJob -ErrorAction SilentlyContinue
        }
    }
}

# ── Step 4: Post-training summary ─────────────────────────────────────────────
if (-not $Background) {
    Write-Step "Step 4/4: Post-training verification..."

    $ManifestPath = 'mens\runs\latest\training_manifest.json'
    if (Test-Path $ManifestPath) {
        $manifest = Get-Content $ManifestPath | ConvertFrom-Json
        $stepsExecuted = $manifest.candle_qlora_training_steps_executed
        $adapterPath   = $manifest.adapter_path

        if ($stepsExecuted -gt 0) {
            Write-Ok "Training succeeded: $stepsExecuted optimizer steps completed."
            if ($adapterPath -and (Test-Path $adapterPath)) {
                $adapterSize = [math]::Round((Get-Item $adapterPath).Length / 1MB, 1)
                Write-Ok "Adapter saved: $adapterPath ($adapterSize MB)"
            }
        } else {
            Write-Fail "Warning: training_manifest shows 0 steps executed — training may have crashed."
            Write-Host "  Check: mens\runs\latest\telemetry.jsonl" -ForegroundColor Yellow
        }
    } else {
        Write-Warn "No training manifest found at $ManifestPath"
    }

    $Elapsed = (Get-Date) - $PipelineStart
    Write-Host ""
    Write-Ok "Pipeline complete in $([math]::Round($Elapsed.TotalMinutes,1)) minutes."
    Write-Host ""
    Write-Host "  Next steps:" -ForegroundColor White
    Write-Host "    vox mens status --run-dir mens\runs\latest" -ForegroundColor Yellow
    Write-Host "    vox mens watch-telemetry --telemetry mens\runs\latest\telemetry.jsonl" -ForegroundColor Yellow
    Write-Host "    vox mens eval-gate --run-dir mens\runs\latest" -ForegroundColor Yellow
    Write-Host ""
}

Pop-Location
