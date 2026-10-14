//! KB Phase 4 tracer: doc tagged with a prefLabel -> card facet line -> altLabel query expansion
//! -> hit, through the real indexer.
#![cfg(feature = "tantivy-lexical")]

mod common;

use vox_db::VoxDb;
use vox_search::library::chunk::chunk_document;
use vox_search::library::{
    IndexDeps, Taxonomy, index_library, load_taxonomy, search, search_with_taxonomy,
};

const TAXONOMY: &str = r#"
x-vox-version: 1
schema_version: 1
domains:
  - prefLabel: platform
topics:
  - prefLabel: orchestration
    broader: platform
    altLabel:
      - vox-orchestrator
    deprecatedAliases:
      - agent-dispatch
"#;

const TAGGED: &str = "---\ntitle: Alpha\ntopic: orchestration\ndomain: platform\n---\n# Alpha\nQuokkaflux scheduling notes.\n";
const OTHER: &str = "# Beta\nUnrelated gardening text.\n";

fn hasher(b: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
}

fn repo(docs: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in docs {
        let p = dir.path().join("docs/src").join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

async fn indexed(docs: &[(&str, &str)]) -> (tempfile::TempDir, VoxDb, std::path::PathBuf) {
    let repo = repo(docs);
    let lex = repo.path().join("lex");
    let db = VoxDb::open_memory().await.unwrap();
    let deps = IndexDeps {
        hasher: &hasher,
        corpus_commit: None,
        trust_tier: vox_db::ContextTrustTier::Trusted,
        is_committed_clean: &|_: &str| true,
    };
    index_library(&db, repo.path(), &lex, &deps).await.unwrap();
    (repo, db, lex)
}

fn paths(hits: &[vox_search::library::LibraryHit]) -> Vec<&str> {
    hits.iter().map(|h| h.doc_path.as_str()).collect()
}

#[tokio::test]
async fn altlabel_query_finds_the_preflabel_tagged_doc() {
    let (_r, db, lex) = indexed(&[("a.md", TAGGED), ("b.md", OTHER)]).await;
    let tax = Taxonomy::from_yaml_str(TAXONOMY).unwrap();

    let hits = search_with_taxonomy(&db, &lex, "vox-orchestrator", 5, Some(&tax))
        .await
        .unwrap();
    assert!(
        paths(&hits).contains(&"docs/src/a.md"),
        "{:?}",
        paths(&hits)
    );

    let hits = search_with_taxonomy(&db, &lex, "agent-dispatch", 5, Some(&tax))
        .await
        .unwrap();
    assert!(
        paths(&hits).contains(&"docs/src/a.md"),
        "deprecated alias: {:?}",
        paths(&hits)
    );

    // Without a taxonomy the same query cannot reach the doc: its body never says it.
    let hits = search(&db, &lex, "vox-orchestrator", 5).await.unwrap();
    assert!(
        !paths(&hits).contains(&"docs/src/a.md"),
        "{:?}",
        paths(&hits)
    );
}

#[tokio::test]
async fn query_without_altlabel_is_unchanged_by_taxonomy() {
    let (_r, db, lex) = indexed(&[("a.md", TAGGED), ("b.md", OTHER)]).await;
    let tax = Taxonomy::from_yaml_str(TAXONOMY).unwrap();
    let with = search_with_taxonomy(&db, &lex, "quokkaflux", 5, Some(&tax))
        .await
        .unwrap();
    let without = search(&db, &lex, "quokkaflux", 5).await.unwrap();
    assert_eq!(with, without);
    assert!(!with.is_empty());
}

#[test]
fn card_carries_facet_lines_and_no_summary_markers() {
    let doc = chunk_document("docs/src/a.md", TAGGED, &hasher);
    let card = &doc.chunks[0].body;
    assert!(card.contains("\ntopic: orchestration"), "{card}");
    assert!(card.contains("\ndomain: platform"), "{card}");

    let text = "# T\n\n<!-- kb:summary -->\nOne line of lede.\nSecond line.\n<!-- /kb:summary -->\n\n## S\nx\n";
    let card = &chunk_document("docs/src/s.md", text, &hasher).chunks[0].body;
    assert!(card.contains("One line of lede.\nSecond line."), "{card}");
    assert!(!card.contains("kb:summary"), "{card}");
}

#[tokio::test]
async fn real_taxonomy_altlabel_finds_tagged_doc() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tax = load_taxonomy(&root).unwrap();
    let alt = "vox-kb-facets";
    let pref = &tax
        .topics
        .iter()
        .find(|c| c.alt_labels.iter().any(|l| l == alt))
        .expect("real taxonomy lists vox-kb-facets as an altLabel")
        .pref_label;
    let doc = format!("---\ntitle: Real\ntopic: {pref}\n---\n# Real\nPlain body text.\n");
    let (_r, db, lex) = indexed(&[("real.md", &doc)]).await;
    let hits = search_with_taxonomy(&db, &lex, alt, 5, Some(&tax))
        .await
        .unwrap();
    assert!(
        paths(&hits).contains(&"docs/src/real.md"),
        "{alt} -> {pref}"
    );
}

#[test]
fn doc_trust_tier_strings_match_context_trust_tier() {
    use vox_db::ContextTrustTier as C;
    let doc: std::collections::BTreeSet<String> = [
        vox_kb_facets::TrustTier::Untrusted,
        vox_kb_facets::TrustTier::Advisory,
        vox_kb_facets::TrustTier::Trusted,
        vox_kb_facets::TrustTier::SystemVerified,
    ]
    .iter()
    .map(|t| {
        serde_json::to_value(t)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    })
    .collect();
    let ctx: std::collections::BTreeSet<String> =
        [C::Untrusted, C::Advisory, C::Trusted, C::SystemVerified]
            .iter()
            .map(|t| t.as_db_str().to_string())
            .collect();
    assert_eq!(doc, ctx);
}
