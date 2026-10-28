//! Stub-backend tests of the run loop (`run.rs`).

use super::*;
use crate::commands::model::eval_diagram::score::attempts;
use crate::commands::model::eval_diagram::score::tests::{VD_BAD, VD_OK, attempt};

const TEMPLATE: &str = "[user]\nDraw this as a {LANG} diagram. {PROMPT}\nReply with only one fenced {FENCE} block.\n\n[repair]\nThe {LANG} {VALIDATOR} rejected it:\n{FEEDBACK}\nReply with only the corrected diagram in one fenced {FENCE} block.\n";
const VD_CARD: &str = "VD CARD";
const MM_REPLY: &str = "```mermaid\nflowchart TD\n  a[\"A\"] --> b[\"B\"]\n```";
const TASKS: [&str; 4] = ["g001", "g002", "s001", "g003"];

/// The synthetic fixture's manifest, tasks and reference outputs, plus a template and two cards.
fn dataset() -> tempfile::TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("eval_diagram_synthetic");
    let dir = tempfile::tempdir().unwrap();
    for f in [
        "manifest.v1.yaml",
        "tasks.v1.jsonl",
        "reference-outputs.jsonl",
    ] {
        std::fs::copy(src.join(f), dir.path().join(f)).unwrap();
    }
    std::fs::write(dir.path().join("prompt-template.txt"), TEMPLATE).unwrap();
    let cards = dir.path().join("cards");
    std::fs::create_dir(&cards).unwrap();
    std::fs::write(cards.join("voxdiagram.txt"), VD_CARD).unwrap();
    std::fs::write(cards.join("mermaid.txt"), "MM CARD").unwrap();
    dir
}

fn cfg(dir: &Path, name: &str, cap: f64) -> RunConfig {
    RunConfig {
        dataset_dir: dir.to_path_buf(),
        run_name: name.into(),
        max_spend_usd: cap,
    }
}

fn header() -> RunHeader {
    let Line::Run(mut h) = super::super::score::tests::header() else {
        unreachable!()
    };
    h.models = ["A", "B"]
        .map(|s| ModelSlot {
            slot: s.into(),
            requested_id: format!("stub{s}"),
            provider: "stub".into(),
            family: format!("org{s}"),
        })
        .to_vec();
    h
}

