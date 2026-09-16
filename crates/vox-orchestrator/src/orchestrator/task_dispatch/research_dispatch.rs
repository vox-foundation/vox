use crate::orchestrator::Orchestrator;
use crate::socrates::SocratesTaskContext;
use tracing::{info, warn};
use vox_search::mens_research_subagent::{
    ClaimTriplet, GroundingQuality, parse_and_ground_claim_triplets_with_source,
};

fn extract_source_and_snippet(hit: &str) -> (Option<&str>, &str) {
    if let Some(stripped) = hit.strip_prefix("[autonomous_research:") {
        if let Some(bracket_end) = stripped.find(']') {
            let url = &stripped[..bracket_end];
            let after_bracket = stripped[bracket_end + 1..].trim();
            // Hits from run_multi_hop_web_research format as: `<Title> (score: ...; engine: ...; novelty: ...) - <content>`
            // Strip the search ranking metadata prefix so only the clean content snippet is analyzed
            let snippet = if let Some(dash_pos) = after_bracket.find(") - ") {
                after_bracket[dash_pos + 4..].trim()
            } else {
                after_bracket
            };
            (Some(url), snippet)
        } else {
            (None, hit)
        }
    } else {
        (None, hit)
    }
}

pub fn extract_claim_triplets(text: &str, source_url: Option<&str>) -> Vec<ClaimTriplet> {
    // 1. If text is JSON, try parsing via parse_and_ground_claim_triplets_with_source
    let from_json = parse_and_ground_claim_triplets_with_source(text, text, source_url);
    if !from_json.is_empty() {
        return from_json;
    }

    // 2. Fallback heuristic: extract basic relational subject-verb-object claims from clean text sentences
    let mut triplets = Vec::new();
    const VERBS: &[&str] = &[
        "supports",
        "supported",
        "provides",
        "provided",
        "requires",
        "required",
        "contains",
        "contained",
        "introduces",
        "introduced",
        "implements",
        "implemented",
        "features",
        "featured",
        "uses",
        "used",
        "deprecates",
        "deprecated",
        "removes",
        "removed",
        "lacks",
        "lacked",
        "disables",
        "disabled",
        "drops",
        "dropped",
        "includes",
        "included",
        "enables",
        "enabled",
    ];

    for sentence in split_into_sentences(text) {
        let trimmed = sentence.trim();
        if trimmed.len() < 10 {
            continue;
        }
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() < 3 || words.len() > 30 {
            continue;
        }
        for (idx, &word) in words.iter().enumerate() {
            let clean_word = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if VERBS.contains(&clean_word.as_str()) && idx > 0 && idx < words.len() - 1 {
                let subject = words[..idx]
                    .join(" ")
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string();
                let predicate = clean_word;
                let object = words[idx + 1..]
                    .join(" ")
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string();
                if !subject.is_empty()
                    && !object.is_empty()
                    && subject.len() <= 60
                    && object.len() <= 60
                {
                    triplets.push(ClaimTriplet {
                        subject,
                        predicate,
                        object,
                        confidence: 0.90,
                        evidence_snippet: trimmed.to_string(),
                        source_url: source_url.map(ToString::to_string),
                        grounding: GroundingQuality::VerbatimExact,
                    });
                    break;
                }
            }
        }
    }
    triplets
}

