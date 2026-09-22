//! Chat research turn: typed trace, quick-research retrieval, citation check (spec §4.3, §4.6).

use std::collections::{BTreeSet, HashSet};
use std::time::Instant;

use serde::Serialize;
use serde_json::{Value, json};
use vox_search::memory_hybrid::HybridSearchHit;
use vox_search::web_dispatcher::{ProviderStatus, WebSearchDispatcher};

use super::research_intent::{ResearchIntent, ResearchMode};

#[derive(Debug, Clone, Serialize)]
pub struct StageRecord {
    pub stage: &'static str,
    pub status: &'static str,
    pub elapsed_ms: Option<u64>,
    pub summary: String,
    pub detail: Value,
}

impl StageRecord {
    pub fn new(
        stage: &'static str,
        status: &'static str,
        elapsed_ms: Option<u64>,
        summary: String,
        detail: Value,
    ) -> Self {
        Self {
            stage,
            status,
            elapsed_ms,
            summary,
            detail,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub n: usize,
    pub url: String,
    pub title: String,
    pub engine: String,
    pub snippet: String,
}

#[derive(Debug)]
pub struct ResearchTrace {
    pub intent: ResearchIntent,
    pub stages: Vec<StageRecord>,
    pub sources: Vec<Source>,
    pub model: Option<String>,
    started: Instant,
}

impl ResearchTrace {
    pub fn new(intent: ResearchIntent) -> Self {
        let detection = StageRecord::new(
            "detection",
            "ok",
            None,
            format!("{} — {}", mode_str(intent.mode), intent.reasons.join("; ")),
            json!({ "explicit": intent.explicit, "reasons": intent.reasons }),
        );
        Self {
            intent,
            stages: vec![detection],
            sources: Vec::new(),
            model: None,
            started: Instant::now(),
        }
    }

    pub fn push(&mut self, s: StageRecord) {
        self.stages.push(s);
    }

    pub fn to_event(&self) -> Value {
        let status = if self.intent.mode == ResearchMode::None {
            "skipped"
        } else if self.stages.iter().any(|s| s.status == "failed") {
            "failed"
        } else if self.stages.iter().any(|s| s.status == "degraded") {
            "degraded"
        } else {
            "ok"
        };
        json!({
            "kind": "research_trace",
            "mode": mode_str(self.intent.mode),
            "explicit": self.intent.explicit,
            "reasons": self.intent.reasons,
            "query": self.intent.query,
            "source_count": self.sources.len(),
            "model": self.model,
            "total_ms": self.started.elapsed().as_millis() as u64,
            "status": status,
            "stages": self.stages,
            "sources": self.sources.iter().map(|s| json!({"n": s.n, "url": s.url, "title": s.title, "engine": s.engine})).collect::<Vec<_>>(),
        })
    }
}

fn mode_str(m: ResearchMode) -> &'static str {
    match m {
        ResearchMode::None => "none",
        ResearchMode::Quick => "quick",
        ResearchMode::Deep => "deep",
    }
}

pub fn sources_from_hits(hits: &[HybridSearchHit], max: usize) -> Vec<Source> {
    let mut seen = HashSet::new();
    hits.iter()
        .filter(|h| h.path.starts_with("http://") || h.path.starts_with("https://"))
        .filter(|h| seen.insert(h.path.clone()))
        .take(max)
        .enumerate()
        .map(|(i, h)| Source {
            n: i + 1,
            url: h.path.clone(),
            title: h.title.clone(),
            engine: h
                .provenance
                .iter()
                .find_map(|p| p.strip_prefix("engine:"))
                .unwrap_or("unknown")
                .to_string(),
            snippet: h.content_snippet.chars().take(600).collect(),
        })
        .collect()
}

pub fn sources_context_block(sources: &[Source]) -> String {
    if sources.is_empty() {
        return "[WEB RESEARCH — 0 SOURCES]\nWeb research ran for this question and returned no sources. \
                Tell the user plainly that the search found nothing; do not invent citations or claim \
                to have found evidence.\n"
            .to_string();
    }
    let mut out = format!(
        "[WEB RESEARCH — {} SOURCES]\nAnswer the user's question from these sources and cite every factual \
         claim inline as [n] using the numbers below. If the sources do not answer the question, say so \
         explicitly instead of guessing.\n",
        sources.len()
    );
    for s in sources {
        out.push_str(&format!(
            "\n[{}] {} — {} ({})\n{}\n",
            s.n,
            s.title,
            s.url,
            s.engine,
            s.snippet.replace('\n', " ")
        ));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CitationCheck {
    pub cited: Vec<usize>,
    pub invalid: Vec<usize>,
}

pub fn check_citations(answer: &str, source_count: usize) -> CitationCheck {
    let mut cited = BTreeSet::new();
    let mut invalid = BTreeSet::new();
    let mut rest = answer;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        let inner = &after[..close];
        let nums: Vec<Option<usize>> = inner
            .split(',')
            .map(|t| t.trim().parse::<usize>().ok())
            .collect();
        if !nums.is_empty() && nums.iter().all(Option::is_some) {
            for n in nums.into_iter().flatten() {
                if (1..=source_count).contains(&n) {
                    cited.insert(n);
                } else {
                    invalid.insert(n);
                }
            }
        }
        rest = &after[close + 1..];
    }
    CitationCheck {
        cited: cited.into_iter().collect(),
        invalid: invalid.into_iter().collect(),
    }
}

pub fn citation_stage(check: &CitationCheck, source_count: usize) -> StageRecord {
    let status = if !check.invalid.is_empty() || (source_count > 0 && check.cited.is_empty()) {
        "failed"
    } else {
        "ok"
    };
    StageRecord::new(
        "citation_check",
        status,
        None,
        format!(
            "cited {}/{} sources{}",
            check.cited.len(),
            source_count,
            if check.invalid.is_empty() {
                String::new()
            } else {
                format!(", invalid markers {:?}", check.invalid)
            }
        ),
        json!(check),
    )
}

/// Quick research: one retrieval wave on the Deep-lane deadline, numbered sources.
/// Returns the context block to inject into the chat prompt.
pub async fn run_quick(state: &crate::ServerState, trace: &mut ResearchTrace) -> String {
    let policy = {
        let cfg = state.orchestrator.config_handle();
        vox_orchestrator::sync_lock::rw_read(&*cfg).effective_search_policy()
    };
    let query = trace.intent.query.clone();
    trace.push(StageRecord::new(
        "queries",
        "ok",
        None,
        format!("1 query: {query}"),
        json!({ "queries": [query] }),
    ));

    let t = Instant::now();
    // Deep-lane deadline: SearXNG does not fit the 1.5 s fast lane (spec §9).
    let report = WebSearchDispatcher::search_with_report(
        &query,
        vox_search::policy::ResearchLane::Deep,
        &policy,
    )
    .await;
    let answered = report
        .providers
        .iter()
        .filter(|p| matches!(p.status, ProviderStatus::Ok { hits } if hits > 0))
        .count();
    let failed = report.providers.iter().any(|p| {
        matches!(
            p.status,
            ProviderStatus::Error { .. } | ProviderStatus::Timeout
        )
    });
    let status = match (report.hits.is_empty(), failed) {
        (true, true) => "failed",
        (true, false) => "empty",
        (false, true) => "degraded",
        (false, false) => "ok",
    };
    trace.push(StageRecord::new(
        "retrieval",
        status,
        Some(t.elapsed().as_millis() as u64),
        format!(
            "{} hits from {answered}/{} providers",
            report.hits.len(),
            report.providers.len()
        ),
        json!({ "providers": report.providers }),
    ));

    trace.sources = sources_from_hits(&report.hits, 8);
    trace.push(StageRecord::new(
        "sources",
        if trace.sources.is_empty() {
            "empty"
        } else {
            "ok"
        },
        None,
        format!("{} web sources kept", trace.sources.len()),
        json!({ "sources": trace.sources }),
    ));
    sources_context_block(&trace.sources)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_tools::chat::research_intent::classify_research_intent;
    use vox_search::memory_hybrid::HybridSearchHit;

    fn hit(url: &str, engine: &str) -> HybridSearchHit {
        HybridSearchHit {
            path: url.into(),
            title: format!("t {url}"),
            content_snippet: "snippet ".repeat(200),
            score: 0.5,
            provenance: vec!["WebResearch".into(), format!("engine:{engine}")],
            potential_contradiction: false,
        }
    }

    #[test]
    fn sources_are_numbered_deduped_web_only_and_capped() {
        let hits = vec![
            hit("https://a.example/1", "searxng"),
            hit("https://a.example/1", "wikipedia"),
            hit("docs/internal.md", "bm25"),
            hit("https://b.example/2", "wikipedia"),
            hit("https://c.example/3", "arxiv"),
        ];
        let s = sources_from_hits(&hits, 2);
        assert_eq!(
            s.iter().map(|x| (x.n, x.url.as_str())).collect::<Vec<_>>(),
            vec![(1, "https://a.example/1"), (2, "https://b.example/2")]
        );
        assert_eq!(s[0].engine, "searxng");
        assert!(s[0].snippet.chars().count() <= 600);
    }

    #[test]
    fn context_block_numbers_sources_and_demands_citations() {
        let s = sources_from_hits(&[hit("https://a.example/1", "searxng")], 8);
        let b = sources_context_block(&s);
        assert!(b.starts_with("[WEB RESEARCH — 1 SOURCES]"), "{b}");
        assert!(
            b.contains("[1] t https://a.example/1 — https://a.example/1 (searxng)"),
            "{b}"
        );
        assert!(b.contains("cite"), "{b}");
    }

    #[test]
    fn empty_context_block_forbids_invented_citations() {
        let b = sources_context_block(&[]);
        assert!(b.starts_with("[WEB RESEARCH — 0 SOURCES]"), "{b}");
        assert!(b.contains("do not invent citations"), "{b}");
    }

    #[test]
    fn citation_check_finds_valid_and_invalid_markers() {
        let c = check_citations("Gemini 3.8 Flash [1][3]. Also [2, 9] and [x] and [10].", 3);
        assert_eq!(c.cited, vec![1, 2, 3]);
        assert_eq!(c.invalid, vec![9, 10]);
    }

    #[test]
    fn citation_stage_fails_on_uncited_answer_with_sources() {
        let st = citation_stage(&check_citations("no markers here", 4), 4);
        assert_eq!(st.status, "failed");
        let ok = citation_stage(&check_citations("fact [1]", 4), 4);
        assert_eq!(ok.status, "ok");
    }

    #[test]
    fn trace_event_for_no_research_is_skipped_with_detection_stage() {
        let t = ResearchTrace::new(classify_research_intent("hi", None, None));
        let e = t.to_event();
        assert_eq!(e["kind"], "research_trace");
        assert_eq!(e["mode"], "none");
        assert_eq!(e["status"], "skipped");
        assert_eq!(e["stages"][0]["stage"], "detection");
        assert!(e["reasons"][0].as_str().unwrap().contains("greeting"));
    }

    #[test]
    fn trace_status_is_failed_when_any_stage_failed() {
        let mut t = ResearchTrace::new(classify_research_intent("/research x y z", None, None));
        t.push(StageRecord::new(
            "retrieval",
            "failed",
            Some(3),
            "boom".into(),
            serde_json::json!({}),
        ));
        assert_eq!(t.to_event()["status"], "failed");
    }
}