fn read(p: PathBuf) -> String {
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn texts(m: &[LlmChatMessage]) -> Vec<(&str, &str)> {
    m.iter()
        .map(|m| (m.role.as_str(), m.content.as_str()))
        .collect()
}

fn recorded(out: &RunOutcome) -> Vec<Line> {
    read_jsonl(&read(out.run_dir.join("attempts.jsonl"))).unwrap()
}

/// A failure after the request was sent: it may have been billed.
const RESET: &str = "HTTP request failed: connection reset";
/// A provider refusal that echoes part of the key in its body.
const REFUSED: &str =
    "LLM API returned error (401): {\"error\":\"Incorrect API key provided: sk-proj-SECRET1234\"}";

/// VoxDiagram: invalid at turn 0, valid after. Mermaid: g001 fails to parse at turns 0 and 1.
#[derive(Default)]
struct Stub {
    calls: Vec<(String, u64, Vec<LlmChatMessage>)>,
    cost: Option<f64>,
    /// Which 1-based calls fail in transport, and with what error text.
    fail: Option<(fn(usize) -> bool, &'static str)>,
    backoffs: Vec<u32>,
    batches: usize,
    fail_batch: Option<usize>,
}

impl Backend for Stub {
    async fn chat(
        &mut self,
        slot: &str,
        messages: Vec<LlmChatMessage>,
        seed: u64,
    ) -> Result<Reply, String> {
        let vd = messages[0].content == VD_CARD;
        let turn = messages.iter().filter(|m| m.role == "assistant").count();
        self.calls.push((slot.to_string(), seed, messages));
        if let Some((fails, error)) = self.fail
            && fails(self.calls.len())
        {
            return Err(error.into());
        }
        let content = match (vd, turn) {
            (true, 0) => VD_BAD,
            (true, _) => VD_OK,
            (false, _) => MM_REPLY,
        };
        Ok(Reply {
            content: content.into(),
            model: format!("stub-{slot}"),
            prompt_tokens: 10,
            completion_tokens: 5,
            cost_usd: self.cost,
            latency_ms: 1,
        })
    }

    async fn mermaid_batch(&mut self, inputs_jsonl: &str) -> Result<String> {
        self.batches += 1;
        if self.fail_batch == Some(self.batches - 1) {
            bail!("node exited 1");
        }
        let outs: Vec<OracleOutput> = read_jsonl::<OracleInput>(inputs_jsonl)?
            .into_iter()
            .map(|i| {
                let bad = i.id.starts_with("g001:")
                    && (i.id.ends_with(":mermaid:0") || i.id.ends_with(":mermaid:1"));
                OracleOutput {
                    ok: !bad,
                    error: bad.then(|| format!("bad {}", i.id)),
                    sets: serde_json::Value::Null,
                    matched: (!bad).then_some(true),
                    id: i.id,
                }
            })
            .collect();
        write_jsonl(&outs)
    }

    async fn backoff(&mut self, retry: u32) {
        self.backoffs.push(retry);
    }
}

fn stub(cost: f64) -> Stub {
    Stub {
        cost: Some(cost),
        ..Stub::default()
    }
}

#[tokio::test]
async fn stub_run_follows_the_round_order_and_the_scorer_accepts_it() {
    let dir = dataset();
    let mut b = stub(0.001);
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    assert_eq!(out.problems, Vec::<String>::new());
    assert!(out.complete && out.stopped.is_none(), "{out:?}");
    let lines = recorded(&out);
    assert!(matches!(lines[0], Line::Run(_)));
    let mut want = Vec::new();
    for s in ["A", "B"] {
        for t in TASKS {
            want.push(format!("{t}:{s}:voxdiagram:0"));
            want.push(format!("{t}:{s}:mermaid:0"));
        }
        for t in TASKS {
            want.push(format!("{t}:{s}:voxdiagram:1"));
            if t == "g001" {
                want.push(format!("{t}:{s}:mermaid:1"));
            }
        }
        want.push(format!("g001:{s}:mermaid:2"));
    }
    let got: Vec<String> = attempts(&lines).map(Attempt::oracle_id).collect();
    assert_eq!(got, want);
    assert_eq!((out.calls, b.calls.len()), (want.len(), want.len()));
    for a in attempts(&lines) {
        let seed = match a.turn {
            0 => sample_seed(42, &a.task_id, 0),
            k => repair_seed(42, &a.task_id, k, 0),
        };
        assert_eq!(a.seed, seed, "{}", a.oracle_id());
        assert_eq!(a.resolved_model, Some(format!("stub-{}", a.slot)));
    }
    // Slot A's g001 Mermaid turn 2 (call 14) carries the whole history.
    let (slot, _, msgs) = &b.calls[13];
    let roles: Vec<&str> = msgs.iter().map(|m| m.role.as_str()).collect();
    assert_eq!(slot, "A");
    assert_eq!(
        roles,
        ["system", "user", "assistant", "user", "assistant", "user"]
    );
    assert_eq!(
        (msgs[0].content.as_str(), msgs[2].content.as_str()),
        ("MM CARD", MM_REPLY)
    );
    assert!(
        msgs[3].content.contains("bad g001:A:mermaid:0"),
        "{}",
        msgs[3].content
    );
    assert!(
        msgs[5].content.contains("bad g001:A:mermaid:1"),
        "{}",
        msgs[5].content
    );

    let d = dir.path();
    let tasks: Vec<Task> = read_jsonl(&read(d.join("tasks.v1.jsonl"))).unwrap();
    let refs: Vec<OracleOutput> = read_jsonl(&read(d.join("reference-outputs.jsonl"))).unwrap();
    let outs: Vec<OracleOutput> =
        read_jsonl(&read(out.run_dir.join("oracle-outputs.jsonl"))).unwrap();
    check_recording(&tasks, &lines, &outs).unwrap();
    assert_eq!(
        oracle_inputs(&tasks, &refs, &lines).unwrap(),
        read(out.run_dir.join("oracle-inputs.jsonl"))
    );
    let rule = parse_manifest(&read(d.join("manifest.v1.yaml")))
        .unwrap()
        .decision_rule;
    let t = tally(&rule, &tasks, &lines, &outs).unwrap();
    assert_eq!(
        read(out.run_dir.join("tallies.json")),
        serde_json::to_string_pretty(&t).unwrap() + "\n"
    );
    assert_eq!(read(out.run_dir.join("tables.md")), render_tables(&t));
}

#[tokio::test]
async fn spend_cap_reserves_the_next_call_and_counts_prior_spend() {
    let dir = dataset();
    let mut b = stub(0.01);
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 0.055), header())
        .await
        .unwrap();
    // The sixth call would need 0.05 + 0.01 (the largest cost seen) > 0.055: it is never made.
    assert_eq!(b.calls.len(), 5);
    assert_eq!(out.stopped.as_deref(), Some("spend_cap"));
    assert!(!out.complete);
    let lines = recorded(&out);
    assert_eq!(attempts(&lines).count(), 5);
    let Some(Line::Stop(s)) = lines.last() else {
        panic!("no stop line")
    };
    assert_eq!(s.reason, "spend_cap");
    assert!(s.spent_usd <= 0.055, "{}", s.spent_usd);
    assert!((s.spent_usd - 0.05).abs() < 1e-9, "{}", s.spent_usd);
    let t: serde_json::Value =
        serde_json::from_str(&read(out.run_dir.join("tallies.json"))).unwrap();
    assert_eq!(t["stopped"], "spend_cap");
    assert_eq!(t["slots"]["A"]["complete"], false);

    // Earlier runs (0.05) and the probe ledger (0.01 + a null cost) count against a new run's cap.
    let probes = [Some(0.01), None].map(|c| ProbeRecord {
        time: "t".into(),
        candidate: "c".into(),
        ok: c.is_some(),
        error: None,
        resolved_model: None,
        cost_usd: c,
    });
    std::fs::write(dir.path().join(PROBE_LEDGER), write_jsonl(&probes).unwrap()).unwrap();
    let mut b = stub(0.01);
    let out = run_dataset(&mut b, &cfg(dir.path(), "r2", 0.075), header())
        .await
        .unwrap();
    assert_eq!(
        (b.calls.len(), out.stopped.as_deref()),
        (1, Some("spend_cap"))
    );
}

