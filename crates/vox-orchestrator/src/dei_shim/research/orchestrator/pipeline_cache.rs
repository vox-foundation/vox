use vox_db::Codex;

use super::super::types::ResearchQuery;
use super::super::types::ResearchResult;

/// Codex `list_memories_by_type` cache short-circuit for identical-ish queries.
///
/// PHASE_0a_STUB: always returns None (no cache). Phase 1 re-enables after vox_db gains
/// `list_memories_by_type`.
#[allow(dead_code)] // PHASE_0a_STUB: cache disabled until vox_db gains list_memories_by_type; re-enabled in Phase 1.
pub(super) async fn research_cache_short_circuit(
    _query: &ResearchQuery,
    _db: &Codex,
) -> Option<ResearchResult> {
    // PHASE_0a_STUB: cache disabled. Phase 1 wires to vox_db::Codex::list_memories_by_type.
    None
}
