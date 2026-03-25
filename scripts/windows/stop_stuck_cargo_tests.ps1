#Requires -Version 5.1
<#
.SYNOPSIS
    Stop cargo-driven unit test runs that are still attached to this workspace.

.DESCRIPTION
    Finds processes whose command line references this repo's target\deps test
    binaries or "cargo.exe" ... "test" while running from the vox workspace.
    Use when a test hangs and blocks rebuilds (LNK1104) or leaves orphans.

.PARAMETER WhatIf
    List matching PIDs without stopping them.
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param()

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$targetDeps = [regex]::Escape((Join-Path $root 'target\debug\deps')) + '|' + [regex]::Escape((Join-Path $root 'target\release\deps'))

$procs = Get-CimInstance Win32_Process |
    Where-Object {
        $cl = $_.CommandLine
        if (-not $cl) { return $false }
        if ($cl -match $targetDeps) { return $true }
        if ($_.Name -ieq 'cargo.exe' -and $cl -match '(^|\s)test(\s|$)' -and $cl -match [regex]::Escape($root)) { return $true }
        return $false
    }

if (-not $procs) {
    Write-Host 'No matching cargo test / workspace test-binary processes found.'
    exit 0
}

foreach ($p in $procs) {
    $line = $p.CommandLine
    if ($PSCmdlet.ShouldProcess("PID $($p.ProcessId)", 'Stop-Process')) {
        Stop-Process -Id $p.ProcessId -Force
        Write-Host "Stopped PID $($p.ProcessId)"
    }
    else {
        Write-Host "Would stop PID $($p.ProcessId): $line"
    }
}
