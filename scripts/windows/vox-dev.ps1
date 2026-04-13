#Requires -Version 5.1
<#
.SYNOPSIS
  Run `vox` from the workspace clone: default `cargo run -p vox-cli`, or PATH `vox` when VOX_USE_PATH=1.

.DESCRIPTION
  Resolves the repo root (VOX_REPO_ROOT, then walk from cwd, then from this script's ../../).
  Forwards all arguments to vox. Cargo incremental build decides whether recompilation is needed.

  Env:
    VOX_REPO_ROOT     - Force workspace root (root Cargo.toml must contain [workspace]).
    VOX_USE_PATH=1    - Use `vox` on PATH when available (may be stale vs this clone).
    VOX_DEV_FEATURES  - Comma-separated extra features for vox-cli (overrides coderabbit auto-detect).
    VOX_DEV_QUIET=1   - Pass --quiet to cargo run.

  Auto: if argv contains the token `coderabbit` and VOX_DEV_FEATURES is unset, adds --features coderabbit.
#>
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$VoxArgs
)
$ErrorActionPreference = 'Stop'
$va = @($VoxArgs)

function Test-WorkspaceToml {
    param([string]$TomlPath)
    if (-not (Test-Path -LiteralPath $TomlPath)) { return $false }
    return $null -ne (Select-String -LiteralPath $TomlPath -Pattern '^\[workspace\]' | Select-Object -First 1)
}

function Get-VoxRepoRoot {
    if ($env:VOX_REPO_ROOT) {
        $r = $env:VOX_REPO_ROOT.TrimEnd('\', '/')
        $toml = Join-Path $r 'Cargo.toml'
        if (Test-WorkspaceToml $toml) { return (Resolve-Path -LiteralPath $r).Path }
        throw "VOX_REPO_ROOT is set but is not a Cargo workspace root: $r"
    }

    $starts = New-Object System.Collections.Generic.List[string]
    $starts.Add((Get-Location).Path) | Out-Null
    $scriptParent = Split-Path -Parent $PSScriptRoot
    $fromScript = Join-Path $scriptParent '..'
    if (Test-Path -LiteralPath $fromScript) {
        $starts.Add((Resolve-Path -LiteralPath $fromScript).Path) | Out-Null
    }

    foreach ($start in $starts) {
        $dir = $start
        while ($dir) {
            $toml = Join-Path $dir 'Cargo.toml'
            if (Test-WorkspaceToml $toml) { return $dir }
            $parent = Split-Path -Parent $dir
            if ($parent -eq $dir) { break }
            $dir = $parent
        }
    }
    throw 'Could not find workspace Cargo.toml with [workspace]. Set VOX_REPO_ROOT or cd into the repo.'
}

function Get-CargoExe {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        return 'cargo'
    }
    $c = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (Test-Path -LiteralPath $c) { return $c }
    throw 'cargo not found on PATH and not at ~/.cargo/bin/cargo.exe'
}

$root = Get-VoxRepoRoot

if ((Test-Path -LiteralPath (Join-Path $root '.git')) -and (-not (Test-Path -LiteralPath (Join-Path $root '.git\hooks\pre-commit')))) {
    Push-Location $root
    try {
        & (Get-CargoExe) run -q -p vox-cli -- ci install-hooks
    }
    finally {
        Pop-Location
    }
}

if ($env:VOX_USE_PATH -eq '1') {
    $voxCmd = Get-Command vox -ErrorAction SilentlyContinue
    if ($null -ne $voxCmd) {
        Push-Location $root
        try {
            & vox @va
            exit $LASTEXITCODE
        }
        finally {
            Pop-Location
        }
    }
}

$featureArg = @()
if ($env:VOX_DEV_FEATURES) {
    $f = $env:VOX_DEV_FEATURES.Trim()
    if ($f.Length -gt 0) {
        $featureArg = @('--features', $f)
    }
}
elseif ($va -contains 'coderabbit') {
    $featureArg = @('--features', 'coderabbit')
}

$cargoArgs = @('run', '-p', 'vox-cli')
if ($env:VOX_DEV_QUIET -eq '1') { $cargoArgs += '--quiet' }
$cargoArgs += $featureArg
$cargoArgs += '--'
$cargoArgs += $va

$cargoExe = Get-CargoExe
Push-Location $root
try {
    & $cargoExe @cargoArgs
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
