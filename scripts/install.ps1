# Thin wrapper around `vox-bootstrap`:
# 1) Prefer local `cargo run -p vox-bootstrap` in a repo checkout (debuggable SSOT path).
# 2) Else use `vox-bootstrap` from PATH if present.
# 3) Else download a standalone `vox-bootstrap` release asset, verify checksum, execute it.
#
# Examples:
#   .\scripts\install.ps1
#   .\scripts\install.ps1 -Dev -InstallClang -Apply
#   .\scripts\install.ps1 -Install
#   .\scripts\install.ps1 -Install -Version v1.2.3
#   .\scripts\install.ps1 plan
#   .\scripts\install.ps1 -Dev plan --human
#Requires -Version 5.1
[CmdletBinding(PositionalBinding = $false)]
param(
    [switch] $Dev,
    [switch] $InstallClang,
    [switch] $Apply,
    [switch] $Install,
    [switch] $SourceOnly,
    [string] $Version,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $Remaining
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$ForceBinary = $env:VOX_USE_BOOTSTRAP_BINARY -eq '1'
$ApiLatest = 'https://api.github.com/repos/vox-foundation/vox/releases/latest'
$ReleaseBase = 'https://github.com/vox-foundation/vox/releases/download'

$pass = [System.Collections.Generic.List[string]]::new()
if ($Dev) { $pass.Add('--dev') }
if ($InstallClang) { $pass.Add('--install-clang') }
if ($Apply) { $pass.Add('--apply') }
if ($Install) { $pass.Add('--install') }
if ($SourceOnly) { $pass.Add('--source-only') }
if ($Version) {
    $pass.Add('--version')
    $pass.Add($Version)
}
foreach ($r in $Remaining) { $pass.Add($r) }
$arr = $pass.ToArray()

function Normalize-Tag([string]$Tag) {
    if (-not $Tag) { return $null }
    if ($Tag.StartsWith('v')) { return $Tag }
    return "v$Tag"
}

function Resolve-Tag([string]$TagFromArgs) {
    $n = Normalize-Tag $TagFromArgs
    if ($n) { return $n }
    $resp = Invoke-RestMethod -Uri $ApiLatest -Method Get
    if (-not $resp.tag_name) {
        throw 'GitHub latest release response missing tag_name'
    }
    return [string]$resp.tag_name
}

function Resolve-TargetTriple {
    if ([Environment]::Is64BitOperatingSystem) {
        return 'x86_64-pc-windows-msvc'
    }
    throw 'unsupported Windows architecture for standalone bootstrap binary'
}

function Verify-Checksum([string]$AssetPath, [string]$ChecksumsPath, [string]$AssetName) {
    $expected = Get-Content $ChecksumsPath |
        ForEach-Object {
            $line = $_.Trim()
            if (-not $line) { return $null }
            $parts = $line -split '\s+'
            if ($parts.Count -lt 2) { return $null }
            if ($parts[1] -eq $AssetName) { return $parts[0].ToLowerInvariant() }
            return $null
        } |
        Where-Object { $_ } |
        Select-Object -First 1
    if (-not $expected) {
        throw "checksum entry not found for $AssetName"
    }

    $actual = (Get-FileHash -Path $AssetPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "checksum mismatch for $AssetName (expected $expected, got $actual)"
    }
}

function Run-StandaloneBootstrap {
    $tag = Resolve-Tag -TagFromArgs $Version
    $triple = Resolve-TargetTriple
    $assetName = "vox-bootstrap-$tag-$triple.zip"
    $tempDir = Join-Path $env:TEMP ("vox-bootstrap-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    try {
        $assetPath = Join-Path $tempDir $assetName
        $checksumsPath = Join-Path $tempDir 'checksums.txt'
        $assetUrl = "$ReleaseBase/$tag/$assetName"
        $checksumsUrl = "$ReleaseBase/$tag/checksums.txt"

        Write-Host "  Downloading standalone bootstrap asset: $assetName"
        Invoke-WebRequest -Uri $assetUrl -OutFile $assetPath -UseBasicParsing
        Invoke-WebRequest -Uri $checksumsUrl -OutFile $checksumsPath -UseBasicParsing
        Verify-Checksum -AssetPath $assetPath -ChecksumsPath $checksumsPath -AssetName $assetName

        $extractDir = Join-Path $tempDir 'extract'
        Expand-Archive -Path $assetPath -DestinationPath $extractDir -Force
        $bootstrapExe = Join-Path $extractDir 'vox-bootstrap.exe'
        if (-not (Test-Path $bootstrapExe)) {
            throw "missing vox-bootstrap.exe in archive $assetName"
        }
        if ($arr.Count -eq 0) {
            & $bootstrapExe
        } else {
            & $bootstrapExe @arr
        }
        exit $LASTEXITCODE
    } finally {
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        }
    }
}

$RepoHasBootstrap = (Test-Path (Join-Path $Root 'Cargo.toml')) -and (Test-Path (Join-Path $Root 'crates\vox-bootstrap\Cargo.toml'))
$CargoCmd = Get-Command cargo -ErrorAction SilentlyContinue

if (-not $ForceBinary -and $RepoHasBootstrap -and $CargoCmd) {
    if ($arr.Count -eq 0) {
        & cargo run --locked -p vox-bootstrap
    } else {
        & cargo run --locked -p vox-bootstrap -- @arr
    }
    exit $LASTEXITCODE
}

$BootstrapCmd = Get-Command vox-bootstrap -ErrorAction SilentlyContinue
if (-not $ForceBinary -and $BootstrapCmd) {
    if ($arr.Count -eq 0) {
        & vox-bootstrap
    } else {
        & vox-bootstrap @arr
    }
    exit $LASTEXITCODE
}

Run-StandaloneBootstrap