#[test]
fn prior_spend_sums_attempt_costs_and_the_probe_ledger() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(prior_spend(dir.path()).unwrap(), 0.0);
    let run = dir.path().join("runs").join("r1");
    std::fs::create_dir_all(&run).unwrap();
    std::fs::create_dir_all(dir.path().join("runs").join("empty")).unwrap();
    let mut lines = vec![super::super::score::tests::header()];
    for cost in [0.25, 0.5] {
        let mut a = attempt("t1", Arm::Mermaid, "A", 0, Some(MM_REPLY));
        a.cost_usd = cost;
        lines.push(Line::Attempt(a));
    }
    lines.push(Line::Stop(Stop {
        reason: "spend_cap".into(),
        spent_usd: 99.0,
    }));
    std::fs::write(run.join("attempts.jsonl"), write_jsonl(&lines).unwrap()).unwrap();
    assert_eq!(prior_spend(dir.path()).unwrap(), 0.75);

    let ledger = "{\"time\":\"t\",\"candidate\":\"c\",\"ok\":true,\"error\":null,\"resolved_model\":\"m\",\"cost_usd\":0.125}\n{\"time\":\"t\",\"candidate\":\"d\",\"ok\":false,\"error\":\"401\",\"resolved_model\":null,\"cost_usd\":null}\n";
    let alone = tempfile::tempdir().unwrap();
    std::fs::write(alone.path().join(PROBE_LEDGER), ledger).unwrap();
    assert_eq!(prior_spend(alone.path()).unwrap(), 0.125);
    std::fs::write(dir.path().join(PROBE_LEDGER), ledger).unwrap();
    assert_eq!(prior_spend(dir.path()).unwrap(), 0.875);

    // A negative cost would lower the spend: refused, in the ledger and in a recording.
    let bad = tempfile::tempdir().unwrap();
    std::fs::write(
        bad.path().join(PROBE_LEDGER),
        ledger.replace("0.125", "-0.5"),
    )
    .unwrap();
    let err = prior_spend(bad.path()).unwrap_err();
    assert!(format!("{err:#}").contains("-0.5"), "{err:#}");
    let Line::Attempt(a) = &mut lines[1] else {
        unreachable!()
    };
    a.cost_usd = -0.25;
    std::fs::write(run.join("attempts.jsonl"), write_jsonl(&lines).unwrap()).unwrap();
    let err = prior_spend(dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("-0.25"), "{err:#}");
}

