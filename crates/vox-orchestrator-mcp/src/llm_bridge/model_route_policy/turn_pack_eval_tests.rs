//! DEC-04 (phase 322-02): labeled turn fixtures, calib-split calibration of
//! every rule, and rule-order controls. `eval.v1.jsonl` is frozen (SHA-256 in
//! `.planning/phases/322-*/evidence/322-02-eval-freeze.txt`) and never feeds a
//! rule confidence.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use serde_json::json;
use vox_orchestrator::config::CostPreference;

use super::resolve::VOX_LOCAL_PREFERRED_TASKS;
use super::turn_pack::{
    Answer, PACK_VERSION, Question, RULES, Rule, TurnPack, evaluate, evaluate_with_rules, tau_for,
    turn_selection_request,
};
use super::types::McpChatModelResolution;
use crate::selective_metrics::{coverage_and_risk, coverage_at_error, ece, laplace_precision};

const CALIB: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/turn_pack/calib.v1.jsonl"
));
const EVAL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/turn_pack/eval.v1.jsonl"
));

#[derive(Debug, Deserialize, PartialEq)]
struct Gold {
    task_shape: String,
    complexity: u8,
    stakes: u8,
    allow_free_tier: bool,
    privacy_local: bool,
}

#[derive(Debug, Deserialize)]
struct Turn {
    id: String,
    pair_id: Option<String>,
    control: Option<String>,
    prompt: String,
    gold: Gold,
}

fn parse(src: &str) -> Vec<Turn> {
    src.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{e}: {l}")))
        .collect()
}

const QUESTIONS: [Question; 5] = [
    Question::TaskShape,
    Question::Complexity,
    Question::Stakes,
    Question::AllowFreeTier,
    Question::PrivacyLocal,
];

/// Gold label in `Rule::answer` encoding.
fn gold_of(g: &Gold, q: Question) -> u8 {
    match q {
        Question::TaskShape => match g.task_shape.as_str() {
            "code_edit" => 0,
            "research" => 1,
            "plan" => 2,
            "chat" => 3,
            other => panic!("unknown task_shape {other}"),
        },
        Question::Complexity => g.complexity,
        Question::Stakes => g.stakes,
        Question::AllowFreeTier => u8::from(g.allow_free_tier),
        Question::PrivacyLocal => u8::from(g.privacy_local),
    }
}

fn answer_of(p: &TurnPack, q: Question) -> Option<Answer> {
    match q {
        Question::TaskShape => p.task_shape,
        Question::Complexity => p.complexity,
        Question::Stakes => p.stakes,
        Question::AllowFreeTier => p.allow_free_tier,
        Question::PrivacyLocal => p.privacy_local,
    }
}

fn fires(rule: &Rule, prompt: &str) -> bool {
    answer_of(
        &evaluate_with_rules(prompt, std::slice::from_ref(rule)),
        rule.question,
    )
    .is_some()
}

fn pairs<'a>(turns: &'a [Turn], control: &str) -> Vec<(&'a Turn, &'a Turn)> {
    let mut by_pair: BTreeMap<&'a str, Vec<&'a Turn>> = BTreeMap::new();
    for t in turns
        .iter()
        .filter(|t| t.control.as_deref() == Some(control))
    {
        by_pair
            .entry(t.pair_id.as_deref().expect("control without pair_id"))
            .or_default()
            .push(t);
    }
    by_pair
        .into_values()
        .map(|v| {
            assert_eq!(v.len(), 2, "pair {} needs exactly two turns", v[0].id);
            (v[0], v[1])
        })
        .collect()
}

fn count(turns: &[Turn], f: impl Fn(&Gold) -> bool) -> usize {
    turns.iter().filter(|t| f(&t.gold)).count()
}

