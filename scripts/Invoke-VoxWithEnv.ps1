<#!
  Load repo .env into this process, cd to Vox root, run a command (cargo, vox, etc.).
  Example:
    .\scripts\Invoke-VoxWithEnv.ps1 cargo check -p vox-cli
#>
param(
    [Parameter(Mandatory = $true, Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$Command
)
$ErrorActionPreference = 'Stop'
if ($Command.Count -eq 0) {
    throw 'Pass a command after the script, e.g. .\Invoke-VoxWithEnv.ps1 cargo check -p vox-cli'
}

. (Join-Path $PSScriptRoot 'Import-VoxDotenv.ps1')

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
Push-Location $root
try {
    & $Command[0] @($Command[1..($Command.Count - 1)])
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
