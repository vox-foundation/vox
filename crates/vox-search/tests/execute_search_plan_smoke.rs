//! Smoke test for `execute_search_plan` with in-memory DB + temp memory tree.

// Task 9c: no real web host from any test (see tests/common/mod.rs).
mod common;

use std::sync::Arc;

use vox_db::{RetrievalMode, SearchCorpus, SearchPlan, VoxDb};
use vox_search::{SearchPolicy, SearchRuntimeContext, execute_search_plan};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_search_plan_hits_memory_or_repo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path();
    let mem_dir = repo.join("memory");
    std::fs::create_dir_all(&mem_dir).expect("mkdir memory");
    std::fs::write(
        mem_dir.join("2026-05-11.md"),
        "# log\n\nuniquekeyword_alpha beta",
    )
    .expect("write log");
    let md_path = mem_dir.join("longterm.md");
    std::fs::write(&md_path, "# lt\n\ngamma uniquekeyword_alpha").expect("write md");

    // Give repo inventory something to match.
    let src_dir = repo.join("src");
    std::fs::create_dir_all(&src_dir).expect("mkdir src");
    std::fs::write(src_dir.join("uniquekeyword_note.rs"), "// note").expect("write rs");

    let db = Arc::new(VoxDb::open_memory().await.expect("open_memory"));
    let ctx = SearchRuntimeContext::new(repo.to_path_buf(), Some(db.clone()), mem_dir, md_path);

    let plan = SearchPlan {
        normalized_query: "uniquekeyword_alpha".into(),
        corpora: vec![SearchCorpus::Memory, SearchCorpus::RepoInventory],
        retrieval_mode: RetrievalMode::FullText,
        ..Default::default()
    };
    let policy = SearchPolicy::default();

    let exec = execute_search_plan(&ctx, "uniquekeyword_alpha", &plan, 8, &policy, None)
        .await
        .expect("execute_search_plan");

    assert!(
        !exec.memory_lines.is_empty() || !exec.repo_lines.is_empty(),
        "expected memory or repo hits; memory={:?} repo={:?}",
        exec.memory_lines,
        exec.repo_lines
    );
}

/// TRUST-01: chunk and knowledge hits are labelled with their stored tier, the envelope minimum
/// is derived from the kept hits, and `exclude_untrusted_hits` drops Untrusted hits together with
/// their typed `unified_hits` without reordering the survivors.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_search_plan_labels_tiers_and_excludes_untrusted_on_request() {
    use vox_db::ContextTrustTier;

    let dir = tempfile::tempdir().expect("tempdir");
    let mem_dir = dir.path().join("memory");
    std::fs::create_dir_all(&mem_dir).expect("mkdir memory");
    let md_path = mem_dir.join("longterm.md");
    let db = Arc::new(VoxDb::open_memory().await.expect("open_memory"));
    let trusted = db
        .upsert_search_document(
            "library:docs/src/t.md",
            "T",
            "text/markdown",
            "h",
            ContextTrustTier::Trusted,
        )
        .await
        .unwrap();
    db.replace_search_document_chunks(trusted, &["quillmark trustedhit".to_string()])
        .await
        .unwrap();
    db.connection()
        .execute_batch(
            "INSERT INTO search_documents (id, source_uri, title) VALUES (800, 'web-ingest:x', 'W'); \
             INSERT INTO search_document_chunks (document_id, chunk_index, body_text, created_at) \
               VALUES (800, 0, 'quillmark webhit', '2000-01-01 00:00:00');",
        )
        .await
        .unwrap();
    db.upsert_knowledge_node(
        "n1",
        "n1",
        "quillmark kgweb",
        Some("external_research"),
        None,
        None,
    )
    .await
    .unwrap();
    let ctx = SearchRuntimeContext::new(dir.path().to_path_buf(), Some(db), mem_dir, md_path);
    let plan = SearchPlan {
        normalized_query: "quillmark".into(),
        corpora: vec![SearchCorpus::DocumentChunks, SearchCorpus::KnowledgeGraph],
        retrieval_mode: RetrievalMode::FullText,
        ..Default::default()
    };

    let mut policy = SearchPolicy::default();
    let all = execute_search_plan(&ctx, "quillmark", &plan, 8, &policy, None)
        .await
        .unwrap();
    assert_eq!(all.chunk_lines.len(), 2, "{:?}", all.chunk_lines);
    assert!(
        all.chunk_lines
            .iter()
            .any(|l| l.starts_with("[tier:trusted] ") && l.contains("trustedhit"))
    );
    assert!(
        all.chunk_lines
            .iter()
            .any(|l| l.starts_with("[tier:untrusted] ") && l.contains("webhit"))
    );
    assert!(all.knowledge_lines[0].starts_with("[tier:untrusted] [node:n1]"));
    assert_eq!(all.min_trust_tier, Some(ContextTrustTier::Untrusted));

    policy.exclude_untrusted_hits = true;
    let pre = execute_search_plan(&ctx, "quillmark", &plan, 8, &policy, None)
        .await
        .unwrap();
    assert_eq!(pre.chunk_lines.len(), 1, "{:?}", pre.chunk_lines);
    assert!(
        pre.chunk_lines[0].starts_with("[tier:trusted] ")
            && pre.chunk_lines[0].contains("trustedhit")
    );
    assert!(pre.knowledge_lines.is_empty(), "{:?}", pre.knowledge_lines);
    assert_eq!(
        pre.unified_hits.len(),
        1,
        "typed hits stay parallel to the lines"
    );
    assert_eq!(pre.min_trust_tier, Some(ContextTrustTier::Trusted));

    let empty = execute_search_plan(&ctx, "nomatchtoken", &plan, 8, &policy, None)
        .await
        .unwrap();
    assert_eq!(
        empty.min_trust_tier, None,
        "nothing kept means no tier, read as Untrusted"
    );
}
