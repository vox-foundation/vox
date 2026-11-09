//! Helpers shared by the hub_card tests.

use std::path::{Path, PathBuf};

use crate::mens::hub_card::card_facts::HeldOut;
use crate::mens::hub_card::facts::{AdapterEvidence, GateState, load_requirement};
use crate::mens::hub_card::index_facts::{HubCell, IndexRow};
use crate::mens::hub_card::{
    CardContext, RunRecords, render_index_readme, render_model_card, repo_name,
};
use crate::mens::hub_sync::{SizeSyncRow, SizeSyncStatus};
use vox_eval::CollateralRequirement;

/// `crates/vox-populi/tests/fixtures/hub_cards`.
pub(crate) fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hub_cards")
}

/// `Ok` when `src` passes the Vox compiler front end (lex, parse, HIR, typeck) and the
/// interpreter smoke check; otherwise the receipt's stage and diagnostics.
pub(crate) fn vox_block_ok(src: &str) -> Result<(), String> {
    let r = vox_corpus::corpus::corpus_readiness::verify_vox_admission(src);
    if r.passed {
        Ok(())
    } else {
        Err(format!("stage {}: {}", r.stage, r.diagnostics.join("; ")))
    }
}

/// Copy the committed held-out evidence into `<run>/evidence`, taking the adapter checkpoint from
/// `adapter_ckpt` (a path under `fixtures/hub_cards/`, e.g. the `legacy_serving` copy).
pub(crate) fn copy_evidence(run: &Path, adapter_ckpt: &str) {
    let ev = run.join("evidence");
    std::fs::create_dir_all(&ev).expect("evidence dir");
    let src = fixture_root().join("evidence");
    for f in [
        "heldout_base_nothink.json",
        "heldout_adapter_nothink.json",
        "heldout_base_nothink.ckpt.jsonl",
    ] {
        std::fs::copy(src.join(f), ev.join(f)).expect("copy fixture");
    }
    std::fs::copy(
        fixture_root().join(adapter_ckpt),
        ev.join("heldout_adapter_nothink.ckpt.jsonl"),
    )
    .expect("copy adapter ckpt");
}

/// A fresh run dir holding the committed held-out evidence.
pub(crate) fn evidence_run(adapter_ckpt: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_evidence(dir.path(), adapter_ckpt);
    dir
}

/// Pin of the 1.7B base the case files were taken from.
pub(crate) const PIN_17: &str = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e"; // gitleaks:allow

/// One card case: a record set in `fixtures/hub_cards/cases/<file>.json` under `key`, plus the
/// held-out evidence it is rendered with.
pub(crate) struct Case {
    pub key: &'static str,
    pub file: &'static str,
    pub size: &'static str,
    pub pin: &'static str,
    /// Adapter checkpoint fixture ([`CURRENT_CKPT`] or [`LEGACY_CKPT`]); `None` for no evidence.
    pub ckpt: Option<&'static str>,
    /// Whether the run dir holds adapter weights.
    pub adapter: bool,
    /// Whether the base checkpoint is present next to the held-out reports.
    pub base_ckpt: bool,
    /// Replace the case's collateral report with [`passing_collateral_report`] for the current rule.
    pub passing: bool,
}

/// A report built from vox_eval's writer pieces under `req`: current rule, pins and grader.
/// The one builder 339-01's facts tests use, shared here under the name 338 retargets to.
pub(crate) fn passing_collateral_report(req: &CollateralRequirement) -> serde_json::Value {
    crate::mens::gate_policy::passing_collateral_report(req)
}

/// 1.7B, retired-rule collateral pass, adapter evidence from the current serving path.
pub(crate) const NEEDS_RE_GATE: Case = Case {
    key: "needs_re_gate_1_7b",
    file: "1_7b.json",
    size: "1.7B",
    pin: PIN_17,
    ckpt: Some(CURRENT_CKPT),
    adapter: true,
    base_ckpt: true,
    passing: false,
};

/// The same records, but the adapter checkpoint predates the serving-path field.
pub(crate) const STALE_SERVING: Case = Case {
    ckpt: Some(LEGACY_CKPT),
    ..NEEDS_RE_GATE
};

/// The 1.7B records with a collateral report that satisfies the current rule.
pub(crate) const PASSED: Case = Case {
    passing: true,
    ..NEEDS_RE_GATE
};

