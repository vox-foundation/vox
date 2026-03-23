# Thin wrapper: ensure rustup, then run **`vox-bootstrap`** (Rust SSOT in `crates/vox-bootstrap`).
#
# Examples:
#   .\scripts\install.ps1
#   .\scripts\install.ps1 -Dev -InstallClang -Apply
#   .\scripts\install.ps1 plan
#   .\scripts\install.ps1 -Dev plan --human
#Requires -Version 5.1
[CmdletBinding()]
param(
    [switch] $Dev,
    [switch] $InstallClang,
    [switch] $Apply,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $Remaining
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$pass = [System.Collections.Generic.List[string]]::new()
if ($Dev) { $pass.Add('--dev') }
if ($InstallClang) { $pass.Add('--install-clang') }
if ($Apply) { $pass.Add('--apply') }
foreach ($r in $Remaining) { $pass.Add($r) }

function Ensure-Cargo {
    if (Get-Command cargo -ErrorAction SilentlyContinue) { return }
    $envPs1 = Join-Path $env:USERPROFILE '.cargo\env.ps1'
    if (Test-Path $envPs1) { . $envPs1 }
    if (Get-Command cargo -ErrorAction SilentlyContinue) { return }

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host '  Installing Rustup via winget …'
        winget install -e --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements
    } else {
        $init = Join-Path $env:TEMP 'rustup-init.exe'
        Write-Host '  Downloading rustup-init.exe …'
        Invoke-WebRequest -Uri 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $init -UseBasicParsing
        & $init -y
    }
    if (Test-Path $envPs1) { . $envPs1 }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'rustup installed but cargo not on PATH — open a new shell or add ~/.cargo/bin.'
    }
}

Ensure-Cargo
$arr = $pass.ToArray()
if ($arr.Count -eq 0) {
    & cargo run --locked -p vox-bootstrap
} else {
    & cargo run --locked -p vox-bootstrap -- @arr
}
exit $LASTEXITCODE
