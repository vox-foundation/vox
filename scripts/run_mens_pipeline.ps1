param(
    [string]$DataDir = "target/dogfood",
    [string]$OutputDir = "mens/runs/v1",
    [string]$Device = "vulkan",
    [switch]$SkipTrain,
    [switch]$StrictGate
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

$releaseVox = Join-Path $repoRoot "target/release/vox.exe"
$debugVox = Join-Path $repoRoot "target/debug/vox.exe"

if (Test-Path $releaseVox) {
    $vox = $releaseVox
} elseif (Test-Path $debugVox) {
    $vox = $debugVox
} else {
    throw "vox binary not found. Build first: cargo build -p vox-cli"
}

$args = @(
    "mens", "pipeline",
    "--data-dir", $DataDir,
    "--output-dir", $OutputDir,
    "--device", $Device
)
if ($SkipTrain) { $args += "--skip-train" }
if ($StrictGate) { $args += "--strict-gate" }

& $vox @args
