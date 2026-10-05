mod components;
mod crud;
mod embedding;
mod kb;
mod knowledge;
mod library;
mod retention;
mod search;

pub use knowledge::{KnowledgeHealthInfo, KnowledgeNodeRecord};
pub use library::{
    KB_CHUNK_SOURCE_TYPE, KB_LIBRARY_CORPUS, KbIndexState, LibraryChunkDiff, LibraryChunkInput,
    LibraryChunkPos, LibraryChunkRow, LibraryDocRow, LibraryDocUpsert, TRUST_TIER_WIRE,
    validate_trust_tier_wire,
};

pub(crate) fn sanitize_fts_query(input: &str) -> String {
    let cleaned = input
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}
