use super::*;
use std::collections::BTreeMap;

#[test]
fn auto_region_merge_preserves_human_prose_and_updates_fences() {
    let existing = "---\ntitle: \"Demo\"\n---\n\n# Demo\n\nHuman intro stays untouched.\n\n<!-- openwiki:auto:start id=\"repo-overview\" -->\nold line\n<!-- openwiki:auto:end id=\"repo-overview\" -->\n\n## Team Rationale\n\nNever overwrite this human decision note.\n";
    let mut regions = BTreeMap::new();
    regions.insert(
        "repo-overview".to_string(),
        "new deterministic line".to_string(),
    );
    let (merged, updated) = merge_openwiki_auto_regions(existing, &regions).expect("merge");
    assert_eq!(updated, vec!["repo-overview".to_string()]);
    assert!(merged.contains("Human intro stays untouched."));
    assert!(merged.contains("Never overwrite this human decision note."));
    assert!(merged.contains("new deterministic line"));
    assert!(!merged.contains("old line"));

    let extracted = extract_openwiki_auto_regions(&merged);
    assert_eq!(
        extracted.get("repo-overview").map(String::as_str),
        Some("new deterministic line")
    );
}

#[test]
fn polyglot_openwiki_synthesizes_five_layers_and_passes_doc_lint() {
    let root =
        std::env::temp_dir().join(format!("vox-polyglot-openwiki-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/components")).unwrap();
    fs::create_dir_all(root.join("migrations")).unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();

    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"poly-svc\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "/// Public health endpoint.\npub fn health_check() -> bool { true }\n#[derive(Clone)]\npub struct HealthRecord { pub ok: bool }\n",
    )
    .unwrap();
    fs::write(
        root.join("package.json"),
        "{\"name\": \"poly-web\", \"version\": \"1.0.0\"}\n",
    )
    .unwrap();
    fs::write(
        root.join("src/components/AtlasPanel.tsx"),
        "export function AtlasPanel() { return null; }\n",
    )
    .unwrap();
    fs::write(
        root.join("migrations/001.sql"),
        concat!(
            "CREATE ",
            "TABLE items (id INTEGER PRIMARY KEY, name TEXT);\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "name: CI Gate\non: [push]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/app.vox"),
        "workflow deploy_app() {\n  fn run() {}\n}\n",
    )
    .unwrap();

    let report = openwiki_build(&root, true, None).expect("build polyglot openwiki");
    assert_eq!(report.generated_pages.len(), 6);
    assert!(report.detected_stacks.contains(&"Rust/Cargo".to_string()));
    assert!(
        report
            .detected_stacks
            .contains(&"TypeScript/Node".to_string())
    );
    assert!(report.detected_stacks.contains(&"SQL".to_string()));
    assert!(report.detected_stacks.contains(&"Vox".to_string()));
    assert!(Path::new(&report.manifest_path).is_file());

    // Add custom human prose to Layer 2 and ensure rebuild leaves it intact
    let api_doc_path = openwiki_pages_dir(&root).join("interfaces-and-apis.md");
    let custom_api = fs::read_to_string(&api_doc_path)
        .unwrap()
        .replace(
            "Document design rationale, invariants, and operational caveats here; OpenWiki preserves prose outside `openwiki:auto` fences.",
            "CUSTOM TEAM NOTE: keep health_check backwards compatible.",
        );
    fs::write(&api_doc_path, &custom_api).unwrap();

    let rerun = openwiki_build(&root, true, None).expect("rebuild");
    assert!(rerun.updated_pages.is_empty(), "{rerun:?}");
    assert!(
        fs::read_to_string(&api_doc_path)
            .unwrap()
            .contains("CUSTOM TEAM NOTE: keep health_check backwards compatible.")
    );

    // Every generated page passes external vox-doc-pipeline --lint-only cleanly with 0 legacy Mermaid
    let args = vec!["vox-doc-pipeline".to_string(), "--lint-only".to_string()];
    assert_eq!(crate::pipeline::lint_in(&root, &args), 0);

    let status = openwiki_status(&root, None).expect("status");
    assert!(status.initialized);
    assert_eq!(status.stale_pages, 0);

    let diff = openwiki_diff(&root, None).expect("diff");
    assert!(!diff.has_changes);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pages_dir_precedence_docs_wiki_then_docs_src_then_docs() {
    let base = std::env::temp_dir().join(format!("vox-owpd-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);

    let bare = base.join("bare");
    fs::create_dir_all(&bare).unwrap();
    assert_eq!(openwiki_pages_rel(&bare), "docs/openwiki");

    let docs_only = base.join("docs_only");
    fs::create_dir_all(docs_only.join("docs")).unwrap();
    assert_eq!(openwiki_pages_rel(&docs_only), "docs/openwiki");

    let docs_src = base.join("docs_src");
    fs::create_dir_all(docs_src.join("docs/src")).unwrap();
    assert_eq!(openwiki_pages_rel(&docs_src), "docs/src/openwiki");

    let legacy = base.join("legacy");
    fs::create_dir_all(legacy.join("docs/src")).unwrap();
    fs::create_dir_all(legacy.join("docs/wiki")).unwrap();
    assert_eq!(openwiki_pages_rel(&legacy), "docs/wiki");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn uncapped_snapshot_covers_every_symbol_in_large_synthetic_repo() {
    let root = std::env::temp_dir().join(format!("vox-owuncap-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"big\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    for f in 0..150 {
        let mut body = String::new();
        for s in 0..3 {
            body.push_str(&format!("pub fn sym_{f}_{s}() -> bool {{ true }}\n"));
        }
        fs::write(root.join(format!("src/m{f}.rs")), body).unwrap();
    }

    let snap = extract_polyglot_wiki_snapshot(&root);
    assert_eq!(snap.interfaces.len(), 450, "no silent entry cap");

    let pages = render_polyglot_openwiki_pages(&snap).expect("render");
    let l2 = pages
        .iter()
        .find(|p| p.rel_path == "interfaces-and-apis.md")
        .expect("layer 2 page");
    assert!(
        l2.full_markdown.contains("450 symbols across 1 modules"),
        "layer page keeps a rollup index"
    );
    assert!(l2.full_markdown.contains("./interfaces-and-apis/src.md"));
    let detail = pages
        .iter()
        .find(|p| p.rel_path == "interfaces-and-apis/src.md")
        .expect("per-module detail sub-page");
    assert!(
        detail.full_markdown.contains("sym_149_2"),
        "last symbol rendered"
    );
    assert!(detail.full_markdown.contains("### `src` (450)"));
    assert!(
        detail.full_markdown.contains("](../../../src/m0.rs#L1)"),
        "sub-page anchors are one directory deeper"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn docs_src_repo_builds_under_docs_src_openwiki_and_never_creates_docs_wiki() {
    let root = std::env::temp_dir().join(format!("vox-owsrc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs/src")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"ds\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn alpha() -> bool { true }\n").unwrap();

    let report = openwiki_build(&root, true, None).expect("build");
    assert!(
        report
            .generated_pages
            .iter()
            .all(|p| p.starts_with("docs/src/openwiki/")),
        "{:?}",
        report.generated_pages
    );
    assert!(root.join("docs/src/openwiki/README.md").is_file());
    assert!(!root.join("docs/wiki").exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn sql_foreign_keys_are_annotated_via_graph_reader() {
    let root = std::env::temp_dir().join(format!("vox-owsqlfk-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("migrations")).unwrap();
    fs::write(
        root.join("migrations/001.sql"),
        concat!(
            "CREATE ",
            "TABLE users (id INTEGER PRIMARY KEY);\n",
            "CREATE ",
            "TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id));\n"
        ),
    )
    .unwrap();

    let snap = extract_polyglot_wiki_snapshot(&root);
    let posts = snap
        .schemas
        .iter()
        .find(|s| s.name == "posts")
        .expect("posts table");
    assert!(
        posts.signature.contains("references users"),
        "{}",
        posts.signature
    );
    let users = snap
        .schemas
        .iter()
        .find(|s| s.name == "users")
        .expect("users table");
    assert!(
        !users.signature.contains("references"),
        "{}",
        users.signature
    );

    let _ = fs::remove_dir_all(&root);
}