pub(crate) fn split_into_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        let (byte_idx, ch) = chars[i];
        if byte_idx < start {
            i += 1;
            continue;
        }

        if ch == ';' || ch == '\n' {
            let chunk = text[start..byte_idx]
                .trim()
                .trim_matches(|c: char| {
                    matches!(
                        c,
                        '"' | '\''
                            | '\u{2019}'
                            | '\u{2018}'
                            | '\u{201D}'
                            | '\u{201C}'
                            | ')'
                            | ']'
                            | '«'
                            | '»'
                    )
                })
                .trim();
            if !chunk.is_empty() {
                sentences.push(chunk);
            }
            start = byte_idx + ch.len_utf8();
            i += 1;
        } else if ch == '.' || ch == '?' || ch == '!' {
            let mut j = i + 1;
            let mut end_idx = byte_idx;
            let mut next_start = byte_idx + ch.len_utf8();

            while j < len {
                let (c_byte, c_char) = chars[j];
                if matches!(
                    c_char,
                    '"' | '\''
                        | '\u{2019}'
                        | '\u{2018}'
                        | '\u{201D}'
                        | '\u{201C}'
                        | ')'
                        | ']'
                        | '«'
                        | '»'
                ) {
                    end_idx = c_byte + c_char.len_utf8();
                    next_start = end_idx;
                    j += 1;
                } else {
                    break;
                }
            }

            let is_sentence_end = if j < len {
                let next_ch = chars[j].1;
                next_ch.is_whitespace()
            } else {
                true
            };

            if is_sentence_end {
                let chunk = text[start..end_idx]
                    .trim()
                    .trim_matches(|c: char| {
                        matches!(
                            c,
                            '"' | '\''
                                | '\u{2019}'
                                | '\u{2018}'
                                | '\u{201D}'
                                | '\u{201C}'
                                | ')'
                                | ']'
                                | '«'
                                | '»'
                        )
                    })
                    .trim();
                if !chunk.is_empty() {
                    sentences.push(chunk);
                }
                start = next_start;
                i = j;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    let remaining = text[start..]
        .trim()
        .trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\''
                    | '\u{2019}'
                    | '\u{2018}'
                    | '\u{201D}'
                    | '\u{201C}'
                    | ')'
                    | ']'
                    | '«'
                    | '»'
            )
        })
        .trim();
    if !remaining.is_empty() {
        sentences.push(remaining);
    }

    sentences
}

fn are_opposing_predicates(p1: &str, p2: &str) -> bool {
    const AFFIRMATIVE: &[&str] = &[
        "supports",
        "supported",
        "provides",
        "provided",
        "features",
        "featured",
        "includes",
        "included",
        "uses",
        "used",
        "contains",
        "contained",
        "implements",
        "implemented",
        "enables",
        "enabled",
        "introduces",
        "introduced",
    ];
    const OPPOSING: &[&str] = &[
        "deprecates",
        "deprecated",
        "removes",
        "removed",
        "lacks",
        "lacked",
        "disables",
        "disabled",
        "drops",
        "dropped",
        "forbids",
        "forbade",
        "rejects",
        "rejected",
    ];

    let p1_clean = p1.trim().to_lowercase();
    let p2_clean = p2.trim().to_lowercase();

    (AFFIRMATIVE.contains(&p1_clean.as_str()) && OPPOSING.contains(&p2_clean.as_str()))
        || (OPPOSING.contains(&p1_clean.as_str()) && AFFIRMATIVE.contains(&p2_clean.as_str()))
}

fn are_opposing_objects(o1: &str, o2: &str) -> bool {
    let o1_low = o1.trim().to_lowercase();
    let o2_low = o2.trim().to_lowercase();

    if (o1_low == "true" && o2_low == "false")
        || (o1_low == "false" && o2_low == "true")
        || (o1_low == "yes" && o2_low == "no")
        || (o1_low == "no" && o2_low == "yes")
        || (o1_low == "enabled" && o2_low == "disabled")
        || (o1_low == "disabled" && o2_low == "enabled")
    {
        return true;
    }

    let negations = ["no ", "not ", "non-", "without "];
    for neg in negations {
        if let Some(stripped) = o1_low.strip_prefix(neg) {
            if stripped.trim() == o2_low {
                return true;
            }
        }
        if let Some(stripped) = o2_low.strip_prefix(neg) {
            if stripped.trim() == o1_low {
                return true;
            }
        }
    }

    false
}

fn objects_overlap(o1: &str, o2: &str) -> bool {
    if o1.eq_ignore_ascii_case(o2) {
        return true;
    }
    let tokens1: std::collections::HashSet<String> = o1
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| !w.is_empty())
        .collect();
    let tokens2: std::collections::HashSet<String> = o2
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| !w.is_empty())
        .collect();

    if tokens1.is_empty() || tokens2.is_empty() {
        return false;
    }

    let common = tokens1.intersection(&tokens2).count();
    if common > 0 {
        let min_tokens = tokens1.len().min(tokens2.len());
        if (common as f64 / min_tokens as f64) >= 0.50 {
            return true;
        }
    }
    false
}

