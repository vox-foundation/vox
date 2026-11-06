//! End-to-End Integration Suite for Milestone `v3.7` (`Phases 193–198`, `DOCS-PROD-*`).

use std::fs;
use std::path::Path;

use vox_doc_pipeline::pipeline::doctest::check_doctests;
use vox_doc_pipeline::pipeline::openwiki::generate_subsystem_hubs;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn phase_193_self_maintaining_naming_and_graduated_facets() {
    let root = repo_root();
    let paths = generate_subsystem_hubs(root, false).expect("subsystem hubs up to date");
    assert_eq!(paths.len(), 7);

    let index_md =
        fs::read_to_string(root.join("docs/src/architecture/subsystems/index.generated.md"))
            .expect("read subsystem index");
    assert!(index_md.contains("title: \"Subsystem Architecture Atlas (L0–L5)\""));
    assert!(!index_md.contains("OpenWiki"));

    let backlog = fs::read_to_string(root.join("contracts/documentation/kb-facet-backlog.v1.yaml"))
        .expect("read kb-facet-backlog");
    assert!(
        !backlog.contains("docs/src/tutorials/tut-getting-started.md"),
        "tut-getting-started.md must be graduated from kb-facet-backlog.v1.yaml"
    );

    let args = vec![
        "vox-doc-pipeline".to_string(),
        "--lint-only".to_string(),
        "--paths".to_string(),
        "tutorials/tut-getting-started.md,how-to/how-to-axis-workbench.md,architecture/openwiki-self-healing-docs-ssot-2026.md".to_string(),
    ];
    let code = vox_doc_pipeline::pipeline::lint_in(root, &args);
    assert_eq!(
        code, 0,
        "updated onboarding and workbench guides must lint cleanly"
    );
}

#[test]
fn phase_194_and_196_sidebar_and_playground_contracts() {
    let root = repo_root();
    let sidebar_src = fs::read_to_string(root.join("docs-astro/src/utils/sidebar.mjs"))
        .expect("read sidebar.mjs");
    assert!(sidebar_src.contains("Pinned — Core Architecture"));
    assert!(sidebar_src.contains("Subsystem Layer Atlas (L0–L5)"));
    assert!(sidebar_src.contains("buildApiCratesSection"));
    assert!(sidebar_src.contains("buildArchitectureSsotSection"));
    assert!(!sidebar_src.contains("text: 'Unverified'"));

    let playground_src =
        fs::read_to_string(root.join("docs-astro/src/components/VoxPlayground.astro"))
            .expect("read VoxPlayground.astro");
    assert!(playground_src.contains("data-target-id=\"runtime\""));
    assert!(playground_src.contains("data-target-id=\"sql\""));
    assert!(playground_src.contains("data-target-id=\"rust\""));
    assert!(playground_src.contains("data-target-id=\"tsx\""));
    assert!(playground_src.contains("compileVoxTargets"));

    let doc_reader_src = fs::read_to_string(
        root.join("crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx"),
    )
    .expect("read DocReader.tsx");
    assert!(!doc_reader_src.contains("<select"));
    assert!(doc_reader_src.contains("kb-pinned-guides-bar"));
}

#[test]
fn phase_198_zero_vox_skip_enforcement_in_doctest_engine() {
    let dummy_path = Path::new("docs/src/tutorials/sample.md");
    let skipped_md = "# Sample\n\n```vox\n// vox:skip\nfn broken(\n```\n";
    let mut errors = Vec::new();
    check_doctests(dummy_path, skipped_md, &mut errors);
    assert_eq!(
        errors.len(),
        1,
        "non-archive ```vox fence with vox:skip must be rejected by check_doctests"
    );

    let valid_md =
        "# Sample\n\n```vox\nfn add(a: int, b: int) to int {\n    return a + b\n}\n```\n";
    let mut valid_errors = Vec::new();
    check_doctests(dummy_path, valid_md, &mut valid_errors);
    assert!(
        valid_errors.is_empty(),
        "valid .vox block must pass: {valid_errors:?}"
    );
}

#[test]
fn phase_199_to_201_show_dont_tell_and_concept_art_b_craft() {
    let root = repo_root();
    let index_mdx = fs::read_to_string(root.join("docs/src/index.mdx")).expect("read index.mdx");
    assert!(
        !index_mdx.contains("Progressive Onboarding"),
        "index.mdx must show, not tell (no 'Progressive Onboarding' meta-heading)"
    );
    assert!(!index_mdx.contains("Tier 1:"));
    assert!(!index_mdx.contains("Tier 2:"));
    assert!(!index_mdx.contains("Tier 3:"));
    assert!(index_mdx.contains("60s:"));
    assert!(index_mdx.contains("5-Min:"));
    assert!(index_mdx.contains("15-Min:"));
    assert!(index_mdx.contains("Run that now"));

    let playground_src =
        fs::read_to_string(root.join("docs-astro/src/components/VoxPlayground.astro"))
            .expect("read VoxPlayground.astro");
    assert!(playground_src.contains("editor-line-gutter"));
    assert!(playground_src.contains("secondary-line-gutter"));
    assert!(playground_src.contains("code-highlight-layer"));
    assert!(playground_src.contains("highlightCodeBlock"));
}
