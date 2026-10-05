//! Library-index retriever for the KB eval: determinism over the Phase 1 fixture, and exact
//! resolution of the real golden set's chunk qrels onto Library chunk ids.
#![cfg(feature = "tantivy-lexical")]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vox_search::kb_eval::golden::{GoldenSet, Split, load_corpus};
use vox_search::kb_eval::library::{LIBRARY_RETRIEVER, LibraryRetriever, evaluate};
use vox_search::kb_ids::{chunk_id, split_chunk_id};
use vox_search::library::chunk::CHUNKER_VERSION;

fn hasher(b: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb_eval")
}

fn real_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/kb/eval")
}

fn retriever(golden: &GoldenSet, depth: usize) -> LibraryRetriever {
    let work = tempfile::tempdir().unwrap();
    let queries: Vec<String> = golden.queries.iter().map(|q| q.text.clone()).collect();
    LibraryRetriever::build_blocking(&golden.corpus, &queries, depth, work.path(), &hasher)
        .expect("build library retriever")
}

fn eval_fixture() -> vox_search::kb_eval::Report {
    let golden = GoldenSet::load(&fixture_dir(), Split::Dev).expect("load fixture");
    evaluate(&golden, &retriever(&golden, 100)).expect("evaluate")
}

#[test]
fn library_retriever_is_deterministic() {
    let (a, b) = (eval_fixture(), eval_fixture());
    assert_eq!(a.to_json_string().unwrap(), b.to_json_string().unwrap());
    assert_eq!(a.render_text(), b.render_text());
    assert_eq!(a.retriever, LIBRARY_RETRIEVER);
    assert_eq!(a.retriever, "library-tantivy-v1");
    assert_eq!(a.chunker, CHUNKER_VERSION);
}

#[test]
fn library_retriever_ranks_the_unique_install_section_first() {
    // q001's unique installer terms live only in alpha's Install section.
    let report = eval_fixture();
    let all = &report.keys["dev/all"];
    assert_eq!(all.queries, 3);
    assert_eq!(all.doc.mrr_at_10, 1.0);
    assert!(all.chunk.recall_at_10 > 0.0);
}

#[test]
fn chunk_qrels_resolve_exactly_to_library_chunks() {
    // Reads only the corpus-id column of each qrels file: no metrics on the held-out split.
    let dir = real_dir();
    let corpus = load_corpus(&dir).expect("load real corpus");
    let work = tempfile::tempdir().unwrap();
    let idx = LibraryRetriever::build_blocking(&corpus, &[], 10, work.path(), &hasher)
        .expect("index the frozen snapshot");
    let known: BTreeSet<String> = idx
        .doc_chunk_ids()
        .values()
        .flat_map(|s| s.iter().cloned())
        .collect();
    let mut unresolved = Vec::new();
    for name in ["dev.chunks.tsv", "test.chunks.tsv"] {
        let text = std::fs::read_to_string(dir.join("qrels").join(name)).unwrap();
        for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let key = line.split('\t').nth(1).expect("corpus-id column");
            let ok = split_chunk_id(key)
                .map(|(doc, hp)| known.contains(&chunk_id(doc, hp)))
                .unwrap_or(false);
            if !ok {
                let doc = split_chunk_id(key).map(|(d, _)| d).unwrap_or(key);
                let have: Vec<&String> = idx
                    .doc_chunk_ids()
                    .get(doc)
                    .map(|s| s.iter().collect())
                    .unwrap_or_default();
                unresolved.push(format!("{name}: {key:?}; doc heading paths: {have:?}"));
            }
        }
    }
    assert!(
        unresolved.is_empty(),
        "chunk qrel keys with no exact Library chunk:\n{}",
        unresolved.join("\n")
    );
}
