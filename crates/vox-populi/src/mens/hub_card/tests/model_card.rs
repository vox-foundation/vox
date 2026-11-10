//! v2 model card: goldens, section order, fact-once rules and the serving-path gate.

mod examples;
mod use_it;

use serde_json::Value;

use super::support::{
    Case, FAILED, MISSING_OUTPUT, NEEDS_RE_GATE, NO_EXAMPLES, NOT_EVALUATED, NOT_TRAINED, PASSED,
    STALE_SERVING, case_run_dir, cx_for, fixture_root, passing_collateral_report, render_case,
};
use crate::mens::hub_card::RunRecords;
use crate::mens::hub_card::card_facts::card_facts;
use crate::mens::hub_card::config::hub_cards;
use crate::mens::hub_card::facts::{AdapterEvidence, Blocker, GateState, load_requirement};
use vox_eval::reasoning_bench::SLICE_NAMES;

/// Every golden card: name, case.
fn cases() -> Vec<(&'static str, Case)> {
    vec![
        ("passed", PASSED),
        ("needs_re_gate", NEEDS_RE_GATE),
        ("stale_serving", STALE_SERVING),
        ("failed", FAILED),
        ("not_evaluated", NOT_EVALUATED),
        ("not_trained", NOT_TRAINED),
        ("no_examples", NO_EXAMPLES),
        ("missing_output", MISSING_OUTPUT),
    ]
}

/// The golden cases whose size has trained weights: they carry a spoke, a gate and every section.
fn trained_cases() -> Vec<(&'static str, Case)> {
    cases()
        .into_iter()
        .filter(|(name, _)| *name != "not_trained")
        .collect()
}

fn golden_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/hub_cards")
}

#[test]
fn goldens() {
    let dir = golden_dir();
    let update = std::env::var_os("VOX_UPDATE_GOLDEN").is_some();
    for (name, case) in cases() {
        let got = render_case(&case);
        let path = dir.join(format!("{name}.md"));
        if update {
            std::fs::write(&path, &got).unwrap();
            continue;
        }
        assert_eq!(got, render_case(&case), "{name}: two renders differ");
        assert!(
            !got.contains("/Users/") && !got.contains("/home/"),
            "{name}: host path leaked"
        );
        assert!(
            got.ends_with('\n') && !got.ends_with("\n\n"),
            "{name}: must end with exactly one newline"
        );
        let want = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e} (run with VOX_UPDATE_GOLDEN=1)", path.display()));
        assert_eq!(
            got, want,
            "golden {name} drifted; review and run with VOX_UPDATE_GOLDEN=1"
        );
    }
}

/// The card after its frontmatter block.
fn body(card: &str) -> &str {
    let rest = card.strip_prefix("---\n").expect("frontmatter opens");
    let end = rest.find("\n---\n").expect("frontmatter closes");
    &rest[end + "\n---\n".len()..]
}

/// Non-code lines of the body, fenced blocks dropped.
fn prose_lines(card: &str) -> Vec<&str> {
    let mut fenced = false;
    body(card)
        .lines()
        .filter(|l| {
            if l.starts_with("```") {
                fenced = !fenced;
                return false;
            }
            !fenced
        })
        .collect()
}

