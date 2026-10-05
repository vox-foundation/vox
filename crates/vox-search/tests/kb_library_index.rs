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

/// `(rowid, hash)` of every library chunk in rowid order.
async fn chunk_ids(db: &VoxDb) -> Vec<(i64, String)> {
    db.library_chunk_positions(KB_LIBRARY_CORPUS)
        .await
        .unwrap()
        .into_iter()
        .map(|p| (p.rowid, p.chunk_hash))
        .collect()
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

    let doc = db
        .library_document_by_path("docs/src/guide.md")
        .await
        .unwrap()
        .expect("one library document row");
    assert_eq!(doc.rel_path, "docs/src/guide.md");
    assert_eq!(
        db.get_kb_index_state(KB_LIBRARY_CORPUS)
            .await
            .unwrap()
            .unwrap()
            .doc_count,
        1
    );

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

/// `(chunk_index, hash)` of one document, ordered by `chunk_index`.
async fn doc_chunks(db: &VoxDb, rel_path: &str) -> Vec<(i64, String)> {
    let mut out: Vec<_> = db
        .library_chunk_positions(KB_LIBRARY_CORPUS)
        .await
        .unwrap()
        .into_iter()
        .filter(|p| p.rel_path == rel_path)
        .map(|p| (p.chunk_index, p.chunk_hash))
        .collect();
    out.sort();
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

    assert_eq!(
        db.embedding_count("kb_chunk", Some(&one.chunk_hash))
            .await
            .unwrap(),
        1
    );
    let same = db
        .library_chunks_by_hashes(std::slice::from_ref(&one.chunk_hash))
        .await
        .unwrap();
    assert_eq!(same[0].body, one.body);
}

// ---- 02-04: prune, orphan vectors, manifest, divergence (IDX-01, IDX-03, IDX-07) ----

use vox_search::library::lexical::LEXICAL_BACKEND;
use vox_search::library::status;

#[tokio::test]
async fn kb_reindex_prune() {
    let repo = three_docs();
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    assert!(
        !search(&db, &lex, "zorkmidword", 5)
            .await
            .unwrap()
            .is_empty()
    );
    let old_b = doc_chunks(&db, "docs/src/b.md").await;

    std::fs::remove_file(repo.path().join("docs/src/c.md")).unwrap();
    std::fs::rename(
        repo.path().join("docs/src/b.md"),
        repo.path().join("docs/src/b2.md"),
    )
    .unwrap();
    let r = run(&db, &repo).await;
    assert_eq!(r.docs_pruned, 2, "{r:?}");

    assert!(doc_chunks(&db, "docs/src/c.md").await.is_empty());
    assert!(doc_chunks(&db, "docs/src/b.md").await.is_empty());
    let new_b = doc_chunks(&db, "docs/src/b2.md").await;
    assert_eq!(new_b.len(), old_b.len());
    assert!(
        new_b.iter().all(|n| old_b.iter().all(|o| o.1 != n.1)),
        "rel_path is in the hash input"
    );
    assert!(
        search(&db, &lex, "zorkmidword", 5)
            .await
            .unwrap()
            .is_empty()
    );
    for gone in ["docs/src/b.md", "docs/src/c.md"] {
        assert!(db.library_document_by_path(gone).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn kb_no_orphan_embeddings() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let all = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    for r in &all {
        db.store_embedding("kb_chunk", &r.chunk_hash, "m", &[1.0], None, None)
            .await
            .unwrap();
    }
    db.store_embedding("memory", "not-a-chunk-hash", "m", &[1.0], None, None)
        .await
        .unwrap();

    std::fs::remove_file(repo.path().join("docs/src/c.md")).unwrap();
    write(
        &repo,
        "b.md",
        &DOC_B.replace("second body", "second body edited"),
    );
    run(&db, &repo).await;

    let live = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    for r in &all {
        let alive = live.iter().any(|l| l.chunk_hash == r.chunk_hash);
        assert_eq!(
            db.embedding_count("kb_chunk", Some(&r.chunk_hash))
                .await
                .unwrap(),
            i64::from(alive),
            "{}: vector kept iff chunk is live",
            r.heading_path
        );
    }
    let kept = all
        .iter()
        .filter(|r| live.iter().any(|l| l.chunk_hash == r.chunk_hash))
        .count();
    assert!(kept > 0 && kept < all.len());
    assert_eq!(
        db.embedding_count("kb_chunk", None).await.unwrap() as usize,
        kept
    );
    assert_eq!(
        db.embedding_count("memory", None).await.unwrap(),
        1,
        "other source types are untouched"
    );
}

#[tokio::test]
async fn kb_manifest_and_cards() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    let r = run(&db, &repo).await;
    let s = db
        .get_kb_index_state(KB_LIBRARY_CORPUS)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(s.corpus_root, repo.path().to_string_lossy());
    assert_eq!(s.corpus_commit.as_deref(), Some("abc123"));
    assert_eq!((s.doc_count, s.card_count), (3, 3));
    assert_eq!(
        s.chunk_count,
        db.library_chunk_count(KB_LIBRARY_CORPUS).await.unwrap()
    );
    assert_eq!(
        s.chunker_version,
        vox_search::library::chunk::CHUNKER_VERSION
    );
    assert_eq!(s.lexical_backend, "tantivy-0.22");
    assert_eq!(s.trust_tier, "trusted");
    assert!(s.embed_model_id.is_none() && s.embed_revision.is_none() && s.embed_dim.is_none());
    assert!(s.duration_ms >= 0 && !s.indexed_at.is_empty());
    assert_eq!(s.errors_json, "[]");
    assert_eq!(r.cards, 3);
}

#[tokio::test]
async fn kb_status_reports_errors() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    write(&repo, "bad.md", "# Bad\n## Fine\nstable text\n");
    run(&db, &repo).await;
    assert_eq!(doc_chunks(&db, "docs/src/bad.md").await.len(), 3);

    // Now the file becomes unreadable: it is reported, the rest still indexes, and the stored
    // doc is NOT pruned (T-02-11: a transient read error must not delete knowledge).
    std::fs::write(repo.path().join("docs/src/bad.md"), [0xff, 0xfe, 0xfd]).unwrap();
    write(&repo, "a.md", &DOC_A.replace("alpha body", "alpha body v2"));
    let r = run(&db, &repo).await;
    assert_eq!(r.errors.len(), 1);
    assert_eq!(r.errors[0].path, "docs/src/bad.md");
    assert_eq!((r.docs_pruned, r.docs_changed), (0, 1), "{r:?}");
    assert_eq!(doc_chunks(&db, "docs/src/bad.md").await.len(), 3);

    let s = db
        .get_kb_index_state(KB_LIBRARY_CORPUS)
        .await
        .unwrap()
        .unwrap();
    assert!(
        s.errors_json.contains("docs/src/bad.md"),
        "{}",
        s.errors_json
    );
}

#[tokio::test]
async fn kb_lexical_backend() {
    let repo = three_docs();
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let st = status(&db, &lex).await.unwrap();
    assert_eq!(st.state.unwrap().lexical_backend, "tantivy-0.22");
    assert_ne!(LEXICAL_BACKEND, "like");
    let hits = search(&db, &lex, "three", 3).await.unwrap();
    assert_eq!(hits[0].heading_path, "B › Three");
}

#[tokio::test]
async fn kb_lexical_divergence_rebuilds() {
    let repo = three_docs();
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    assert!(
        !status(&db, &lex).await.unwrap().consistent,
        "nothing indexed yet"
    );
    run(&db, &repo).await;
    let st = status(&db, &lex).await.unwrap();
    assert!(st.consistent, "{st:?}");
    assert_eq!(st.lexical_docs, Some(st.db_chunk_count as u64));

    std::fs::remove_dir_all(&lex).unwrap();
    assert!(!status(&db, &lex).await.unwrap().consistent);
    let r = run(&db, &repo).await;
    assert_eq!(r.docs_changed, 0);
    assert!(status(&db, &lex).await.unwrap().consistent);
}

#[tokio::test]
async fn kb_chunker_version_change_forces_rechunk() {
    let repo = three_docs();
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let mut state = db
        .get_kb_index_state(KB_LIBRARY_CORPUS)
        .await
        .unwrap()
        .unwrap();
    state.chunker_version = "old".into();
    db.put_kb_index_state(&state).await.unwrap();
    let r = run(&db, &repo).await;
    assert_eq!((r.docs_unchanged, r.docs_seen), (0, 3), "{r:?}");
    let s = db
        .get_kb_index_state(KB_LIBRARY_CORPUS)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        s.chunker_version,
        vox_search::library::chunk::CHUNKER_VERSION
    );
}

