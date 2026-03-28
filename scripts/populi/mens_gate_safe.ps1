# Windows-safe `vox ci mesh-gate` (alias: `mens-gate`): delegates to Rust `--windows-isolated-runner` (isolated target dir + temp vox.exe).
# Use -Detach for Cursor / agent sessions with wall-clock limits (fires child pwsh, returns immediately).
# Logs: default under target/mens-gate-logs/ when -Detach without -LogFile (then passed as --gate-log-file).
[CmdletBinding()]
param(
  [Parameter(Mandatory = $false)]
  [ValidateSet("training", "ci_full", "m1m4")]
  [string]$Profile = "training",

  [string]$LogFile = "",

  [switch]$Detach
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")

if ($Detach) {
  if (-not $LogFile) {
    $logDir = Join-Path $repoRoot "target\mens-gate-logs"
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null
    $ts = Get-Date -Format "yyyyMMdd_HHmmss"
    $LogFile = Join-Path $logDir ("mens_gate_{0}_{1}.log" -f $Profile, $ts)
  }
  $here = $PSCommandPath
  $shell = $null
  if (Get-Command pwsh -ErrorAction SilentlyContinue) {
    $shell = (Get-Command pwsh).Source
  }
  elseif (Get-Command powershell -ErrorAction SilentlyContinue) {
    $shell = (Get-Command powershell).Source
  }
  else {
    throw "Neither pwsh nor powershell found on PATH"
  }
  $startArgs = @(
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", $here,
    "-Profile", $Profile,
    "-LogFile", $LogFile
  )
  Start-Process -FilePath $shell -ArgumentList $startArgs -WorkingDirectory $repoRoot | Out-Null
  Write-Host "Detached mens-gate (profile=$Profile). Tail log:"
  Write-Host "  Get-Content `"$LogFile`" -Wait -Tail 40"
  exit 0
}

Set-Location $repoRoot

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = "cargo" }

$voxDebug = Join-Path $repoRoot "target\debug\vox.exe"
$voxRelease = Join-Path $repoRoot "target\release\vox.exe"
$vox = $null
if (Test-Path $voxRelease) { $vox = $voxRelease }
elseif (Test-Path $voxDebug) { $vox = $voxDebug }
else {
  Write-Host "Building vox-cli (debug) for mens-gate…"
  & $cargo build -p vox-cli -q
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $vox = $voxDebug
}

$gateArgs = @("ci", "mens-gate", "--profile", $Profile, "--isolated-runner")
if ($LogFile) {
  $gateArgs += @("--gate-log-file", $LogFile)
}

& $vox @gateArgs
exit $LASTEXITCODE
