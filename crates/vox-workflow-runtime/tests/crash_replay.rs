#![allow(missing_docs)]
//! Phase 3.2: crash + resume proof.
//!
//! Seeds a completed activity into the journal via `VoxDbTracker`, then runs
//! `interpret_workflow_durable` against the same `run_id`. Verifies the
//! seeded `charge_card` activity is REPLAYED from the journal (interpreter
//! emits `ActivityReplayed` and skips re-execution) while `send_receipt`
//! still executes fresh, consuming the *replayed* value.
//!
//! This is the durable runtime's core promise: crash mid-workflow, restart
//! on another node, and completed activities don't re-execute — the journal
//! is the source of truth.
//!
//! Refs ADR-019 v1 journal contract; ADR-021 generated-vs-interpreted parity.

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::workflow::interpret_workflow_durable;
use vox_workflow_runtime::{VoxDbTracker, WorkflowTracker};

const GOLDEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/golden/durable_workflow_real.vox"
));

/// Replicates `vox_workflow_runtime::workflow::run::derive_activity_id`, which is
/// `pub(crate)` and therefore not reachable from an integration test. The
/// algorithm is locked in by `derive_activity_id_is_deterministic` + the 32-hex
/// shape test inside that module; this copy must stay in sync. If those unit
/// tests change, update here too.
fn derive_activity_id(
    workflow_name: &str,
    enclosing_fn: &str,
    ordinal: u32,
    iteration: &[u32],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(workflow_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(enclosing_fn.as_bytes());
    hasher.update(b"\0");
    hasher.update(ordinal.to_le_bytes().as_ref());
    for index in iteration {
        hasher.update(b"\0");
        hasher.update(index.to_le_bytes().as_ref());
    }
    let hash = hasher.finalize();
    let bytes = &hash.as_bytes()[..16];
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test]
async fn workflow_resumes_seeded_activity_from_journal() {
    let module = parse(lex(GOLDEN)).expect("durable_workflow_real.vox parses");
    let hir = lower_module(&module);

    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let run_id = "crash-replay-test-1".to_string();
    let mut tracker = VoxDbTracker::new(db.clone(), run_id.clone());

    // Seed: pretend `charge_card` (the only call to it in the `checkout`
    // workflow) already completed in a prior crashed run. The interpreter
    // uses the same BLAKE3-derived activity_id, so a record under that id is
    // what `is_activity_completed` + `load_activity_result` will return on
    // resume.
    //
    // The ordinal is computed by the real `stamp_call_ordinals` (not
    // hardcoded to `0`): ordinals are namespaced per callee so two different
    // activities never collide (see `vox-compiler`'s
    // `eval::activity_hook::stamp_call_ordinals` docs), so `charge_card`'s
    // ordinal is not a small sequential position.
    //
    // The seeded value is a real journal-JSON `Result::Ok` payload (the shape
    // `VoxValue::to_journal_json` actually produces for `charge_card`'s
    // `Result[str]` return type) — deliberately DIFFERENT from what a fresh
    // `charge_card(500)` call would produce (`"tx_500"`), so a passing test
    // proves replay happened rather than coincidentally matching a fresh
    // execution.
    let ordinals = vox_compiler::eval::activity_hook::stamp_call_ordinals(&hir);
    let checkout_fn = hir
        .functions
        .iter()
        .find(|f| f.name == "checkout")
        .expect("checkout function in HIR");
    let charge_card_span = match checkout_fn.body.first() {
        Some(vox_compiler::hir::nodes::HirStmt::Let {
            value: vox_compiler::hir::nodes::HirExpr::Try(try_expr),
            ..
        }) => match try_expr.target.as_ref() {
            vox_compiler::hir::nodes::HirExpr::Call(_, _, _, span) => *span,
            other => panic!("expected `charge_card(amount)` call, got {other:?}"),
        },
        other => panic!("expected `let tx = charge_card(amount)?`, got {other:?}"),
    };
    let charge_card_ordinal = *ordinals
        .get(&charge_card_span)
        .expect("charge_card ordinal must be stamped");
    let first_charge_id = derive_activity_id("checkout", "checkout", charge_card_ordinal, &[0]);
    let seeded_result = json!({"__vox": "Ok", "value": "tx_SEEDED_500"});

    tracker
        .on_activity_completed("checkout", "charge_card", &first_charge_id, &seeded_result)
        .await
        .expect("seed charge_card completion into journal");

    // Now run the workflow on the same run_id, with the same call argument a
    // real caller would have used (`checkout(amount: int)`). The seeded
    // activity should surface as ActivityReplayed; send_receipt should still
    // execute fresh, on top of the replayed (not freshly-computed) value.
    let journal = interpret_workflow_durable(&hir, "checkout", vec![json!(500)], &mut tracker)
        .await
        .expect("checkout resumes against seeded journal");

    let event_names: Vec<&str> = journal
        .iter()
        .filter_map(|e| e.get("event").and_then(|v| v.as_str()))
        .collect();

    // ── Assertion 1: seeded charge_card replays from journal ────────────────
    let replayed_for_charge: Vec<&serde_json::Value> = journal
        .iter()
        .filter(|e| {
            e.get("event").and_then(|v| v.as_str()) == Some("ActivityReplayed")
                && e.get("activity").and_then(|v| v.as_str()) == Some("charge_card")
        })
        .collect();
    assert_eq!(
        replayed_for_charge.len(),
        1,
        "expected exactly one ActivityReplayed for seeded charge_card; full events={event_names:?}"
    );
    let replayed = replayed_for_charge[0];
    assert_eq!(
        replayed.get("activity_id").and_then(|v| v.as_str()),
        Some(first_charge_id.as_str()),
        "ActivityReplayed activity_id must match seeded id; entry={replayed}"
    );
    assert_eq!(
        replayed.get("replay_source").and_then(|v| v.as_str()),
        Some("workflow_activity_log"),
        "replay_source must be workflow_activity_log on journal replay; entry={replayed}"
    );
    assert_eq!(
        replayed.get("result"),
        Some(&seeded_result),
        "the seeded result payload must be embedded back into the journal verbatim; entry={replayed}"
    );

    // ── Assertion 2: send_receipt executes fresh, on the REPLAYED value ────
    // If charge_card had (incorrectly) re-executed instead of replaying, this
    // would see "tx_500" instead of the seeded "tx_SEEDED_500" — this
    // assertion is what actually proves the replayed value flowed through the
    // workflow body, not just that a replay *event* was logged.
    let completed_for_receipt: Vec<&serde_json::Value> = journal
        .iter()
        .filter(|e| {
            e.get("event").and_then(|v| v.as_str()) == Some("ActivityCompleted")
                && e.get("activity").and_then(|v| v.as_str()) == Some("send_receipt")
        })
        .collect();
    assert_eq!(
        completed_for_receipt.len(),
        1,
        "expected exactly one fresh ActivityCompleted for un-seeded send_receipt; full events={event_names:?}"
    );
    assert_eq!(
        completed_for_receipt[0].get("result"),
        Some(&json!({"__vox": "Ok", "value": "emailed:tx_SEEDED_500"})),
        "send_receipt must consume the REPLAYED charge_card value, not a freshly computed one; entry={:?}",
        completed_for_receipt[0]
    );

    // send_receipt should NOT have a Replayed entry.
    let replayed_for_receipt = journal.iter().any(|e| {
        e.get("event").and_then(|v| v.as_str()) == Some("ActivityReplayed")
            && e.get("activity").and_then(|v| v.as_str()) == Some("send_receipt")
    });
    assert!(
        !replayed_for_receipt,
        "send_receipt was not seeded, so it must not appear as ActivityReplayed; full events={event_names:?}"
    );

    // ── Assertion 3: workflow terminates cleanly ────────────────────────────
    assert_eq!(
        event_names.first().copied(),
        Some("WorkflowStarted"),
        "first event must be WorkflowStarted; got {event_names:?}"
    );
    assert_eq!(
        event_names.last().copied(),
        Some("WorkflowCompleted"),
        "last event must be WorkflowCompleted; got {event_names:?}"
    );
}

#[tokio::test]
async fn workflow_runs_fresh_when_journal_is_empty() {
    // Control: same workflow, no seed. Neither activity should appear as
    // Replayed — both must run fresh. Anchors the crash-replay assertion
    // above so a future change that always emits ActivityReplayed cannot
    // silently pass the seeded test.
    let module = parse(lex(GOLDEN)).expect("durable_workflow_real.vox parses");
    let hir = lower_module(&module);

    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut tracker = VoxDbTracker::new(db, "crash-replay-control-1");

    let journal = interpret_workflow_durable(&hir, "checkout", vec![json!(500)], &mut tracker)
        .await
        .expect("checkout runs fresh against empty journal");

    let event_names: Vec<&str> = journal
        .iter()
        .filter_map(|e| e.get("event").and_then(|v| v.as_str()))
        .collect();

    let any_replayed = event_names.contains(&"ActivityReplayed");
    assert!(
        !any_replayed,
        "fresh run must not emit ActivityReplayed; got {event_names:?}"
    );

    // Both activities must have actually run (ActivityCompleted), with the
    // real, freshly-computed values — anchoring against the seeded test's
    // deliberately-different "tx_SEEDED_500" marker.
    let charge_completed = journal.iter().find(|e| {
        e.get("event").and_then(|v| v.as_str()) == Some("ActivityCompleted")
            && e.get("activity").and_then(|v| v.as_str()) == Some("charge_card")
    });
    assert_eq!(
        charge_completed.and_then(|e| e.get("result")),
        Some(&json!({"__vox": "Ok", "value": "tx_500"})),
        "fresh charge_card(500) must compute tx_500; journal={journal:#?}"
    );

    assert_eq!(
        event_names.last().copied(),
        Some("WorkflowCompleted"),
        "fresh run must terminate with WorkflowCompleted; got {event_names:?}"
    );
}
