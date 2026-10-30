//! CAT-02 (phase 322-03): ChatARC fixtures, calib-split calibration of the
//! Choice bins, and the held-out report. `eval.v1.jsonl` is frozen (SHA-256 in
//! `.planning/phases/322-*/evidence/322-03-eval-freeze.txt`) and never feeds a
//! bin.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::selective_metrics::{coverage_and_risk, coverage_at_error, ece, laplace_precision};

const CATALOG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/chatarc/catalog.v1.json"
));
const CALIB: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/chatarc/calib.v1.jsonl"
));
const EVAL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/chatarc/eval.v1.jsonl"
));

/// TAU_CHOICE above 1.0 means ChatARC never acts (fail closed).
const DISABLED: bool = TAU_CHOICE > 1.0;

#[derive(Deserialize)]
struct Catalog {
    skills: Vec<Skill>,
    tools: Vec<Tool>,
    commands: Vec<Command>,
}

#[derive(Deserialize)]
struct Skill {
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct Tool {
    name: String,
    description: String,
}

#[derive(Deserialize)]
struct Command {
    invocation: String,
    about: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
struct Gold {
    kind: String,
    id: Option<String>,
}

#[derive(Deserialize)]
struct Query {
    id: String,
    pair_id: Option<String>,
    control: Option<String>,
    query: String,
    gold: Gold,
}

fn catalog() -> Catalog {
    serde_json::from_str(CATALOG).expect("catalog.v1.json parses")
}

fn parse(src: &str) -> Vec<Query> {
    src.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{e}: {l}")))
        .collect()
}

fn candidates(cat: &Catalog) -> Vec<ActionCandidate> {
    let tools: Vec<&'static McpToolRegistryEntry> = cat
        .tools
        .iter()
        .map(|t| {
            &*Box::leak(Box::new(McpToolRegistryEntry {
                name: Box::leak(t.name.clone().into_boxed_str()),
                description: Box::leak(t.description.clone().into_boxed_str()),
                product_lane: "ai",
                http_read_role_eligible: true,
                exposure: vox_mcp_registry::Exposure::Public,
                side_effect_class: vox_mcp_registry::SideEffectClass::ReadOnly,
                requires: &[],
            }))
        })
        .collect();
    let hints: Vec<CommandHint> = cat
        .commands
        .iter()
        .map(|c| CommandHint {
            invocation: c.invocation.clone(),
            about: c.about.clone(),
        })
        .collect();
    let skills: Vec<SkillManifest> = cat
        .skills
        .iter()
        .map(|s| SkillManifest {
            id: s.id.clone(),
            name: s.name.clone(),
            version: "1.0.0".to_string(),
            description: s.description.clone(),
            tags: s.tags.clone(),
            ..Default::default()
        })
        .collect();
    let mut cands = skill_candidates(&skills);
    cands.extend(tool_candidates(&tools));
    cands.extend(command_candidates(&hints));
    cands
}

fn as_gold(kind: ActionKind, id: &str) -> Gold {
    Gold {
        kind: kind.tag().to_string(),
        id: Some(id.to_string()),
    }
}

fn none_gold() -> Gold {
    Gold {
        kind: "none".to_string(),
        id: None,
    }
}

/// One query's shortlist outcome: top-1 action (if any) with its margin, and
/// whether gold is among the shortlisted actions.
struct Scored {
    top: Option<(Gold, f32)>,
    gold_in_shortlist: bool,
}

fn score(cands: &[ActionCandidate], q: &Query) -> Scored {
    let options = shortlist(cands, &q.query);
    let actions: Vec<(&ActionCandidate, u64)> = options
        .iter()
        .filter_map(|o| match o {
            ChatArcOption::Action { candidate, score } => Some((candidate, *score)),
            ChatArcOption::None => None,
        })
        .collect();
    let top = actions.first().map(|(c, s1)| {
        let s2 = actions.get(1).map_or(0, |(_, s)| *s);
        (as_gold(c.kind, &c.id), margin(*s1, s2))
    });
    let gold_in_shortlist = actions
        .iter()
        .any(|(c, _)| as_gold(c.kind, &c.id) == q.gold);
    Scored {
        top,
        gold_in_shortlist,
    }
}

/// Per bin: (fired, correct) over calib queries with a non-empty shortlist.
fn calib_table(cands: &[ActionCandidate]) -> Vec<(usize, usize)> {
    let mut table = vec![(0usize, 0usize); CHOICE_BINS.len()];
    for q in parse(CALIB) {
        if let Some((top, m)) = score(cands, &q).top {
            let b = bin_index(m, &CHOICE_BINS).expect("bin 0 starts at 0.0");
            table[b].0 += 1;
            table[b].1 += usize::from(top == q.gold);
        }
    }
    table
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[test]
fn chatarc_fixtures_are_well_formed() {
    let cat = catalog();
    let known: HashSet<Gold> = candidates(&cat)
        .iter()
        .map(|c| as_gold(c.kind, &c.id))
        .collect();
    let (calib, eval) = (parse(CALIB), parse(EVAL));
    let calib_queries: HashSet<&str> = calib.iter().map(|q| q.query.as_str()).collect();
    assert!(
        eval.iter()
            .all(|q| !calib_queries.contains(q.query.as_str()))
    );
    for (name, split) in [("calib", &calib), ("eval", &eval)] {
        assert!(split.len() >= 20, "{name}: {} queries", split.len());
        let mut per_kind: BTreeMap<&str, usize> = BTreeMap::new();
        for q in split.iter() {
            assert!(
                q.gold == none_gold() || known.contains(&q.gold),
                "{name} {}: gold {:?} not in catalog",
                q.id,
                q.gold
            );
            *per_kind.entry(q.gold.kind.as_str()).or_default() += 1;
        }
        for kind in ["none", "skill", "tool", "command"] {
            assert!(
                per_kind.get(kind).copied().unwrap_or(0) >= 3,
                "{name}: {per_kind:?}"
            );
        }
        let mut pairs: BTreeMap<&str, Vec<&Query>> = BTreeMap::new();
        for q in split
            .iter()
            .filter(|q| q.control.as_deref() == Some("dual_wording"))
        {
            pairs
                .entry(q.pair_id.as_deref().expect("pair_id"))
                .or_default()
                .push(q);
        }
        assert!(
            pairs.len() >= 3,
            "{name}: {} dual_wording pairs",
            pairs.len()
        );
        assert!(
            pairs
                .values()
                .all(|p| p.len() == 2 && p[0].gold == p[1].gold)
        );
    }
}

#[test]
fn choice_bins_match_calib_split() {
    let table = calib_table(&candidates(&catalog()));
    let expected: Vec<(f32, f64)> = CHOICE_BINS
        .iter()
        .zip(&table)
        .map(|((lo, _), (fired, correct))| (*lo, round2(laplace_precision(*correct, *fired))))
        .collect();
    let drift = CHOICE_BINS
        .iter()
        .zip(&expected)
        .any(|((_, p), (_, e))| (f64::from(*p) - e).abs() > 0.005);
    assert!(
        !drift,
        "CHOICE_BINS drift from calib. (fired, correct) per bin: {table:?}; expected bins: {expected:?}"
    );
}

#[test]
fn chatarc_eval_report() {
    let cands = candidates(&catalog());
    let eval = parse(EVAL);
    let n = eval.len();
    let mut items: Vec<(f32, bool)> = Vec::new();
    let (mut recall_hit, mut recall_n) = (0usize, 0usize);
    let (mut none_ok, mut none_n) = (0usize, 0usize);
    let mut e2e_ok = 0usize;
    let mut finals: BTreeMap<String, Gold> = BTreeMap::new();
    let mut per_item = Vec::new();
    for q in &eval {
        let s = score(&cands, q);
        if let Some((top, m)) = &s.top {
            let confidence = bin_index(*m, &CHOICE_BINS).map_or(0.0, |i| CHOICE_BINS[i].1);
            items.push((confidence, *top == q.gold));
            per_item.push(json!({"id": q.id, "top": top.id, "margin": m, "confidence": confidence, "correct": *top == q.gold}));
        }
        let pick = decide(&cands, &q.query, &CHOICE_BINS, TAU_CHOICE);
        let output = pick.map_or_else(none_gold, |p| as_gold(p.kind, &p.id));
        if q.gold == none_gold() {
            none_n += 1;
            none_ok += usize::from(output == none_gold());
        } else {
            recall_n += 1;
            recall_hit += usize::from(s.gold_in_shortlist);
        }
        e2e_ok += usize::from(output == q.gold);
        finals.insert(q.id.clone(), output);
    }
    let mut pairs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for q in eval
        .iter()
        .filter(|q| q.control.as_deref() == Some("dual_wording"))
    {
        pairs
            .entry(q.pair_id.as_deref().unwrap_or_default())
            .or_default()
            .push(&q.id);
    }
    let agree = pairs
        .values()
        .filter(|ids| ids.len() == 2 && finals[ids[0]] == finals[ids[1]])
        .count();
    let (coverage, risk) = coverage_and_risk(&items, n, TAU_CHOICE);
    let frac = |a: usize, b: usize| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    let report = json!({
        "version": CHATARC_VERSION,
        "eval_sha256": hex::encode(vox_crypto::interop_sha256(EVAL.as_bytes())),
        "n": n,
        "tau": TAU_CHOICE,
        "bins": CHOICE_BINS,
        "answered": items.len(),
        "coverage_at_tau": coverage,
        "risk_at_tau": risk,
        "ece": ece(&items, 10),
        "coverage_at_error_0_05": coverage_at_error(&items, n, 0.05),
        "coverage_at_error_0_10": coverage_at_error(&items, n, 0.10),
        "recall_at_k": frac(recall_hit, recall_n),
        "none_abstention": frac(none_ok, none_n),
        "end_to_end_accuracy": frac(e2e_ok, n),
        "dual_wording_agreement": {"pairs": pairs.len(), "agreement": frac(agree, pairs.len())},
        "disabled": DISABLED,
        "items": per_item,
    });
    println!("CHATARC_EVAL {report}");
    assert!(
        DISABLED || risk <= 0.10,
        "risk at TAU_CHOICE {risk} > 0.10: fail closed (TAU_CHOICE 1.01, chatarc.v2)"
    );
}
