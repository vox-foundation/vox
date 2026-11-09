use super::*;
use crate::mens::hub_card::RunRecords;
use crate::mens::hub_card::tests::support::{CURRENT_CKPT, evidence_run, vox_block_ok};
use serde_json::json;

fn sample(pass: bool, preview: &str) -> Value {
    json!({
        "pass": pass, "completion_preview": preview,
        "checks": {"non_empty": true, "parse_ok": true, "typecheck_ok": true,
                   "diag_errors": 0, "anti_stub_pass": true, "pass_compile": true,
                   "pass_ast": true, "pass_exec": null}
    })
}

/// A report from `(id, description, sample)` rows.
fn report(rows: &[(&str, &str, Value)]) -> Value {
    json!({"results": rows.iter().map(|(id, d, s)| json!({
        "id": id, "description": d, "samples": [s]
    })).collect::<Vec<_>>()})
}

fn pair(
    id: &str,
    len: usize,
    base: bool,
    adapter: bool,
) -> ((String, String, Value), (String, String, Value)) {
    let d = "x".repeat(len);
    (
        (id.into(), d.clone(), sample(base, "")),
        (id.into(), d, sample(adapter, "")),
    )
}

/// Build base and adapter reports from `(id, description length, base pass, adapter pass)`.
fn reports(rows: &[(&str, usize, bool, bool)]) -> (Value, Value) {
    let (b, a): (Vec<_>, Vec<_>) = rows
        .iter()
        .map(|(id, len, b, a)| pair(id, *len, *b, *a))
        .unzip();
    let view = |v: &[(String, String, Value)]| {
        report(
            &v.iter()
                .map(|(i, d, s)| (i.as_str(), d.as_str(), s.clone()))
                .collect::<Vec<_>>(),
        )
    };
    (view(&b), view(&a))
}

fn ids(v: &[Example]) -> Vec<&str> {
    v.iter().map(|e| e.id.as_str()).collect()
}

fn fixture() -> RunRecords {
    RunRecords::load(
        evidence_run(CURRENT_CKPT).path(),
        "4B",
        "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c",
    )
}

fn results(v: &Value) -> &Vec<Value> {
    v["results"].as_array().unwrap()
}

#[test]
fn real_sample_joins_uniquely() {
    let rec = fixture();
    for (report, texts) in [
        (
            rec.heldout_base.as_ref().unwrap(),
            rec.heldout_base_ckpt.as_ref().unwrap(),
        ),
        (
            rec.heldout_adapter.as_ref().unwrap(),
            rec.heldout_adapter_ckpt.as_ref().unwrap(),
        ),
    ] {
        assert!(!results(report).is_empty());
        for r in results(report) {
            let preview = r["samples"][0]["completion_preview"].as_str().unwrap();
            assert!(
                full_text(preview, texts).is_some(),
                "{} did not join uniquely",
                r["id"]
            );
        }
    }
}

#[test]
fn ambiguous_join_renders_no_recorded_output() {
    let texts = vec![
        "fn a() {}\n// one".to_string(),
        "fn a() {}\n// two".to_string(),
    ];
    // A 240-char preview that both texts share only when it is short; use the shared prefix as the whole preview.
    let both = vec!["fn a() {}".to_string(), "fn a() {}".to_string()];
    assert_eq!(full_text("fn a() {}", &both), None);
    assert_eq!(
        full_text("fn a() {}\n// one", &texts),
        Some("fn a() {}\n// one")
    );

    let base = report(&[("i", "d", sample(false, "bad"))]);
    let adapter = report(&[("i", "d", sample(true, "fn a() {}"))]);
    let ex = pick_examples(&base, &adapter, None, Some(&both)).unwrap();
    assert_eq!(ex.wins[0].adapter.code, None);
    let ex = pick_examples(&base, &adapter, None, Some(&["fn a() {}".to_string()])).unwrap();
    assert_eq!(ex.wins[0].adapter.code.as_deref(), Some("fn a() {}"));
}

