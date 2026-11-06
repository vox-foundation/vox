use super::*;

/// Honest classification of *why* the resolved model is what it is, derived
/// from the resolved model id (never the requested one) — see Phase B of
/// `docs/superpowers/plans/2026-08-28-chat-harness-unification.md`. Classifying
/// from the request meant a pinned model the resolver silently ignored still
/// read "Your pick" in `ModelBadge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionSource {
    /// `requested == resolved`: the user's pin was honored.
    UserOverride,
    /// No per-request pref, but the process-global sticky override matches
    /// the resolved model.
    Global,
    /// No pref at all; the auto-selector chose.
    AutoRouted,
    /// A pref was requested but the resolved model differs from it (removed
    /// from the registry, capability mismatch, etc.).
    Fallback,
}

impl SelectionSource {
    /// Classify from the RESOLVED model, not the requested one — that is the
    /// whole point of this type. `requested` is the per-request pin (if any),
    /// `resolved` is the model id the turn actually used, `global` is the
    /// process-global sticky override (if any).
    #[must_use]
    pub fn classify(requested: Option<&str>, resolved: Option<&str>, global: Option<&str>) -> Self {
        match (requested, resolved) {
            (Some(r), Some(res)) if r == res => SelectionSource::UserOverride,
            (Some(_), Some(_)) => SelectionSource::Fallback,
            (None, Some(res)) if global == Some(res) => SelectionSource::Global,
            _ => SelectionSource::AutoRouted,
        }
    }
}

/// Envelope payload for `ModelBadge`: the resolved model, an honest
/// [`SelectionSource`], and — for anything other than a plain user pin — the
/// rationale explaining why.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SelectionDto {
    pub model: String,
    pub source: SelectionSource,
    pub rationale: Option<String>,
}

impl SelectionDto {
    pub(super) fn new(
        requested: Option<&str>,
        resolved: &str,
        global: Option<&str>,
        rationale: Option<String>,
    ) -> Self {
        Self {
            model: resolved.to_string(),
            source: SelectionSource::classify(requested, Some(resolved), global),
            rationale,
        }
    }

    /// A fallback selection always carries a rationale — never a silent `None`.
    #[cfg(test)]
    pub(super) fn fallback(model: impl Into<String>, rationale: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            source: SelectionSource::Fallback,
            rationale: Some(rationale.into()),
        }
    }
}

/// [`try_run_agent_turn`]'s success payload. A named struct (rather than a
/// 4-tuple) now that the model-selection rationale rides along too — see
/// Fix Task 7.
#[derive(Debug)]
pub(super) struct AgentTurnResult {
    pub(super) text: String,
    pub(super) model_used: String,
    pub(super) tokens: u64,
    pub(super) prompt_tokens: u64,
    pub(super) completion_tokens: u64,
    pub(super) cache_read_tokens: u64,
    pub(super) cost_usd: Option<f64>,
    pub(super) ttft_ms: Option<u64>,
    pub(super) tpot_ms: Option<f64>,
    pub(super) classified_spoke: Option<String>,
    pub(super) turn_tool_ledger: Vec<String>,
    /// Human-readable reason the model was chosen (`SelectionReason::to_string()`),
    /// when the rationale-carrying resolver produced one. Surfaced to the GUI as
    /// `data.selection_reason` for `ModelBadge`'s tooltip.
    pub(super) selection_reason: Option<String>,
    /// The strict pin the model was resolved through (Task 13), if any.
    pub(super) pinned: Option<String>,
    /// Chat-turn-visible events derived from tool results this turn (Phase E
    /// Task E1) — empty on every path except the real agent loop
    /// ([`super::agent_loop::run_agent_turn`]), which is the only path that
    /// dispatches tool calls at all.
    pub(super) events: Vec<Value>,
}

