# Workspace bootstrap for PSReadLine command validation against Vox exec-policy (optional).
# VS Code: set terminal profile to pwsh -File this script (see .vscode/settings.json).

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$env:VOX_REPO_ROOT = $RepoRoot

function Invoke-VoxExecPolicyCheck {
    param([string]$Line)
    if ([string]::IsNullOrWhiteSpace($Line)) {
        return $true
    }
    Push-Location $RepoRoot
    try {
        $policy = Join-Path $RepoRoot 'contracts/terminal/exec-policy.v1.yaml'
        & vox shell check --payload $Line --policy $policy
        return $LASTEXITCODE -eq 0
    }
    finally {
        Pop-Location
    }
}

if ($Host.Name -eq 'ConsoleHost') {
    $pr = Get-Module -ListAvailable PSReadLine
    if ($pr) {
        Import-Module PSReadLine -ErrorAction SilentlyContinue
    }
    if (Get-Command Set-PSReadLineOption -ErrorAction SilentlyContinue) {
        Set-PSReadLineOption -CommandValidationHandler {
            param([System.Management.Automation.Language.CommandAst]$CommandAst)
            $line = $CommandAst.Extent.Text
            if (-not (Invoke-VoxExecPolicyCheck -Line $line)) {
                throw "Vox exec-policy rejected: $line"
            }
        }
    }
}
