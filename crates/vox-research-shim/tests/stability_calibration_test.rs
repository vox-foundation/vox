use vox_research_shim::research::claims::Claim;
use vox_research_shim::research::orchestrator::wave::WaveExecutionPlan;
use vox_research_shim::research::verifier::{ClaimVerdict, Verdict};

#[test]
fn test_stability_blocks_premature_exit_when_unverified_claims_exist() {
    let mut plan = WaveExecutionPlan::new("session-calibration", 3);
    // 5 claims: 3 supported (60%), 2 unverified (40%)
    plan.resolved_verdicts = vec![
        ClaimVerdict {
            claim: Claim {
                claim_id: 1,
                text: "c1".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.95,
            supporting_count: 1,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 2,
                text: "c2".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.95,
            supporting_count: 1,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 3,
                text: "c3".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.95,
            supporting_count: 1,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 4,
                text: "c4".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Unverified,
            confidence: 0.50,
            supporting_count: 0,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 0.5,
        },
        ClaimVerdict {
            claim: Claim {
                claim_id: 5,
                text: "c5".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Unverified,
            confidence: 0.50,
            supporting_count: 0,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 0.5,
        },
    ];

    // Must NOT early terminate when 40% of claims are unverified!
    assert!(
        !plan.should_early_terminate(),
        "Premature termination blocked: 40% unverified claims remain"
    );
}

#[test]
fn test_stability_allows_early_exit_when_high_support_and_no_contradictions() {
    let mut plan = WaveExecutionPlan::new("session-calibration-pass", 3);
    // 5 claims: 5 supported (100%), 0 unverified, resample stability 1.0
    plan.resolved_verdicts = (1..=5)
        .map(|id| ClaimVerdict {
            claim: Claim {
                claim_id: id,
                text: format!("claim-{id}"),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported,
            confidence: 0.98,
            supporting_count: 2,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 1.0,
        })
        .collect();

    assert!(plan.compute_stability() >= 0.85);
    assert!(
        plan.should_early_terminate(),
        "Should early terminate when all claims supported and verified"
    );
}
