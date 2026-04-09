# Deprecated: use run_qwen35_qlora_real_4080.ps1 (Qwen 3.5 SSOT; old name referred to legacy paths only).
param(
    [switch]$SkipBuild,
    [switch]$CpuOnlyBuild
)
Write-Warning "run_qwen25_qlora_real_4080.ps1 is deprecated; use scripts/run_qwen35_qlora_real_4080.ps1"
& "$PSScriptRoot\run_qwen35_qlora_real_4080.ps1" @PSBoundParameters
exit $LASTEXITCODE
