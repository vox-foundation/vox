//! Skill disclosure for the chat system prompt, following the agentskills.io
//! three-tier progressive-disclosure model:
//!
//! * **Tier 1** ([`render_skill_catalog`]): name + description only (~100
//!   tokens/skill), injected for every chat turn so the model knows which
//!   skills exist and when each applies. Works for prompt-only models (MENS)
//!   that cannot call tools.
//! * **Tier 2** (the `vox_skill_use` tool, separate): full SKILL.md body loaded
//!   on demand by tool-calling models.
//! * **Pinned** ([`render_pinned_skill`]): when the user explicitly selects a
//!   skill, its full body is injected directly — no tool round-trip — so even
//!   the prompt-only MENS path honors it.
//!
//! Both renderers are pure and content-stable (fixed sort order, fixed caps)
//! so they never bust the DeepSeek/Anthropic prompt-prefix cache that
//! `build_system_prompt` is careful to preserve. Task 3.1: the tier-1 sort
//! ranks by `reliability_scores` (`entity_type = 'skill'`) descending when
//! present, alphabetical otherwise/as tiebreak — never HashMap iteration
//! order — so it stays deterministic given the same input.

/// A tier-1 catalog entry: just the spec-required name + description, plus an
/// optional reliability score (`reliability_scores` `entity_type = 'skill'`).
/// `None` means "no observations recorded yet" — this must NOT be treated as
/// a low/zero score; unproven skills rank after scored ones, not below them.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    pub reliability: Option<f64>,
}

/// Hard upper bound on the ChatARC lexical shortlist size `K` (`CAT-01`).
pub const MAX_CHATARC_SHORTLIST_K: usize = 10;

/// Explicit abstention option always included in `(shortlist ∪ {none})` (`CAT-02`).
pub const CHATARC_NONE_CHOICE_ID: &str = "none";

/// Source family for a composed ChatARC shortlist item (`CAT-01`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ChatArcActionKind {
    Skill,
    McpTool,
    CliCommand,
}

/// One candidate composed on the fly from existing discovery sources (`CAT-01`).
#[derive(Debug, Clone, PartialEq)]
pub struct ChatArcCandidate {
    /// Canonical shortlist identifier (`skill:<name>`, `mcp:<tool>`, or `cli.<path>`).
    pub id: String,
    pub kind: ChatArcActionKind,
    pub display_name: String,
    pub description: String,
    pub lexical_score: usize,
    /// Optional skill reliability prior (`None` = unproven, never treated as 0.0).
    pub reliability_prior: Option<f64>,
    /// Static MCP tool name when `kind == ChatArcActionKind::McpTool`, used to hoist
    /// directly into [`crate::llm_bridge::tool_selection::TurnContext::pin_names`] (`CAT-02`).
    pub static_mcp_tool_name: Option<&'static str>,
    /// Canonical CLI invocation (`vox ...`) when `kind == ChatArcActionKind::CliCommand`.
    pub cli_invocation: Option<String>,
}

/// Dynamic top-`K` (`K <= 10`) ChatARC shortlist composed from existing skill, MCP tool,
/// and CLI command catalog sources (`CAT-01`).
#[derive(Debug, Clone, PartialEq)]
pub struct ChatArcShortlist {
    pub candidates: Vec<ChatArcCandidate>,
    pub includes_none_option: bool,
}

impl ChatArcShortlist {
    /// Option IDs presented to the calibrated selector: `(shortlist ∪ {none})` (`CAT-01`, `CAT-02`).
    #[must_use]
    pub fn option_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.candidates.iter().map(|c| c.id.clone()).collect();
        if self.includes_none_option {
            ids.push(CHATARC_NONE_CHOICE_ID.to_string());
        }
        ids
    }

    /// True when the active shortlist contains an existing PR tool or CLI command (`DEC-02`).
    #[must_use]
    pub fn has_pr_tool(&self) -> bool {
        fn tokens(s: &str) -> Vec<String> {
            s.split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|t| !t.is_empty())
                .map(str::to_ascii_lowercase)
                .collect()
        }
        // Token-based (not substring): a PR tool names `pr`/`pull request` as a whole token
        // alongside a creating verb, so `improve_prompt` or `sprint` never match.
        self.candidates.iter().any(|c| {
            if c.kind == ChatArcActionKind::Skill {
                return false;
            }
            let mut toks = tokens(&c.id);
            toks.extend(tokens(&c.display_name));
            let has = |w: &str| toks.iter().any(|t| t == w);
            let pr_noun = has("pr") || (has("pull") && has("request"));
            pr_noun
                && (has("create")
                    || has("open")
                    || has("draft")
                    || has("branch")
                    || has("ship")
                    || has("submit"))
        })
    }
}

