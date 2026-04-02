# Extract CommandAst names, bound parameter names, string literals, and parse errors as JSON.
# Input: environment variable VOX_SHELL_CHECK_PAYLOAD (full PowerShell source to analyze).
# Output: one JSON object on stdout; stderr for fatal wrapper errors only.
$ErrorActionPreference = 'Stop'
$raw = $env:VOX_SHELL_CHECK_PAYLOAD
if (-not $raw) {
    Write-Error 'VOX_SHELL_CHECK_PAYLOAD is not set.'
    exit 2
}

$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($raw, [ref]$tokens, [ref]$errors)

$parseErrors = @(
    foreach ($e in $errors) {
        @{
            message = $e.Message
            text    = $e.Extent.Text
        }
    }
)

$cmdAsts = $ast.FindAll({ $args[0] -is [System.Management.Automation.Language.CommandAst] }, $true)
$commands = @(
    foreach ($c in $cmdAsts) {
        $name = $c.GetCommandName()
        $params = [System.Collections.Generic.List[string]]::new()
        foreach ($el in $c.CommandElements) {
            if ($el -is [System.Management.Automation.Language.CommandParameterAst]) {
                [void]$params.Add($el.ParameterName)
            }
        }
        @{
            name        = $name
            parameters  = @($params)
        }
    }
)

$stringNodes = $ast.FindAll({ $args[0] -is [System.Management.Automation.Language.StringConstantExpressionAst] }, $true)
$stringLiterals = [System.Collections.Generic.List[string]]::new()
foreach ($s in $stringNodes) {
    if ($null -ne $s.Value -and $s.Value -ne '') {
        [void]$stringLiterals.Add([string]$s.Value)
    }
}

# Expandable strings ("...$var...") — include constant template and full source extent
# so URL allowlisting sees literals inside double-quoted strings with subexpressions.
$expandableNodes = $ast.FindAll({ $args[0] -is [System.Management.Automation.Language.ExpandableStringExpressionAst] }, $true)
foreach ($ex in $expandableNodes) {
    if ($null -ne $ex.Value -and $ex.Value -ne '') {
        [void]$stringLiterals.Add([string]$ex.Value)
    }
    $extText = $ex.Extent.Text
    if ($null -ne $extText -and $extText -ne '' -and $extText -ne $ex.Value) {
        [void]$stringLiterals.Add($extText)
    }
}

$out = @{
    parse_errors    = $parseErrors
    commands        = $commands
    string_literals = @($stringLiterals)
}
$out | ConvertTo-Json -Depth 10 -Compress
