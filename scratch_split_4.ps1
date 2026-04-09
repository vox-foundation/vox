$source = "c:\Users\Owner\vox\AI Agent Context and Handoff Research.md"
$content = Get-Content $source

Function Write-Chunk {
    param($title, $startLine, $endLine, $dest)
    $out = @()
    $out += "---"
    $out += "title: $title"
    $out += "---"
    $out += $content[$startLine..$endLine]
    $out += ""
    $out += "*(Original Source: AI Agent Context and Handoff Research)*"
    Set-Content -Path $dest -Value $out -Encoding UTF8
}

# 1. Compaction: Lines 13 to 42
Write-Chunk "Empirical Evidence for Context Compaction Strategies" 13 42 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-empirical-compaction-2026.md"

# 2. Bleed: Lines 44 to 65
Write-Chunk "Documented Failure Modes: Context Bleed and Session Identity Confusion" 44 65 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-context-bleed-2026.md"

# 3. Protocols: Lines 67 to 93
Write-Chunk "State of the Art for Context-Aware Agent Handoff Protocols" 67 93 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-sota-protocols-2026.md"

# 4. Retrieval Policies: Lines 95 to 125
Write-Chunk "Evidence Base for Context Retrieval Policies" 95 125 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-retrieval-policies-2026.md"

# 5. A2A sharing: Lines 127 to 146
Write-Chunk "Cross-Agent Evidence Sharing in A2A Protocol Implementations" 127 146 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-a2a-evidence-sharing-2026.md"

# 6. Truncation: Lines 148 to 168
Write-Chunk "Production Evidence: Context Truncation as a Silent Failure Mode" 148 168 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-truncation-failure-2026.md"

# 7. Catalog: Lines 170 to 181
Write-Chunk "Production Failure Mode Catalog with Mitigations" 170 181 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-failure-catalog-2026.md"

# 8. Design Patterns: Lines 182 to 201
Write-Chunk "Design Pattern Recommendations for Platform Gaps" 182 201 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-design-patterns-2026.md"

# 9. Checklist: Lines 203 to 213
Write-Chunk "Architecture Decision Checklist for Implementing Agent Handoff Continuity" 203 213 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-checklist-2026.md"

# Works cited and images: Lines 214 to 280
Write-Chunk "Works Cited: AI Agent Context and Handoff" 214 280 "c:\Users\Owner\vox\docs\src\architecture\research-agent-handoff-works-cited-2026.md"

# Delete the lossy summary
Remove-Item -Path "c:\Users\Owner\vox\docs\src\architecture\research-ai-agent-handoff-2026.md" -ErrorAction SilentlyContinue

Rename-Item -Path $source -NewName "AI_Agent_Context_and_Handoff_Research_processed_do_not_delete.md" -ErrorAction SilentlyContinue