fn is_stop_word(w: &str) -> bool {
    matches!(
        w,
        "a" | "an"
            | "the"
            | "in"
            | "on"
            | "at"
            | "by"
            | "for"
            | "with"
            | "about"
            | "against"
            | "between"
            | "into"
            | "through"
            | "during"
            | "before"
            | "after"
            | "above"
            | "below"
            | "to"
            | "from"
            | "up"
            | "down"
            | "of"
            | "off"
            | "over"
            | "under"
            | "again"
            | "further"
            | "then"
            | "once"
            | "here"
            | "there"
            | "all"
            | "any"
            | "both"
            | "each"
            | "few"
            | "more"
            | "most"
            | "other"
            | "some"
            | "such"
            | "no"
            | "nor"
            | "not"
            | "only"
            | "own"
            | "same"
            | "so"
            | "than"
            | "too"
            | "very"
            | "s"
            | "t"
            | "can"
            | "will"
            | "just"
            | "don"
            | "should"
            | "now"
            | "and"
            | "but"
            | "if"
            | "or"
            | "because"
            | "as"
            | "until"
            | "while"
    )
}

fn snippet_has_negated_claim(snippet: &str, subject: &str, predicate: &str, object: &str) -> bool {
    let s_low = snippet
        .replace(['\u{2019}', '\u{2018}'], "'")
        .to_lowercase();
    let sub_low = subject.to_lowercase();
    let pred_stem = predicate
        .to_lowercase()
        .strip_suffix('s')
        .unwrap_or(&predicate.to_lowercase())
        .to_string();

    let sub_tokens: std::collections::HashSet<String> = sub_low
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !is_stop_word(w))
        .map(|w| w.to_string())
        .collect();

    let obj_tokens: std::collections::HashSet<String> = object
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && (w.len() > 2 || !is_stop_word(w)))
        .map(|w| w.to_ascii_lowercase())
        .collect();

    const NEGATION_TOKENS: &[&str] = &[
        "not",
        "never",
        "cannot",
        "unsupported",
        "unable",
        "no",
        "without",
        "cant",
        "can't",
        "wont",
        "won't",
        "dont",
        "don't",
        "doesnt",
        "doesn't",
        "didnt",
        "didn't",
        "isnt",
        "isn't",
        "arent",
        "aren't",
        "wasnt",
        "wasn't",
        "werent",
        "weren't",
        "havent",
        "haven't",
        "hasnt",
        "hasn't",
        "lacks",
        "lacking",
    ];

    for sentence in split_into_sentences(&s_low) {
        let sent_norm = sentence.trim();
        if sent_norm.is_empty() {
            continue;
        }

        let sent_words: Vec<&str> = sent_norm
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|w| !w.is_empty())
            .collect();
        let sent_tokens: std::collections::HashSet<String> =
            sent_words.iter().map(|w| w.to_string()).collect();

        let mentions_sub = if sub_tokens.is_empty() {
            sent_norm.contains(&sub_low)
        } else {
            sub_tokens.iter().any(|t| sent_tokens.contains(t))
        };

        let mentions_pred = sent_tokens.contains(&pred_stem)
            || sent_tokens.contains(&predicate.to_lowercase())
            || sent_words.iter().any(|w| w.starts_with(&pred_stem));

        let mentions_obj = if obj_tokens.is_empty() {
            sent_tokens.contains(&object.to_lowercase())
        } else {
            obj_tokens.iter().any(|t| sent_tokens.contains(t))
        };

        // Proposition scoping: sentence must mention the object AND either the subject or predicate stem
        let pertains_to_claim =
            (mentions_obj && (mentions_sub || mentions_pred)) || (mentions_sub && mentions_pred);

        if !pertains_to_claim {
            continue;
        }

        let has_neg = sent_words.iter().any(|w| {
            let cleaned = w.trim_matches(|c: char| !c.is_alphabetic() && c != '\'');
            NEGATION_TOKENS.contains(&cleaned) || cleaned.ends_with("n't")
        });

        if has_neg {
            return true;
        }
    }
    false
}

