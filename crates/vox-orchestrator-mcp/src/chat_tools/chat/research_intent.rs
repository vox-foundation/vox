//! Deterministic research-intent classifier for chat turns (spec §4.1).
//! Replaces the confidence-gate heuristic that fired web research on "hi" (D1).

use serde::Serialize;

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
    pub explicit: bool,
    pub reasons: Vec<String>,
    pub query: String,
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

fn intent(mode: ResearchMode, explicit: bool, reason: String, query: &str) -> ResearchIntent {
    ResearchIntent {
        mode,
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
    let trimmed = prompt.trim();
    let lower = trimmed.to_ascii_lowercase();

    // 1. Explicit.
    if let Some(rest) = strip_command(trimmed, "/deepresearch") {
        return intent(
            ResearchMode::Deep,
            true,
            "explicit: /deepresearch".into(),
            rest,
        );
    }
    if lower.starts_with("/research search")
        || lower.starts_with("/research-search")
        || lower.starts_with("/research --search")
    {
        return intent(
            ResearchMode::None,
            true,
            "local knowledge-base search (/research search)".into(),
            trimmed,
        );
    }
    if let Some(rest) = strip_command(trimmed, "/research") {
        return intent(
            ResearchMode::Quick,
            true,
            "explicit: /research".into(),
            rest,
        );
    }
    match force {
        Some(true) => {
            let deep = scope.is_some_and(|s| s.eq_ignore_ascii_case("deep"));
            let mode = if deep {
                ResearchMode::Deep
            } else {
                ResearchMode::Quick
            };
            return intent(mode, true, "explicit: force_research".into(), trimmed);
        }
        Some(false) => {
            return intent(
                ResearchMode::None,
                true,
                "explicit: force_research=false".into(),
                trimmed,
            );
        }
        None => {}
    }

    // 2. Skip.
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
        return intent(
            ResearchMode::None,
            false,
            "skip: greeting / small talk".into(),
            trimmed,
        );
    }
    if words.len() < 3 {
        return intent(
            ResearchMode::None,
            false,
            "skip: fewer than 3 words".into(),
            trimmed,
        );
    }
    if trimmed.contains("```") || words.iter().any(|w| looks_like_path(w)) {
        return intent(
            ResearchMode::None,
            false,
            "skip: references code or files".into(),
            trimmed,
        );
    }
    if CODE_VERBS.iter().any(|v| lower.starts_with(v)) {
        return intent(
            ResearchMode::None,
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
        return intent(
            ResearchMode::None,
            false,
            format!("skip: refers to this codebase (\"{cue}\")"),
            trimmed,
        );
    }

    // 3. Deep.
    let padded = format!(" {lower} ");
    if let Some(cue) = DEEP_CUES.iter().find(|c| padded.contains(*c)) {
        return intent(
            ResearchMode::Deep,
            false,
            format!("comparative / survey cue: \"{}\"", cue.trim()),
            trimmed,
        );
    }

    // 4. Quick.
    if let Some(cue) = EVIDENCE_CUES.iter().find(|c| padded.contains(*c)) {
        return intent(
            ResearchMode::Quick,
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
        return intent(
            ResearchMode::Quick,
            false,
            format!("question + time-sensitive cue: \"{cue}\""),
            trimmed,
        );
    }

    intent(ResearchMode::None, false, "no research cue".into(), trimmed)
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
}