#[tokio::test]
async fn a_reply_without_a_cost_stops_the_run() {
    let dir = dataset();
    let mut b = Stub::default();
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    assert_eq!(
        (b.calls.len(), out.stopped.as_deref()),
        (1, Some("cost_unknown"))
    );
    let lines = recorded(&out);
    assert_eq!(lines.len(), 3);
    assert!(matches!(&lines[2], Line::Stop(s) if s.reason == "cost_unknown"));
}

#[tokio::test]
async fn transport_errors_back_off_retry_the_same_slot_three_times_then_are_recorded() {
    let dir = dataset();
    let mut b = Stub {
        fail: Some((|n| (2..=5).contains(&n), RESET)),
        ..stub(0.001)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    let lines = recorded(&out);
    let first: Vec<&Attempt> = attempts(&lines).take(3).collect();
    assert_eq!(
        first[1].transport_error.as_deref(),
        Some("HTTP request failed")
    );
    assert_eq!(
        (first[1].retries, first[1].raw_response.is_none()),
        (3, true)
    );
    // Each of the 4 failed calls may have been billed: each is charged the largest cost seen.
    assert!(
        (first[1].cost_usd - 0.004).abs() < 1e-12,
        "{}",
        first[1].cost_usd
    );
    for a in [first[0], first[2]] {
        assert_eq!((a.retries, a.transport_error.is_none()), (0, true));
    }
    assert!(
        b.calls[1..5]
            .iter()
            .all(|c| c.0 == "A" && c.1 == b.calls[1].1 && texts(&c.2) == texts(&b.calls[1].2))
    );
    assert_eq!(b.backoffs, [1, 2, 3]);
    assert!(out.problems.is_empty(), "{:?}", out.problems);
    assert!((prior_spend(dir.path()).unwrap() - out.spent_usd).abs() < 1e-12);

    let dir = dataset();
    let mut b = Stub {
        fail: Some((|n| (2..=3).contains(&n), RESET)),
        ..stub(0.001)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    let lines = recorded(&out);
    let a = attempts(&lines).nth(1).unwrap();
    assert_eq!((a.retries, a.raw_response.is_some()), (2, true));
    assert_eq!(a.transport_error, None);
    assert!((a.cost_usd - 0.003).abs() < 1e-12, "{}", a.cost_usd);
    assert_eq!(b.backoffs, [1, 2]);
}

#[tokio::test]
async fn failed_calls_are_charged_so_the_cap_still_stops_the_run() {
    // A provider that bills and then fails every call after the first.
    let dir = dataset();
    let mut b = Stub {
        fail: Some((|n| n >= 2, RESET)),
        ..stub(0.01)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 0.055), header())
        .await
        .unwrap();
    assert_eq!(out.stopped.as_deref(), Some("spend_cap"));
    assert_eq!(b.calls.len(), 5);
    assert!(out.spent_usd <= 0.055, "{}", out.spent_usd);
    assert!((prior_spend(dir.path()).unwrap() - out.spent_usd).abs() < 1e-12);

    // The cap stops it mid-retry: the charged attempt is still recorded, so later runs count it.
    let dir = dataset();
    let mut b = Stub {
        fail: Some((|n| n >= 2, RESET)),
        ..stub(0.01)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 0.035), header())
        .await
        .unwrap();
    assert_eq!(
        (b.calls.len(), out.stopped.as_deref()),
        (3, Some("spend_cap"))
    );
    let lines = recorded(&out);
    let lost = attempts(&lines).nth(1).expect("the charged attempt");
    assert_eq!(
        (lost.retries, lost.transport_error.as_deref()),
        (2, Some("HTTP request failed"))
    );
    assert!((prior_spend(dir.path()).unwrap() - 0.03).abs() < 1e-12);
}

