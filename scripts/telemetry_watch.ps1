$ErrorActionPreference = "Stop"

$TelemetryPath = "target/dogfood/telemetry.jsonl"
$LogPath       = "target/dogfood/train.err.log"
$CR            = "`r"
$CLR           = "$CR" + (" " * 80) + $CR   # wipe a full line

$lastSizeTel      = 0
$lastSizeErr      = 0
$tableHeaderPrinted = $false

Write-Host "Monitoring QLoRA Training Pipeline" -ForegroundColor Cyan
Write-Host "==================================`n"

function Read-FileTail([string]$Path, [ref]$Offset) {
    $fs     = [System.IO.File]::Open($Path, 'Open', 'Read', 'ReadWrite')
    $reader = New-Object System.IO.StreamReader($fs)
    $reader.BaseStream.Seek($Offset.Value, 'Begin') | Out-Null
    $text   = $reader.ReadToEnd()
    $Offset.Value = $reader.BaseStream.Position
    $reader.Close(); $fs.Close()
    return $text
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

                if      ($clean -match "^\s*Compiling\s+(.+)") { $prefix = "[Build]  Compiling $($matches[1])";   $color = 'DarkGray' }
                elseif  ($clean -match "Finished .release.")    { $prefix = "[Build]  Compilation complete!";        $color = 'Green'    }
                elseif  ($clean -match "Downloading (.+)")      { $prefix = "[Net]    Downloading $($matches[1])";   $color = 'Yellow'   }
                elseif  ($clean -match "Architecture:(.+)")     { $prefix = "[Engine] Architecture:$($matches[1])";  $color = 'Cyan'     }
                elseif  ($clean -match "Tokenizer:(.+)")        { $prefix = "[Engine] Tokenizer:$($matches[1])";     $color = 'Cyan'     }
                elseif  ($clean -match "GPU.CPU fallback")      { $prefix = "[WARN]   GPU fallback to CPU!";         $color = 'Red'      }
                elseif  ($clean -match "QLoRA preflight OK")    { $prefix = "[Engine] Preflight OK";                 $color = 'Green'    }
                elseif  ($clean -match "(Error:|panicked at)")  { $prefix = "[FATAL]  $clean";                       $color = 'Red'      }
                elseif  ($clean -match "no training rows")      { $prefix = "[FATAL]  Empty dataset! $clean";        $color = 'Red'      }
                elseif  ($clean -match "\[Epoch ")              { $prefix = "[Steps]  $clean";                       $color = 'Magenta'  }

                if ($prefix) {
                    Write-Host ($CR + $prefix.PadRight(80)) -NoNewline -ForegroundColor $color
                    if ($color -in 'Green', 'Red', 'Cyan') { Write-Host "" }  # newline for important events
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
                Write-Host ("{0,-8} {1,-6} {2,-12} {3,-10} {4}" -f "Step", "Epoch", "Loss", "Tok/s", "ETA") -ForegroundColor Yellow
                Write-Host ("-" * 52) -ForegroundColor DarkYellow
                $tableHeaderPrinted = $true
            }

            $raw = Read-FileTail -Path $TelemetryPath -Offset ([ref]$lastSizeTel)
            foreach ($tline in ($raw -split "`n")) {
                if ($tline -match '"event":"train') {
                    try {
                        $d    = $tline | ConvertFrom-Json
                        $step = if ($d.step) { $d.step } else { $d.global_step }
                        $epoch = $d.epoch
                        $loss  = "{0:N4}" -f [double]$d.loss
                        $tps   = if ($d.tokens_per_sec) { "{0:N1}" -f [double]$d.tokens_per_sec } else { "—" }
                        $eta   = if ($d.eta_sec) {
                            $s = [double]$d.eta_sec
                            if    ($s -ge 3600) { "{0:N1}h"  -f ($s / 3600) }
                            elseif($s -ge 60)   { "{0:N0}m"  -f ($s / 60)   }
                            else                 { "{0:N0}s"  -f $s          }
                        } else { "cal…" }

                        Write-Host ("{0,-8} {1,-6} {2,-12} {3,-10} {4}" -f $step, $epoch, $loss, $tps, $eta)
                    } catch {}
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