/// The first paragraph after the H1.
fn status_line(card: &str) -> String {
    let lines = prose_lines(card);
    let h1 = lines
        .iter()
        .position(|l| l.starts_with("# "))
        .expect("title");
    lines[h1 + 1..]
        .iter()
        .skip_while(|l| l.is_empty())
        .take_while(|l| !l.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The body after the title and the status line paragraph.
fn body_after_status(card: &str) -> String {
    let lines = prose_lines(card);
    let h1 = lines.iter().position(|l| l.starts_with("# ")).unwrap();
    lines[h1 + 1..]
        .iter()
        .skip_while(|l| l.is_empty())
        .skip_while(|l| !l.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whole-word, case-insensitive count of `needle` in `hay`.
fn count_word(hay: &str, needle: &str) -> usize {
    let (hay, needle) = (hay.to_lowercase(), needle.to_lowercase());
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '-');
    hay.match_indices(&needle)
        .filter(|(i, _)| {
            !word(hay[..*i].chars().next_back()) && !word(hay[i + needle.len()..].chars().next())
        })
        .count()
}

/// `trained` as a whole word, not as the tail of `not trained`.
fn count_trained(hay: &str) -> usize {
    count_word(hay, "trained") - count_word(hay, "not trained")
}

fn all_labels() -> [&'static str; 4] {
    [
        GateState::Passed,
        GateState::NeedsReGate,
        GateState::Failed,
        GateState::NotEvaluated,
    ]
    .map(GateState::label)
}

fn load(case: &Case, edit: impl FnOnce(&std::path::Path)) -> RunRecords {
    let tmp = tempfile::tempdir().unwrap();
    let run = case_run_dir(case, tmp.path());
    edit(&run);
    RunRecords::load(&run, case.size, case.pin)
}

#[test]
fn card_facts_needs_re_gate_case() {
    let case = NEEDS_RE_GATE;
    let rec = load(&case, |_| {});
    let f = card_facts(&cx_for(&case), &rec);
    assert!(f.trained);
    assert_eq!(f.spoke.as_ref().map(|s| s.id.as_str()), Some("vox-lang"));
    assert_eq!(f.gate.state, GateState::NeedsReGate);
    let Some(Blocker::Stale {
        recorded_slice_n,
        current_slice_n,
        ..
    }) = &f.gate.blocker
    else {
        panic!("expected a stale blocker, got {:?}", f.gate.blocker);
    };
    assert_eq!(*recorded_slice_n, Some(32));
    assert_ne!(Some(*current_slice_n), *recorded_slice_n);
    assert!(f.collateral.is_empty() && f.alpha.is_none());
    assert_eq!(f.adapter_evidence, AdapterEvidence::Current);
    let h = f.heldout.as_ref().expect("held-out numbers");
    assert!(h.base_rate.is_some() && h.adapter_rate.is_some());
    let cfg = hub_cards().unwrap();
    assert_eq!(f.why, cfg.sizes[case.size].why);
    assert_eq!(f.peak, Some((27007, "run_summary".to_string())));
}

#[test]
fn sections_in_spec_order() {
    for (name, case) in trained_cases() {
        let card = render_case(&case);
        let h2: Vec<&str> = prose_lines(&card)
            .into_iter()
            .filter_map(|l| l.strip_prefix("## "))
            .collect();
        assert_eq!(
            h2,
            [
                "Why this size",
                "Examples",
                "Use it",
                "Results",
                "Training",
                "Files"
            ],
            "{name}"
        );
        for h in prose_lines(&card).iter().filter(|l| l.starts_with("## ")) {
            assert!(
                h.split(' ').count() <= 4,
                "{name}: heading over three words: {h}"
            );
        }
    }
}

#[test]
fn removed_sections_are_gone() {
    for (name, case) in cases() {
        let card = render_case(&case);
        for gone in [
            "## What this is",
            "## Other sizes",
            "## Links",
            "PeftModel",
            "transformers / PEFT",
        ] {
            assert!(!card.contains(gone), "{name}: still has `{gone}`");
        }
    }
}

#[test]
fn gate_state_stated_once() {
    for (name, case) in trained_cases() {
        let card = render_case(&case);
        let status = status_line(&card);
        let label = card_facts(&cx_for(&case), &load(&case, |_| {}))
            .gate
            .state
            .label();
        assert_eq!(count_word(&status, label), 1, "{name}: {status}");
        let rest = body_after_status(&card);
        assert!(
            !rest.contains("#state-legend"),
            "{name}: the gate or trained fact is linked outside the status line"
        );
    }
}

#[test]
fn trained_and_spoke_stated_once() {
    for (name, case) in trained_cases() {
        let card = render_case(&case);
        let status = status_line(&card);
        assert_eq!(count_trained(&status), 1, "{name}: {status}");
        assert_eq!(count_word(&status, "vox-lang"), 1, "{name}: {status}");
        let rest = body_after_status(&card);
        assert!(!rest.contains("#hub-and-spoke"), "{name}");
        assert_eq!(count_word(&rest, "not trained"), 0, "{name}");
    }
}

#[test]
fn not_trained_card_states_only_the_trained_fact() {
    let card = render_case(&NOT_TRAINED);
    let reason = read_json("cases/states.json")["not_trained_32b"]["train-record.json"]["reason"]
        .as_str()
        .unwrap()
        .to_string();
    let status = status_line(&card);
    assert_eq!(count_word(&status, "not trained"), 1, "{status}");
    assert!(status.starts_with("32B is "), "{status}");
    assert!(status.contains(&format!("“{reason}”")), "{status}");
    for label in all_labels() {
        assert_eq!(count_word(&card, label), 0, "{label}: {card}");
    }
    assert_eq!(count_word(&card, "vox-lang"), 0, "{card}");
    let tags: Vec<&str> = frontmatter(&card)
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .collect();
    assert!(tags.contains(&"not-trained"), "{tags:?}");
    assert!(
        !tags
            .iter()
            .any(|t| t.starts_with("gate-") || t.starts_with("spoke-"))
    );
    let h2: Vec<&str> = prose_lines(&card)
        .into_iter()
        .filter_map(|l| l.strip_prefix("## "))
        .collect();
    assert_eq!(h2, ["Why this size"], "{card}");
    assert!(!frontmatter(&card).contains("model-index"));
}

#[test]
fn failed_card_quotes_the_refusal() {
    let report = &read_json("cases/states.json")["failed_0_6b"]["collateral_damage_report.json"];
    let refusal = format!(
        "the collateral report status is `{}` on `{}`",
        report["status"].as_str().unwrap(),
        report["failed_on"].as_str().unwrap()
    );
    let card = render_case(&FAILED);
    let status = status_line(&card);
    assert!(
        status.contains(&format!("refused 0.6B: “{refusal}”")),
        "{status}"
    );
    assert_eq!(card.matches(&refusal).count(), 1, "{card}");
    let facts = card_facts(&cx_for(&FAILED), &load(&FAILED, |_| {}));
    assert_eq!(facts.gate.state, GateState::Failed);
    assert!(facts.collateral.is_empty() && facts.alpha.is_none());
}

#[test]
fn not_evaluated_card_names_the_missing_records() {
    let facts = card_facts(&cx_for(&NOT_EVALUATED), &load(&NOT_EVALUATED, |_| {}));
    assert_eq!(facts.gate.state, GateState::NotEvaluated);
    let Some(Blocker::Missing(names)) = &facts.gate.blocker else {
        panic!("expected a Missing blocker, got {:?}", facts.gate.blocker);
    };
    assert_eq!(
        names,
        &["collateral_damage_report.json", "gate_receipt.json"]
    );
    let status = status_line(&render_case(&NOT_EVALUATED));
    assert!(
        status.contains("the records that decide 14B are missing"),
        "{status}"
    );
    for n in names {
        assert_eq!(
            status.matches(&format!("`{n}`")).count(),
            1,
            "{n}: {status}"
        );
    }
}

#[test]
fn blurbs_state_no_fact() {
    let cfg = hub_cards().unwrap();
    for (size, entry) in &cfg.sizes {
        for sentence in &entry.why {
            let mut facts = vec!["trained"];
            facts.extend(all_labels());
            facts.extend(cfg.spokes.iter().map(|s| s.id.as_str()));
            for fact in facts {
                assert_eq!(
                    count_word(sentence, fact),
                    0,
                    "{size}: `{fact}` in {sentence}"
                );
            }
        }
    }
}

fn read_json(rel: &str) -> Value {
    let text = std::fs::read_to_string(fixture_root().join(rel)).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn stale_serving_card_omits_adapter_score() {
    let report = read_json("evidence/heldout_adapter_nothink.json");
    let (passed, total, rate) = (
        report["passed_at_1"].as_f64().unwrap(),
        report["total"].as_u64().unwrap(),
        report["pass_rate_at_1"].as_f64().unwrap(),
    );
    let adapter_cell = format!("{}/{total} ({:.1}%)", passed.round() as u64, rate * 100.0);
    let base = read_json("evidence/heldout_base_nothink.json");
    let base_cell = format!(
        "{}/{} ({:.1}%)",
        base["passed_at_1"].as_f64().unwrap().round() as u64,
        base["total"].as_u64().unwrap(),
        base["pass_rate_at_1"].as_f64().unwrap() * 100.0
    );
    assert_ne!(adapter_cell, base_cell, "fixture needs distinct sides");

    // Current serving path: the adapter score and its model-index metric are shown.
    let current = render_case(&NEEDS_RE_GATE);
    assert!(current.contains(&adapter_cell) && current.contains(&base_cell));
    assert!(current.contains(&format!("value: {rate:.4}")) && current.contains("type: pass@1"));
    assert!(!current.contains("is not shown"));

    // Serving field missing: no adapter number anywhere, base numbers stay, one sentence says why.
    let stale = render_case(&STALE_SERVING);
    assert!(!stale.contains(&adapter_cell), "{stale}");
    assert!(!stale.contains(&format!("{rate:.4}")), "{stale}");
    assert!(
        !stale.contains("model-index") && !stale.contains("pass@1 ("),
        "{stale}"
    );
    assert!(stale.contains(&base_cell));
    assert_eq!(
        stale
            .matches("The adapter score for 1.7B is not shown")
            .count(),
        1
    );
    assert!(stale.contains("has no serving field"));

    // Another serving path is named.
    let other = std::fs::read_to_string(fixture_root().join(super::support::CURRENT_CKPT))
        .unwrap()
        .replace("nf4_base_plus_lora_arm v2", "folded_into_base v1");
    let case = Case {
        ckpt: Some(super::support::CURRENT_CKPT),
        ..NEEDS_RE_GATE
    };
    let rec = load(&case, |run| {
        std::fs::write(
            run.join("evidence/heldout_adapter_nothink.ckpt.jsonl"),
            &other,
        )
        .unwrap();
    });
    let card = crate::mens::hub_card::render_model_card(&cx_for(&case), &rec);
    assert!(
        card.contains("records the serving path `folded_into_base v1`"),
        "{card}"
    );
    assert!(!card.contains(&adapter_cell));

    // No adapter checkpoint at all: a report alone is never accepted.
    let rec = load(&NEEDS_RE_GATE, |run| {
        std::fs::remove_file(run.join("evidence/heldout_adapter_nothink.ckpt.jsonl")).unwrap();
    });
    let card = crate::mens::hub_card::render_model_card(&cx_for(&NEEDS_RE_GATE), &rec);
    assert!(
        card.contains("no adapter checkpoint records its serving path"),
        "{card}"
    );
    assert!(!card.contains(&adapter_cell) && !card.contains("type: pass@1"));
}

#[test]
fn frontmatter_is_valid_hub_metadata() {
    for (name, case) in trained_cases() {
        let card = render_case(&case);
        let fm_text = card
            .strip_prefix("---\n")
            .and_then(|r| r.split("\n---\n").next())
            .unwrap();
        let fm: serde_yaml::Value = serde_yaml::from_str(fm_text).unwrap();
        for key in [
            "license",
            "library_name",
            "pipeline_tag",
            "tags",
            "language",
        ] {
            assert!(!fm[key].is_null(), "{name}: missing `{key}`");
        }
        assert!(!fm["base_model"].as_str().unwrap().contains('@'), "{name}");
        assert_eq!(fm["base_model_relation"].as_str(), Some("adapter"));
        assert_eq!(fm["base_model_revision"].as_str().map(str::len), Some(40));
        let tags: Vec<&str> = fm["tags"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|t| t.as_str())
            .collect();
        let gate_tag = card_facts(&cx_for(&case), &load(&case, |_| {}))
            .gate
            .state
            .tag();
        assert!(
            tags.contains(&"spoke-vox-lang") && tags.contains(&gate_tag),
            "{name}"
        );
        assert!(!fm_text.contains("verified"), "{name}");
    }
}

/// The YAML text between the card's opening and closing `---`.
fn frontmatter(card: &str) -> &str {
    let rest = card.strip_prefix("---\n").expect("frontmatter opens");
    &rest[..rest.find("\n---\n").expect("frontmatter closes")]
}

#[test]
fn passed_card_shows_slice_rows() {
    let card = render_case(&PASSED);
    let report = passing_collateral_report(&load_requirement(None).unwrap());
    let fm = frontmatter(&card);
    assert_eq!(fm.matches("type: accuracy").count(), SLICE_NAMES.len());
    assert_eq!(fm.matches("type: pass@1").count(), 1, "{fm}");
    for name in SLICE_NAMES {
        let m = &report["reasoning_slices"]["benches"][name]["accuracy"];
        let row = card
            .lines()
            .find(|l| l.starts_with(&format!("| {name}, ")))
            .unwrap_or_else(|| panic!("no Results row for {name}\n{card}"));
        assert!(row.contains("1.7B"), "{row}");
        for part in [
            format!("{} items", m["n"].as_u64().unwrap()),
            format!(
                "{:.3} then {:.3}",
                m["pre"].as_f64().unwrap(),
                m["post"].as_f64().unwrap()
            ),
            format!("{:.4}", m["p_value"].as_f64().unwrap()),
        ] {
            assert!(row.contains(&part), "{part} not in {row}");
        }
        assert!(fm.contains(&format!("name: {name} accuracy")), "{fm}");
    }
}

#[test]
fn no_stale_numbers_without_current_rule() {
    // A report with the shape of a current one but recorded under a retired rule, carrying
    // numbers no other part of the card can produce.
    let mut stale = passing_collateral_report(&load_requirement(None).unwrap());
    stale["reasoning_slices"]["rule"] = "retired".into();
    for name in SLICE_NAMES {
        let m = &mut stale["reasoning_slices"]["benches"][name]["accuracy"];
        m["pre"] = 0.123.into();
        m["post"] = 0.456.into();
        m["p_value"] = 0.0789.into();
    }
    let rec = load(&PASSED, |run| {
        std::fs::write(run.join("collateral_damage_report.json"), stale.to_string()).unwrap();
    });
    assert_eq!(
        card_facts(&cx_for(&PASSED), &rec).gate.state,
        GateState::NeedsReGate
    );
    let card = crate::mens::hub_card::render_model_card(&cx_for(&PASSED), &rec);

    // The retired-rule report of the 1.7B fixture: its slice names and accuracies stay off the card.
    let old = &read_json("cases/1_7b.json")["needs_re_gate_1_7b"]["collateral_damage_report.json"];
    let mut forbidden: Vec<String> = SLICE_NAMES.iter().map(|s| s.to_string()).collect();
    for r in old["reports"].as_array().unwrap() {
        forbidden.push(r["benchmark"].as_str().unwrap().to_string());
        for k in ["pre", "post"] {
            forbidden.push(format!("{:.3}", r[k].as_f64().unwrap()));
        }
    }
    let fixture_card = render_case(&NEEDS_RE_GATE);
    for text in [&card, &fixture_card] {
        for f in &forbidden {
            assert!(!text.contains(f.as_str()), "stale `{f}` on a card\n{text}");
        }
        assert!(!frontmatter(text).contains("type: accuracy"));
    }
    for n in ["0.123", "0.456", "0.0789"] {
        assert!(!card.contains(n), "stale `{n}` on the card\n{card}");
    }

    // The same rows are there once the rule is current.
    let passed = render_case(&PASSED);
    assert!(SLICE_NAMES.iter().all(|s| passed.contains(s)));
}

#[test]
fn invalid_protocol_report_shows_no_heldout_numbers() {
    // Both held-out reports were scored under another prompt protocol (the retired think128 one).
    let case = Case {
        ckpt: None,
        ..NEEDS_RE_GATE
    };
    let rec = load(&case, |run| {
        let ev = run.join("evidence");
        std::fs::create_dir_all(&ev).unwrap();
        for side in ["base", "adapter"] {
            std::fs::write(
                ev.join(format!("heldout_{side}_nothink.json")),
                r#"{"prompt_protocol":"think128","total":8,"passed_at_1":7.0,"pass_rate_at_1":0.875}"#,
            )
            .unwrap();
        }
    });
    assert_eq!(rec.invalid_evidence.len(), 2);
    let facts = card_facts(&cx_for(&case), &rec);
    assert!(facts.heldout.is_none());
    let card = crate::mens::hub_card::render_model_card(&cx_for(&case), &rec);
    assert!(
        card.contains("No held-out report under the current prompt protocol `qwen3_non_thinking_codegen_v1` exists for 1.7B"),
        "{card}"
    );
    for number in ["87.5%", "7/8", "0.8750", "type: pass@1"] {
        assert!(!card.contains(number), "`{number}` on the card\n{card}");
    }
    // A current report: no such sentence.
    assert!(!render_case(&NEEDS_RE_GATE).contains("No held-out report"));
}