// ---- 02-10: CR-01 — a partial, empty or missing walk must never prune the Library ----

async fn try_run(
    db: &VoxDb,
    repo: &tempfile::TempDir,
) -> anyhow::Result<vox_search::library::IndexReport> {
    index_library(db, repo.path(), &repo.path().join("lex"), &deps()).await
}

#[cfg(unix)]
#[tokio::test]
async fn kb_reindex_unreadable_subdir_keeps_rows() {
    use std::os::unix::fs::PermissionsExt;
    let repo = tree(&[("a.md", DOC_A), ("sub/b.md", DOC_B), ("c.md", DOC_C)]);
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    run(&db, &repo).await;
    let sub_path = "docs/src/sub/b.md";
    let snapshot = doc_chunks(&db, sub_path).await;
    assert!(!snapshot.is_empty());
    for (_, hash) in &snapshot {
        db.store_embedding("kb_chunk", hash, "m", &[0.5, 0.25], None, None)
            .await
            .unwrap();
    }

    let sub = repo.path().join("docs/src/sub");
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&sub).is_ok() {
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
        return; // permissions are bypassed (root): the scenario cannot be staged
    }
    write(
        &repo,
        "a.md",
        &DOC_A.replace("alpha body", "alpha body quuxword"),
    );
    let res = try_run(&db, &repo).await;
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let r = res.unwrap();

    assert_eq!(r.docs_pruned, 0, "{r:?}");
    assert!(r.errors.iter().any(|e| e.path == "docs/src/sub"), "{r:?}");
    assert!(
        r.errors.iter().any(|e| e.error.contains("prune skipped")),
        "{r:?}"
    );
    let host = repo.path().to_string_lossy();
    assert!(
        r.errors
            .iter()
            .all(|e| !e.error.contains(host.as_ref()) && !e.path.contains(host.as_ref())),
        "{r:?}"
    );
    assert_eq!(doc_chunks(&db, sub_path).await, snapshot);
    for (_, hash) in &snapshot {
        assert_eq!(db.embedding_count("kb_chunk", Some(hash)).await.unwrap(), 1);
    }
    let hits = search(&db, &lex, "quuxword", 5).await.unwrap();
    assert!(
        hits.iter().any(|h| h.doc_path == "docs/src/a.md"),
        "{hits:?}"
    );
}