/// Phase 153 (`SOTA-TELEM-CLOSE-02`): Deterministic fallback cost calculation from
/// `ModelSpec` (`cost_per_1k_input`, `cost_per_1k_output`, `cache_read_cost_per_1k`)
/// when the wire response omits `cost_usd`.
pub(crate) fn compute_turn_cost_usd(
    model: &vox_orchestrator::models::ModelSpec,
    is_free: bool,
    wire_cost_usd: Option<f64>,
    prompt_tokens: u64,
    completion_tokens: u64,
    cache_read_tokens: u64,
) -> Option<f64> {
    if let Some(c) = wire_cost_usd
        && c >= 0.0
    {
        return Some(c);
    }
    if is_free || model.is_free {
        return Some(0.0);
    }
    let in_rate = if model.cost_per_1k_input > 0.0 {
        model.cost_per_1k_input
    } else {
        model.cost_per_1k
    };
    let out_rate = if model.cost_per_1k_output > 0.0 {
        model.cost_per_1k_output
    } else {
        model.cost_per_1k
    };
    if in_rate <= 0.0 && out_rate <= 0.0 {
        return None;
    }
    let cache_rate = if model.cache_read_cost_per_1k > 0.0 {
        model.cache_read_cost_per_1k
    } else {
        in_rate * 0.1
    };
    let uncached_in = prompt_tokens.saturating_sub(cache_read_tokens) as f64;
    let cached_in = cache_read_tokens.min(prompt_tokens) as f64;
    let out_tok = completion_tokens as f64;
    Some((uncached_in * in_rate + cached_in * cache_rate + out_tok * out_rate) / 1000.0)
}

/// Phase 153 (`SOTA-TELEM-CLOSE-03`): Map an 8-spoke classification slug to its
/// canonical scoreboard `task_category`.
pub(crate) fn task_category_for_spoke_slug(spoke: Option<&str>) -> String {
    match spoke.map(str::trim) {
        Some("rust_workspace" | "vox_lang" | "speech_to_code") => "CodeGen".to_string(),
        Some("agent_harness" | "mcp_tool_chain") => "ToolUse".to_string(),
        Some("structured_shell") => "Parsing".to_string(),
        Some("populi_mesh" | "skill_creation") => "Architecture".to_string(),
        _ => "General".to_string(),
    }
}

/// Model preference for this turn: per-request pick, then the process-global
/// override, then nothing (the auto-selector runs). Blanks are not picks — the
/// resolver treats `""` as an explicit, unresolvable model id, so an empty
/// composer field must neither shadow the global nor reach it.
pub fn effective_model_pref(request: Option<&str>, global: Option<&str>) -> Option<String> {
    request
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| global.map(str::trim).filter(|s| !s.is_empty()))
        .map(str::to_string)
}

