use vox_db::Codex;

use super::super::types::ResearchQuery;
use super::super::types::ResearchResult;
use super::config::ResearchConfig;

/// Returns `true` when `result` contains a substantive non-abstaining synthesis answer
/// safe to store in or serve from `research_cache`.
pub(super) fn is_cacheable_research_result(result: &ResearchResult) -> bool {
    let trimmed = result.answer.trim();
    !trimmed.is_empty() && !trimmed.starts_with(super::config::ABSTENTION_MARKER)
}

/// Codex `list_memories_by_type` cache short-circuit for identical-ish queries.
pub(super) async fn research_cache_short_circuit(
    query: &ResearchQuery,
    db: &Codex,
    config: &ResearchConfig,
) -> Option<ResearchResult> {
    let key = research_cache_key(query);
    let now = current_unix_secs();
    let entries = db.list_memories_by_type("research_cache", 200).await.ok()?;
    entries
        .into_iter()
        .filter_map(|entry| serde_json::from_str::<ResearchCacheEntry>(&entry).ok())
        .find(|entry| {
            entry.key == key
                && now.saturating_sub(entry.created_at_unix_secs) <= config.cache_ttl_secs
                && is_cacheable_research_result(&entry.result)
        })
        .map(|entry| entry.result)
}

/// Persist a completed research result for future cache short-circuit lookups.
pub(super) async fn research_cache_store(
    query: &ResearchQuery,
    result: &ResearchResult,
    db: &Codex,
    _config: &ResearchConfig,
) {
    if !is_cacheable_research_result(result) {
        return;
    }
    let key = research_cache_key(query);
    let node_id = format!("research_cache:{key}");
    let entry = ResearchCacheEntry {
        key: key.clone(),
        created_at_unix_secs: current_unix_secs(),
        result: result.clone(),
    };
    let Ok(content) = serde_json::to_string(&entry) else {
        tracing::warn!("research cache entry serialization failed");
        return;
    };
    // Must use `knowledge_nodes` — `research_cache_short_circuit` reads via
    // `list_memories_by_type`, not the episodic `memories` table.
    if let Err(e) = db
        .upsert_knowledge_node(&node_id, &key, &content, Some("research_cache"), None, None)
        .await
    {
        tracing::warn!(error = %e, "research cache store failed");
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ResearchCacheEntry {
    key: String,
    created_at_unix_secs: u64,
    result: ResearchResult,
}

fn research_cache_key(query: &ResearchQuery) -> String {
    let normalized_query = query
        .query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let site_scope = query
        .site_scope
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_ascii_lowercase();
    let raw = format!(
        "{}|{:?}|{}|{}|{:?}|{:?}|{}|{}",
        normalized_query,
        query.scope,
        query.max_sources,
        query.verify_claims,
        query.lane,
        query.domain_mode,
        query.waves,
        site_scope,
    );
    format!("{:016x}", super::helpers::fnv1a_hash(&raw))
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::types::{
        ResearchMetadata, ResearchScope, RetrievalDiagnostics, RoutingTier,
    };

    fn query(text: &str, scope: ResearchScope) -> ResearchQuery {
        ResearchQuery {
            query: text.to_string(),
            scope,
            max_sources: 8,
            persist_to_docs: false,
            verify_claims: true,
            site_scope: None,
            domain_mode: Default::default(),
            waves: 1,
            lane: vox_search::policy::ResearchLane::Fast,
        }
    }

    fn minimal_result() -> ResearchResult {
        ResearchResult {
            answer: "cached answer".to_string(),
            sources: vec![],
            citations: vec![],
            research_metadata: ResearchMetadata {
                session_id: 42,
                duration_ms: 1,
                provider: "test".to_string(),
                routing_tier: RoutingTier::Direct,
                confidence: 0.5,
                subquery_count: 1,
                source_count: 0,
                claim_verdicts: vec![],
                retrieval_diagnostics: RetrievalDiagnostics::default(),
                quality_score: 50,
                planner_degraded: false,
                competence: None,
                self_verification: None,
                citation_audit: None,
                corroboration_counts: vec![],
                wave_count: 1,
                wave_stability: None,
                low_grounding_evidence: false,
                subqueries: vec![],
                synthesis_model: String::new(),
                judge_error: None,
                served_from_cache: false,
                claims_extracted_count: 0,
                claims_verified_count: 0,
                usage: Default::default(),
            },
        }
    }

    #[test]
    fn cache_key_distinguishes_lanes() {
        let mut q = query("same question", ResearchScope::Web);
        q.lane = vox_search::policy::ResearchLane::Fast;
        let fast = research_cache_key(&q);
        q.lane = vox_search::policy::ResearchLane::Deep;
        let deep = research_cache_key(&q);
        assert_ne!(fast, deep);
    }

    #[test]
    fn cache_key_changes_with_scope_and_normalizes_whitespace() {
        let a = research_cache_key(&query("  Deep   Research  ", ResearchScope::Web));
        let b = research_cache_key(&query("deep research", ResearchScope::Web));
        let c = research_cache_key(&query("deep research", ResearchScope::Local));

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn research_cache_entry_serde_roundtrip() {
        let q = query("deep research", ResearchScope::Web);
        let entry = ResearchCacheEntry {
            key: research_cache_key(&q),
            created_at_unix_secs: 1_700_000_000,
            result: minimal_result(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let decoded: ResearchCacheEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.key, entry.key);
        assert_eq!(decoded.created_at_unix_secs, entry.created_at_unix_secs);
        assert_eq!(decoded.result.answer, entry.result.answer);
    }

    #[test]
    fn cache_key_distinguishes_domain_mode_waves_and_site_scope() {
        let base = query("solid state electrolytes", ResearchScope::Web);
        let base_key = research_cache_key(&base);

        let mut scholarly = base.clone();
        scholarly.domain_mode = crate::research::types::ResearchDomainMode::Scholarly;
        assert_ne!(base_key, research_cache_key(&scholarly));

        let mut multi_wave = base.clone();
        multi_wave.waves = 3;
        assert_ne!(base_key, research_cache_key(&multi_wave));

        let mut scoped = base;
        scoped.site_scope = Some("arxiv.org".to_string());
        assert_ne!(base_key, research_cache_key(&scoped));
    }

    #[test]
    fn is_cacheable_research_result_rejects_empty_and_abstention_answers() {
        let mut empty = minimal_result();
        empty.answer = "   \n ".to_string();
        assert!(!is_cacheable_research_result(&empty));

        let mut abstained = minimal_result();
        abstained.answer = format!(
            "{} Missing benchmark data.",
            super::super::config::ABSTENTION_MARKER
        );
        assert!(!is_cacheable_research_result(&abstained));

        assert!(is_cacheable_research_result(&minimal_result()));
    }

    #[tokio::test]
    async fn cache_store_and_short_circuit_skip_empty_and_abstention_results() {
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("memory db");
        let cfg = ResearchConfig::default();
        let q = query("cache hygiene test", ResearchScope::Web);

        let mut empty = minimal_result();
        empty.answer.clear();
        research_cache_store(&q, &empty, &db, &cfg).await;
        assert!(research_cache_short_circuit(&q, &db, &cfg).await.is_none());

        let mut abstained = minimal_result();
        abstained.answer = format!(
            "{} Insufficient citations.",
            super::super::config::ABSTENTION_MARKER
        );
        research_cache_store(&q, &abstained, &db, &cfg).await;
        assert!(research_cache_short_circuit(&q, &db, &cfg).await.is_none());

        // Even if a stale abstention was pre-seeded directly in knowledge_nodes, short-circuit ignores it.
        let key = research_cache_key(&q);
        let stale = serde_json::to_string(&ResearchCacheEntry {
            key: key.clone(),
            created_at_unix_secs: current_unix_secs(),
            result: abstained,
        })
        .expect("serialize stale");
        db.upsert_knowledge_node(
            &format!("research_cache:{key}"),
            &key,
            &stale,
            Some("research_cache"),
            None,
            None,
        )
        .await
        .expect("seed stale");
        assert!(research_cache_short_circuit(&q, &db, &cfg).await.is_none());

        // Real substantive result stores and short-circuits cleanly.
        let good = minimal_result();
        research_cache_store(&q, &good, &db, &cfg).await;
        let hit = research_cache_short_circuit(&q, &db, &cfg)
            .await
            .expect("substantive result should cache");
        assert_eq!(hit.answer, "cached answer");
    }
}
