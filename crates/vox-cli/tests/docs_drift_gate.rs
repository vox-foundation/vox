//! Integration tests for Docs Drift Gate, Baseline, Suppressions, and Attestation (DRIFT-04, DRIFT-05, DRIFT-06).

use chrono::NaiveDate;
use std::collections::HashSet;
use vox_doc_pipeline::claims::findings::{FindingScores, RealityAuditFinding};
use vox_doc_pipeline::claims::{
    BaselineFile, DocSuppression, GrandfatheredDeficit, SuppressionsFile, evaluate_drift_gate,
};

fn sample_finding(id: &str, fp: &str, doc_path: &str, target: &str) -> RealityAuditFinding {
    RealityAuditFinding {
        id: id.to_string(),
        claim_ids: vec![target.to_string()],
        doc_paths: vec![doc_path.to_string()],
        code_paths: vec![target.to_string()],
        contract_paths: Vec::new(),
        claim_type: "cli_command".to_string(),
        classification: "missing".to_string(),
        secondary_tags: Vec::new(),
        scores: FindingScores {
            impact: 3,
            blast_radius: 2,
            staleness: 1,
            enforcement_gap: 3,
            tractability: 3,
        },
        priority_score: 54,
        priority_band: "P1".to_string(),
        recommended_action: format!("Update mention {}", target),
        owner_candidate: None,
        status: "missing".to_string(),
        notes: None,
        source: Some("machine_audit".to_string()),
        fingerprint: Some(fp.to_string()),
        detector: Some("claims-extractor".to_string()),
    }
}

#[test]
fn docs_audit_cli_help_includes_drift_flags() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["audit", "docs", "--help"])
        .output()
        .expect("failed to execute vox audit docs --help");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains("--check-drift"),
        "help should contain --check-drift:\n{s}"
    );
    assert!(
        s.contains("--tighten-baseline"),
        "help should contain --tighten-baseline:\n{s}"
    );
    assert!(
        s.contains("--generate-baseline"),
        "help should contain --generate-baseline:\n{s}"
    );
    assert!(s.contains("--attest"), "help should contain --attest:\n{s}");
}

#[test]
fn drift_gate_accepts_baselined_deficits() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    let baseline = BaselineFile {
        schema_version: 1,
        generated_at: "2026-10-13T00:00:00Z".to_string(),
        deficits: vec![GrandfatheredDeficit {
            fingerprint: "fp-baselined".to_string(),
            page_path: "docs/src/reference/cli.md".to_string(),
            raw_text: "vox old-cmd".to_string(),
        }],
    };

    let findings = vec![sample_finding(
        "doc-deficit-0001",
        "fp-baselined",
        "docs/src/reference/cli.md",
        "vox old-cmd",
    )];

    let suppressions = SuppressionsFile {
        schema_version: 1,
        suppressions: Vec::new(),
    };

    let result = evaluate_drift_gate(&findings, &baseline, &suppressions, today);
    assert!(
        result.passed,
        "gate should pass when all deficits are in baseline"
    );
    assert_eq!(result.new_deficits.len(), 0);
    assert_eq!(result.active_baselined_count, 1);
}

#[test]
fn drift_gate_blocks_new_unsuppressed_deficits() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    let baseline = BaselineFile {
        schema_version: 1,
        generated_at: "2026-10-13T00:00:00Z".to_string(),
        deficits: Vec::new(),
    };

    let findings = vec![sample_finding(
        "doc-deficit-0002",
        "fp-new-broken",
        "docs/src/tutorials/hello.md",
        "vox missing-command",
    )];

    let suppressions = SuppressionsFile {
        schema_version: 1,
        suppressions: Vec::new(),
    };

    let result = evaluate_drift_gate(&findings, &baseline, &suppressions, today);
    assert!(
        !result.passed,
        "gate must fail on new unsuppressed deficits"
    );
    assert_eq!(result.new_deficits.len(), 1);
}

#[test]
fn drift_gate_allows_valid_suppression_and_rejects_expired() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    let baseline = BaselineFile {
        schema_version: 1,
        generated_at: "2026-10-13T00:00:00Z".to_string(),
        deficits: Vec::new(),
    };

    let findings = vec![sample_finding(
        "doc-deficit-0003",
        "fp-suppressed",
        "docs/src/tutorials/hello.md",
        "vox missing-command",
    )];

    // Active suppression (expires in 2099)
    let active_suppression = DocSuppression {
        id: "sup-001".to_string(),
        page_path: "docs/src/tutorials/hello.md".to_string(),
        target: "vox missing-command".to_string(),
        owner: "@docs-team".to_string(),
        reason: "Refactoring command in next PR".to_string(),
        expires_after: "2099-01-01".to_string(),
    };
    let suppressions_active = SuppressionsFile {
        schema_version: 1,
        suppressions: vec![active_suppression],
    };

    let result_active = evaluate_drift_gate(&findings, &baseline, &suppressions_active, today);
    assert!(result_active.passed, "gate should pass when suppressed");
    assert_eq!(result_active.new_deficits.len(), 0);

    // Expired suppression (expired in 2020)
    let expired_suppression = DocSuppression {
        id: "sup-002".to_string(),
        page_path: "docs/src/tutorials/hello.md".to_string(),
        target: "vox missing-command".to_string(),
        owner: "@docs-team".to_string(),
        reason: "Expired work".to_string(),
        expires_after: "2020-01-01".to_string(),
    };
    let suppressions_expired = SuppressionsFile {
        schema_version: 1,
        suppressions: vec![expired_suppression],
    };

    let result_expired = evaluate_drift_gate(&findings, &baseline, &suppressions_expired, today);
    assert!(
        !result_expired.passed,
        "gate must fail with expired suppressions"
    );
    assert_eq!(result_expired.expired_suppression_errors.len(), 1);
}

#[test]
fn tighten_baseline_shrinks_when_deficits_resolved() {
    let mut baseline = BaselineFile {
        schema_version: 1,
        generated_at: "2026-10-13T00:00:00Z".to_string(),
        deficits: vec![
            GrandfatheredDeficit {
                fingerprint: "fp-resolved".to_string(),
                page_path: "docs/src/reference/fixed.md".to_string(),
                raw_text: "resolved".to_string(),
            },
            GrandfatheredDeficit {
                fingerprint: "fp-still-broken".to_string(),
                page_path: "docs/src/reference/broken.md".to_string(),
                raw_text: "still-broken".to_string(),
            },
        ],
    };

    let mut active_fps = HashSet::new();
    active_fps.insert("fp-still-broken".to_string());

    let removed = baseline.tighten(&active_fps);
    assert_eq!(removed, 1, "should remove 1 resolved deficit");
    assert_eq!(baseline.deficits.len(), 1);
    assert_eq!(baseline.deficits[0].fingerprint, "fp-still-broken");
}
