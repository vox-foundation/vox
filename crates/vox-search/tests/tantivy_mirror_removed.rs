//! Guard (IDX-07): the writer-less Tantivy docs mirror stays deleted.
//!
//! Nothing ever wrote that index, so every consumer read an always-empty list.
//! The one lexical leg is `vox_search::library` (indexer-owned). This test fails
//! if any mirror symbol reappears in the files that used to carry it.

use std::path::{Path, PathBuf};

const BANNED: &[&str] = &[
    "TantivyDocsIndex",
    "TantivyDocHit",
    "tantivy_doc_lines",
    "tantivy_index_root",
    "tantivy_supplemental_lines",
    "lexical_tantivy",
    "[tantivy:",
];

const FILES: &[&str] = &[
    "src/lib.rs",
    "src/execution.rs",
    "src/bundle.rs",
    "src/a2a_contract.rs",
    "src/rrf.rs",
    "src/policy.rs",
    "../vox-research-shim/src/research/orchestrator/web_gather.rs",
    "../vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs",
    "../vox-orchestrator-mcp/src/memory_tools/retrieval.rs",
];

fn manifest_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn tantivy_mirror_removed() {
    assert!(
        !manifest_path("src/lexical_tantivy.rs").exists(),
        "the Tantivy docs mirror module must stay deleted"
    );
    for rel in FILES {
        let src = std::fs::read_to_string(manifest_path(rel))
            .unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for banned in BANNED {
            assert!(
                !src.contains(banned),
                "{rel} mentions removed Tantivy mirror symbol `{banned}`"
            );
        }
    }
}
