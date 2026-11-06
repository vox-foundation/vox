//! False Tool-Refusal Detector & 1-Retry Recovery (`T3-GUARD-REFUSAL-03`).
//!
//! Adapted from `vibheksoni/pi-t3chat` (`tools.ts` `detectsToolRefusal` lines 239–249):
//! Hosted chat models occasionally hallucinate a refusal ("I don't have access to your local
//! filesystem or bash tools") even when tool definitions are present in the turn payload.
//! When `active_tool_count > 0 && tool_calls_emitted == 0 && !already_retried_refusal`,
//! `should_retry_false_tool_refusal` checks the first `350` characters for a co-occurring
//! refusal phrase + capability noun and triggers a single corrective retry.

/// Maximum character prefix inspected for false tool-refusal phrases.
pub const REFUSAL_PREFIX_CHAR_LIMIT: usize = 350;

/// Maximum corrective retries per turn when a model hallucinates a tool-capability refusal (`pi-t3chat` `maxCorrections = 2`).
pub const MAX_REFUSAL_CORRECTIONS: usize = 2;

/// Corrective reminder injected when a false tool refusal is detected.
pub const FALSE_TOOL_REFUSAL_RECOVERY_REMINDER: &str = "[SYSTEM TOOL CORRECTION] [HARNESS TOOL CAPABILITY REMINDER]: You DO have active workspace, file, search, and shell tools in this session. Do NOT claim you lack access to local files, terminal, or tools. Invoke the appropriate tool from your active tool list now.";

/// Build a `[SYSTEM TOOL CORRECTION]` prompt listing up to 16 active tool names (`T3-PROMPT-LOOP-03`).
#[must_use]
pub fn build_tool_refusal_recovery_reminder(tool_names: &[&str]) -> String {
    if tool_names.is_empty() {
        return FALSE_TOOL_REFUSAL_RECOVERY_REMINDER.to_string();
    }
    let preview: Vec<&str> = tool_names.iter().copied().take(16).collect();
    format!(
        "{FALSE_TOOL_REFUSAL_RECOVERY_REMINDER} Available tools include: {}.",
        preview.join(", ")
    )
}

/// Returns `true` if `tool_name` is a search/retrieval tool subject to the per-turn search circuit breaker
/// (`NotT3Chat` `totalSearches >= 3`, `T3-PROMPT-LOOP-03`).
#[must_use]
pub fn is_search_circuit_breaker_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "vox_search_query"
            | "vox_kb_search"
            | "vox_memory_search"
            | "vox_symbol_search"
            | "vox_doc_search"
            | "web_search"
            | "search_web"
    )
}

const REFUSAL_PHRASES: &[&str] = &[
    "i don't have access",
    "i do not have access",
    "i cannot access",
    "i can't access",
    "i'm unable to access",
    "i am unable to access",
    "i don't have the ability",
    "i do not have the ability",
    "no access to your",
    "cannot run commands",
    "can't run commands",
    "cannot read local files",
    "can't read local files",
];

const CAPABILITY_NOUNS: &[&str] = &[
    "filesystem",
    "file system",
    "local files",
    "terminal",
    "shell",
    "bash",
    "workspace",
    "repository",
    "codebase",
    "tools",
    "commands",
    "directory",
];

/// Returns `true` if `assistant_text` begins with a false tool-refusal claim while tools are active
/// and no tool calls were emitted on the turn.
#[must_use]
pub fn should_retry_false_tool_refusal(
    assistant_text: &str,
    active_tool_count: usize,
    tool_calls_emitted: usize,
    already_retried_refusal: bool,
) -> bool {
    if active_tool_count == 0 || tool_calls_emitted > 0 || already_retried_refusal {
        return false;
    }
    let head: String = assistant_text
        .chars()
        .take(REFUSAL_PREFIX_CHAR_LIMIT)
        .collect::<String>()
        .to_ascii_lowercase();
    let has_refusal = REFUSAL_PHRASES.iter().any(|p| head.contains(p));
    let has_capability = CAPABILITY_NOUNS.iter().any(|c| head.contains(c));
    has_refusal && has_capability
}

#[cfg(test)]
mod tests {
    use super::should_retry_false_tool_refusal;

    #[test]
    fn detects_false_tool_refusal_only_when_tools_active_and_not_retried() {
        let refusal = "I apologize, but I don't have access to your local filesystem or bash terminal to inspect that file.";
        assert!(should_retry_false_tool_refusal(refusal, 12, 0, false));
        // Already retried once -> false
        assert!(!should_retry_false_tool_refusal(refusal, 12, 0, true));
        // No tools active -> false
        assert!(!should_retry_false_tool_refusal(refusal, 0, 0, false));
        // Tool calls already emitted -> false
        assert!(!should_retry_false_tool_refusal(refusal, 12, 1, false));
        // Normal architectural answer -> false
        assert!(!should_retry_false_tool_refusal(
            "Here is the implementation of VoxIdAllocatorV2:",
            12,
            0,
            false
        ));
    }
}