/// Composer tier -> resolution constraints. Deliberately NOT `cognitive_profile`:
/// that field's values are `fast|reasoning|creative` and setting it switches the
/// turn off the agent loop onto `mcp_infer_completion`, losing tool calls and
/// the selection rationale.
pub fn resolution_for_tier(
    tier: Option<&str>,
    base: McpChatModelResolution,
) -> McpChatModelResolution {
    match tier.map(str::trim) {
        Some("local") => McpChatModelResolution {
            enforce_free_tier_only: true,
            // Restore the pre-tier-routing degradation path (previously
            // unconditional for every call site). This is safe to keep on for
            // `local`: `enforce_free_tier_if_needed` (model_route_policy/policy.rs)
            // gates every returned model through free-tier enforcement, so
            // `allow_cheapest_fallback` here can only ever widen which *free*
            // model gets picked when the primary `decide()` selection comes up
            // empty -- it can never leak a paid model back out.
            allow_cheapest_fallback: true,
            ..base
        },
        Some("mesh") => McpChatModelResolution {
            free_tier_latency_critical: true,
            // Same reasoning as `local` above -- restore the fallback rather
            // than silently dropping it just because a tier was named.
            allow_cheapest_fallback: true,
            ..base
        },
        Some("cloud") => McpChatModelResolution {
            // Explicitly paid-preferring and observably different from both
            // "auto" (`_` arm, which always allows degrading to a cheap/free
            // fallback) and from `McpChatModelResolution::default()`: never let
            // a failed/absent primary pick quietly degrade to the cheapest free
            // model, and bias `decide()`'s `SelectionRequest.intent.complexity`
            // (resolve.rs `build_selection_request`) toward stronger models so a
            // "cloud" pick doesn't land on a weak model by the scorer's default.
            allow_cheapest_fallback: false,
            enforce_free_tier_only: false,
            complexity: base.complexity.max(8),
            ..base
        },
        Some(other) => {
            // Unknown/typo'd tier (e.g. "Cloud", "remote"): `tier` is a
            // free-text `Option<String>` at the Rust type level even though the
            // composer only ever sends "local"/"mesh"/"cloud"/omitted, so a typo
            // is representable. Explicit decision (not silent aliasing): treat
            // it exactly like "no tier" (auto/default), and log it so the typo
            // is debug-visible instead of just quietly "working" with no trace.
            // Known gap: this function has no channel back to the caller's
            // `selection_reason`/rationale to surface the typo in the GUI (see
            // `try_run_agent_turn`'s `selection_reason` plumbing) -- wiring
            // that through is out of scope for this fix.
            tracing::warn!(
                target: "vox_mcp::chat_tier",
                tier = other,
                "unrecognized chat tier; falling back to auto/default resolution"
            );
            McpChatModelResolution {
                allow_cheapest_fallback: true,
                ..base
            }
        }
        None => McpChatModelResolution {
            allow_cheapest_fallback: true,
            ..base
        },
    }
}

/// Mask of retrieval legs enabled for a given composer `kb_scope` (`auto|docs|code|memory|verified|off`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(crate) struct KbScopeMask {
    pub enabled: bool,
    pub rrf: bool,
    pub memory: bool,
    pub knowledge: bool,
    pub chunks: bool,
    pub repo: bool,
}

impl KbScopeMask {
    pub(crate) fn from_kb_scope(kb_scope: Option<&str>) -> Self {
        match kb_scope.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("off") => Self {
                enabled: false,
                rrf: false,
                memory: false,
                knowledge: false,
                chunks: false,
                repo: false,
            },
            Some("docs") => Self {
                enabled: true,
                rrf: false,
                memory: false,
                knowledge: true,
                chunks: true,
                repo: false,
            },
            Some("code") => Self {
                enabled: true,
                rrf: true,
                memory: false,
                knowledge: false,
                chunks: false,
                repo: true,
            },
            Some("memory") => Self {
                enabled: true,
                rrf: false,
                memory: true,
                knowledge: false,
                chunks: false,
                repo: false,
            },
            Some("verified") => Self {
                enabled: true,
                rrf: true,
                memory: true,
                knowledge: true,
                chunks: true,
                repo: false,
            },
            _ => Self {
                enabled: true,
                rrf: true,
                memory: true,
                knowledge: true,
                chunks: true,
                repo: true,
            },
        }
    }
}

/// Real-time persisted session chat posture (`gui.chat_posture.<session_id>` / `gui.chat_posture.default`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PersistedSessionChatPosture {
    #[serde(default)]
    pub clutch: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub oversight_level: Option<String>,
    #[serde(default)]
    pub kb_scope: Option<String>,
    #[serde(default)]
    pub grounding_check_enabled: Option<bool>,
    #[serde(default)]
    pub compaction_strategy: Option<String>,
}

