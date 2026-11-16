//! Deterministic research-intent classifier for chat turns (spec §4.1).
//! Replaces the confidence-gate heuristic that fired web research on "hi" (D1).

use serde::Serialize;
use vox_orchestrator_types::socrates_policy::{
    CragEscalationPolicy, ResearchContinuumMode, ResearchContinuumTier,
};
use vox_research_shim::research::types::{ResearchDomainMode, ResearchScope};

pub const MIN_RESEARCH_WAVES: usize = 1;
pub const MAX_RESEARCH_WAVES: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResearchMode {
    None,
    Quick,
    Deep,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResearchIntent {
    pub mode: ResearchMode,
    pub continuum_mode: ResearchContinuumMode,
    pub initial_tier: ResearchContinuumTier,
    pub scope: ResearchScope,
    pub domain_mode: ResearchDomainMode,
    pub site_scope: Option<String>,
    pub waves: usize,
    pub explicit: bool,
    pub reasons: Vec<String>,
    pub query: String,
}

/// Parameters controlling continuum research classification.
#[derive(Debug, Clone, Default)]
pub struct ResearchIntentParams<'a> {
    pub prompt: &'a str,
    pub research_mode: Option<&'a str>,
    pub force_research: Option<bool>,
    pub research_scope: Option<&'a str>,
    pub domain_mode: Option<&'a str>,
    pub site_scope: Option<&'a str>,
    pub waves: Option<usize>,
    pub socrates: Option<&'a vox_orchestrator::socrates::SocratesGateOutcome>,
}

const GREETINGS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "yo",
    "sup",
    "thanks",
    "thank you",
    "thx",
    "ok",
    "okay",
    "cool",
    "nice",
    "bye",
    "good morning",
    "good night",
    "how are you",
];
const CODE_VERBS: &[&str] = &[
    "fix ",
    "refactor ",
    "implement ",
    "write a function",
    "write a test",
    "add a test",
    "rename ",
    "debug ",
    "edit ",
    "update the code",
    "change the code",
];
const DEEP_CUES: &[&str] = &[
    "compare",
    "comparison",
    " vs ",
    " vs. ",
    "versus",
    "trade-off",
    "tradeoff",
    "pros and cons",
    "state of the art",
    "literature review",
    "deep dive",
    "comprehensive overview",
    "survey of",
];
const TIME_CUES: &[&str] = &[
    "latest",
    "current",
    "currently",
    "newest",
    "recent",
    "recently",
    "today",
    "this week",
    "this month",
    "this year",
    "right now",
    "version",
    "release",
    "released",
    "price",
    "pricing",
    "news",
    "announced",
    "as of",
];
const EVIDENCE_CUES: &[&str] = &[
    "look up",
    "search for",
    "search the web",
    "find sources",
    "sources on",
    "cite ",
    "citations",
    "according to",
];
const LOCAL_CODE_CUES: &[&str] = &[
    "this crate",
    "this repo",
    "this repository",
    "this branch",
    "this codebase",
    "this project",
    "this file",
    "this function",
    "this module",
    "this implementation",
    "current implementation",
    "this code",
    "our code",
    "our repo",
    "the codebase",
    "commit",
    "commits",
    "pull request",
    "this pr",
];
const QUESTION_WORDS: &[&str] = &[
    "what", "what's", "whats", "who", "when", "where", "which", "is", "are", "does", "did",
    "how many", "how much",
];

fn parse_scope(scope: Option<&str>) -> ResearchScope {
    match scope.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("local") => ResearchScope::Local,
        Some("both") => ResearchScope::Both,
        _ => ResearchScope::Web,
    }
}

fn parse_domain_mode(domain: Option<&str>) -> ResearchDomainMode {
    domain
        .and_then(|d| d.trim().parse::<ResearchDomainMode>().ok())
        .unwrap_or_default()
}