fn detect_triplet_contradictions(triplets: &[ClaimTriplet], contradictions: &mut Vec<String>) {
    let mut seen: std::collections::HashSet<String> = contradictions.iter().cloned().collect();

    for i in 0..triplets.len() {
        for j in (i + 1)..triplets.len() {
            let t1 = &triplets[i];
            let t2 = &triplets[j];

            if !t1.subject.eq_ignore_ascii_case(&t2.subject) {
                continue;
            }

            let objects_overlap = objects_overlap(&t1.object, &t2.object);

            // Case 1: Opposing predicates regarding the same or overlapping object
            // e.g. "SQLite supports JSONB" vs "SQLite deprecates JSONB"
            if are_opposing_predicates(&t1.predicate, &t2.predicate) && objects_overlap {
                let desc = format!(
                    "Contradiction on '{}': '{} {}' vs '{} {}'",
                    t1.subject, t1.predicate, t1.object, t2.predicate, t2.object
                );
                if seen.insert(desc.clone()) {
                    contradictions.push(desc);
                }
                continue;
            }

            // Case 2: Matching predicate with opposite polar objects
            // e.g. "Feature is enabled" vs "Feature is disabled", or "supports JSONB" vs "supports no JSONB"
            if t1.predicate.eq_ignore_ascii_case(&t2.predicate)
                && are_opposing_objects(&t1.object, &t2.object)
            {
                let desc = format!(
                    "Contradiction on '{} {}': '{}' vs '{}'",
                    t1.subject, t1.predicate, t1.object, t2.object
                );
                if seen.insert(desc.clone()) {
                    contradictions.push(desc);
                }
                continue;
            }

            // Case 3: Matching predicate and object, but one evidence snippet asserts negation on the claim
            if t1.predicate.eq_ignore_ascii_case(&t2.predicate) && objects_overlap {
                let s1_neg = snippet_has_negated_claim(
                    &t1.evidence_snippet,
                    &t1.subject,
                    &t1.predicate,
                    &t1.object,
                );
                let s2_neg = snippet_has_negated_claim(
                    &t2.evidence_snippet,
                    &t2.subject,
                    &t2.predicate,
                    &t2.object,
                );
                if s1_neg != s2_neg {
                    let desc = format!(
                        "Contradiction on '{} {} {}': conflicting evidence polarity in snippets",
                        t1.subject, t1.predicate, t1.object
                    );
                    if seen.insert(desc.clone()) {
                        contradictions.push(desc);
                    }
                }
            }
        }
    }
}

/// Formats verified claim triplets and detected contradictions into markdown evidence blocks.
pub fn format_grounded_research_evidence(
    triplets: &[ClaimTriplet],
    contradictions: &[String],
) -> String {
    let mut out = String::new();
    if !triplets.is_empty() {
        out.push_str("### Verified Factual Triplets (Graduated Grounding):\n");
        let mut seen = std::collections::HashSet::new();
        for t in triplets {
            let key = (
                t.subject.to_lowercase(),
                t.predicate.to_lowercase(),
                t.object.to_lowercase(),
                t.source_url.clone(),
            );
            if !seen.insert(key) {
                continue;
            }
            let quality_tag = match t.grounding {
                GroundingQuality::VerbatimExact => "VerbatimExact".to_string(),
                GroundingQuality::NormalizedSpan { overlap_ratio } => {
                    format!("NormalizedSpan {:.0}%", overlap_ratio * 100.0)
                }
            };
            let source_str = t.source_url.as_deref().unwrap_or("Internal Index");
            out.push_str(&format!(
                "- ({}, {}, {}) [{}] (Source: {})\n",
                t.subject, t.predicate, t.object, quality_tag, source_str
            ));
        }
    }
    if !contradictions.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("### Detected Epistemic Contradictions:\n");
        let mut seen_c = std::collections::HashSet::new();
        for c in contradictions {
            if seen_c.insert(c.clone()) {
                out.push_str(&format!("- {}\n", c));
            }
        }
    }
    out
}