/// Write the case's records and evidence into `tmp` and return the run dir.
pub(crate) fn case_run_dir(case: &Case, tmp: &Path) -> PathBuf {
    let run = tmp.join(format!("qwen3-{}", case.size.to_ascii_lowercase()));
    std::fs::create_dir_all(&run).expect("run dir");
    let text =
        std::fs::read_to_string(fixture_root().join("cases").join(case.file)).expect("case file");
    let all: serde_json::Value = serde_json::from_str(&text).expect("case json");
    let records = all[case.key].as_object().expect("case key is an object");
    for (rel, record) in records {
        std::fs::write(run.join(rel), record.to_string()).expect("write record");
    }
    if case.passing {
        let req = load_requirement(None).expect("current collateral requirement");
        let report = passing_collateral_report(&req).to_string();
        std::fs::write(run.join("collateral_damage_report.json"), report).expect("collateral");
        let ladder_hash =
            crate::mens::gate_policy::policy_blake3(None, "mens/config/eval-gates-ladder.yaml")
                .expect("ladder policy blake3");
        let receipt = serde_json::json!({
            "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
            "policy": "mens/config/eval-gates-ladder.yaml",
            "policy_blake3": ladder_hash,
            "overall_passed": true,
            "gates": [
                {
                    "name": "leakage",
                    "passed": true,
                    "block": true,
                    "message": "no bench-answer / corpus-completion n-gram overlap detected"
                },
                {
                    "name": "anti_stub",
                    "passed": true,
                    "block": true,
                    "message": "anti_stub_pass=1.000 (min=0.920) placeholder_rate=0.000 (max=0.080) trivial_rate=0.000 (max=0.080) richness_mean=0.125 (min=0.125) [metrics:RecomputedFromSamples]"
                },
                {
                    "name": "pass_at_k",
                    "passed": true,
                    "block": true,
                    "message": "pass@1=0.327 (min=0.250) pass@k=0.327 (min=0.250; k=1 collapses to pass@1) baseline(pass@1=0.058,pass@k=0.058) max_drop=0.030"
                },
                {
                    "name": "review_recurrence",
                    "passed": true,
                    "block": false,
                    "message": "no repeated review comments"
                },
                {
                    "name": "collateral_damage",
                    "passed": true,
                    "block": true,
                    "message": "collateral pass"
                }
            ]
        });
        std::fs::write(run.join("gate_receipt.json"), receipt.to_string()).expect("receipt");
    }
    if case.adapter {
        std::fs::write(run.join("candle_qlora_adapter.safetensors"), b"w").expect("adapter");
    }
    if let Some(ckpt) = case.ckpt {
        copy_evidence(&run, ckpt);
        if !case.base_ckpt {
            std::fs::remove_file(run.join("evidence/heldout_base_nothink.ckpt.jsonl"))
                .expect("remove base ckpt");
        }
    }
    run
}

/// The card context for a case: its size and pin, namespace `ns`.
pub(crate) fn cx_for(case: &Case) -> CardContext<'static> {
    CardContext {
        size: case.size,
        base_pin: case.pin,
        namespace: "ns",
    }
}

/// Load the case's records the way `vox mens hub sync` does, and render its card.
pub(crate) fn render_case(case: &Case) -> String {
    let tmp = tempfile::tempdir().expect("tempdir");
    let run = case_run_dir(case, tmp.path());
    let rec = RunRecords::load(&run, case.size, case.pin);
    render_model_card(&cx_for(case), &rec)
}

/// The current-serving adapter checkpoint fixture.
pub(crate) const CURRENT_CKPT: &str = "evidence/heldout_adapter_nothink.ckpt.jsonl";
/// The real 4B adapter checkpoint header, which carries no serving field.
pub(crate) const LEGACY_CKPT: &str = "evidence/legacy_serving/heldout_adapter_nothink.ckpt.jsonl";

const PIN_06: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca"; // gitleaks:allow
const PIN_14: &str = "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18"; // gitleaks:allow
const PIN_32: &str = "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137"; // gitleaks:allow

/// 0.6B: trained, the collateral check failed on a slice.
pub(crate) const FAILED: Case = Case {
    key: "failed_0_6b",
    file: "states.json",
    size: "0.6B",
    pin: PIN_06,
    ..NEEDS_RE_GATE
};

/// 14B: trained, no collateral report and no gate receipt.
pub(crate) const NOT_EVALUATED: Case = Case {
    key: "not_evaluated_14b",
    file: "states.json",
    size: "14B",
    pin: PIN_14,
    ..NEEDS_RE_GATE
};

/// 32B: a train record that says it was not trained; no adapter and no evidence.
pub(crate) const NOT_TRAINED: Case = Case {
    key: "not_trained_32b",
    file: "states.json",
    size: "32B",
    pin: PIN_32,
    ckpt: None,
    adapter: false,
    ..NEEDS_RE_GATE
};

/// The needs-re-gate records with a current adapter checkpoint and no base checkpoint.
pub(crate) const MISSING_OUTPUT: Case = Case {
    base_ckpt: false,
    ..NEEDS_RE_GATE
};

/// The passed records with no held-out reports or checkpoints at all.
pub(crate) const NO_EXAMPLES: Case = Case {
    ckpt: None,
    ..PASSED
};

const PIN_4: &str = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c"; // gitleaks:allow
const PIN_8: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"; // gitleaks:allow

