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

// ---- 02-04: incremental reindex (IDX-01) ----

const DOC_A: &str = "# A\nAlpha intro.\n\n## Only\nalpha body\n";
const DOC_B: &str =
    "# B\nBravo intro.\n\n## One\nfirst body\n\n## Two\nsecond body\n\n## Three\nthird body\n";
const DOC_C: &str = "# C\nCharlie intro.\n\n## Only\nzorkmidword here\n";

fn tree(docs: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in docs {
        write(&dir, name, text);
    }
    dir
}

fn write(dir: &tempfile::TempDir, name: &str, text: &str) {
    let p = dir.path().join("docs/src").join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn three_docs() -> tempfile::TempDir {
    tree(&[("a.md", DOC_A), ("b.md", DOC_B), ("c.md", DOC_C)])
}

async fn run(db: &VoxDb, repo: &tempfile::TempDir) -> vox_search::library::IndexReport {
    index_library(db, repo.path(), &repo.path().join("lex"), &deps())
        .await
        .unwrap()
}

/// `(chunk_index, hash)` of one document in storage order.
async fn doc_chunks(db: &VoxDb, rel_path: &str) -> Vec<(i64, String)> {
    let mut rows = db
        .connection()
        .query(
            &format!(
                "SELECT c.chunk_index, c.chunk_hash FROM search_document_chunks c \
                 JOIN search_documents d ON d.id = c.document_id \
                 WHERE d.rel_path = '{rel_path}' ORDER BY c.chunk_index"
            ),
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
async fn kb_reindex_unchanged() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let before = chunk_ids(&db).await;
    let second = run(&db, &repo).await;
    assert_eq!(
        (second.docs_unchanged, second.docs_changed),
        (3, 0),
        "{second:?}"
    );
    assert_eq!((second.chunks_inserted, second.chunks_deleted), (0, 0));
    assert_eq!(chunk_ids(&db).await, before);
}

#[tokio::test]
async fn kb_reindex_one_section() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let before = chunk_ids(&db).await;
    let (a_before, c_before) = (
        doc_chunks(&db, "docs/src/a.md").await,
        doc_chunks(&db, "docs/src/c.md").await,
    );

    write(
        &repo,
        "b.md",
        &DOC_B.replace("second body", "second body edited"),
    );
    let r = run(&db, &repo).await;
    assert_eq!((r.docs_changed, r.docs_unchanged), (1, 2), "{r:?}");
    assert_eq!((r.chunks_deleted, r.chunks_inserted), (1, 1), "{r:?}");

    let after = chunk_ids(&db).await;
    assert_eq!(after.len(), before.len());
    let kept = before.iter().filter(|b| after.contains(b)).count();
    assert_eq!(
        kept,
        before.len() - 1,
        "every other (rowid, hash) pair survives"
    );
    assert_eq!(doc_chunks(&db, "docs/src/a.md").await, a_before);
    assert_eq!(doc_chunks(&db, "docs/src/c.md").await, c_before);
}

#[tokio::test]
async fn kb_reindex_insert_middle_section() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let before = chunk_ids(&db).await;

    let edited = DOC_B.replace("## Two\n", "## Mid\nmiddle body\n\n## Two\n");
    write(&repo, "b.md", &edited);
    let r = run(&db, &repo).await;
    assert_eq!((r.chunks_inserted, r.chunks_deleted), (1, 0), "{r:?}");

    let after = chunk_ids(&db).await;
    assert!(
        before.iter().all(|b| after.contains(b)),
        "kept rows keep rowid and hash"
    );

    let b_chunks = doc_chunks(&db, "docs/src/b.md").await;
    let idx: Vec<i64> = b_chunks.iter().map(|c| c.0).collect();
    assert_eq!(idx, (0..b_chunks.len() as i64).collect::<Vec<_>>());
    let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    let order: Vec<&str> = rows
        .iter()
        .filter(|r| r.doc_path == "docs/src/b.md")
        .map(|r| r.heading_path.as_str())
        .collect();
    let pos = |h: &str| order.iter().position(|p| *p == h).unwrap();
    assert!(
        pos("B › One") < pos("B › Mid") && pos("B › Mid") < pos("B › Two"),
        "{order:?}"
    );
}

#[tokio::test]
async fn kb_kept_hash_keeps_vector() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    let one = rows.iter().find(|r| r.heading_path == "B › One").unwrap();
    db.store_embedding("kb_chunk", &one.chunk_hash, "m", &[0.5, 0.25], None, None)
        .await
        .unwrap();

    write(
        &repo,
        "b.md",
        &DOC_B.replace("## Two\n", "## Mid\nmiddle body\n\n## Two\n"),
    );
    run(&db, &repo).await;

    let mut q = db
        .connection()
        .query(
            &format!(
                "SELECT COUNT(*) FROM embeddings WHERE source_type = 'kb_chunk' AND source_id = '{}'",
                one.chunk_hash
            ),
            (),
        )
        .await
        .unwrap();
    assert_eq!(q.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);
    let same = db
        .library_chunks_by_hashes(std::slice::from_ref(&one.chunk_hash))
        .await
        .unwrap();
    assert_eq!(same[0].body, one.body);
}
