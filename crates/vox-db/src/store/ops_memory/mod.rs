mod components;
mod crud;
mod embedding;
mod kb;
mod kb_findings;
mod kb_legacy;
mod knowledge;
mod library;
mod retention;
mod search;

pub use kb_findings::{KbFindingCounts, KbFindingRow, NewKbFinding};
pub use kb_legacy::{LegacyKbCounts, LegacyKbEntry};
pub use knowledge::{KnowledgeHealthInfo, KnowledgeNodeRecord};
pub use library::{
    KB_CHUNK_SOURCE_TYPE, KB_LIBRARY_CORPUS, KbIndexState, LibraryChunkDiff, LibraryChunkInput,
    LibraryChunkPos, LibraryChunkRow, LibraryDocRow, LibraryDocUpsert,
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
