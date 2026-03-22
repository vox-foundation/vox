$ErrorActionPreference = "Stop"

$TelemetryPath = "target/dogfood/telemetry.jsonl"
$LogPath       = "target/dogfood/train.err.log"
$CR            = "`r"
$CLR           = "$CR" + (" " * 80) + $CR   # wipe a full line

$lastSizeTel        = 0
$lastSizeErr        = 0
$tableHeaderPrinted = $false
$prevLoss           = $null
$recentLosses       = [System.Collections.Generic.Queue[double]]::new()
$maxRecentLosses    = 3

Write-Host "Monitoring QLoRA Training Pipeline  (telemetry v2)" -ForegroundColor Cyan
Write-Host "====================================================`n"
Write-Host "Key: " -NoNewline
Write-Host "VCB=bad_vocab  HID=last_hidden  SEQ=short_seq  " -ForegroundColor DarkGray -NoNewline
Write-Host "(zeros are healthy)" -ForegroundColor Green
Write-Host ""

function Read-FileTail([string]$Path, [ref]$Offset) {
    $fs     = [System.IO.File]::Open($Path, 'Open', 'Read', 'ReadWrite')
    $reader = New-Object System.IO.StreamReader($fs)
    $reader.BaseStream.Seek($Offset.Value, 'Begin') | Out-Null
    $text   = $reader.ReadToEnd()
    $Offset.Value = $reader.BaseStream.Position
    $reader.Close(); $fs.Close()
    return $text
}

function Format-ETA([double]$s) {
    if    ($s -ge 3600) { return "{0:N1}h"  -f ($s / 3600) }
    elseif($s -ge 60)   { return "{0:N0}m"  -f ($s / 60)   }
    else                { return "{0:N0}s"  -f $s           }
}

function Get-LossTrend([double]$loss) {
    if ($null -eq $script:prevLoss) { return "  " }
    $delta = $loss - $script:prevLoss
    if    ($delta -lt -0.005) { return [char]0x2193 }  # ↓ improving
    elseif($delta -gt  0.005) { return [char]0x2191 }  # ↑ degrading
    else                      { return [char]0x2192 }  # → stable
}

function Get-LossColor([string]$trend) {
    if ($trend -eq [string][char]0x2193) { return 'Green'  }
    if ($trend -eq [string][char]0x2191) { return 'Red'    }
    return 'Yellow'
}

function Get-MovingAvg {
    if ($script:recentLosses.Count -eq 0) { return $null }
    $sum = 0; $script:recentLosses | ForEach-Object { $sum += $_ }
    return $sum / $script:recentLosses.Count
}