#[test]
fn missing_checkpoint_renders_no_recorded_output() {
    let base = report(&[("i", "d", sample(false, "bad"))]);
    let adapter = report(&[("i", "d", sample(true, "fn a() {}"))]);
    let ex = pick_examples(&base, &adapter, None, None).unwrap();
    let w = &ex.wins[0];
    assert_eq!(
        (w.base.code.as_deref(), w.adapter.code.as_deref()),
        (None, None)
    );
    // Verdicts still come from the stored report fields.
    assert_eq!(w.base.verdict, Verdict::Rejected("pass".into()));
    assert_eq!(w.adapter.verdict, Verdict::Passes);
}

#[test]
fn shown_code_is_the_scored_code() {
    assert_eq!(shown_code("```vox\nfn a() {}\n```"), "fn a() {}");
    assert_eq!(
        shown_code("<think>x</think>\n```\nfn b() {}\n```\nthanks"),
        "fn b() {}"
    );
    assert_eq!(shown_code("```vox\nfn c() {}"), "fn c() {}");
    assert_eq!(shown_code("<think>never closed"), "");
    assert_eq!(shown_code("  fn d() {}  "), "fn d() {}");
}

#[test]
fn verdict_from_stored_checks() {
    let with = |pass: bool, checks: Value| json!({"pass": pass, "checks": checks});
    let ok = json!({"non_empty": true, "parse_ok": true, "typecheck_ok": true, "diag_errors": 0,
                    "anti_stub_pass": true, "pass_compile": true, "pass_ast": true, "pass_exec": null});
    let patch = |k: &str, v: Value| {
        let mut c = ok.clone();
        c[k] = v;
        c
    };
    assert_eq!(verdict_of(&with(true, ok.clone())), Verdict::Passes);
    assert_eq!(
        verdict_of(&with(false, patch("non_empty", json!(false)))),
        Verdict::Empty
    );
    let mut parse = patch("parse_ok", json!(false));
    parse["parse_error"] = json!("Parsing failed with 3 error(s)");
    assert_eq!(
        verdict_of(&with(false, parse)),
        Verdict::ParseErrors(Some(3))
    );
    let mut ty = patch("typecheck_ok", json!(false));
    ty["diag_errors"] = json!(2);
    assert_eq!(verdict_of(&with(false, ty)), Verdict::TypeErrors(Some(2)));
    assert_eq!(
        verdict_of(&with(false, patch("anti_stub_pass", json!(false)))),
        Verdict::Stub
    );
    assert_eq!(
        verdict_of(&with(false, patch("pass_exec", json!(false)))),
        Verdict::Rejected("pass_exec".into())
    );
    // No sample or no checks: stated, not guessed.
    assert_eq!(
        verdict_of(&Value::Null),
        Verdict::Rejected("no sample".into())
    );
}

#[test]
fn verdict_phrases_are_the_docs_list() {
    let got: Vec<&str> = [
        Verdict::Passes,
        Verdict::Empty,
        Verdict::ParseErrors(Some(1)),
        Verdict::TypeErrors(None),
        Verdict::Stub,
        Verdict::Rejected("pass_exec".into()),
    ]
    .iter()
    .map(Verdict::phrase)
    .collect();
    assert_eq!(
        got,
        [
            "passes",
            "no code",
            "does not parse",
            "type errors",
            "stub",
            "rejected by"
        ]
    );
}

#[test]
fn wins_are_the_two_shortest_by_length_then_id() {
    let (b, a) = reports(&[
        ("d", 30, false, true),
        ("b", 12, false, true),
        ("a", 12, false, true),
        ("e", 50, false, true),
    ]);
    let ex = pick_examples(&b, &a, None, None).unwrap();
    assert_eq!(ids(&ex.wins), ["a", "b"]);
    assert_eq!(ex.fixes, 4);
}