/// 4B: gate passed, adapter evidence from the current serving path.
const PASSED_4B: Case = Case {
    size: "4B",
    pin: PIN_4,
    ..PASSED
};

fn sync_row(case: &Case, status: SizeSyncStatus, version: Option<&str>) -> SizeSyncRow {
    SizeSyncRow {
        size: case.size.to_string(),
        base_pin: case.pin.to_string(),
        repo_id: format!("ns/{}", repo_name(case.size)),
        local_dir: format!("mens/runs/qwen3-{}", case.size.to_ascii_lowercase()),
        sync_status: status,
        gate_status: None,
        final_loss: None,
        measured_peak_mb: None,
        action_taken: None,
        latest_version: version.map(str::to_string),
    }
}

/// One entry per ladder size, built from `cases` (sync status and Hub tag alongside each).
fn ladder(
    tmp: &Path,
    cases: [(Case, SizeSyncStatus, Option<&str>); 6],
) -> Vec<(SizeSyncRow, RunRecords)> {
    cases
        .iter()
        .map(|(case, status, version)| {
            let run = case_run_dir(case, tmp);
            let rec = RunRecords::load(&run, case.size, case.pin);
            (sync_row(case, *status, *version), rec)
        })
        .collect()
}

/// The ladder as the index sees it with `four_b` as the 4B case: 0.6B failed, 1.7B needs a
/// re-gate with evidence from an earlier serving path, 8B needs a re-gate with current-serving
/// evidence, 14B not evaluated, 32B not trained.
fn ladder_with(tmp: &Path, four_b: Case) -> Vec<(SizeSyncRow, RunRecords)> {
    use SizeSyncStatus::{Diverged, InSync, LocalOnly, NotTrained};
    ladder(
        tmp,
        [
            (FAILED, Diverged, Some("v5")),
            (STALE_SERVING, LocalOnly, None),
            (four_b, InSync, Some("v2")),
            (
                Case {
                    size: "8B",
                    pin: PIN_8,
                    ..NEEDS_RE_GATE
                },
                LocalOnly,
                None,
            ),
            (NOT_EVALUATED, LocalOnly, None),
            (NOT_TRAINED, NotTrained, None),
        ],
    )
}

/// The full ladder with a passed 4B whose adapter evidence is current.
pub(crate) fn ladder_entries(tmp: &Path) -> Vec<(SizeSyncRow, RunRecords)> {
    ladder_with(tmp, PASSED_4B)
}

/// The ladder with no passed size: 4B needs a re-gate like 8B.
pub(crate) fn ladder_entries_no_passed(tmp: &Path) -> Vec<(SizeSyncRow, RunRecords)> {
    ladder_with(
        tmp,
        Case {
            passing: false,
            ..PASSED_4B
        },
    )
}

/// The ladder with a passed 4B whose adapter checkpoint predates the serving-path field.
pub(crate) fn ladder_entries_passed_but_stale(tmp: &Path) -> Vec<(SizeSyncRow, RunRecords)> {
    ladder_with(
        tmp,
        Case {
            ckpt: Some(LEGACY_CKPT),
            ..PASSED_4B
        },
    )
}

/// `(name, markdown)` for one model card per entry and the index, rendered the way
/// `write_local_cards` renders them (namespace `ns`).
pub(crate) fn cards_of(entries: &[(SizeSyncRow, RunRecords)]) -> Vec<(String, String)> {
    let mut cards: Vec<(String, String)> = entries
        .iter()
        .map(|(row, rec)| {
            let cx = CardContext {
                size: &row.size,
                base_pin: &row.base_pin,
                namespace: "ns",
            };
            (repo_name(&row.size), render_model_card(&cx, rec))
        })
        .collect();
    cards.push(("vox-mens-index".into(), render_index_readme("ns", entries)));
    cards
}

/// The full fixture ladder ([`ladder_entries`]) as rendered cards.
pub(crate) fn ladder_cards() -> Vec<(String, String)> {
    let tmp = tempfile::tempdir().expect("tempdir");
    cards_of(&ladder_entries(tmp.path()))
}

/// A synthetic index row for the pick rule: trained, with an adapter pass@1 of `rate` out of 100
/// items when given.
pub(crate) fn index_row(
    size: &str,
    gate: GateState,
    evidence: AdapterEvidence,
    rate: Option<f64>,
) -> IndexRow {
    IndexRow {
        size: size.to_string(),
        repo_id: format!("ns/{}", repo_name(size)),
        trained: true,
        spoke_id: None,
        gate,
        adapter_evidence: evidence,
        hub: HubCell::NotPublished,
        peak_mb: None,
        heldout: rate.map(|r| HeldOut {
            total: 100,
            base_passed: Some(10),
            base_rate: Some(0.1),
            adapter_passed: Some((r * 100.0).round() as u64),
            adapter_rate: Some(r),
        }),
        cells: Vec::new(),
    }
}
