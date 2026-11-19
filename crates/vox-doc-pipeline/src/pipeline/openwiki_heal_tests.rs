use super::*;
use crate::pipeline::openwiki_polyglot::openwiki_init;

fn setup_polyglot_fixture(name: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("vox-openwiki-heal-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"heal-svc\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "/// Authenticate user session.\npub fn verify_session(token: &str) -> bool {\n    !token.is_empty()\n}\n",
    )
    .unwrap();
    openwiki_init(&root, None).expect("init openwiki");
    root
}

#[test]
fn tier1_blast_radius_heals_signature_and_shifted_line_anchors_without_llm() {
    let root = setup_polyglot_fixture("tier1");

    // Add human prose containing a valid #L2-L4 anchor to interfaces-and-apis.md
    let api_doc_path = root.join("docs/wiki/interfaces-and-apis.md");
    let mut md = fs::read_to_string(&api_doc_path).unwrap();
    md.push_str("\nCustom human note pointing to [`verify_session`](../../src/lib.rs#L2-L4).\n");
    fs::write(&api_doc_path, &md).unwrap();

    // Mutate src/lib.rs: insert header comments so verify_session shifts lines & changes signature
    fs::write(
        root.join("src/lib.rs"),
        "// Header 1\n// Header 2\n// Header 3\n// Header 4\n/// Authenticate user session v2.\npub fn verify_session(token: &str, strict: bool) -> bool {\n    strict && !token.is_empty()\n}\n",
    )
    .unwrap();

    let report = heal_tier1_deterministic(&root, &["src/lib.rs".to_string()], true, None)
        .expect("tier1 heal");

    assert_eq!(report.llm_calls, 0);
    assert!(
        report
            .blast_radius
            .impacted_docs
            .iter()
            .any(|d| d.doc_path == "docs/wiki/interfaces-and-apis.md"),
        "{:?}",
        report.blast_radius
    );
    assert!(report.repaired_anchors >= 1, "{report:?}");

    let healed = fs::read_to_string(&api_doc_path).unwrap();
    assert!(
        healed.contains("verify_session(token: &str, strict: bool)"),
        "{healed}"
    );
    assert!(healed.contains("../../src/lib.rs#L6-L8"), "{healed}");
    assert!(healed.contains("Custom human note pointing to"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn tier2_four_gates_accepts_grounded_patch_and_rejects_violations() {
    let root = setup_polyglot_fixture("tier2");
    let rel_doc = "docs/wiki/interfaces-and-apis.md";

    let candidate = synthesize_grounded_prose_patch(
        &root,
        rel_doc,
        "verify_session",
        "Returns true only for non-empty tokens.",
        "[src/lib.rs:L2-L4]",
        None,
    )
    .expect("synthesize");
    assert!(candidate.gate_report.passed, "{:?}", candidate.gate_report);
    assert!(candidate.unified_diff.contains("verify_session"));

    // Demote negation / add retired symbol -> Gate 1 fails
    let orig = fs::read_to_string(root.join(rel_doc)).unwrap();
    let bad_retired = orig.replace(
        "## Human Architecture Notes",
        "## Human Architecture Notes\nProbeOnlyExecutor",
    );
    let bad_gate1 = verify_tier2_candidate_four_gates(
        &root,
        rel_doc,
        &orig,
        &bad_retired,
        &["[src/lib.rs:L2-L4]".to_string()],
    );
    assert!(!bad_gate1.rewrite_guard_passed);
    assert!(!bad_gate1.passed);

    // Inject legacy mermaid -> Gate 3 fails
    let bad_mermaid = format!("{orig}\n```mermaid\ngraph TD\nA-->B\n```\n");
    let bad_gate3 = verify_tier2_candidate_four_gates(
        &root,
        rel_doc,
        &orig,
        &bad_mermaid,
        &["[src/lib.rs:L2-L4]".to_string()],
    );
    assert!(!bad_gate3.diagram_check_passed);
    assert!(!bad_gate3.passed);

    // Apply valid patch and test demote_doc_to_legacy
    apply_heal_patch_to_file(&root, rel_doc, &candidate.patched_markdown).expect("apply");
    let on_disk = fs::read_to_string(root.join(rel_doc)).unwrap();
    assert!(on_disk.contains("Returns true only for non-empty tokens."));

    let demoted = demote_doc_to_legacy(&root, rel_doc).expect("demote");
    assert!(demoted.contains("status: \"legacy\""));

    let _ = fs::remove_dir_all(&root);
}