fn assert_balanced(name: &str, turns: &[Turn], id_prefix: char) {
    assert!(turns.len() >= 24, "{name}: {} turns", turns.len());
    let ids: HashSet<&str> = turns.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids.len(), turns.len(), "{name}: duplicate id");
    assert!(turns.iter().all(|t| t.id.starts_with(id_prefix)), "{name}");
    for (control, min) in [("dual_wording", 4), ("option_order", 2)] {
        let ps = pairs(turns, control);
        assert!(ps.len() >= min, "{name}: {} {control} pairs", ps.len());
        for (a, b) in ps {
            assert_eq!(a.gold, b.gold, "{name}: {} vs {}", a.id, b.id);
            assert_ne!(a.prompt, b.prompt, "{name}: {}", a.id);
        }
    }
    for shape in ["code_edit", "research", "plan", "chat"] {
        let n = count(turns, |g| g.task_shape == shape);
        assert!(n >= 3, "{name}: task_shape {shape} x{n}");
    }
    for v in 0..=2 {
        assert!(
            count(turns, |g| g.complexity == v) >= 3,
            "{name}: complexity {v}"
        );
        assert!(count(turns, |g| g.stakes == v) >= 2, "{name}: stakes {v}");
    }
    assert!(
        turns
            .iter()
            .all(|t| t.gold.complexity <= 2 && t.gold.stakes <= 2)
    );
    assert!(
        count(turns, |g| g.allow_free_tier) >= 4,
        "{name}: free true"
    );
    assert!(
        count(turns, |g| !g.allow_free_tier) >= 4,
        "{name}: free false"
    );
    assert!(
        count(turns, |g| g.privacy_local) >= 3,
        "{name}: privacy true"
    );
}

