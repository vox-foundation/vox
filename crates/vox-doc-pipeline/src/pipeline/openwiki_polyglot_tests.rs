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
    let api_doc_path = root.join("docs/wiki/interfaces-and-apis.md");
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