fn parse_site_scope(site: Option<&str>) -> Option<String> {
    site.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn clamp_waves(waves: Option<usize>, mode: ResearchMode) -> usize {
    match waves {
        Some(w) => w.clamp(MIN_RESEARCH_WAVES, MAX_RESEARCH_WAVES),
        None if mode == ResearchMode::Deep => {
            usize::from(CragEscalationPolicy::default().max_auto_waves)
                .clamp(MIN_RESEARCH_WAVES, MAX_RESEARCH_WAVES)
        }
        None => MIN_RESEARCH_WAVES,
    }
}

fn build_intent(
    mode: ResearchMode,
    continuum_mode: ResearchContinuumMode,
    explicit: bool,
    reason: String,
    query: &str,
    scope: ResearchScope,
    domain_mode: ResearchDomainMode,
    site_scope: Option<String>,
    waves_override: Option<usize>,
) -> ResearchIntent {
    let policy = CragEscalationPolicy::default();
    let initial_tier = match continuum_mode {
        ResearchContinuumMode::Off => ResearchContinuumTier::Direct,
        ResearchContinuumMode::Fast => ResearchContinuumTier::QuickWeb,
        ResearchContinuumMode::Deep => ResearchContinuumTier::DeepMultiWave,
        ResearchContinuumMode::Auto => match mode {
            ResearchMode::Deep => ResearchContinuumTier::DeepMultiWave,
            ResearchMode::Quick => ResearchContinuumTier::QuickWeb,
            ResearchMode::None => {
                if explicit
                    || reason.starts_with("skip:")
                    || reason.starts_with("local knowledge-base")
                {
                    ResearchContinuumTier::Direct
                } else {
                    policy.initial_tier_for_mode(ResearchContinuumMode::Auto)
                }
            }
        },
    };
    ResearchIntent {
        mode,
        continuum_mode,
        initial_tier,
        scope,
        domain_mode,
        site_scope,
        waves: clamp_waves(waves_override, mode),
        explicit,
        reasons: vec![reason],
        query: query.trim().to_string(),
    }
}

/// `"/research foo"` → `Some("foo")`; requires a word boundary after the command.
fn strip_command<'a>(prompt: &'a str, cmd: &str) -> Option<&'a str> {
    let rest = prompt.strip_prefix(cmd)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

fn looks_like_path(word: &str) -> bool {
    const EXTS: &[&str] = &[
        ".rs", ".ts", ".tsx", ".js", ".py", ".md", ".toml", ".json", ".vox", ".yml", ".yaml",
    ];
    word.starts_with('@')
        || word.starts_with("./")
        || word.starts_with("crates/")
        || EXTS.iter().any(|e| {
            word.trim_end_matches(|c: char| !c.is_alphanumeric())
                .ends_with(e)
        })
}