/// Calibrated choice over `(shortlist ∪ {none})` (`CAT-02`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChatArcChoice {
    pub chosen_id: Option<String>,
    pub confidence: f64,
    pub threshold: f64,
}

/// Reason why ChatARC fell open to today's lexical/deferred behavior (`CAT-02`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatArcFailOpenReason {
    DisabledOrOutage,
    SelectedNone,
    BelowConfidenceThreshold { confidence: f64, threshold: f64 },
    CandidateNotInShortlist { chosen_id: String },
}

/// Outcome of applying a calibrated [`ChatArcChoice`] to a chat turn (`CAT-02`).
#[derive(Debug, Clone, PartialEq)]
pub enum ChatArcSelectionOutcome {
    ActivatedSkill {
        skill_name: String,
        confidence: f64,
    },
    HoistedMcpTool {
        tool_name: &'static str,
        confidence: f64,
    },
    CliCommandHint {
        capability_id: String,
        invocation: String,
        hint_markdown: String,
        confidence: f64,
    },
    FellOpenToLexical {
        reason: ChatArcFailOpenReason,
    },
}

fn score_candidate_lexical_relevance(name: &str, description: &str, utterance: &str) -> usize {
    let prompt_lower = utterance.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();
    let stripped_name = name_lower
        .strip_prefix("vox_")
        .or_else(|| name_lower.strip_prefix("vox "))
        .unwrap_or(&name_lower);
    let desc_lower = description.to_ascii_lowercase();

    let mut score = 0usize;
    if !stripped_name.is_empty() && prompt_lower.contains(stripped_name) {
        score += 300;
    }
    for raw_tok in prompt_lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if raw_tok.len() < 3
            || matches!(
                raw_tok,
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "that"
                    | "this"
                    | "can"
                    | "you"
                    | "how"
                    | "what"
                    | "vox"
            )
        {
            continue;
        }
        if stripped_name.contains(raw_tok) {
            score += 40;
        }
        if desc_lower.contains(raw_tok) {
            score += 15;
        }
    }
    score
}

