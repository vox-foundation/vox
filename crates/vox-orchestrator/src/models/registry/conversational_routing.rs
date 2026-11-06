use super::*;

fn record_signal(
    scores: &mut [i32; 8],
    matched_signals: &mut Vec<String>,
    spoke: vox_corpus::training::contract::OrganicSpokeKind,
    pts: i32,
    signal: &str,
) {
    use vox_corpus::training::contract::OrganicSpokeKind;
    let idx = OrganicSpokeKind::ALL
        .iter()
        .position(|&s| s == spoke)
        .unwrap_or(0);
    scores[idx] += pts;
    matched_signals.push(format!("{}+{pts}({signal})", spoke.as_str()));
}

impl ModelRegistry {
    /// Score a user utterance across all 8 [`vox_corpus::training::contract::OrganicSpokeKind`]
    /// spokes, taking into account optional `prior_spoke` conversational state for anaphora,
    /// follow-up continuations ("in that file", "what we just built"), and self-repair requests
    /// ("wait, that gave a borrow error—can you fix it?").
    #[must_use]
    pub fn score_conversational_turn_across_spokes(
        prompt: &str,
        prior_spoke: Option<vox_corpus::training::contract::OrganicSpokeKind>,
    ) -> (
        vox_corpus::training::contract::OrganicSpokeKind,
        i32,
        Option<vox_corpus::training::contract::OrganicSpokeKind>,
        i32,
        Vec<String>,
        ConversationalTurnMode,
        bool,
    ) {
        use vox_corpus::training::contract::OrganicSpokeKind;

        let lower = prompt.to_ascii_lowercase();
        let mut scores = [0i32; 8];
        let mut matched_signals: Vec<String> = Vec::new();

        // 1. Primary deliverable action / everyday human intent phrases (+5 points)
        for phrase in [
            "um ",
            "uh ",
            "er ",
            "transcribe voice",
            "spoken voice",
            "dictating this",
            "dictated this",
            "on my phone",
            "voice note",
        ] {
            if (phrase.ends_with(' ') && lower.starts_with(phrase))
                || (!phrase.ends_with(' ') && lower.contains(phrase))
                || lower.contains(": um ")
                || lower.contains(" um define ")
            {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::SpeechToCode,
                    5,
                    phrase.trim(),
                );
                break;
            }
        }
        for phrase in [
            "write a .vox",
            "implement .vox",
            "in .vox",
            "in vox",
            ".vox actor",
            "bare-keyword",
            "in that .vox file",
            "@table",
            "ret n + 1",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::VoxLang,
                    5,
                    phrase,
                );
            }
        }
        for phrase in [
            "implement rust",
            "in rust",
            "implement `pub fn",
            "implement pub fn",
            "rust companion",
            "rust helper",
            "rust function",
            "in that rust module",
            "borrow error",
            "items.push",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::RustWorkspace,
                    5,
                    phrase,
                );
            }
        }
        for phrase in [
            "invoke mcp",
            "call mcp",
            "emit mcp",
            "mcp tool call",
            "check the knowledge base",
            "search the knowledge base",
            "check the docs",
            "search the docs",
            "compiler check tool",
            "vox_kb_store",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::McpToolChain,
                    5,
                    phrase,
                );
            }
        }
        for phrase in [
            "author a skill.md",
            "author skill.md",
            "agentskills specification",
            "compliant skill.md",
            "reusable skill",
            "save what we just built as a reusable skill",
            "turn what we just built into a skill",
            "so the rest of the team and other agents can use",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::SkillCreation,
                    5,
                    phrase,
                );
            }
        }
        for phrase in [
            "emit pwsh",
            "emit a structured powershell",
            "powershell pipeline",
            "terminal command",
            "what example .vox files are in",
            "what files are in",
            "list the files in",
            "how big they are",
            "check file sizes",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::StructuredShell,
                    5,
                    phrase,
                );
            }
        }
        for phrase in [
            "decompose goal",
            "emit a `vox-dei` taskplan",
            "taskplanenvelope",
            "help me plan out",
            "help me plan",
            "plan out the steps",
            "break this down into steps",
            "what steps should we take",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::AgentHarness,
                    5,
                    phrase,
                );
            }
        }
        for phrase in [
            "dispatch verified",
            "populimeshdispatchenvelope",
            "across populi mesh",
            "gpu worker nodes",
            "across 3 gpu",
            "gpu cluster",
            "24gb vram",
        ] {
            if lower.contains(phrase) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::PopuliMesh,
                    5,
                    phrase,
                );
            }
        }

        // 2. Domain-specific token evidence (+2 points each)
        for kw in [
            "asr",
            "voice",
            "spoken",
            "speech",
            "dictate",
            "walking to get coffee",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::SpeechToCode,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "populi",
            "mesh",
            "node-lease",
            "replica",
            "federat",
            "iroh",
            "cluster",
            "worker nodes",
            "a2a",
            "apple silicon",
            "mac studio",
            "quic",
            "vram",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::PopuliMesh,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "skill.md",
            "agentskills",
            "skill_creation",
            "vox-category",
            "author a skill",
            "into a skill",
            "other agents can use",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::SkillCreation,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "pwsh",
            "powershell",
            "exec-policy",
            "shell command",
            "get-childitem",
            "convertto-json",
            "examples/golden",
            "rg ",
            "git status",
            "git diff",
            "nushell",
            "to json",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::StructuredShell,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "mcp",
            "tool call",
            "vox_compiler_check",
            "vox_repo_status",
            "vox_skill_search",
            "vox_kb_search",
            "knowledge base",
            "@island",
            "retired tool",
            "callgraph",
            "find references",
            "symbol search",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::McpToolChain,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "harness plan",
            "write_manifest",
            "workflow plan",
            "verification_gate",
            "budget_tokens",
            "roadmap",
            "steps first",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::AgentHarness,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "rust ",
            "rustc",
            "borrow checker",
            "pub fn ",
            "crate ",
            "&items[0]",
            "slice of numbers",
            "window size",
            "cargo.toml",
            "lifetime",
            "trait ",
            "impl ",
            "serde",
            "async fn",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::RustWorkspace,
                    2,
                    kw,
                );
            }
        }
        for kw in [
            "actor ",
            "workflow ",
            "table ",
            "activity ",
            ".vox",
            "component ",
            "server ",
            "query ",
            "mutation ",
            "state_machine ",
            "routes ",
        ] {
            if lower.contains(kw) {
                record_signal(
                    &mut scores,
                    &mut matched_signals,
                    OrganicSpokeKind::VoxLang,
                    2,
                    kw,
                );
            }
        }

        // 3. Multi-turn conversational repair & context carryover detection
        let is_repair_turn = [
            "can you fix",
            "fix it",
            "fix that",
            "syntax error",
            "borrow error",
            "failed as a retired tool",
            "didn't compile",
            "won't compile",
            "why did that fail",
        ]
        .iter()
        .any(|cue| lower.contains(cue));

        let references_prior_context = [
            "in that .vox file",
            "that file",
            "that rust module",
            "that function",
            "that tool call",
            "that note",
            "that actor we wrote earlier",
            "what we just built",
            "wait, i tried",
            "wait, when i also wrote",
            "hold on,",
        ]
        .iter()
        .any(|cue| lower.contains(cue));

        let mut used_prior_turn_context = false;
        if let Some(prev) = prior_spoke {
            if references_prior_context || is_repair_turn {
                used_prior_turn_context = true;
                // If the turn is an elliptic repair/follow-up on the active spoke, boost prior spoke
                let max_explicit = scores.iter().copied().max().unwrap_or(0);
                if max_explicit == 0 {
                    record_signal(
                        &mut scores,
                        &mut matched_signals,
                        prev,
                        4,
                        "prior_turn_carryover",
                    );
                } else if is_repair_turn {
                    record_signal(
                        &mut scores,
                        &mut matched_signals,
                        prev,
                        2,
                        "prior_turn_repair_affinity",
                    );
                }
            }
        }

        let turn_mode = if is_repair_turn {
            ConversationalTurnMode::SelfRepairLoop
        } else if used_prior_turn_context {
            ConversationalTurnMode::ContextualContinuation
        } else {
            ConversationalTurnMode::FreshIntent
        };

        let mut best_spoke = prior_spoke.unwrap_or(OrganicSpokeKind::VoxLang);
        let mut best_score = 0i32;
        let mut runner_up_spoke = None;
        let mut runner_up_score = 0i32;

        for (idx, &score) in scores.iter().enumerate() {
            let candidate = OrganicSpokeKind::ALL[idx];
            if score > best_score {
                if best_score > 0 {
                    runner_up_spoke = Some(best_spoke);
                    runner_up_score = best_score;
                }
                best_score = score;
                best_spoke = candidate;
            } else if score > runner_up_score && score > 0 && candidate != best_spoke {
                runner_up_score = score;
                runner_up_spoke = Some(candidate);
            }
        }

        (
            best_spoke,
            best_score,
            runner_up_spoke,
            runner_up_score,
            matched_signals,
            turn_mode,
            used_prior_turn_context,
        )
    }

    /// Pre-send spoke-aware prompt enhancer (`thom-chat` `/api/enhance-prompt` + `@<rule_name>` preservation, `VOX-HARNESS-T3-03`).
    #[must_use]
    pub fn enhance_user_prompt_for_spoke(
        raw_prompt: &str,
        prior_spoke: Option<vox_corpus::training::contract::OrganicSpokeKind>,
    ) -> super::spec::EnhancedPromptReceipt {
        use vox_corpus::training::contract::OrganicSpokeKind;

        let mut preserved_rule_tags = Vec::new();
        for token in raw_prompt.split_whitespace() {
            let trimmed = token.trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '@' && c != '_' && c != '-' && c != '.' && c != '/'
            });
            if trimmed.starts_with('@')
                && trimmed.len() > 1
                && !preserved_rule_tags.contains(&trimmed.to_string())
            {
                preserved_rule_tags.push(trimmed.to_string());
            }
        }

        let (spoke, _score, _runner_up, _runner_score, _signals, _mode, _carryover) =
            Self::score_conversational_turn_across_spokes(raw_prompt, prior_spoke);

        let cleaned: String = raw_prompt
            .split_whitespace()
            .filter(|w| {
                let lw = w.to_ascii_lowercase();
                lw != "um" && lw != "uh" && lw != "er"
            })
            .collect::<Vec<_>>()
            .join(" ");

        let spoke_directive = match spoke {
            OrganicSpokeKind::VoxLang => {
                "[Spoke: VoxLang — bare-keyword blocks (`actor`, `workflow`, `component`), `ret` returns]"
            }
            OrganicSpokeKind::RustWorkspace => {
                "[Spoke: RustWorkspace — idiomatic Rust 2024, explicit result/borrow safety]"
            }
            OrganicSpokeKind::McpToolChain => {
                "[Spoke: McpToolChain — invoke canonical `vox_*` MCP tool]"
            }
            OrganicSpokeKind::SkillCreation => {
                "[Spoke: SkillCreation — agentskills.io compliant `SKILL.md`]"
            }
            OrganicSpokeKind::StructuredShell => {
                "[Spoke: StructuredShell — single-step PowerShell (`pwsh`) command]"
            }
            OrganicSpokeKind::AgentHarness => {
                "[Spoke: AgentHarness — structured `TaskPlanEnvelope` decomposition]"
            }
            OrganicSpokeKind::PopuliMesh => {
                "[Spoke: PopuliMesh — distributed `PopuliMeshDispatchEnvelope`]"
            }
            OrganicSpokeKind::SpeechToCode => {
                "[Spoke: SpeechToCode — disfluency-normalized code intent]"
            }
        };

        let mut enhanced_prompt = format!("{spoke_directive} {cleaned}");
        for tag in &preserved_rule_tags {
            if !enhanced_prompt.contains(tag) {
                enhanced_prompt.push(' ');
                enhanced_prompt.push_str(tag);
            }
        }

        super::spec::EnhancedPromptReceipt {
            original_prompt: raw_prompt.to_string(),
            enhanced_prompt,
            preserved_rule_tags,
            target_spoke: spoke.as_str().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_corpus::training::contract::OrganicSpokeKind;

    const MCP_PROMPT: &str = "Invoke MCP tool call vox_compiler_check on a Rust crate";

    #[test]
    fn scoring_picks_the_mcp_spoke_with_positive_evidence() {
        let (spoke, score, _runner_up, _runner_score, signals, _mode, used_prior) =
            ModelRegistry::score_conversational_turn_across_spokes(MCP_PROMPT, None);
        assert_eq!(spoke, OrganicSpokeKind::McpToolChain);
        assert!(score > 0);
        assert!(!signals.is_empty());
        assert!(!used_prior);
    }

    #[test]
    fn enhancer_prefixes_spoke_directive_drops_fillers_and_keeps_rule_tags() {
        let raw = format!("um {MCP_PROMPT} @my_rule");
        let receipt = ModelRegistry::enhance_user_prompt_for_spoke(&raw, None);
        assert_eq!(receipt.original_prompt, raw);
        assert_eq!(
            receipt.target_spoke,
            OrganicSpokeKind::McpToolChain.as_str()
        );
        assert!(receipt.enhanced_prompt.starts_with("[Spoke: McpToolChain"));
        assert!(!receipt.enhanced_prompt.contains(" um "));
        assert_eq!(receipt.preserved_rule_tags, vec!["@my_rule".to_string()]);
        assert!(receipt.enhanced_prompt.contains("@my_rule"));
    }
}
