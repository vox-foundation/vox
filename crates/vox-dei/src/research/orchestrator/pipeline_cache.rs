use vox_db::Codex;

use super::super::types::{
    ResearchMetadata, ResearchQuery, ResearchResult, RetrievalDiagnostics, RoutingTier,
};

/// Codex `list_memories_by_type` cache short-circuit for identical-ish queries.
pub(super) async fn research_cache_short_circuit(
    query: &ResearchQuery,
    db: &Codex,
) -> Option<ResearchResult> {
    let q = query.query.trim();
    let q_words: Vec<&str> = q.split_whitespace().filter(|w| w.len() >= 4).collect();
    if q_words.is_empty() {
        return None;
    }
    let memories = db.list_memories_by_type("research_result", 64).await.ok()?;
    let mut best: Option<(f64, vox_db::MemoryEntry)> = None;
    for mem in memories {
        let content_lower = mem.content.to_lowercase();
        let hit = q_words
            .iter()
            .any(|w| content_lower.contains(&w.to_lowercase()));
        if !hit || mem.content.trim().is_empty() {
            continue;
        }
        let age_hours = chrono::DateTime::parse_from_rfc3339(mem.created_at.trim())
            .ok()
            .map(|dt| {
                (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds() as f64
                    / 3600.0
            })
            .unwrap_or(f64::MAX);
        if age_hours < 24.0 {
            if best.as_ref().map_or(true, |(best_age, _)| age_hours < *best_age) {
                best = Some((age_hours, mem));
            }
        }
    }
    let (age_hours, mem) = best?;
    tracing::info!("Research cache hit from Codex (age: {:.1}h)", age_hours);
    Some(ResearchResult {
        answer: format!(
            "[Cached result — {:.1}h old]\n\n{}",
            age_hours, mem.content
        ),
        sources: Vec::new(),
        citations: Vec::new(),
        research_metadata: ResearchMetadata {
            session_id: 0,
            duration_ms: 0,
            provider: "cache".to_string(),
            routing_tier: RoutingTier::Direct,
            confidence: 1.0,
            subquery_count: 0,
            source_count: 0,
            claim_verdicts: Vec::new(),
            retrieval_diagnostics: RetrievalDiagnostics::default(),
            quality_score: 100,
            competence: None,
            self_verification: None,
        },
    })
}