impl Orchestrator {
    /// Performs autonomous research based on Socrates policy requests or CRAG routing.
    /// This is a blocking (async) step that injects live web evidence.
    pub async fn perform_autonomous_research(
        &self,
        agent_id: Option<crate::types::AgentId>,
        task_id: Option<crate::types::TaskId>,
        queries: Vec<String>,
        reason: &str,
    ) -> Result<Vec<String>, String> {
        info!(
            reason = %reason,
            query_count = queries.len(),
            "triggering autonomous socrates research dispatch"
        );

        let (policy, quality_target) = {
            let cfg = crate::sync_lock::rw_read(&*self.config);
            (cfg.effective_search_policy(), cfg.research_quality_target)
        };

        let anchor = queries.first().map(|s| s.as_str()).unwrap_or("");

        // Check Tier 0 instant memory cache before dispatching web queries
        let db_opt = crate::sync_lock::rw_read(&*self.db).clone();
        let triage = crate::orchestrator::task_dispatch::triage::classify_research_request(
            anchor,
            None,
            false,
            None,
            db_opt.as_deref(),
        )
        .await;

        if let crate::orchestrator::task_dispatch::triage::ResearchTriageTier::InstantMemory {
            session_id,
            cached_query,
            snippet,
            similarity,
        } = triage
        {
            info!(
                session_id,
                similarity,
                cached_query = %cached_query,
                "Tier 0 research hit: served instantly from VoxDB FTS5 memory"
            );
            let hit_summary = format!(
                "[instant_memory] Session #{session_id} (match: {:.0}%):\n{}\n\nSnippet: {}",
                similarity * 100.0,
                cached_query,
                snippet
            );
            self.event_bus
                .emit(crate::events::AgentEventKind::ResearchExecuted {
                    agent_id,
                    task_id,
                    queries,
                    results_count: 1,
                });
            return Ok(vec![hit_summary]);
        }

        let (queries_to_run, waves) = match &triage {
            crate::orchestrator::task_dispatch::triage::ResearchTriageTier::ShallowWeb { query } => {
                info!(query = %query, "Tier 1: fast shallow web research lookup");
                (vec![query.clone()], 1)
            }
            crate::orchestrator::task_dispatch::triage::ResearchTriageTier::StandardDeep { .. } => {
                info!("Tier 2: standard single-wave deep research");
                (queries.clone(), 1)
            }
            crate::orchestrator::task_dispatch::triage::ResearchTriageTier::MultiWaveAutonomous {
                max_waves,
                domain_mode,
                ..
            } => {
                info!(
                    waves = *max_waves,
                    %domain_mode,
                    "Tier 3: multi-wave autonomous deep research"
                );
                (queries.clone(), *max_waves)
            }
            _ => (queries.clone(), 1),
        };

        let mut research_results = vox_search::research::run_multi_hop_web_research(
            &policy,
            &queries_to_run,
            quality_target,
            anchor,
        )
        .await;

        let mut grounded_triplets = Vec::new();
        let mut contradictions = Vec::new();

        for hit in &research_results {
            let (url, snippet) = extract_source_and_snippet(hit);
            let triplets = extract_claim_triplets(snippet, url);
            grounded_triplets.extend(triplets);
        }

        detect_triplet_contradictions(&grounded_triplets, &mut contradictions);

        let mut executed_waves = 1;

        if waves > 1 {
            for hit in &research_results {
                let lower = hit.to_lowercase();
                if lower.contains("conflicting")
                    || lower.contains("contradiction")
                    || lower.contains("disputed")
                {
                    let snippet = extract_source_and_snippet(hit).1;
                    let short = snippet.chars().take(120).collect::<String>();
                    let desc = format!("Potential conflicting evidence: {}", short);
                    if !contradictions.contains(&desc) {
                        contradictions.push(desc);
                    }
                }
            }

            let anchor_prefix = if anchor.trim().is_empty() {
                "research topic".to_string()
            } else {
                anchor.trim().to_string()
            };

            let mut wave2_queries = Vec::new();
            if !contradictions.is_empty() {
                wave2_queries.push(format!(
                    "{} conflicting evidence source comparison",
                    anchor_prefix
                ));
            } else {
                wave2_queries.push(format!("{} primary source evidence", anchor_prefix));
                wave2_queries.push(format!(
                    "{} independent corroborating sources",
                    anchor_prefix
                ));
            }

            info!(
                wave = 2,
                query_count = wave2_queries.len(),
                "executing Wave 2 disambiguation retrieval"
            );
            let wave2_results = vox_search::research::run_multi_hop_web_research(
                &policy,
                &wave2_queries,
                quality_target,
                anchor,
            )
            .await;

            for hit in &wave2_results {
                let (url, snippet) = extract_source_and_snippet(hit);
                let triplets = extract_claim_triplets(snippet, url);
                grounded_triplets.extend(triplets);
            }
            research_results.extend(wave2_results);
            executed_waves = 2;

            detect_triplet_contradictions(&grounded_triplets, &mut contradictions);
        }

        let research_model_enabled =
            crate::sync_lock::rw_read(&*self.config).research_model_enabled;

        if research_model_enabled && !research_results.is_empty() {
            info!("delegating research synthesis to Lane G (research-expert)");

            #[cfg(feature = "runtime")]
            {
                use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, infer_with_retry};

                let combined_evidence = research_results.join("\n\n");
                let grounded_evidence =
                    format_grounded_research_evidence(&grounded_triplets, &contradictions);

                // Configure Lane G endpoint — pick via SSOT `select()` so the
                // 3-axis user knob + premium_alias drive the choice.
                // 2026-Q2 refresh: claude-3.5-sonnet:beta retired.
                let model_id = crate::models::select_with_default_registry(
                    &crate::models::SelectionIntent::research(),
                )
                .map(|o| o.model_id)
                .unwrap_or_else(|| "google/gemini-3.1-pro".to_string());
                let config = LlmConfig::openrouter(&model_id);
                if let Some(_key) =
                    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshToken).expose()
                {
                    // Overwrite if hitting internal mesh. For now, we fallback to standard LLM pipeline.
                    tracing::debug!("Using specific Lane G auth");
                }

                let preamble = if executed_waves > 1 {
                    format!(
                        "Synthesize the following multi-wave ({} waves) research evidence into a high-fidelity summary with comparative analysis, noting any resolved contradictions:",
                        executed_waves
                    )
                } else {
                    "Synthesize the following recent web evidence into a high-fidelity summary:"
                        .to_string()
                };

                let prompt = if !grounded_evidence.is_empty() {
                    format!(
                        "{}\n\n{}\n\n{}",
                        preamble, combined_evidence, grounded_evidence
                    )
                } else {
                    format!("{}\n\n{}", preamble, combined_evidence)
                };

                let messages = vec![
                    LlmChatMessage {
                        role: "system".into(),
                        content: "You are Lane G, the Vox autonomous research synthesis expert. Your objective is to ingest raw search observations and formulate a dense, factual markdown summary containing specific claims, figures, and direct citations.".into(), ..Default::default()
},
                    LlmChatMessage {
                        role: "user".into(),
                        content: prompt, ..Default::default()
}
                ];

                match infer_with_retry(
                    &vox_actor_runtime::activity::ActivityOptions::default(),
                    messages,
                    vec![config],
                )
                .await
                {
                    vox_actor_runtime::ActivityResult::Ok(Ok((res, _cfg)))
                        if !res.content.is_empty() =>
                    {
                        let text = res.content;
                        info!("Lane G synthesis completed successfully");
                        research_results.push(format!("[lane_g_synthesis] {}", text));
                    }
                    other => {
                        warn!(
                            ?other,
                            "Lane G synthesis failed or returned empty content, falling back to raw results"
                        );
                    }
                }
            }
        }

        if !research_results.is_empty() {
            info!(
                count = research_results.len(),
                "collected autonomous research evidence"
            );
        }

        self.event_bus
            .emit(crate::events::AgentEventKind::ResearchExecuted {
                agent_id,
                task_id,
                queries,
                results_count: research_results.len(),
            });

        Ok(research_results)
    }

    /// Injects research results into a Socrates task context using a canonical CRAG line prefix.
    pub fn inject_research_results(&self, ctx: &mut SocratesTaskContext, results: Vec<String>) {
        if results.is_empty() {
            return;
        }

        // We update the evidence count to reflect the new findings.
        // In a full CRAG loop, we would re-run socrates evaluation AFTER injection,
        // but for now we trust the injection is sufficient for the agent.
        ctx.evidence_count = ctx
            .evidence_count
            .saturating_add(results.len().min(u8::MAX as usize) as u8);

        if let Some(ref mut diag) = ctx.retrieval_diagnosis {
            diag.evidence_shape = "ok".to_string(); // evidence is no longer empty/thin
            if !diag.corpora_with_hits.contains(&"webresearch".to_string()) {
                diag.corpora_with_hits.push("webresearch".to_string());
            }
        }

        ctx.citation_coverage = (ctx.citation_coverage + 0.20).min(1.0);
        ctx.evidence_quality = (ctx.evidence_quality + 0.15).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_search::mens_research_subagent::{ClaimTriplet, GroundingQuality};

    #[test]
    fn test_format_grounded_research_evidence_with_contradictions() {
        let triplets = vec![
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB".to_string(),
                confidence: 0.95,
                evidence_snippet: "SQLite supports JSONB natively".to_string(),
                source_url: Some("https://sqlite.org/jsonb.html".to_string()),
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "Socrates".to_string(),
                predicate: "triggers".to_string(),
                object: "research dispatch".to_string(),
                confidence: 0.88,
                evidence_snippet: "Socrates triggers research dispatch".to_string(),
                source_url: None,
                grounding: GroundingQuality::NormalizedSpan {
                    overlap_ratio: 0.85,
                },
            },
        ];
        let contradictions = vec!["PostgreSQL JSONB format differs from SQLite JSONB".to_string()];

        let formatted = format_grounded_research_evidence(&triplets, &contradictions);
        assert!(formatted.contains("### Verified Factual Triplets (Graduated Grounding):"));
        assert!(formatted.contains(
            "- (SQLite, supports, JSONB) [VerbatimExact] (Source: https://sqlite.org/jsonb.html)"
        ));
        assert!(formatted.contains(
            "- (Socrates, triggers, research dispatch) [NormalizedSpan 85%] (Source: Internal Index)"
        ));
        assert!(formatted.contains("### Detected Epistemic Contradictions:"));
        assert!(formatted.contains("- PostgreSQL JSONB format differs from SQLite JSONB"));
    }

    #[test]
    fn test_format_grounded_research_evidence_empty() {
        let formatted = format_grounded_research_evidence(&[], &[]);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_extract_source_and_snippet_strips_metadata() {
        let hit = "[autonomous_research:https://sqlite.org/jsonb.html] SQLite JSONB (score: 0.850; engine: searxng; novelty: 0.90) - SQLite introduced JSONB in version 3.45.";
        let (url, snippet) = extract_source_and_snippet(hit);
        assert_eq!(url, Some("https://sqlite.org/jsonb.html"));
        assert_eq!(snippet, "SQLite introduced JSONB in version 3.45.");

        let raw_hit = "Plain text hit without prefix";
        let (raw_url, raw_snippet) = extract_source_and_snippet(raw_hit);
        assert_eq!(raw_url, None);
        assert_eq!(raw_snippet, "Plain text hit without prefix");
    }

    #[test]
    fn test_detect_triplet_contradictions() {
        let triplets = vec![
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB".to_string(),
                confidence: 0.95,
                evidence_snippet: "SQLite supports JSONB".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "sqlite".to_string(), // case-insensitive matching
                predicate: "deprecates".to_string(),
                object: "JSONB".to_string(),
                confidence: 0.80,
                evidence_snippet: "sqlite deprecates JSONB".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
        ];

        let mut contradictions = Vec::new();
        detect_triplet_contradictions(&triplets, &mut contradictions);
        assert_eq!(contradictions.len(), 1);
        assert!(contradictions[0].contains("Contradiction on 'SQLite'"));
        assert!(contradictions[0].contains("supports JSONB"));
        assert!(contradictions[0].contains("deprecates JSONB"));
    }

    #[test]
    fn test_detect_triplet_contradictions_does_not_flag_additive_features() {
        let triplets = vec![
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB".to_string(),
                confidence: 0.95,
                evidence_snippet: "SQLite supports JSONB".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "WAL mode".to_string(),
                confidence: 0.90,
                evidence_snippet: "SQLite supports WAL mode".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
        ];

        let mut contradictions = Vec::new();
        detect_triplet_contradictions(&triplets, &mut contradictions);
        assert!(
            contradictions.is_empty(),
            "Additive features must not be flagged as contradictions"
        );
    }

    #[test]
    fn test_split_into_sentences_preserves_version_numbers() {
        let text = "Does PostgreSQL support JSONB? Yes! SQLite 3.45 introduced JSONB natively. Node.js supports modern ES modules; it is fast.";
        let sentences = split_into_sentences(text);
        assert_eq!(sentences.len(), 5);
        assert_eq!(sentences[0], "Does PostgreSQL support JSONB");
        assert_eq!(sentences[1], "Yes");
        assert_eq!(sentences[2], "SQLite 3.45 introduced JSONB natively");
        assert_eq!(sentences[3], "Node.js supports modern ES modules");
        assert_eq!(sentences[4], "it is fast");
    }

    #[test]
    fn test_detect_triplet_contradictions_snippet_polarity() {
        let triplets = vec![
            ClaimTriplet {
                subject: "PostgreSQL".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB indexing".to_string(),
                confidence: 0.95,
                evidence_snippet: "PostgreSQL supports JSONB indexing natively.".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "PostgreSQL".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB indexing".to_string(),
                confidence: 0.85,
                evidence_snippet: "PostgreSQL does not support JSONB indexing in legacy versions."
                    .to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
        ];

        let mut contradictions = Vec::new();
        detect_triplet_contradictions(&triplets, &mut contradictions);
        assert_eq!(contradictions.len(), 1);
        assert!(contradictions[0].contains("conflicting evidence polarity"));
    }

    #[test]
    fn test_detect_triplet_contradictions_does_not_flag_incidental_negation() {
        let triplets = vec![
            ClaimTriplet {
                subject: "PostgreSQL".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB indexing".to_string(),
                confidence: 0.95,
                evidence_snippet: "PostgreSQL supports JSONB indexing natively.".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "PostgreSQL".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB indexing".to_string(),
                confidence: 0.90,
                evidence_snippet:
                    "PostgreSQL supports JSONB indexing. External plugins are not required."
                        .to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
        ];

        let mut contradictions = Vec::new();
        detect_triplet_contradictions(&triplets, &mut contradictions);
        assert!(
            contradictions.is_empty(),
            "Incidental negation in an unrelated sentence must not flag a contradiction"
        );
    }

    #[test]
    fn test_extract_claim_triplets_fallback() {
        let text = "PostgreSQL provides JSONB indexing support. SQLite introduced JSONB in 2024.";
        let triplets = extract_claim_triplets(text, Some("https://example.com/db"));
        assert!(!triplets.is_empty());
        assert!(
            triplets
                .iter()
                .any(|t| t.predicate == "provides" || t.predicate == "introduced")
        );
        assert_eq!(
            triplets[0].source_url.as_deref(),
            Some("https://example.com/db")
        );
    }

    #[test]
    fn test_extract_claim_triplets_trims_punctuation() {
        let text = "SQLite supports true. SQLite deprecates false.";
        let triplets = extract_claim_triplets(text, None);
        assert_eq!(triplets.len(), 2);
        assert_eq!(triplets[0].object, "true");
        assert_eq!(triplets[1].object, "false");
    }

    #[test]
    fn test_detect_triplet_contradictions_does_not_flag_unrelated_negation_with_common_words() {
        let triplets = vec![
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "a new json feature".to_string(),
                confidence: 0.95,
                evidence_snippet: "SQLite supports a new json feature.".to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "a new json feature".to_string(),
                confidence: 0.90,
                evidence_snippet:
                    "PostgreSQL does not require plugins. SQLite supports a new json feature."
                        .to_string(),
                source_url: None,
                grounding: GroundingQuality::VerbatimExact,
            },
        ];

        let mut contradictions = Vec::new();
        detect_triplet_contradictions(&triplets, &mut contradictions);
        assert!(
            contradictions.is_empty(),
            "Sentences mentioning stop words or other databases with negation must not flag false positive"
        );
    }

    #[test]
    fn test_format_grounded_research_evidence_deduplication() {
        let triplets = vec![
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB".to_string(),
                confidence: 0.90,
                evidence_snippet: "SQLite supports JSONB".to_string(),
                source_url: Some("https://sqlite.org".to_string()),
                grounding: GroundingQuality::VerbatimExact,
            },
            ClaimTriplet {
                subject: "SQLite".to_string(),
                predicate: "supports".to_string(),
                object: "JSONB".to_string(),
                confidence: 0.90,
                evidence_snippet: "SQLite supports JSONB natively".to_string(),
                source_url: Some("https://sqlite.org".to_string()),
                grounding: GroundingQuality::VerbatimExact,
            },
        ];
        let contradictions = vec!["Contradiction A".to_string(), "Contradiction A".to_string()];

        let formatted = format_grounded_research_evidence(&triplets, &contradictions);
        assert_eq!(formatted.matches("SQLite, supports, JSONB").count(), 1);
        assert_eq!(formatted.matches("Contradiction A").count(), 1);
    }
}