pub(crate) async fn load_persisted_session_chat_posture(
    state: &ServerState,
    session_id: Option<&str>,
) -> PersistedSessionChatPosture {
    let Some(ref db) = state.db else {
        return PersistedSessionChatPosture::default();
    };
    let sid = session_id.map(str::trim).filter(|s| !s.is_empty());
    if let Some(s) = sid {
        let key = format!("gui.chat_posture.{s}");
        if let Ok(Some(raw)) = db.get_user_preference("local_user", &key).await
            && let Ok(parsed) = serde_json::from_str::<PersistedSessionChatPosture>(&raw)
        {
            return parsed;
        }
    }
    if let Ok(Some(raw)) = db
        .get_user_preference("local_user", "gui.chat_posture.default")
        .await
        && let Ok(parsed) = serde_json::from_str::<PersistedSessionChatPosture>(&raw)
    {
        return parsed;
    }
    PersistedSessionChatPosture::default()
}

pub(crate) async fn persist_session_chat_posture(
    state: &ServerState,
    session_id: Option<&str>,
    posture: &PersistedSessionChatPosture,
) {
    let Some(ref db) = state.db else {
        return;
    };
    let Ok(serialized) = serde_json::to_string(posture) else {
        return;
    };
    if let Some(s) = session_id.map(str::trim).filter(|s| !s.is_empty()) {
        let key = format!("gui.chat_posture.{s}");
        let _ = db
            .set_user_preference("local_user", &key, &serialized)
            .await;
    }
    let _ = db
        .set_user_preference("local_user", "gui.chat_posture.default", &serialized)
        .await;
}

/// Formats a concise governance directive when composer `mode`, `oversight_level`, or `dry_run` is set.
pub(crate) fn build_governance_context_hint(
    mode: Option<&str>,
    oversight_level: Option<&str>,
    dry_run: Option<bool>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(m) = mode.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("mode={m}"));
    }
    if let Some(o) = oversight_level.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("oversight_level={o}"));
    }
    if dry_run == Some(true) {
        parts.push("dry_run=true (read-only simulation; do not mutate files)".to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("[EXECUTION GOVERNANCE]: {}", parts.join(", ")))
    }
}