#[test]
fn fixtures_are_well_formed_and_balanced() {
    let calib = parse(CALIB);
    let eval = parse(EVAL);
    assert_balanced("calib", &calib, 'c');
    assert_balanced("eval", &eval, 'e');
    let calib_prompts: HashSet<&str> = calib.iter().map(|t| t.prompt.as_str()).collect();
    for t in &eval {
        assert!(
            !calib_prompts.contains(t.prompt.as_str()),
            "{} in both splits",
            t.id
        );
    }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[test]
fn rule_confidences_match_calib_split() {
    let calib = parse(CALIB);
    let mut table = String::from("rule id                fired correct expected current\n");
    let mut drift = false;
    for rule in RULES {
        let fired: Vec<&Turn> = calib.iter().filter(|t| fires(rule, &t.prompt)).collect();
        let correct = fired
            .iter()
            .filter(|t| gold_of(&t.gold, rule.question) == rule.answer)
            .count();
        let expected = round2(laplace_precision(correct, fired.len()));
        let ok = (f64::from(rule.confidence) - expected).abs() <= 0.005;
        drift |= !ok;
        table.push_str(&format!(
            "{:<22} {:>5} {:>7} {expected:>8.2} {:>7.2}{}\n",
            rule.id,
            fired.len(),
            correct,
            rule.confidence,
            if ok { "" } else { "  <- drift" }
        ));
    }
    assert!(
        !drift,
        "rule confidences differ from calib Laplace precision:\n{table}"
    );
}

fn all_prompts() -> Vec<String> {
    parse(CALIB)
        .into_iter()
        .chain(parse(EVAL))
        .map(|t| t.prompt)
        .collect()
}

#[test]
fn rule_order_does_not_change_answers() {
    let forward: Vec<Rule> = RULES.to_vec();
    let reversed: Vec<Rule> = RULES.iter().rev().cloned().collect();
    let mut rotated = RULES.to_vec();
    rotated.rotate_left(RULES.len() / 2);
    for p in all_prompts() {
        let base = evaluate_with_rules(&p, &forward);
        assert_eq!(base, evaluate_with_rules(&p, &reversed), "reversed: {p}");
        assert_eq!(base, evaluate_with_rules(&p, &rotated), "rotated: {p}");
    }
    let tie = |id, answer| Rule {
        id,
        question: Question::TaskShape,
        any_of: &["fix"],
        answer,
        confidence: 0.9,
    };
    let a = evaluate_with_rules("fix it", &[tie("t.b", 1), tie("t.a", 0)]);
    let b = evaluate_with_rules("fix it", &[tie("t.a", 0), tie("t.b", 1)]);
    assert_eq!(a, b);
    assert_eq!(a.task_shape.map(|x| x.value), Some(0));
}

#[test]
fn rule_order_control_is_non_vacuous() {
    let contested = all_prompts().into_iter().any(|p| {
        QUESTIONS.iter().any(|&q| {
            RULES
                .iter()
                .filter(|r| r.question == q && fires(r, &p))
                .count()
                >= 2
        })
    });
    assert!(
        contested,
        "no fixture prompt fires two rules on one question"
    );
}

fn ratio(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}

/// Per question, the fraction of `control` pairs whose two packs give the same
/// answer value (abstention counts as an answer).
fn agreement(turns: &[Turn], control: &str) -> serde_json::Value {
    let ps = pairs(turns, control);
    let same = |a: &Turn, b: &Turn, q| {
        answer_of(&evaluate(&a.prompt), q).map(|x| x.value)
            == answer_of(&evaluate(&b.prompt), q).map(|x| x.value)
    };
    let mut per_question = serde_json::Map::new();
    for q in QUESTIONS {
        let n = ps.iter().filter(|(a, b)| same(a, b, q)).count();
        per_question.insert(format!("{q:?}"), json!(ratio(n, ps.len())));
    }
    let all = ps
        .iter()
        .filter(|(a, b)| QUESTIONS.iter().all(|&q| same(a, b, q)))
        .count();
    json!({ "pairs": ps.len(), "all_questions": ratio(all, ps.len()), "per_question": per_question })
}

/// DEC-04 held-out report: prints one `TURN_PACK_EVAL <json>` line and fails if
/// any enabled question's risk at its τ exceeds 0.10.
#[test]
fn selective_eval_report() {
    let eval = parse(EVAL);
    let n_total = eval.len();
    let packs: Vec<TurnPack> = eval.iter().map(|t| evaluate(&t.prompt)).collect();
    let mut questions = serde_json::Map::new();
    let mut over_risk = Vec::new();
    for q in QUESTIONS {
        let items: Vec<(f32, bool)> = eval
            .iter()
            .zip(&packs)
            .filter_map(|(t, p)| {
                answer_of(p, q).map(|a| (a.confidence, a.value == gold_of(&t.gold, q)))
            })
            .collect();
        let correct = items.iter().filter(|i| i.1).count();
        let tau = tau_for(q);
        let disabled = tau > 1.0;
        let (coverage, risk) = coverage_and_risk(&items, n_total, tau);
        if !disabled && risk > 0.10 {
            over_risk.push(format!("{q:?} risk@tau {risk:.3}"));
        }
        questions.insert(
            format!("{q:?}"),
            json!({
                "n": n_total,
                "answered": items.len(),
                "accuracy": ratio(correct, items.len()),
                "ece": ece(&items, 10),
                "tau": tau,
                "coverage_at_tau": coverage,
                "risk_at_tau": risk,
                "coverage_at_error_0_05": coverage_at_error(&items, n_total, 0.05),
                "coverage_at_error_0_10": coverage_at_error(&items, n_total, 0.10),
                "disabled": disabled,
            }),
        );
    }
    // A default chat turn: caller left task and complexity unset, no clutch.
    let res = McpChatModelResolution {
        task_unset: true,
        complexity_unset: true,
        ..Default::default()
    };
    let derived = VOX_LOCAL_PREFERRED_TASKS.contains(&res.task_category);
    let request = |pack: Option<&TurnPack>| {
        format!(
            "{:?}",
            turn_selection_request(pack, &res, derived, CostPreference::Performance, vec![])
        )
    };
    let unpatched = request(None);
    let patched_turns = packs
        .iter()
        .filter(|p| request(Some(p)) != unpatched)
        .count();
    let report = json!({
        "pack_version": PACK_VERSION,
        "eval_sha256": hex::encode(vox_crypto::interop_sha256(EVAL.as_bytes())),
        "n": n_total,
        "questions": questions,
        "dual_wording_agreement": agreement(&eval, "dual_wording"),
        "option_order_agreement": agreement(&eval, "option_order"),
        "patched_turns": patched_turns,
    });
    println!("TURN_PACK_EVAL {report}");
    assert!(
        over_risk.is_empty(),
        "risk at tau above 0.10: {over_risk:?}"
    );
}