#[test]
fn loss_example_present_or_stated_none() {
    let (b, a) = reports(&[
        ("long", 40, true, false),
        ("short", 9, false, false),
        ("mid", 20, false, false),
        ("ok", 5, true, true),
    ]);
    let ex = pick_examples(&b, &a, None, None).unwrap();
    assert_eq!(ex.loss.as_ref().map(|e| e.id.as_str()), Some("short"));
    assert_eq!((ex.fixes, ex.breaks, ex.total), (0, 1, 4));

    let (b, a) = reports(&[("x", 9, false, true), ("y", 3, true, true)]);
    assert_eq!(pick_examples(&b, &a, None, None).unwrap().loss, None);
}

#[test]
fn totals_are_counted_from_data() {
    let rec = fixture();
    let (b, a) = (rec.heldout_base.unwrap(), rec.heldout_adapter.unwrap());
    let pass = |r: &Value| r["samples"][0]["pass"].as_bool() == Some(true);
    let (mut total, mut fixes, mut breaks) = (0, 0, 0);
    for ar in results(&a) {
        if let Some(br) = results(&b).iter().find(|r| r["id"] == ar["id"]) {
            total += 1;
            fixes += usize::from(pass(ar) && !pass(br));
            breaks += usize::from(pass(br) && !pass(ar));
        }
    }
    let ex = pick_examples(&b, &a, None, None).unwrap();
    assert_eq!(
        (ex.total, ex.fixes, ex.breaks, ex.unmatched_ids),
        (total, fixes, breaks, 0)
    );
    assert!(
        fixes > 0 && breaks > 0,
        "the sample must exercise both counts"
    );
}

#[test]
fn ids_in_one_report_only_are_counted_unmatched() {
    let (b, mut a) = reports(&[("both", 5, false, true)]);
    a["results"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "only_adapter", "description": "d", "samples": [sample(true, "")]}));
    let ex = pick_examples(&b, &a, None, None).unwrap();
    assert_eq!((ex.total, ex.fixes, ex.unmatched_ids), (1, 1, 1));
    assert_eq!(ids(&ex.wins), ["both"]);
    assert_eq!(pick_examples(&json!({}), &a, None, None), None);
}

#[test]
fn fixture_adapter_wins_pass_vox_admission() {
    let rec = fixture();
    let ex = pick_examples(
        rec.heldout_base.as_ref().unwrap(),
        rec.heldout_adapter.as_ref().unwrap(),
        rec.heldout_base_ckpt.as_deref(),
        rec.heldout_adapter_ckpt.as_deref(),
    )
    .unwrap();
    assert_eq!(ex.wins.len(), 2);
    for w in &ex.wins {
        let code = w
            .adapter
            .code
            .as_deref()
            .unwrap_or_else(|| panic!("{} has no output", w.id));
        vox_block_ok(code).unwrap_or_else(|e| panic!("{}: {e}", w.id));
    }
}

#[test]
fn fence_for_outlasts_backtick_runs() {
    assert_eq!(fence_for("no backticks"), "```");
    assert_eq!(fence_for("a `b` c ``d``"), "```");
    assert_eq!(fence_for("x ````` y"), "``````");
    assert_eq!(fence_for("```vox\nfn a() {}\n```"), "````");
    assert_eq!(fence_for(""), "```");
}

#[test]
fn use_it_pick_skips_shown_wins() {
    // a and b are the wins; c passes on both sides and is the shortest non-win adapter pass.
    let (b, a) = reports(&[
        ("a", 5, false, true),
        ("b", 6, false, true),
        ("c", 7, true, true),
        ("d", 8, false, true),
        ("e", 3, true, false),
    ]);
    let ex = pick_examples(&b, &a, None, None).unwrap();
    assert_eq!(ids(&ex.wins), ["a", "b"]);
    assert_eq!(ex.use_it.as_ref().map(|e| e.id.as_str()), Some("c"));
    // Only wins pass: nothing is left to show.
    let (b, a) = reports(&[("a", 5, false, true), ("e", 3, true, false)]);
    assert_eq!(pick_examples(&b, &a, None, None).unwrap().use_it, None);
}