while ($true) {
    # ---- 1. Stream engine stderr (compilation + runtime banner) ----
    if (Test-Path $LogPath) {
        $cur = (Get-Item $LogPath).Length
        if ($cur -gt $lastSizeErr) {
            $raw = Read-FileTail -Path $LogPath -Offset ([ref]$lastSizeErr)
            $lines = $raw -split "`r?`n|`r"
            foreach ($line in $lines) {
                $clean = $line -replace "`e\[[0-9;]*[a-zA-Z]", ""
                if ([string]::IsNullOrWhiteSpace($clean)) { continue }

                $prefix = $null
                $color  = 'Gray'

                if      ($clean -match "^\s*Compiling\s+(.+)") { $prefix = "[Build]  Compiling $($matches[1])";      $color = 'DarkGray' }
                elseif  ($clean -match "Finished .release.")   { $prefix = "[Build]  Compilation complete!";         $color = 'Green'    }
                elseif  ($clean -match "Downloading (.+)")     { $prefix = "[Net]    Downloading $($matches[1])";    $color = 'Yellow'   }
                elseif  ($clean -match "Architecture:(.+)")    { $prefix = "[Engine] Architecture:$($matches[1])";   $color = 'Cyan'     }
                elseif  ($clean -match "Tokenizer:(.+)")       { $prefix = "[Engine] Tokenizer:$($matches[1])";      $color = 'Cyan'     }
                elseif  ($clean -match "GPU.CPU fallback")     { $prefix = "[WARN]   GPU fallback to CPU!";          $color = 'Red'      }
                elseif  ($clean -match "QLoRA preflight OK")   { $prefix = "[Engine] Preflight OK";                  $color = 'Green'    }
                elseif  ($clean -match "(Error:|panicked at)") { $prefix = "[FATAL]  $clean";                        $color = 'Red'      }
                elseif  ($clean -match "no training rows")     { $prefix = "[FATAL]  Empty dataset! $clean";         $color = 'Red'      }
                elseif  ($clean -match "\[Epoch ")             { $prefix = "[Steps]  $clean";                        $color = 'Magenta'  }

                if ($prefix) {
                    Write-Host ($CR + $prefix.PadRight(80)) -NoNewline -ForegroundColor $color
                    if ($color -in 'Green', 'Red', 'Cyan') { Write-Host "" }
                }
            }
        }
    }

    # ---- 2. Stream high-fidelity JSONL telemetry ----
    if (Test-Path $TelemetryPath) {
        $cur = (Get-Item $TelemetryPath).Length
        if ($cur -gt $lastSizeTel) {
            if (-not $tableHeaderPrinted) {
                Write-Host ($CLR)
                Write-Host ""
                Write-Host ("{0,-7} {1,-5} {2,-10} {3,-8} {4,-8} {5,-8} {6}" -f `
                    "Step", "Epoch", "Loss(MA3)", "Tok/s", "ETA", "Trend", "Skips") -ForegroundColor Yellow
                Write-Host ("-" * 65) -ForegroundColor DarkYellow
                $tableHeaderPrinted = $true
            }

            $raw = Read-FileTail -Path $TelemetryPath -Offset ([ref]$lastSizeTel)
            foreach ($tline in ($raw -split "`n")) {
                if ([string]::IsNullOrWhiteSpace($tline)) { continue }
                try {
                    $d = $tline | ConvertFrom-Json
                } catch { continue }

                $ev = $d.event

                # ── Step event (most common) ──────────────────────────────
                if ($ev -in 'step', 'train_step') {
                    $step  = if ($d.step) { $d.step } elseif ($d.global_step) { $d.global_step } else { "?" }
                    $epoch = if ($d.epoch) { $d.epoch } else { "?" }
                    $loss  = [double]$d.loss

                    $trend = Get-LossTrend $loss
                    $tcolor = Get-LossColor $trend

                    # update moving average queue
                    $script:recentLosses.Enqueue($loss)
                    while ($script:recentLosses.Count -gt $script:maxRecentLosses) { $script:recentLosses.Dequeue() | Out-Null }
                    $script:prevLoss = $loss

                    $ma  = Get-MovingAvg
                    $lossStr = if ($null -ne $ma) { "{0:N4}({1:N4})" -f $loss, $ma } else { "{0:N4}" -f $loss }
                    $tps = if ($d.tokens_per_sec) { "{0:N1}" -f [double]$d.tokens_per_sec } else { "—" }
                    $eta = if ($d.eta_seconds_remaining) { Format-ETA([double]$d.eta_seconds_remaining) }
                          elseif ($d.eta_sec)            { Format-ETA([double]$d.eta_sec) }
                          else                           { "cal…" }

                    # Skips: only display non-zero values to reduce noise
                    $skipsStr = ""
                    $vcb = if ($d.skips_bad_vocab)    { [int]$d.skips_bad_vocab }    else { 0 }
                    $hid = if ($d.skips_last_hidden)  { [int]$d.skips_last_hidden }  else { 0 }
                    $seq = if ($d.skips_short_seq)    { [int]$d.skips_short_seq }    else { 0 }
                    if ($vcb -gt 0) { $skipsStr += "VCB:$vcb " }
                    if ($hid -gt 0) { $skipsStr += "HID:$hid " }
                    if ($seq -gt 0) { $skipsStr += "SEQ:$seq " }
                    if ($skipsStr -eq "") { $skipsStr = "OK" }

                    $row = "{0,-7} {1,-5} {2,-10} {3,-8} {4,-8} {5,-8} {6}" -f `
                        $step, $epoch, $lossStr, $tps, $eta, $trend, $skipsStr
                    Write-Host $row -ForegroundColor $tcolor
                }

                # ── Eval epoch loss ───────────────────────────────────────
                elseif ($ev -eq 'eval_epoch_loss') {
                    $ep      = if ($d.epoch) { $d.epoch } else { "?" }
                    $eloss   = if ($d.eval_loss) { "{0:N4}" -f [double]$d.eval_loss } else { "n/a" }
                    $epairs  = if ($d.eval_pairs) { $d.eval_pairs } else { "?" }
                    Write-Host ("  [Eval] Epoch {0}  eval_loss={1}  pairs={2}" -f $ep, $eloss, $epairs) -ForegroundColor Cyan
                }

                # ── Epoch summary ─────────────────────────────────────────
                elseif ($ev -eq 'epoch_summary') {
                    $ep       = if ($d.epoch) { $d.epoch } else { "?" }
                    $mloss    = if ($d.epoch_train_loss_mean) { "{0:N4}" -f [double]$d.epoch_train_loss_mean } else { "n/a" }
                    $steps    = if ($d.epoch_train_steps) { $d.epoch_train_steps } else { "?" }
                    $wallsec  = if ($d.epoch_wall_seconds) { Format-ETA([double]$d.epoch_wall_seconds) } else { "?" }
                    Write-Host ("  [Epoch {0} done] mean_loss={1}  steps={2}  dur={3}" -f $ep, $mloss, $steps, $wallsec) `
                        -ForegroundColor Magenta
                }

                # ── Checkpoint saved ──────────────────────────────────────
                elseif ($ev -eq 'checkpoint_saved') {
                    $gstep = if ($d.checkpoint_global_step) { $d.checkpoint_global_step } else { "?" }
                    $cpath = if ($d.checkpoint_path) { $d.checkpoint_path } else { "unknown" }
                    Write-Host ("  [Ckpt] step={0}  path={1}" -f $gstep, (Split-Path $cpath -Leaf)) -ForegroundColor Green
                }

                # ── Corpus coverage ───────────────────────────────────────
                elseif ($ev -eq 'corpus_coverage') {
                    $ratio   = if ($null -ne $d.coverage_ratio)  { "{0:P1}" -f [double]$d.coverage_ratio } else { "?" }
                    $covered = if ($null -ne $d.covered_types)   { $d.covered_types } else { "?" }
                    $total   = if ($null -ne $d.total_types)      { $d.total_types } else { "?" }
                    $balance = if ($null -ne $d.balance_score)   { "{0:N3}" -f [double]$d.balance_score } else { "?" }
                    $missing = if ($d.missing_types)              { ($d.missing_types -join ", ") } else { "none" }
                    Write-Host ("  [Coverage] {0}/{1} types ({2})  balance={3}" -f $covered, $total, $ratio, $balance) -ForegroundColor Yellow
                    if ($missing -ne "none") {
                        Write-Host ("  [Coverage] missing: {0}" -f $missing) -ForegroundColor DarkYellow
                    }
                }

                # ── train_start banner ────────────────────────────────────
                elseif ($ev -eq 'train_start') {
                    $pairs  = if ($d.pairs_loaded) { $d.pairs_loaded } else { "?" }
                    $epochs = if ($d.epochs) { $d.epochs } else { "?" }
                    $kernel = if ($d.execution_kernel) { $d.execution_kernel } else { "unknown" }
                    Write-Host ("  [Start] kernel={0}  pairs={1}  epochs={2}" -f $kernel, $pairs, $epochs) -ForegroundColor Cyan
                }
            }
        }
    }

    # ---- 3. If no files exist yet, pulse a waiting indicator ----
    if (-not (Test-Path $LogPath) -and -not (Test-Path $TelemetryPath)) {
        $dots = "." * (([int](Get-Date).Second % 4) + 1)
        Write-Host ($CR + "[Waiting for process to start$dots]".PadRight(60)) -NoNewline -ForegroundColor DarkGray
    }

    Start-Sleep -Milliseconds 500
}
