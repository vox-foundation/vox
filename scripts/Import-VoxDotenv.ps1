<#!
  Load repository-root .env into the **current process** (Cursor / PowerShell session).
  Usage:
    . "C:\Users\Owner\vox\scripts\Import-VoxDotenv.ps1"
  Optional root:
    . "...\Import-VoxDotenv.ps1" -Root "D:\other\vox\checkout"
#>
param(
    [string]$Root = ""
)

function Import-VoxDotenv {
    param(
        [string]$Root,
        [string]$FileName = ".env"
    )
    $scriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
    if (-not $Root) {
        $Root = (Resolve-Path (Join-Path $scriptDir "..")).Path
    }
    $path = Join-Path $Root $FileName
    if (-not (Test-Path $path)) {
        Write-Warning "Import-VoxDotenv: missing $path — copy .env.example to .env and set VOX_GITHUB_TOKEN."
        return
    }
    Get-Content -LiteralPath $path -ErrorAction Stop | ForEach-Object {
        $line = $_.TrimEnd()
        if ($line -match '^\s*#' -or $line -eq '') { return }
        $eq = $line.IndexOf('=')
        if ($eq -lt 1) { return }
        $k = $line.Substring(0, $eq).Trim()
        $v = $line.Substring($eq + 1).Trim()
        if ($v.Length -ge 2 -and $v.StartsWith('"') -and $v.EndsWith('"')) {
            $v = $v.Substring(1, $v.Length - 2)
        }
        if ($k -match '^[A-Za-z_][A-Za-z0-9_]*$') {
            [System.Environment]::SetEnvironmentVariable($k, $v, 'Process')
        }
    }
    Write-Host "Import-VoxDotenv: loaded $path (process scope only)." -ForegroundColor DarkGreen
}

$__importRoot = $Root
$scriptDirForBootstrap = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $__importRoot) {
    $__importRoot = (Resolve-Path (Join-Path $scriptDirForBootstrap "..")).Path
}
Import-VoxDotenv -Root $__importRoot
