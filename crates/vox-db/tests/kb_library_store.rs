//! Store-level checks for the KB library readers and the hash-diff renumber path.
//! The FTS5 engine characterization (IDX-07) lives in `turso_fts_probe.rs`, not here.

use vox_db::{KB_LIBRARY_CORPUS, LibraryChunkInput, LibraryDocUpsert, VoxDb};

fn chunk(hash: char, heading: &str, body: &str) -> LibraryChunkInput {
    LibraryChunkInput {
        chunk_hash: hash.to_string().repeat(64),
        chunk_kind: "section".into(),
        heading_path: heading.into(),
        anchor: None,
        token_estimate: 1,
        body: body.into(),
    }
}

fn doc(corpus: &'static str, rel_path: &'static str) -> LibraryDocUpsert<'static> {
    LibraryDocUpsert {
        corpus,
        rel_path,
        title: "Guide",
        file_hash: "fh",
        frontmatter_json: "{\"title\":\"Guide\"}",
        indexed_commit: Some("abc"),
        index_error: None,
        trust_tier: "trusted",
    }
}

#[tokio::test]
async fn kb_store_diff_renumber() {
    let db = VoxDb::open_memory().await.unwrap();
    let d = doc(KB_LIBRARY_CORPUS, "docs/src/guide.md");
    let (a, b, c, x) = (
        chunk('a', "A", "alpha"),
        chunk('b', "B", "beta"),
        chunk('c', "C", "gamma"),
        chunk('d', "X", "new"),
    );
    db.apply_library_document(&d, &[a.clone(), b.clone(), c.clone()])
        .await
        .unwrap();
    let diff = db
        .apply_library_document(&d, &[a.clone(), x.clone(), b.clone(), c.clone()])
        .await
        .unwrap();
    assert_eq!((diff.inserted, diff.kept, diff.deleted), (1, 3, 0));
    let order: Vec<String> = db
        .library_doc_chunks(KB_LIBRARY_CORPUS, "docs/src/guide.md")
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.heading_path)
        .collect();
    assert_eq!(order, ["A", "X", "B", "C"]);
}

#[tokio::test]
async fn kb_store_readers_never_return_other_corpora() {
    let db = VoxDb::open_memory().await.unwrap();
    db.apply_library_document(
        &doc(KB_LIBRARY_CORPUS, "docs/src/guide.md"),
        &[chunk('a', "A", "x")],
    )
    .await
    .unwrap();
    db.apply_library_document(&doc("", "docs/src/web.md"), &[chunk('b', "W", "web row")])
        .await
        .unwrap();

    let row = db
        .library_document_by_path("docs/src/guide.md")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            row.title.as_str(),
            row.file_hash.as_str(),
            row.indexed_commit.as_deref()
        ),
        ("Guide", "fh", Some("abc"))
    );
    assert!(
        db.library_document_by_path("docs/src/web.md")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        db.library_chunk_by_hash(&"b".repeat(64))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        db.library_chunk_by_hash(&"a".repeat(64))
            .await
            .unwrap()
            .is_some()
    );
}
