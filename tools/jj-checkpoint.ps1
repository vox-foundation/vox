<#
.SYNOPSIS
    Snapshot the current working-copy state in JJ with a timestamped or custom message.

.DESCRIPTION
    Calls `jj describe` to name the mutable working-copy commit.
    JJ auto-snapshots all working-tree changes — no explicit `add` or `commit` required.
    Safe to call repeatedly; each call just renames the working-copy commit.

.PARAMETER Message
    Human-readable description for the checkpoint.
    Defaults to "ai-checkpoint <ISO timestamp>".

.PARAMETER ShowLog
    If set, prints the last 5 JJ log entries after checkpointing.

.EXAMPLE
    .\tools\jj-checkpoint.ps1
    .\tools\jj-checkpoint.ps1 -Message "before orchestrator refactor"
    .\tools\jj-checkpoint.ps1 -Message "post-session snapshot 2026-03-24" -ShowLog
#>

param(
    [string]$Message = "ai-checkpoint $(Get-Date -Format 'yyyy-MM-ddTHH:mm')",
    [switch]$ShowLog
)

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot

try {
    Write-Host "[jj] Snapshotting working copy..." -ForegroundColor Cyan
    jj describe --message $Message
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[jj] describe failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
    Write-Host "[jj] Checkpoint set: $Message" -ForegroundColor Green

    if ($ShowLog) {
        Write-Host ""
        Write-Host "[jj] Recent history:" -ForegroundColor Cyan
        jj log --no-graph -n 6
    }
} finally {
    Pop-Location
}
