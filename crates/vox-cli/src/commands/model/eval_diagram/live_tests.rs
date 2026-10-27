//! Tests of the pure live-run checks and the probe ledger (`live.rs`); no model is ever called.

use std::cell::Cell;

use super::*;
use crate::commands::model::eval_diagram::read_jsonl;

#[test]
fn vendor_org_is_the_part_before_the_slash() {
    let id = format!("{}/{}", "acme", "coder-9b");
    assert_eq!(vendor_org(&id), "acme");
    assert_eq!(vendor_org("bare"), "bare");
}

#[test]
fn an_environment_override_is_refused() {
    let err = refuse_override(&SelectionReason::EnvOverride {
        env_var: "VOX_MODEL_FORCE",
    })
    .unwrap_err();
    assert!(err.to_string().starts_with(HUMAN), "{err}");
    refuse_override(&SelectionReason::Scored).unwrap();
}

fn reply(cost: Option<f64>) -> Reply {
    Reply {
        content: "OK".into(),
        model: "served".into(),
        prompt_tokens: 3,
        completion_tokens: 1,
        cost_usd: cost,
        latency_ms: 1,
    }
}

fn ledger(dir: &Path) -> Vec<ProbeRecord> {
    read_jsonl(&std::fs::read_to_string(dir.join(PROBE_LEDGER)).unwrap()).unwrap()
}

#[tokio::test]
async fn probes_are_ledgered_one_by_one_and_the_first_priced_reply_wins() {
    let dir = tempfile::tempdir().unwrap();
    let ids: Vec<String> = ["c1", "c2", "c3", "c4", "c5"].map(String::from).to_vec();
    let calls = Cell::new(0);
    let (hit, recs) = probe_candidates(dir.path(), 25.0, &ids, |id| {
        calls.set(calls.get() + 1);
        let n = calls.get();
        async move {
            match n {
                1 | 3 => Err(format!("{id}: 401")),
                2 => Ok(reply(None)),
                _ => Ok(reply(Some(0.5))),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!((hit.as_deref(), calls.get()), (Some("c4"), 4));
    assert_eq!(ledger(dir.path()), recs);
    assert_eq!(
        recs.iter().map(|r| r.ok).collect::<Vec<_>>(),
        [false, false, false, true]
    );
    assert_eq!(recs[0].error.as_deref(), Some("c1: 401"));
    assert_eq!(recs[1].error.as_deref(), Some("reply carried no cost"));
    assert_eq!(recs[3].resolved_model.as_deref(), Some("served"));
    assert_eq!(prior_spend(dir.path()).unwrap(), 0.5);

    // Once the ledger reaches the cap, nothing more is probed.
    let calls = Cell::new(0);
    let err = probe_candidates(dir.path(), 0.5, &ids, |_| {
        calls.set(calls.get() + 1);
        async { Ok(reply(Some(0.1))) }
    })
    .await
    .unwrap_err();
    assert!(err.to_string().starts_with(HUMAN), "{err}");
    assert_eq!((calls.get(), ledger(dir.path()).len()), (0, 4));

    // Three failures: three ledger lines, no slot.
    let dir = tempfile::tempdir().unwrap();
    let (hit, recs) = probe_candidates(dir.path(), 25.0, &ids[..3], |id| async move {
        Err(format!("{id}: down"))
    })
    .await
    .unwrap();
    assert_eq!((hit, recs.len(), ledger(dir.path()).len()), (None, 3, 3));
}