/// Compose existing discovery sources (skill roots, MCP tool entries, CLI command catalog rows)
/// into a lexical top-`K` (`K <= 10`) shortlist without introducing a third registry (`CAT-01`).
#[must_use]
pub fn build_chatarc_shortlist(
    utterance: &str,
    skills: &[CatalogEntry],
    mcp_tools: &'static [vox_mcp_registry::McpToolRegistryEntry],
    cli_rows: &[vox_capability_registry::IndexRow],
    requested_k: usize,
) -> ChatArcShortlist {
    let k = requested_k.clamp(1, MAX_CHATARC_SHORTLIST_K);
    let mut pool: Vec<ChatArcCandidate> = Vec::new();

    // 1. Skills discovery source (`CatalogEntry`, carrying `reliability: Option<f64>`)
    for s in skills {
        let base_lex = score_candidate_lexical_relevance(&s.name, &s.description, utterance);
        if base_lex > 0 {
            let prior_bonus = s
                .reliability
                .map(|r| (r.clamp(0.0, 1.0) * 40.0).round() as usize)
                .unwrap_or(0);
            pool.push(ChatArcCandidate {
                id: format!("skill:{}", s.name),
                kind: ChatArcActionKind::Skill,
                display_name: s.name.clone(),
                description: s.description.clone(),
                lexical_score: base_lex + prior_bonus,
                reliability_prior: s.reliability,
                static_mcp_tool_name: None,
                cli_invocation: None,
            });
        }
    }

    // 2. MCP tool registry discovery source (`TOOL_REGISTRY`)
    for tool in mcp_tools {
        if tool.name.starts_with("vox_chat_") {
            continue;
        }
        let lex = score_candidate_lexical_relevance(tool.name, tool.description, utterance);
        if lex > 0 {
            pool.push(ChatArcCandidate {
                id: format!("mcp:{}", tool.name),
                kind: ChatArcActionKind::McpTool,
                display_name: tool.name.to_string(),
                description: tool.description.to_string(),
                lexical_score: lex,
                reliability_prior: None,
                static_mcp_tool_name: Some(tool.name),
                cli_invocation: None,
            });
        }
    }

    // 3. CLI command catalog discovery source (`vox_capability_registry::bundled_index()` `RowKind::Cli`)
    for row in cli_rows {
        if row.kind != vox_capability_registry::RowKind::Cli {
            continue;
        }
        let Some(cli_ref) = row.cli.as_ref() else {
            continue;
        };
        let desc = row.summary.as_deref().unwrap_or(row.title.as_str());
        let lex = score_candidate_lexical_relevance(&cli_ref.invocation, desc, utterance);
        if lex > 0 {
            pool.push(ChatArcCandidate {
                id: row.id.clone(),
                kind: ChatArcActionKind::CliCommand,
                display_name: cli_ref.invocation.clone(),
                description: desc.to_string(),
                lexical_score: lex,
                reliability_prior: None,
                static_mcp_tool_name: None,
                cli_invocation: Some(cli_ref.invocation.clone()),
            });
        }
    }

    // Sort by lexical score descending, then reliability prior (present > absent, higher > lower,
    // without treating None as 0.0 below unproven items), then kind, then id ascending.
    let cmp_cand = |a: &ChatArcCandidate, b: &ChatArcCandidate| {
        b.lexical_score
            .cmp(&a.lexical_score)
            .then_with(|| match (a.reliability_prior, b.reliability_prior) {
                (Some(ra), Some(rb)) => rb.total_cmp(&ra),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.id.cmp(&b.id))
    };
    pool.sort_by(cmp_cand);

    // Ensure source diversity across the three discovery families when `k >= 3` so a large
    // MCP/CLI surface cannot crowd out the top matching skill/tool/command, then fill
    // remaining slots in strict score order up to `k`.
    let mut selected: Vec<ChatArcCandidate> = Vec::with_capacity(k);
    if k >= 3 {
        for family in [
            ChatArcActionKind::Skill,
            ChatArcActionKind::McpTool,
            ChatArcActionKind::CliCommand,
        ] {
            if let Some(best_of_family) = pool.iter().find(|c| c.kind == family) {
                selected.push(best_of_family.clone());
            }
        }
    }
    for cand in pool {
        if selected.len() >= k {
            break;
        }
        if !selected.iter().any(|s| s.id == cand.id) {
            selected.push(cand);
        }
    }
    selected.sort_by(cmp_cand);

    ChatArcShortlist {
        candidates: selected,
        includes_none_option: true,
    }
}

/// Env flag enabling the live ChatARC shortlist choice and post-tool confidence escalation
/// in `run_agent_turn`. Unset/false ⇒ today's behavior unchanged (`DEC-03`).
pub const DECISION_PACKS_ENV: &str = "VOX_TURN_DECISION_PACKS";

/// True when [`DECISION_PACKS_ENV`] is set to a truthy value.
#[must_use]
pub fn decision_packs_enabled() -> bool {
    std::env::var(DECISION_PACKS_ENV)
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

/// Default confidence threshold for [`lexical_dominance_choice`].
pub const CHATARC_DEFAULT_THRESHOLD: f64 = 0.75;

/// Deterministic selector over `(shortlist ∪ {none})`: confidence is the top candidate's share of
/// the top-two lexical score mass (a lone candidate scores 0.5 and so abstains). Fails open to
/// `none` on an empty shortlist.
#[must_use]
pub fn lexical_dominance_choice(shortlist: &ChatArcShortlist, threshold: f64) -> ChatArcChoice {
    let Some(top) = shortlist.candidates.first() else {
        return ChatArcChoice {
            chosen_id: Some(CHATARC_NONE_CHOICE_ID.to_string()),
            confidence: 0.0,
            threshold,
        };
    };
    let top_s = top.lexical_score as f64;
    let second_s = shortlist
        .candidates
        .get(1)
        .map(|c| c.lexical_score as f64)
        .unwrap_or(top_s);
    ChatArcChoice {
        chosen_id: Some(top.id.clone()),
        confidence: top_s / (top_s + second_s).max(1.0),
        threshold,
    }
}

/// Calibrated choice over a lexical top-`K` (`K <= 10`) shortlist, consumed by tool offering.
pub fn apply_chatarc_choice_to_turn(
    shortlist: &ChatArcShortlist,
    choice: Option<&ChatArcChoice>,
    turn_ctx: &mut crate::llm_bridge::tool_selection::TurnContext,
) -> ChatArcSelectionOutcome {
    let Some(choice) = choice else {
        return ChatArcSelectionOutcome::FellOpenToLexical {
            reason: ChatArcFailOpenReason::DisabledOrOutage,
        };
    };
    let Some(raw_id) = choice.chosen_id.as_deref().map(str::trim) else {
        return ChatArcSelectionOutcome::FellOpenToLexical {
            reason: ChatArcFailOpenReason::SelectedNone,
        };
    };
    if raw_id.is_empty() || raw_id.eq_ignore_ascii_case(CHATARC_NONE_CHOICE_ID) {
        return ChatArcSelectionOutcome::FellOpenToLexical {
            reason: ChatArcFailOpenReason::SelectedNone,
        };
    }
    if !choice.confidence.is_finite() || choice.confidence < choice.threshold {
        return ChatArcSelectionOutcome::FellOpenToLexical {
            reason: ChatArcFailOpenReason::BelowConfidenceThreshold {
                confidence: choice.confidence,
                threshold: choice.threshold,
            },
        };
    }
    let Some(cand) = shortlist
        .candidates
        .iter()
        .find(|c| c.id == raw_id || c.display_name == raw_id)
    else {
        return ChatArcSelectionOutcome::FellOpenToLexical {
            reason: ChatArcFailOpenReason::CandidateNotInShortlist {
                chosen_id: raw_id.to_string(),
            },
        };
    };

    match cand.kind {
        ChatArcActionKind::Skill => {
            if turn_ctx.active_skill_id.is_none() {
                turn_ctx.active_skill_id = Some(cand.display_name.clone());
            }
            ChatArcSelectionOutcome::ActivatedSkill {
                skill_name: cand.display_name.clone(),
                confidence: choice.confidence,
            }
        }
        ChatArcActionKind::McpTool => {
            if let Some(static_name) = cand.static_mcp_tool_name {
                if !turn_ctx.pin_names.contains(&static_name) {
                    turn_ctx.pin_names.push(static_name);
                }
                ChatArcSelectionOutcome::HoistedMcpTool {
                    tool_name: static_name,
                    confidence: choice.confidence,
                }
            } else {
                ChatArcSelectionOutcome::FellOpenToLexical {
                    reason: ChatArcFailOpenReason::CandidateNotInShortlist {
                        chosen_id: raw_id.to_string(),
                    },
                }
            }
        }
        ChatArcActionKind::CliCommand => {
            let invocation = cand
                .cli_invocation
                .clone()
                .unwrap_or_else(|| cand.display_name.clone());
            let hint_markdown = format!(
                "[CHATARC CLI COMMAND HINT: Run `{}` — {}]",
                invocation, cand.description
            );
            ChatArcSelectionOutcome::CliCommandHint {
                capability_id: cand.id.clone(),
                invocation,
                hint_markdown,
                confidence: choice.confidence,
            }
        }
    }
}

/// Max chars of a description rendered in the tier-1 catalog. Matches the
/// documented SKILL.md authoring standard (1,024 chars) so the "when to use
/// this" half of the description — the part that drives activation — is no
/// longer cut off. Worst case 64 skills × ~1024 chars ≈ 16k tokens of raw
/// text (less after `format!` overhead is amortized); no other documented
/// system-prompt token budget in this codebase caps this section tighter, so
/// this raise is not currently in conflict with anything else found.
const DESC_CAP: usize = 1024;
/// Max bytes of a pinned skill body injected directly.
const PINNED_BODY_CAP: usize = 32 * 1024;

/// Render the `## Skills` system-prompt section. Ranked by reliability
/// (present data first, higher score first) then alphabetically (both as a
/// tiebreak among scored skills and as the sort for all no-data skills), and
/// length-capped so the section is content-stable across turns (cache-safe).
/// Empty input → empty string (no section, no cache churn).
///
/// When `entries.len() > max`, the dropped names are logged via
/// `tracing::info!` — the cap silently dropping a late-ranked skill was the
/// original bug (Task 3.1 / finding F6a); it is no longer silent.
pub(crate) fn render_skill_catalog(entries: &[CatalogEntry], max: usize) -> String {
    if entries.is_empty() || max == 0 {
        return String::new();
    }
    let mut sorted: Vec<&CatalogEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| match (a.reliability, b.reliability) {
        (Some(ra), Some(rb)) => rb
            .partial_cmp(&ra)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });
    if sorted.len() > max {
        let dropped: Vec<&str> = sorted[max..].iter().map(|e| e.name.as_str()).collect();
        tracing::info!(
            dropped_count = dropped.len(),
            dropped_skills = %dropped.join(", "),
            cap = max,
            "skill catalog truncated: skills fell off past the tier-1 cap"
        );
    }
    sorted.truncate(max);
    let mut out = String::from(
        "\n\n## Skills\nInstalled skills (name — when to use). To apply one, call the \
         `vox_skill_use` tool with its name to load the full instructions, then follow them. \
         If tools are unavailable, state which skill applies and proceed by its description.\n",
    );
    for e in sorted {
        let mut d = e.description.replace('\n', " ");
        if d.chars().count() > DESC_CAP {
            d = d.chars().take(DESC_CAP).collect::<String>();
            d.push('…');
        }
        out.push_str(&format!("- {} — {}\n", e.name, d));
    }
    out
}

/// Render the `## Active skill` section for a user-pinned skill, injecting the
/// full SKILL.md body so prompt-only models honor it without a tool call.
pub(crate) fn render_pinned_skill(name: &str, body: &str) -> String {
    let mut b = body;
    if b.len() > PINNED_BODY_CAP {
        // Truncate on a char boundary at or below the cap.
        let mut end = PINNED_BODY_CAP;
        while end > 0 && !b.is_char_boundary(end) {
            end -= 1;
        }
        b = &b[..end];
    }
    format!(
        "\n\n## Active skill: {name}\nThe user pinned this skill — follow these instructions \
         for this task:\n\n{b}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(n: &str, d: &str) -> CatalogEntry {
        CatalogEntry {
            name: n.into(),
            description: d.into(),
            reliability: None,
        }
    }

    fn sr(n: &str, d: &str, r: f64) -> CatalogEntry {
        CatalogEntry {
            name: n.into(),
            description: d.into(),
            reliability: Some(r),
        }
    }

    #[test]
    fn renders_alphabetical_capped_catalog() {
        let txt = render_skill_catalog(
            &[
                s("zeta", "Last"),
                s("brainstorming", "Socratic design refinement"),
            ],
            64,
        );
        let b = txt.find("brainstorming").unwrap();
        let z = txt.find("zeta").unwrap();
        assert!(b < z, "alphabetical fallback for no-data skills");
        assert!(txt.contains("## Skills"));
        assert!(txt.contains("vox_skill_use"));
    }

    #[test]
    fn empty_registry_renders_nothing() {
        assert_eq!(render_skill_catalog(&[], 64), "");
        assert_eq!(render_skill_catalog(&[s("a", "b")], 0), "");
    }

    #[test]
    fn caps_entry_count_and_description_length() {
        let many: Vec<CatalogEntry> = (0..100)
            .map(|i| s(&format!("skill-{i:03}"), &"x".repeat(2000)))
            .collect();
        let txt = render_skill_catalog(&many, 10);
        assert_eq!(txt.matches("\n- ").count(), 10);
        assert!(!txt.contains(&"x".repeat(1100)), "descriptions truncated");
        // DESC_CAP raised to 1024: a 1000-char description must survive intact.
        let one = render_skill_catalog(&[s("only", &"y".repeat(1000))], 64);
        assert!(one.contains(&"y".repeat(1000)), "1000-char desc not cut");
        assert!(!one.contains(&"y".repeat(1001)));
    }

    #[test]
    fn reliability_ranks_above_no_data_and_higher_above_lower() {
        let txt = render_skill_catalog(
            &[
                s("zzz-no-data", "no reliability yet"),
                sr("low-rel", "scored low", 0.2),
                sr("high-rel", "scored high", 0.9),
                s("aaa-no-data", "no reliability yet either"),
            ],
            64,
        );
        let high = txt.find("high-rel").unwrap();
        let low = txt.find("low-rel").unwrap();
        let aaa = txt.find("aaa-no-data").unwrap();
        let zzz = txt.find("zzz-no-data").unwrap();
        assert!(high < low, "higher reliability ranks first");
        assert!(low < aaa, "any scored skill ranks above any no-data skill");
        assert!(low < zzz, "any scored skill ranks above any no-data skill");
        assert!(
            aaa < zzz,
            "no-data skills fall back to alphabetical among themselves"
        );
    }

    #[test]
    fn reliability_tiebreak_is_alphabetical() {
        let txt = render_skill_catalog(&[sr("zeta", "z", 0.5), sr("alpha", "a", 0.5)], 64);
        let a = txt.find("alpha").unwrap();
        let z = txt.find("zeta").unwrap();
        assert!(a < z, "equal reliability breaks alphabetically");
    }

    #[test]
    fn truncation_drops_lowest_ranked_first() {
        // No-data skills sort last, so with reliability data present and a
        // tight cap, the no-data skill should be the one dropped, not silent
        // — the drop is also observable via render_skill_catalog's tracing
        // (exercised indirectly here: capacity 1 keeps only the scored skill).
        let txt = render_skill_catalog(&[sr("kept", "has data", 0.7), s("dropped", "no data")], 1);
        assert!(txt.contains("kept"));
        assert!(!txt.contains("dropped"));
    }

    #[test]
    fn determinism_same_input_twice_is_byte_identical() {
        let entries = [
            sr("charlie", "c desc", 0.5),
            s("alpha", "a desc"),
            sr("bravo", "b desc", 0.5),
            s("delta", "d desc"),
        ];
        let first = render_skill_catalog(&entries, 64);
        let second = render_skill_catalog(&entries, 64);
        assert_eq!(
            first, second,
            "cache-stability: identical input -> identical output"
        );
    }

    #[test]
    fn pinned_skill_section_contains_full_body() {
        let txt = render_pinned_skill("tdd", "# TDD\nRED-GREEN-REFACTOR.");
        assert!(txt.contains("## Active skill: tdd"));
        assert!(txt.contains("RED-GREEN-REFACTOR"));
        assert!(txt.contains("follow these instructions"));
    }

    #[test]
    fn pinned_body_is_capped_on_char_boundary() {
        let body = "é".repeat(40_000); // 80 KB, multibyte — must not panic
        let txt = render_pinned_skill("big", &body);
        assert!(txt.len() < PINNED_BODY_CAP + 256);
    }

    #[test]
    fn cat_01_chatarc_composes_skills_mcp_tools_and_cli_commands_capped_at_10() {
        let skills = vec![
            sr(
                "systematic-debugging",
                "Diagnose test failures and compiler errors systematically",
                0.92,
            ),
            s(
                "gsd-pr-branch",
                "Create a clean PR branch by filtering out planning commits",
            ),
        ];
        let cli_rows = &vox_capability_registry::bundled_index().rows;
        let shortlist = build_chatarc_shortlist(
            "check compiler errors and run git status or create pr branch",
            &skills,
            vox_mcp_registry::TOOL_REGISTRY,
            cli_rows,
            25, // requests 25, must clamp to MAX_CHATARC_SHORTLIST_K (10)
        );
        assert!(
            shortlist.candidates.len() <= MAX_CHATARC_SHORTLIST_K,
            "ChatARC shortlist must enforce K <= 10"
        );
        assert!(shortlist.includes_none_option);
        assert_eq!(
            shortlist.option_ids().last().map(String::as_str),
            Some(CHATARC_NONE_CHOICE_ID)
        );
        assert!(
            shortlist
                .candidates
                .iter()
                .any(|c| c.kind == ChatArcActionKind::Skill),
            "shortlist must include matching skill entries"
        );
        assert!(
            shortlist
                .candidates
                .iter()
                .any(|c| c.kind == ChatArcActionKind::McpTool),
            "shortlist must include matching MCP tool entries"
        );
        assert!(
            shortlist
                .candidates
                .iter()
                .any(|c| c.kind == ChatArcActionKind::CliCommand),
            "shortlist must include matching CLI command catalog rows"
        );
    }

    #[test]
    fn has_pr_tool_is_token_based_and_lexical_choice_abstains_when_ambiguous() {
        let mk = |id: &str, name: &str, kind, score| ChatArcCandidate {
            id: id.into(),
            kind,
            display_name: name.into(),
            description: String::new(),
            lexical_score: score,
            reliability_prior: None,
            static_mcp_tool_name: None,
            cli_invocation: None,
        };
        let pr = ChatArcShortlist {
            candidates: vec![mk(
                "mcp:vox_git_pr_create",
                "vox_git_pr_create",
                ChatArcActionKind::McpTool,
                9,
            )],
            includes_none_option: true,
        };
        assert!(pr.has_pr_tool());
        let not_pr = ChatArcShortlist {
            candidates: vec![
                mk(
                    "mcp:vox_improve_prompt",
                    "vox_improve_prompt",
                    ChatArcActionKind::McpTool,
                    9,
                ),
                mk(
                    "skill:sprint-create",
                    "sprint-create",
                    ChatArcActionKind::Skill,
                    8,
                ),
            ],
            includes_none_option: true,
        };
        assert!(!not_pr.has_pr_tool());

        let tie = lexical_dominance_choice(&not_pr, CHATARC_DEFAULT_THRESHOLD);
        assert!(tie.confidence < tie.threshold);
        let clear = ChatArcShortlist {
            candidates: vec![
                mk("mcp:a", "a", ChatArcActionKind::McpTool, 90),
                mk("mcp:b", "b", ChatArcActionKind::McpTool, 10),
            ],
            includes_none_option: true,
        };
        let c = lexical_dominance_choice(&clear, CHATARC_DEFAULT_THRESHOLD);
        assert!(c.confidence >= c.threshold);
    }

    #[test]
    fn cat_02_calibrated_choice_activates_skill_hoists_tool_hints_cli_and_fails_open() {
        let skills = vec![sr(
            "test-driven-development",
            "Write failing test first before implementation code",
            0.88,
        )];
        let cli_rows = &vox_capability_registry::bundled_index().rows;
        let shortlist = build_chatarc_shortlist(
            "write failing test first and check vox compiler syntax or git status",
            &skills,
            vox_mcp_registry::TOOL_REGISTRY,
            cli_rows,
            10,
        );

        let base_ctx = crate::llm_bridge::tool_selection::TurnContext {
            permission_mode: None,
            lanes: vec!["ai", "app"],
            active_skill_id: None,
            max_tools: 16,
            exclude_name_prefixes: vec!["vox_chat_"],
            pin_names: vec!["vox_spawn_agent"],
            query: None,
        };

        // 1. Outage / None -> fails open without modifying TurnContext
        let mut ctx_outage = base_ctx.clone();
        let out_none = apply_chatarc_choice_to_turn(&shortlist, None, &mut ctx_outage);
        assert_eq!(
            out_none,
            ChatArcSelectionOutcome::FellOpenToLexical {
                reason: ChatArcFailOpenReason::DisabledOrOutage
            }
        );
        assert_eq!(ctx_outage.pin_names, base_ctx.pin_names);
        assert_eq!(ctx_outage.active_skill_id, None);

        // 2. Below confidence threshold -> fails open without modifying TurnContext
        let mut ctx_low = base_ctx.clone();
        let low_choice = ChatArcChoice {
            chosen_id: Some("skill:test-driven-development".to_string()),
            confidence: 0.45,
            threshold: 0.75,
        };
        let out_low = apply_chatarc_choice_to_turn(&shortlist, Some(&low_choice), &mut ctx_low);
        assert!(matches!(
            out_low,
            ChatArcSelectionOutcome::FellOpenToLexical {
                reason: ChatArcFailOpenReason::BelowConfidenceThreshold { .. }
            }
        ));
        assert_eq!(ctx_low.active_skill_id, None);

        // 3. Explicit "none" abstention -> fails open
        let mut ctx_abstain = base_ctx.clone();
        let abstain_choice = ChatArcChoice {
            chosen_id: Some("none".to_string()),
            confidence: 0.99,
            threshold: 0.70,
        };
        let out_abstain =
            apply_chatarc_choice_to_turn(&shortlist, Some(&abstain_choice), &mut ctx_abstain);
        assert_eq!(
            out_abstain,
            ChatArcSelectionOutcome::FellOpenToLexical {
                reason: ChatArcFailOpenReason::SelectedNone
            }
        );

        // 4. High-confidence skill choice -> activates skill in TurnContext
        let mut ctx_skill = base_ctx.clone();
        let skill_choice = ChatArcChoice {
            chosen_id: Some("skill:test-driven-development".to_string()),
            confidence: 0.91,
            threshold: 0.75,
        };
        let out_skill =
            apply_chatarc_choice_to_turn(&shortlist, Some(&skill_choice), &mut ctx_skill);
        assert_eq!(
            out_skill,
            ChatArcSelectionOutcome::ActivatedSkill {
                skill_name: "test-driven-development".to_string(),
                confidence: 0.91,
            }
        );
        assert_eq!(
            ctx_skill.active_skill_id.as_deref(),
            Some("test-driven-development")
        );

        // 5. High-confidence MCP tool choice -> hoists into TurnContext.pin_names
        let mcp_cand = shortlist
            .candidates
            .iter()
            .find(|c| c.kind == ChatArcActionKind::McpTool)
            .expect("mcp candidate in shortlist");
        let mut ctx_mcp = base_ctx.clone();
        let mcp_choice = ChatArcChoice {
            chosen_id: Some(mcp_cand.id.clone()),
            confidence: 0.89,
            threshold: 0.75,
        };
        let out_mcp = apply_chatarc_choice_to_turn(&shortlist, Some(&mcp_choice), &mut ctx_mcp);
        assert!(matches!(
            out_mcp,
            ChatArcSelectionOutcome::HoistedMcpTool { .. }
        ));
        assert!(
            ctx_mcp
                .pin_names
                .contains(&mcp_cand.static_mcp_tool_name.unwrap()),
            "chosen MCP tool must be hoisted into TurnContext.pin_names"
        );

        // 6. High-confidence CLI command choice -> emits CLI command hint
        let cli_cand = shortlist
            .candidates
            .iter()
            .find(|c| c.kind == ChatArcActionKind::CliCommand)
            .expect("cli candidate in shortlist");
        let mut ctx_cli = base_ctx;
        let cli_choice = ChatArcChoice {
            chosen_id: Some(cli_cand.id.clone()),
            confidence: 0.93,
            threshold: 0.75,
        };
        let out_cli = apply_chatarc_choice_to_turn(&shortlist, Some(&cli_choice), &mut ctx_cli);
        assert!(
            matches!(out_cli, ChatArcSelectionOutcome::CliCommandHint { ref hint_markdown, .. } if hint_markdown.contains("[CHATARC CLI COMMAND HINT:"))
        );
    }
}