/// Phase 149 (`SOTA-PREAMBLE-01..03`): Intent-gated autonomous retrieval preamble.
///
/// Under `kb_scope = None` or `"auto"`, skips the 5-leg `run_retrieval_bundle` fan-out
/// (Tantivy + embeddings + RRF + memory + KG + chunks + repo) on:
/// - Phatic / conversational greetings (`<= 3` words of phatic tokens without `?` or `/`),
/// - Self-contained `@file`/`@symbol` or `selected_text` code edits that do not ask for
///   broader search/memory/architecture context.
///
/// Explicit non-`"auto"` `kb_scope` settings (`"memory"`, `"knowledge"`, `"chunks"`, `"repo"`,
/// `"docs"`, `"code"`, `"verified"`) always run when enabled.
pub(crate) fn should_run_auto_chat_preamble(
    raw_prompt: &str,
    kb_scope: Option<&str>,
    has_explicit_code_context: bool,
) -> (bool, Option<&'static str>) {
    let scope_norm = kb_scope.map(str::trim).unwrap_or("auto");
    if !scope_norm.is_empty() && !scope_norm.eq_ignore_ascii_case("auto") {
        return (true, None);
    }

    let trimmed = raw_prompt.trim();
    if trimmed.is_empty() {
        return (false, Some("empty_prompt"));
    }
    let lower = trimmed.to_ascii_lowercase();
    let clean = lower.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    let words: Vec<&str> = clean.split_whitespace().collect();
    const PHATIC_WORDS: &[&str] = &[
        "hi",
        "hello",
        "hey",
        "yo",
        "sup",
        "thanks",
        "thank",
        "you",
        "thx",
        "ty",
        "ok",
        "okay",
        "k",
        "cool",
        "great",
        "awesome",
        "nice",
        "good",
        "morning",
        "afternoon",
        "evening",
        "bye",
        "goodbye",
        "ping",
        "test",
        "yes",
        "no",
        "yep",
        "nope",
        "sure",
        "got",
        "it",
        "understood",
        "ack",
        "there",
    ];
    if !trimmed.starts_with('/')
        && !trimmed.contains('?')
        && !words.is_empty()
        && words.len() <= 3
        && words.iter().all(|w| PHATIC_WORDS.contains(w))
    {
        return (false, Some("phatic_or_conversational_greeting"));
    }

    if has_explicit_code_context {
        const REPO_SEARCH_CUES: &[&str] = &[
            "search",
            "where ",
            " across ",
            "remember",
            "memory",
            "knowledge",
            "docs",
            "architecture",
            "other callers",
            "find all",
        ];
        if !REPO_SEARCH_CUES.iter().any(|cue| lower.contains(cue)) {
            return (false, Some("self_contained_code_context"));
        }
    }

    const REPO_SEARCH_CUES_FAST: &[&str] = &[
        "search",
        "where ",
        " across ",
        "remember",
        "memory",
        "knowledge",
        "docs",
        "architecture",
        "other callers",
        "find all",
        "grounded in",
        "how does",
    ];
    let has_search_cue = REPO_SEARCH_CUES_FAST.iter().any(|cue| lower.contains(cue));
    if !has_search_cue {
        let is_voice_dictation = ["um ", "uh ", "er ", "um, ", "uh, "]
            .iter()
            .any(|p| lower.starts_with(p))
            || lower.contains("dictating this")
            || lower.contains("transcribe voice")
            || lower.contains("open brace");
        if is_voice_dictation {
            return (false, Some("self_contained_speech_to_code"));
        }
        if trimmed.contains("```") {
            return (false, Some("self_contained_inline_code_snippet"));
        }
    }

    (true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_source_classifies_from_resolved_model() {
        assert_eq!(
            SelectionSource::classify(Some("a"), Some("a"), None),
            SelectionSource::UserOverride
        );
        assert_eq!(
            SelectionSource::classify(Some("a"), Some("b"), Some("b")),
            SelectionSource::Fallback
        );
        assert_eq!(
            SelectionSource::classify(None, Some("g"), Some("g")),
            SelectionSource::Global
        );
        assert_eq!(
            SelectionSource::classify(None, Some("x"), Some("g")),
            SelectionSource::AutoRouted
        );
        assert_eq!(
            SelectionSource::classify(None, None, None),
            SelectionSource::AutoRouted
        );
    }

    #[test]
    fn effective_model_pref_prefers_request_and_ignores_blanks() {
        assert_eq!(
            effective_model_pref(Some(" m1 "), Some("g")).as_deref(),
            Some("m1")
        );
        assert_eq!(
            effective_model_pref(Some("  "), Some(" g ")).as_deref(),
            Some("g")
        );
        assert_eq!(effective_model_pref(None, Some("")), None);
    }

    #[test]
    fn spoke_slug_maps_to_scoreboard_category() {
        assert_eq!(task_category_for_spoke_slug(Some("vox_lang")), "CodeGen");
        assert_eq!(
            task_category_for_spoke_slug(Some(" mcp_tool_chain ")),
            "ToolUse"
        );
        assert_eq!(
            task_category_for_spoke_slug(Some("structured_shell")),
            "Parsing"
        );
        assert_eq!(
            task_category_for_spoke_slug(Some("populi_mesh")),
            "Architecture"
        );
        assert_eq!(task_category_for_spoke_slug(Some("unknown")), "General");
        assert_eq!(task_category_for_spoke_slug(None), "General");
    }

    #[test]
    fn governance_hint_lists_only_set_fields() {
        assert_eq!(
            build_governance_context_hint(None, Some(" "), Some(false)),
            None
        );
        assert_eq!(
            build_governance_context_hint(Some("plan"), Some("high"), None).as_deref(),
            Some("[EXECUTION GOVERNANCE]: mode=plan, oversight_level=high")
        );
    }
}