#[tokio::test]
async fn a_billable_failure_before_any_priced_call_stops_the_run() {
    let dir = dataset();
    let mut b = Stub {
        fail: Some((|_| true, RESET)),
        ..stub(0.01)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    assert_eq!(
        (b.calls.len(), out.stopped.as_deref()),
        (1, Some("cost_unknown"))
    );
    let lines = recorded(&out);
    assert_eq!(attempts(&lines).count(), 1);
}

#[tokio::test]
async fn a_provider_refusal_is_retried_uncharged_and_its_body_never_recorded() {
    let dir = dataset();
    let mut b = Stub {
        fail: Some((|n| n <= 4, REFUSED)),
        ..stub(0.001)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    let lines = recorded(&out);
    let a = attempts(&lines).next().unwrap();
    assert_eq!(
        (a.retries, a.transport_error.as_deref(), a.cost_usd),
        (3, Some("LLM API returned error (401)"), 0.0)
    );
    let text = read(out.run_dir.join("attempts.jsonl"));
    assert!(
        !text.contains("sk-proj") && !text.contains("SECRET"),
        "{text}"
    );
    assert!(out.problems.is_empty(), "{:?}", out.problems);
}

#[test]
fn only_the_error_class_and_status_are_kept() {
    for (raw, want) in [
        (REFUSED, "LLM API returned error (401)"),
        (
            "LLM API returned error (503): upstream sk-or-v1-abc",
            "LLM API returned error (503)",
        ),
        (
            "Failed to parse response JSON: error decoding response body",
            "Failed to parse response JSON",
        ),
        ("RATE_LIMITED: retry after 3 s", "RATE_LIMITED"),
        (
            "CONTEXT_LENGTH_EXCEEDED: LLM API returned error (400): too long",
            "CONTEXT_LENGTH_EXCEEDED",
        ),
        ("activity cancelled", "activity cancelled"),
        ("Unauthorized sk-proj-SECRET1234", "unclassified error"),
        ("LLM API returned error (4x1)", "unclassified error"),
    ] {
        assert_eq!(redact_error(raw), want, "{raw}");
    }
}

#[tokio::test]
async fn an_unsafe_run_name_is_refused_before_any_call() {
    let dir = dataset();
    let mut b = stub(0.001);
    for name in [
        "", ".", "..", "../x", "a/b", "a\\b", ".hidden", "a b", "r1\n",
    ] {
        let err = run_dataset(&mut b, &cfg(dir.path(), name, 25.0), header())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("run name"), "{name:?}: {err}");
    }
    assert!(b.calls.is_empty() && !dir.path().join("runs").exists());
    check_run_name("2026-10-28_r2.v1").unwrap();
}

#[tokio::test]
async fn a_failed_mermaid_batch_stops_the_run_and_still_derives_the_files() {
    let dir = dataset();
    let mut b = Stub {
        fail_batch: Some(0),
        ..stub(0.001)
    };
    let out = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap();
    assert_eq!(out.stopped.as_deref(), Some("oracle_failure"));
    assert!(!out.complete);
    assert!(
        out.problems.iter().any(|p| p.contains("node exited 1")),
        "{:?}",
        out.problems
    );
    let lines = recorded(&out);
    assert_eq!(attempts(&lines).count(), 8);
    assert!(matches!(lines.last(), Some(Line::Stop(s)) if s.reason == "oracle_failure"));
    let t: serde_json::Value =
        serde_json::from_str(&read(out.run_dir.join("tallies.json"))).unwrap();
    assert_eq!(t["stopped"], "oracle_failure");
    assert!(read(out.run_dir.join("tables.md")).contains("Run stopped early: oracle_failure."));
}

#[tokio::test]
async fn an_existing_run_directory_or_a_bad_cap_is_refused_before_any_call() {
    let dir = dataset();
    std::fs::create_dir_all(dir.path().join("runs").join("r1")).unwrap();
    let mut b = stub(0.001);
    let err = run_dataset(&mut b, &cfg(dir.path(), "r1", 25.0), header())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err}");
    for cap in [0.0, -1.0, 25.01, f64::NAN] {
        assert!(check_cap(cap).is_err(), "{cap}");
        let err = run_dataset(&mut b, &cfg(dir.path(), "r9", cap), header())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("spend cap"), "{err}");
    }
    check_cap(25.0).unwrap();
    assert!(b.calls.is_empty() && !dir.path().join("runs").join("r9").exists());
}
