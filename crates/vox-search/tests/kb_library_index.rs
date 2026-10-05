//! Tracer: one markdown doc -> kb_ids sections -> hash-keyed rows -> Tantivy -> library::search.
#![cfg(feature = "tantivy-lexical")]

mod common;

use vox_db::{KB_LIBRARY_CORPUS, VoxDb};
use vox_search::kb_ids;
use vox_search::library::{IndexDeps, chunk::chunk_document, index_library, search};

const GUIDE: &str = "---\ntitle: Guide\n---\n# Guide\nIntro paragraph.\n\n## Install\nRun the installer to get turbofrobnicate working.\n\n## Usage\nCall it.\n";

fn hasher(b: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
}

fn deps() -> IndexDeps<'static> {
    IndexDeps {
        hasher: &hasher,
        corpus_commit: Some("abc123".into()),
        trust_tier: "trusted",
    }
}

fn repo(guide: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("docs/src/guide.md");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, guide).unwrap();
    dir
}

async fn chunk_ids(db: &VoxDb) -> Vec<(i64, String)> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, chunk_hash FROM search_document_chunks ORDER BY id",
            (),
        )
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        out.push((r.get(0).unwrap(), r.get(1).unwrap()));
    }
    out
}

#[tokio::test]
async fn kb_tracer_one_doc_end_to_end() {
    let repo = repo(GUIDE);
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    let report = index_library(&db, repo.path(), &lex, &deps())
        .await
        .unwrap();
    assert_eq!((report.docs_seen, report.errors.len()), (1, 0));
    assert_eq!(report.lexical_docs as usize, report.chunks_inserted);

    let mut rows = db
        .connection()
        .query("SELECT corpus, rel_path FROM search_documents", ())
        .await
        .unwrap();
    let r = rows.next().await.unwrap().expect("one document row");
    assert_eq!(r.get::<String>(0).unwrap(), KB_LIBRARY_CORPUS);
    assert_eq!(r.get::<String>(1).unwrap(), "docs/src/guide.md");
    assert!(rows.next().await.unwrap().is_none());

    let allowed = kb_ids::doc_heading_paths("docs/src/guide.md", GUIDE);
    let chunks = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    assert!(chunks.len() >= 3);
    for c in &chunks {
        assert_eq!(c.chunk_hash.len(), 64);
        assert!(c.chunk_hash.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(allowed.contains(&c.heading_path), "{}", c.heading_path);
    }

    let hits = search(&db, &lex, "turbofrobnicate", 5).await.unwrap();
    assert_eq!(hits[0].heading_path, "Guide › Install");
    let install = chunks
        .iter()
        .find(|c| c.heading_path == "Guide › Install")
        .unwrap();
    assert_eq!(hits[0].chunk_hash, install.chunk_hash);
    assert!(hits[0].snippet.contains("turbofrobnicate"));
}

#[test]
fn chunk_h1_not_duplicated() {
    let doc = chunk_document("docs/src/guide.md", GUIDE, &hasher);
    assert!(doc.chunks.iter().all(|c| c.heading_path != "Guide › Guide"));
    assert!(doc.chunks.iter().any(|c| c.heading_path == "Guide"));

    let renamed = GUIDE.replace("# Guide\n", "# Other Name\n");
    let doc = chunk_document("docs/src/guide.md", &renamed, &hasher);
    let h1 = doc
        .chunks
        .iter()
        .find(|c| c.body.starts_with("# Other Name"))
        .unwrap();
    assert_eq!(h1.heading_path, "Guide");
    assert!(
        doc.chunks
            .iter()
            .all(|c| !c.heading_path.contains("Other Name"))
    );
}

#[tokio::test]
async fn kb_tracer_reindex_is_stable() {
    let repo = repo(GUIDE);
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    index_library(&db, repo.path(), &lex, &deps())
        .await
        .unwrap();
    let before = chunk_ids(&db).await;
    let second = index_library(&db, repo.path(), &lex, &deps())
        .await
        .unwrap();
    assert_eq!(second.chunks_inserted, 0);
    assert_eq!(second.chunks_deleted, 0);
    assert_eq!(chunk_ids(&db).await, before);
    assert_eq!(second.lexical_docs as usize, before.len());
}