/// Lowercase, replace every non-alphanumeric/`'` char with a space, collapse
/// whitespace runs, and pad with a single space at each end — so cue lookups
/// via `normalized.contains(" cue ")` match on word boundaries, not substrings
/// (e.g. "commit" must not match inside "committee").
fn normalize_words(s: &str) -> String {
    let collapsed: String = s
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect();
    format!(
        " {} ",
        collapsed.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

fn has_recent_year(lower: &str) -> bool {
    lower
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|t| (t.len() == 4).then(|| t.parse::<u32>().ok()).flatten())
        .any(|y| (2020..=2100).contains(&y))
}

pub fn classify_research_intent(
    prompt: &str,
    force: Option<bool>,
    scope: Option<&str>,
) -> ResearchIntent {
    classify_research_intent_with_socrates(prompt, force, scope, None)
}

/// Classify research intent with optional evidence from the Socrates risk/confidence gate.
pub fn classify_research_intent_with_socrates(
    prompt: &str,
    force: Option<bool>,
    scope: Option<&str>,
    socrates: Option<&vox_orchestrator::socrates::SocratesGateOutcome>,
) -> ResearchIntent {
    classify_research_intent_from_params(ResearchIntentParams {
        prompt,
        research_mode: None,
        force_research: force,
        research_scope: scope,
        domain_mode: None,
        site_scope: None,
        waves: None,
        socrates,
    })
}

/// Classify research intent using full orthogonal continuum controls (`research_mode`,
/// `research_scope`, `domain_mode`, `site_scope`, `waves`) and optional Socrates outcome.
pub fn classify_research_intent_from_params(params: ResearchIntentParams<'_>) -> ResearchIntent {
    let trimmed = params.prompt.trim();
    let lower = trimmed.to_ascii_lowercase();
    let scope_val = parse_scope(params.research_scope);
    let domain_mode = parse_domain_mode(params.domain_mode);
    let site_scope = parse_site_scope(params.site_scope);
    let make = |mode: ResearchMode,
                continuum_mode: ResearchContinuumMode,
                explicit: bool,
                reason: String,
                query: &str| {
        build_intent(
            mode,
            continuum_mode,
            explicit,
            reason,
            query,
            scope_val.clone(),
            domain_mode,
            site_scope.clone(),
            params.waves,
        )
    };

    // 0. Explicit continuum research_mode override (`off | auto | fast | deep`).
    let explicit_continuum = params
        .research_mode
        .and_then(|raw| raw.trim().parse::<ResearchContinuumMode>().ok());
    if let Some(cmode) = explicit_continuum {
        match cmode {
            ResearchContinuumMode::Off => {
                return make(
                    ResearchMode::None,
                    ResearchContinuumMode::Off,
                    true,
                    "explicit: research_mode=off".into(),
                    trimmed,
                );
            }
            ResearchContinuumMode::Fast => {
                let q = strip_command(trimmed, "/research").unwrap_or(trimmed);
                return make(
                    ResearchMode::Quick,
                    ResearchContinuumMode::Fast,
                    true,
                    "explicit: research_mode=fast".into(),
                    q,
                );
            }
            ResearchContinuumMode::Deep => {
                let q = strip_command(trimmed, "/deepresearch").unwrap_or(trimmed);
                return make(
                    ResearchMode::Deep,
                    ResearchContinuumMode::Deep,
                    true,
                    "explicit: research_mode=deep".into(),
                    q,
                );
            }
            ResearchContinuumMode::Auto => {}
        }
    }

    // 1. Explicit slash commands, force flags, and legacy scope aliases (only when research_mode is None).
    if params.research_mode.is_none()
        && params
            .research_scope
            .is_some_and(|s| s.trim().eq_ignore_ascii_case("none"))
    {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Off,
            true,
            "explicit: scope=none".into(),
            trimmed,
        );
    }
    if let Some(rest) = strip_command(trimmed, "/deepresearch") {
        return make(
            ResearchMode::Deep,
            ResearchContinuumMode::Deep,
            true,
            "explicit: /deepresearch".into(),
            rest,
        );
    }
    if lower.starts_with("/research search")
        || lower.starts_with("/research-search")
        || lower.starts_with("/research --search")
    {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Off,
            true,
            "local knowledge-base search (/research search)".into(),
            trimmed,
        );
    }
    if let Some(rest) = strip_command(trimmed, "/research") {
        return make(
            ResearchMode::Quick,
            ResearchContinuumMode::Fast,
            true,
            "explicit: /research".into(),
            rest,
        );
    }
    match params.force_research {
        Some(true) => {
            let deep = params
                .research_scope
                .is_some_and(|s| s.trim().eq_ignore_ascii_case("deep"));
            let (mode, cmode) = if deep {
                (ResearchMode::Deep, ResearchContinuumMode::Deep)
            } else {
                (ResearchMode::Quick, ResearchContinuumMode::Fast)
            };
            return make(
                mode,
                cmode,
                true,
                "explicit: force_research".into(),
                trimmed,
            );
        }
        Some(false) => {
            return make(
                ResearchMode::None,
                ResearchContinuumMode::Off,
                true,
                "explicit: force_research=false".into(),
                trimmed,
            );
        }
        None => {
            if params.research_mode.is_none()
                && params
                    .research_scope
                    .is_some_and(|s| s.trim().eq_ignore_ascii_case("deep"))
            {
                return make(
                    ResearchMode::Deep,
                    ResearchContinuumMode::Deep,
                    true,
                    "legacy: research_scope=deep".into(),
                    trimmed,
                );
            }
        }
    }

    // 2. Skip trivial or code-local work before Socrates or deep analysis.
    let bare: String = lower
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '\'')
        .collect();
    let bare = bare.trim();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if GREETINGS
        .iter()
        .any(|g| bare == *g || (bare.starts_with(&format!("{g} ")) && words.len() <= 4))
    {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Auto,
            false,
            "skip: greeting / small talk".into(),
            trimmed,
        );
    }
    if words.len() < 3 {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Auto,
            false,
            "skip: fewer than 3 words".into(),
            trimmed,
        );
    }
    if trimmed.contains("```") || words.iter().any(|w| looks_like_path(w)) {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Auto,
            false,
            "skip: references code or files".into(),
            trimmed,
        );
    }
    if CODE_VERBS.iter().any(|v| lower.starts_with(v)) {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Auto,
            false,
            "skip: coding / edit request".into(),
            trimmed,
        );
    }

    let normalized = normalize_words(trimmed);
    if let Some(cue) = LOCAL_CODE_CUES
        .iter()
        .find(|c| normalized.contains(&format!(" {c} ")))
    {
        return make(
            ResearchMode::None,
            ResearchContinuumMode::Auto,
            false,
            format!("skip: refers to this codebase (\"{cue}\")"),
            trimmed,
        );
    }

    // 3. Socrates autonomous gate evaluation (when available).
    if let Some(outcome) = params.socrates {
        if outcome.research_decision.should_research {
            let mode = match outcome.band {
                vox_orchestrator_types::socrates_policy::RiskBand::Low => ResearchMode::Deep,
                _ => ResearchMode::Quick,
            };
            let query_to_use = outcome
                .research_decision
                .suggested_query
                .as_deref()
                .unwrap_or(trimmed);
            return make(
                mode,
                ResearchContinuumMode::Auto,
                false,
                format!(
                    "socrates auto: {} (band: {:?}, conf: {:.2})",
                    outcome.research_decision.trigger, outcome.band, outcome.confidence
                ),
                query_to_use,
            );
        } else if outcome.band == vox_orchestrator_types::socrates_policy::RiskBand::High
            && outcome.confidence >= 0.70
        {
            return make(
                ResearchMode::None,
                ResearchContinuumMode::Auto,
                false,
                format!(
                    "socrates auto: grounded in knowledge (conf: {:.2})",
                    outcome.confidence
                ),
                trimmed,
            );
        }
    }

    // 4. Deep cues.
    let padded = format!(" {lower} ");
    if let Some(cue) = DEEP_CUES.iter().find(|c| padded.contains(*c)) {
        return make(
            ResearchMode::Deep,
            ResearchContinuumMode::Auto,
            false,
            format!("comparative / survey cue: \"{}\"", cue.trim()),
            trimmed,
        );
    }

    // 5. Quick.
    if let Some(cue) = EVIDENCE_CUES.iter().find(|c| padded.contains(*c)) {
        return make(
            ResearchMode::Quick,
            ResearchContinuumMode::Auto,
            false,
            format!("evidence request: \"{}\"", cue.trim()),
            trimmed,
        );
    }
    let is_question = trimmed.ends_with('?')
        || QUESTION_WORDS
            .iter()
            .any(|q| lower.starts_with(&format!("{q} ")));
    let time_cue = TIME_CUES
        .iter()
        .find(|c| padded.contains(&format!(" {c} ")) || padded.contains(&format!(" {c}?")))
        .map(|c| (*c).to_string())
        .or_else(|| has_recent_year(&lower).then(|| "a recent year".to_string()));
    if let (true, Some(cue)) = (is_question, time_cue) {
        return make(
            ResearchMode::Quick,
            ResearchContinuumMode::Auto,
            false,
            format!("question + time-sensitive cue: \"{cue}\""),
            trimmed,
        );
    }

    make(
        ResearchMode::None,
        ResearchContinuumMode::Auto,
        false,
        "no research cue".into(),
        trimmed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(p: &str) -> ResearchMode {
        classify_research_intent(p, None, None).mode
    }

    #[test]
    fn skips_small_talk_and_code_work() {
        for p in [
            "hi",
            "hello!",
            "thanks",
            "thank you so much",
            "ok",
            "how are you?",
            "fix the bug in crates/vox-cli/src/main.rs",
            "refactor this function to use iterators",
            "write a test for parse_config",
            "what does ```let x = 1;``` do in this file",
            "explain @crates/vox-search/src/rrf.rs",
            "rename foo to bar",
        ] {
            assert_eq!(mode(p), ResearchMode::None, "{p:?}");
        }
    }

    #[test]
    fn quick_for_time_sensitive_or_evidence_questions() {
        for p in [
            "What is the latest Gemini Flash model on OpenRouter and when was it released?",
            "what's the current version of tokio?",
            "who won the 2026 world cup?",
            "Is there any news about the Rust 2027 edition?",
            "look up the pricing for Tavily search",
            "find sources on retrieval augmented generation evaluation",
        ] {
            assert_eq!(mode(p), ResearchMode::Quick, "{p:?}");
        }
    }

    #[test]
    fn deep_for_comparative_or_survey_questions() {
        for p in [
            "compare SearXNG and Tavily for agent web search",
            "SearXNG vs Tavily for an agent: which is better?",
            "what are the trade-offs between RRF and learned rerankers?",
            "give me a deep dive on speculative decoding",
            "state of the art in long-context retrieval",
        ] {
            assert_eq!(mode(p), ResearchMode::Deep, "{p:?}");
        }
    }

    #[test]
    fn none_for_timeless_or_repo_questions() {
        for p in [
            "how do I reverse a vector in rust",
            "what does this orchestrator do",
            "explain how the chat composer works",
        ] {
            assert_eq!(mode(p), ResearchMode::None, "{p:?}");
        }
    }

    #[test]
    fn skips_questions_about_this_codebase() {
        for p in [
            "what's the latest commit on this branch?",
            "is the current implementation thread-safe?",
            "which version of serde does this crate use?",
            "what does the latest release of this crate change?",
        ] {
            assert_eq!(mode(p), ResearchMode::None, "{p:?}");
        }
        // still Quick: no local-code cue present
        assert_eq!(mode("who won the 2026 world cup?"), ResearchMode::Quick);
        assert_eq!(
            mode("what's the current version of tokio?"),
            ResearchMode::Quick
        );
    }

    #[test]
    fn local_code_cues_match_on_word_boundaries_not_substrings() {
        // "commit" must not match inside "committee" / "commitment".
        assert_eq!(
            mode("what's the latest committee decision on EU AI regulation?"),
            ResearchMode::Quick
        );
        assert_eq!(
            mode("what is the current commitment deadline for the 2026 climate pledge?"),
            ResearchMode::Quick
        );
        // plural "commits" still matches as its own cue.
        assert_eq!(
            mode("show me the latest commits on this branch?"),
            ResearchMode::None
        );
    }

    #[test]
    fn slash_commands_are_explicit_and_stripped() {
        let q = classify_research_intent("/research latest tokio release", None, None);
        assert_eq!(
            (q.mode, q.explicit, q.query.as_str()),
            (ResearchMode::Quick, true, "latest tokio release")
        );
        let d = classify_research_intent("  /deepresearch compare a and b ", None, None);
        assert_eq!(
            (d.mode, d.explicit, d.query.as_str()),
            (ResearchMode::Deep, true, "compare a and b")
        );
        // /research search stays on the local KB path, not web research
        assert_eq!(mode("/research search rrf fusion"), ResearchMode::None);
        assert_eq!(mode("/research-search rrf fusion"), ResearchMode::None);
    }

    #[test]
    fn force_flag_overrides_heuristics() {
        assert_eq!(
            classify_research_intent("hi", Some(true), None).mode,
            ResearchMode::Quick
        );
        assert_eq!(
            classify_research_intent("hi", Some(true), Some("deep")).mode,
            ResearchMode::Deep
        );
        assert_eq!(
            classify_research_intent("what is the latest tokio?", Some(false), None).mode,
            ResearchMode::None
        );
    }

    #[test]
    fn every_decision_carries_a_reason() {
        for p in [
            "hi",
            "latest tokio version?",
            "compare a vs b please",
            "how do I sort",
        ] {
            assert!(
                !classify_research_intent(p, None, None).reasons.is_empty(),
                "{p:?}"
            );
        }
    }

    #[test]
    fn scope_none_disables_research() {
        assert_eq!(
            classify_research_intent("latest tokio release", None, Some("none")).mode,
            ResearchMode::None
        );
    }

    #[test]
    fn socrates_eval_guides_auto_research_level() {
        use vox_orchestrator::socrates::SocratesGateOutcome;
        use vox_orchestrator_types::socrates_policy::{
            RiskBand, RiskDecision, SocratesResearchDecision,
        };

        // When Socrates flags low risk / abstain band, escalate to Deep research
        let deep_outcome = SocratesGateOutcome {
            decision: RiskDecision::Abstain,
            confidence: 0.20,
            contradiction_ratio: 0.10,
            band: RiskBand::Low,
            research_decision: SocratesResearchDecision {
                should_research: true,
                trigger: "Insufficient Evidence (Abstain Band)".into(),
                suggested_query: Some("canonical tokio internals".into()),
            },
        };
        let intent = classify_research_intent_with_socrates(
            "explain tokio internals",
            None,
            Some("auto"),
            Some(&deep_outcome),
        );
        assert_eq!(intent.mode, ResearchMode::Deep);
        assert_eq!(intent.query, "canonical tokio internals");
        assert!(intent.reasons[0].contains("socrates auto: Insufficient Evidence"));

        // When Socrates flags clarification band, route to Quick research
        let quick_outcome = SocratesGateOutcome {
            decision: RiskDecision::Ask,
            confidence: 0.45,
            contradiction_ratio: 0.05,
            band: RiskBand::Medium,
            research_decision: SocratesResearchDecision {
                should_research: true,
                trigger: "Weak Evidence (Clarification Band)".into(),
                suggested_query: None,
            },
        };
        let intent_quick = classify_research_intent_with_socrates(
            "explain tokio internals",
            None,
            Some("auto"),
            Some(&quick_outcome),
        );
        assert_eq!(intent_quick.mode, ResearchMode::Quick);
        assert!(intent_quick.reasons[0].contains("socrates auto: Weak Evidence"));

        // When Socrates has high confidence and grounded knowledge, skip research
        let confident_outcome = SocratesGateOutcome {
            decision: RiskDecision::Answer,
            confidence: 0.85,
            contradiction_ratio: 0.0,
            band: RiskBand::High,
            research_decision: SocratesResearchDecision {
                should_research: false,
                trigger: "Sufficient Confidence".into(),
                suggested_query: None,
            },
        };
        let intent_none = classify_research_intent_with_socrates(
            "explain tokio internals",
            None,
            Some("auto"),
            Some(&confident_outcome),
        );
        assert_eq!(intent_none.mode, ResearchMode::None);
        assert!(intent_none.reasons[0].contains("grounded in knowledge"));
    }

    #[test]
    fn continuum_params_preserve_mode_scope_domain_and_clamp_waves() {
        let deep_local = classify_research_intent_from_params(ResearchIntentParams {
            prompt: "audit crag architecture",
            research_mode: Some("deep"),
            research_scope: Some("local"),
            domain_mode: Some("scholarly"),
            site_scope: Some("arxiv.org"),
            waves: Some(99),
            ..Default::default()
        });
        assert_eq!(deep_local.mode, ResearchMode::Deep);
        assert_eq!(deep_local.continuum_mode, ResearchContinuumMode::Deep);
        assert_eq!(
            deep_local.initial_tier,
            ResearchContinuumTier::DeepMultiWave
        );
        assert_eq!(deep_local.scope, ResearchScope::Local);
        assert_eq!(deep_local.domain_mode, ResearchDomainMode::Scholarly);
        assert_eq!(deep_local.site_scope.as_deref(), Some("arxiv.org"));
        assert_eq!(deep_local.waves, 5);

        let zero_waves = classify_research_intent_from_params(ResearchIntentParams {
            prompt: "audit crag",
            research_mode: Some("fast"),
            research_scope: Some("both"),
            domain_mode: Some("codegen"),
            waves: Some(0),
            ..Default::default()
        });
        assert_eq!(zero_waves.mode, ResearchMode::Quick);
        assert_eq!(zero_waves.continuum_mode, ResearchContinuumMode::Fast);
        assert_eq!(zero_waves.initial_tier, ResearchContinuumTier::QuickWeb);
        assert_eq!(zero_waves.scope, ResearchScope::Both);
        assert_eq!(zero_waves.domain_mode, ResearchDomainMode::CodeGen);
        assert_eq!(zero_waves.waves, 1);

        let deep_default_waves = classify_research_intent_from_params(ResearchIntentParams {
            prompt: "survey prior art",
            research_mode: Some("deep"),
            ..Default::default()
        });
        assert_eq!(deep_default_waves.waves, 3);

        let off = classify_research_intent_from_params(ResearchIntentParams {
            prompt: "compare a vs b",
            research_mode: Some("off"),
            force_research: Some(true),
            ..Default::default()
        });
        assert_eq!(off.mode, ResearchMode::None);
        assert_eq!(off.continuum_mode, ResearchContinuumMode::Off);
        assert_eq!(off.initial_tier, ResearchContinuumTier::Direct);

        // Greetings in auto mode stay on Direct (must not auto-trigger CRAG).
        let greeting = classify_research_intent_from_params(ResearchIntentParams {
            prompt: "hi",
            research_mode: Some("auto"),
            ..Default::default()
        });
        assert_eq!(greeting.mode, ResearchMode::None);
        assert_eq!(greeting.initial_tier, ResearchContinuumTier::Direct);

        // Non-trivial general auto turn starts at LocalRag when no cues force quick/deep.
        let general_auto = classify_research_intent_from_params(ResearchIntentParams {
            prompt: "summarize our internal policy document",
            research_mode: Some("auto"),
            ..Default::default()
        });
        assert_eq!(general_auto.mode, ResearchMode::None);
        assert_eq!(general_auto.initial_tier, ResearchContinuumTier::LocalRag);
    }
}
