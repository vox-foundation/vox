#![allow(unused, dead_code)]

#[path = "../src/commands/mod.rs"]
mod commands;
#[path = "../src/config/mod.rs"]
mod config;

use commands::gui_db_pool::GuiDbPool;
use commands::research::{
    DocDraftPreview, PublishDocResult, generate_research_doc_draft, publish_research_doc,
};
use tauri::Manager;
use tempfile::tempdir;

#[tokio::test]
async fn test_research_doc_draft_and_publish_flow() {
    let dir = tempdir().expect("tempdir");
    let repo_root = dir.path().to_path_buf();
    unsafe {
        std::env::set_var("VOX_REPOSITORY_ROOT", &repo_root);
    }

    let arch_dir = repo_root.join("docs").join("src").join("architecture");
    std::fs::create_dir_all(&arch_dir).expect("create arch dir");
    let index_path = arch_dir.join("research-index.md");
    let initial_index = "# Research Index\n\n## Strategic & Value Proposition\n\n- [Existing](existing-2026.md) — Existing doc.\n";
    std::fs::write(&index_path, initial_index).expect("write initial index");

    let app = tauri::test::mock_app();
    app.manage(GuiDbPool::connect_memory().await.expect("memory pool"));
    let pool = app.state::<GuiDbPool>();

    // 1. Generate draft
    let session_id = 42;
    let draft = generate_research_doc_draft(pool.clone(), session_id)
        .await
        .expect("draft generation succeeds");

    assert_eq!(draft.slug, "session-42-research");
    assert_eq!(draft.filename, "session-42-research-2026.md");
    assert!(draft.is_valid);
    assert!(draft.validation_errors.is_empty());
    assert!(
        draft
            .markdown_content
            .contains("title: \"Research Session #42 Architecture SSOT (2026)\"")
    );
    assert!(
        draft
            .markdown_content
            .contains("category: \"Architecture SSOTs\"")
    );
    assert!(draft.markdown_content.contains("status: \"current\""));
    assert!(
        draft
            .markdown_content
            .contains("## 1. Executive Summary & Codebase Reality")
    );

    // 2. Publish draft
    let result = publish_research_doc(
        pool.clone(),
        session_id,
        draft.slug.clone(),
        draft.markdown_content.clone(),
    )
    .await
    .expect("publishing succeeds");

    assert_eq!(
        result.relative_path,
        "docs/src/architecture/session-42-research-2026.md"
    );
    assert!(result.indexed);
    let published_file = std::path::PathBuf::from(&result.file_path);
    assert!(published_file.exists());
    let written = std::fs::read_to_string(&published_file).expect("read written doc");
    assert_eq!(written, draft.markdown_content);

    // 3. Verify research-index.md updated
    let updated_index = std::fs::read_to_string(&index_path).expect("read index");
    assert!(updated_index.contains("session-42-research-2026.md"));

    // 4. Verify slug ending with -2026.md is handled idempotently
    let result2 = publish_research_doc(
        pool.clone(),
        session_id,
        "session-42-research-2026.md".into(),
        draft.markdown_content.clone(),
    )
    .await
    .expect("re-publishing with .md slug succeeds");
    assert_eq!(
        result2.relative_path,
        "docs/src/architecture/session-42-research-2026.md"
    );
}

#[tokio::test]
async fn test_publish_research_doc_rejects_path_traversal() {
    let app = tauri::test::mock_app();
    app.manage(GuiDbPool::connect_memory().await.expect("memory pool"));
    let pool = app.state::<GuiDbPool>();

    let malicious_slugs = vec![
        "../../etc/passwd",
        "..\\..\\windows\\system32",
        "nested/path",
        ".hidden",
    ];

    for bad_slug in malicious_slugs {
        let res = publish_research_doc(
            pool.clone(),
            1,
            bad_slug.to_string(),
            "# Malicious".to_string(),
        )
        .await;

        assert!(res.is_err(), "Slug {bad_slug} should have been rejected");
        assert!(res.unwrap_err().contains("Invalid slug"));
    }
}
