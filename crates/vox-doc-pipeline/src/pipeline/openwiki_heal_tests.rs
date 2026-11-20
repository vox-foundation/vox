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
    let pages_rel = crate::pipeline::openwiki_polyglot::openwiki_pages_rel(&root);
    let api_rel = format!("{pages_rel}/interfaces-and-apis.md");
    let api_doc_path = root.join(&api_rel);
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
            .any(|d| d.doc_path == api_rel),
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
    let rel_doc_owned = format!(
        "{}/interfaces-and-apis.md",
        crate::pipeline::openwiki_polyglot::openwiki_pages_rel(&root)
    );
    let rel_doc = rel_doc_owned.as_str();

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

#[test]
fn line_scoped_fix_changes_exactly_one_token_on_one_line_and_is_gated() {
    let root = setup_polyglot_fixture("linefix");
    let rel_doc = format!(
        "{}/interfaces-and-apis.md",
        crate::pipeline::openwiki_polyglot::openwiki_pages_rel(&root)
    );
    let path = root.join(&rel_doc);
    let mut md = fs::read_to_string(&path).unwrap();
    md.push_str("\nUse `old_helper` for sessions.\nKeep this line as is.\n");
    fs::write(&path, &md).unwrap();
    let line_no = md
        .split('\n')
        .position(|l| l.contains("`old_helper`"))
        .unwrap()
        + 1;

    let c = synthesize_line_scoped_fix(
        &root,
        &rel_doc,
        line_no,
        "old_helper",
        "verify_session",
        "[src/lib.rs:L2-L4]",
    )
    .expect("fix");
    let before: Vec<&str> = md.split('\n').collect();
    let after: Vec<&str> = c.patched_markdown.split('\n').collect();
    assert_eq!(before.len(), after.len());
    let changed: Vec<usize> = (0..before.len())
        .filter(|i| before[*i] != after[*i])
        .collect();
    assert_eq!(
        changed,
        vec![line_no - 1],
        "only the target line may change"
    );
    assert!(after[line_no - 1].contains("`verify_session`"));
    assert!(c.unified_diff.contains("-Use `old_helper` for sessions."));
    assert!(c.gate_report.passed, "{:?}", c.gate_report);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        md,
        "planning must not write"
    );

    // Refuses to guess when the token is not on that line.
    assert!(
        synthesize_line_scoped_fix(
            &root,
            &rel_doc,
            line_no,
            "absent",
            "x",
            "[src/lib.rs:L2-L4]"
        )
        .is_err()
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn auto_fix_only_trusts_exact_crate_renames_not_prefix_or_command_matches() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let registry = crate::claims::GroundTruthRegistry::load(root).expect("registry");
    assert_eq!(
        registry.exact_retired_crate_replacement("vox-pm"),
        Some("vox-package")
    );
    // Prefix extension of a retired name and a command containing it: never auto-fixed.
    assert_eq!(registry.exact_retired_crate_replacement("vox-dei-d"), None);
    assert_eq!(
        registry.exact_retired_crate_replacement("vox ci no-vox-dei-import"),
        None
    );
    // Ambiguous in the contract (HITL crate vs old orchestrator): not in the exact table.
    assert_eq!(registry.exact_retired_crate_replacement("vox-dei"), None);

    // And on the real corpus no planned fix may ever target those tokens.
    let doc = "docs/src/adr/022-orchestrator-bootstrap-and-daemon-boundaries.md";
    for c in plan_retired_symbol_fixes(root, &registry, doc).expect("plan") {
        assert!(!c.summary.contains("`vox-dei"), "{}", c.summary);
        assert!(!c.summary.contains("no-vox-dei-import"), "{}", c.summary);
    }
}

#[test]
fn autofix_never_targets_point_in_time_records() {
    for blocked in [
        "docs/src/adr/022-orchestrator-bootstrap-and-daemon-boundaries.md",
        "docs/src/reference/changelog.md",
        "docs/src/architecture/history/2026-04-core-ten-charter.md",
        "docs/src/architecture/2026-05-08-crate-org-followup-design.md",
        "docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md",
        "docs/src/archive/x.md",
    ] {
        assert!(!is_autofix_eligible_path(blocked), "{blocked}");
    }
    for ok in [
        "docs/src/reference/populi.md",
        "docs/src/how-to/how-to-model-routing.md",
        "docs/src/architecture/where-things-live.md",
        "docs/src/architecture/model-orchestration-ssot-audit-2026.md",
    ] {
        assert!(is_autofix_eligible_path(ok), "{ok}");
    }
}

#[test]
fn retired_crate_rename_is_planned_as_one_gated_line_and_writes_nothing() {
    let real_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let registry = crate::claims::GroundTruthRegistry::load(real_root).expect("registry");

    let root = setup_polyglot_fixture("retired");
    // The replacement crate must exist for its manifest to serve as evidence.
    fs::create_dir_all(root.join("crates/vox-package")).unwrap();
    fs::write(
        root.join("crates/vox-package/Cargo.toml"),
        "[package]\nname = \"vox-package\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let rel_doc = format!(
        "{}/interfaces-and-apis.md",
        crate::pipeline::openwiki_polyglot::openwiki_pages_rel(&root)
    );
    let path = root.join(&rel_doc);
    let mut md = fs::read_to_string(&path).unwrap();
    md.push_str("\nBuild it with the `vox-pm` crate.\nAnd leave this line alone.\n");
    fs::write(&path, &md).unwrap();

    let plans = plan_retired_symbol_fixes(&root, &registry, &rel_doc).expect("plan");
    assert_eq!(plans.len(), 1, "{plans:?}");
    let c = &plans[0];
    let b: Vec<&str> = md.split('\n').collect();
    let a: Vec<&str> = c.patched_markdown.split('\n').collect();
    assert_eq!(a.len(), b.len());
    assert_eq!((0..a.len()).filter(|i| a[*i] != b[*i]).count(), 1);
    assert!(
        c.patched_markdown
            .contains("Build it with the `vox-package` crate.")
    );
    assert!(c.gate_report.passed, "{:?}", c.gate_report);
    assert!(
        c.evidence_citations
            .iter()
            .all(|e| !crate::claims::llm_verifier::is_doc_self_citation(&rel_doc, e))
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        md,
        "planning must not write"
    );

    // Applying writes exactly that one fix and is idempotent.
    let applied = apply_retired_symbol_fixes(&root, &registry, &rel_doc).expect("apply");
    assert_eq!(applied.len(), 1, "{applied:?}");
    assert!(
        apply_retired_symbol_fixes(&root, &registry, &rel_doc)
            .expect("re-apply")
            .is_empty()
    );
    let _ = fs::remove_dir_all(&root);
}
