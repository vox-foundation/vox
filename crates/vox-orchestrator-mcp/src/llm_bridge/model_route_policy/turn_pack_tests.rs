//! DEC-01 / DEC-03 tests for the turn decision pack (phase 322-01).
//!
//! `GOLDEN_G*` are `format!("{:?}")` strings of `build_selection_request`,
//! captured from the unmodified code before the pack existed
//! (`.planning/phases/322-*/evidence/322-01-goldens-before.txt`).

use vox_orchestrator::config::CostPreference::{self, Economy, Performance};
use vox_orchestrator::mode::{ClutchProfile, RiskPosture};
use vox_orchestrator::models::Capability;
use vox_orchestrator::types::TaskCategory;

use super::turn_pack::{
    Answer, Question, Rule, TAU_APPLY, TAU_FREE, TurnPack, UNCALIBRATED, evaluate,
    evaluate_with_rules, turn_selection_request,
};
use super::types::McpChatModelResolution;

const GOLDEN_G1: &str = r"ModelSelectionRequest { intent: SelectionIntent { task: CodeGen, axes: SelectionAxes { cost: 70, responsiveness: 15, intelligence: 15 }, complexity: 5, context_size_hint: None, caller_hint: None, prefer_local: true, max_cost_usd_per_call: None, cacheable_workload: false, allow_free_in_performance_mode: false }, required_capabilities: [], candidate_scope: AllProviders }";
const GOLDEN_G2: &str = r"ModelSelectionRequest { intent: SelectionIntent { task: Research, axes: SelectionAxes { cost: 33, responsiveness: 33, intelligence: 34 }, complexity: 10, context_size_hint: None, caller_hint: None, prefer_local: false, max_cost_usd_per_call: None, cacheable_workload: false, allow_free_in_performance_mode: false }, required_capabilities: [SupportsToolUse], candidate_scope: AllProviders }";
const GOLDEN_G3: &str = r"ModelSelectionRequest { intent: SelectionIntent { task: CodeGen, axes: SelectionAxes { cost: 15, responsiveness: 15, intelligence: 70 }, complexity: 8, context_size_hint: None, caller_hint: None, prefer_local: true, max_cost_usd_per_call: None, cacheable_workload: true, allow_free_in_performance_mode: false }, required_capabilities: [], candidate_scope: AllProviders }";
const GOLDEN_G4: &str = r"ModelSelectionRequest { intent: SelectionIntent { task: CodeGen, axes: SelectionAxes { cost: 15, responsiveness: 15, intelligence: 70 }, complexity: 3, context_size_hint: None, caller_hint: None, prefer_local: true, max_cost_usd_per_call: None, cacheable_workload: false, allow_free_in_performance_mode: false }, required_capabilities: [SupportsReasoning], candidate_scope: AllProviders }";
const GOLDEN_G5: &str = r"ModelSelectionRequest { intent: SelectionIntent { task: Planning, axes: SelectionAxes { cost: 70, responsiveness: 15, intelligence: 15 }, complexity: 7, context_size_hint: None, caller_hint: None, prefer_local: false, max_cost_usd_per_call: None, cacheable_workload: false, allow_free_in_performance_mode: false }, required_capabilities: [], candidate_scope: AllProviders }";

/// One golden fixture: the resolution (both unset markers on), the derived
/// `prefer_local`, the cost preference, the capabilities and the golden string.
struct Fixture {
    res: McpChatModelResolution,
    derived_prefer_local: bool,
    preference: CostPreference,
    caps: Vec<Capability>,
    golden: &'static str,
}

fn res_of(
    task_category: TaskCategory,
    complexity: u8,
    cacheable: bool,
    clutch: Option<ClutchProfile>,
    risk: Option<RiskPosture>,
) -> McpChatModelResolution {
    McpChatModelResolution {
        task_category,
        complexity,
        free_tier_fill_in_middle: cacheable,
        clutch,
        risk,
        task_unset: true,
        complexity_unset: true,
        ..Default::default()
    }
}

fn fixture(i: usize) -> Fixture {
    use TaskCategory::{CodeGen, Planning, Research};
    let (res, derived_prefer_local, preference, caps, golden) = match i {
        1 => (
            res_of(CodeGen, 5, false, None, None),
            true,
            Economy,
            vec![],
            GOLDEN_G1,
        ),
        2 => (
            res_of(Research, 10, false, None, None),
            false,
            Performance,
            vec![Capability::SupportsToolUse],
            GOLDEN_G2,
        ),
        3 => (
            res_of(
                CodeGen,
                8,
                true,
                Some(ClutchProfile::Genius),
                Some(RiskPosture::Moderate),
            ),
            true,
            Performance,
            vec![],
            GOLDEN_G3,
        ),
        4 => (
            res_of(
                CodeGen,
                3,
                false,
                Some(ClutchProfile::Free),
                Some(RiskPosture::Low),
            ),
            true,
            Economy,
            vec![Capability::SupportsReasoning],
            GOLDEN_G4,
        ),
        5 => (
            res_of(
                Planning,
                7,
                false,
                Some(ClutchProfile::Efficiency),
                Some(RiskPosture::Moderate),
            ),
            false,
            Economy,
            vec![],
            GOLDEN_G5,
        ),
        _ => unreachable!("fixtures are G1..G5"),
    };
    Fixture {
        res,
        derived_prefer_local,
        preference,
        caps,
        golden,
    }
}

fn request_debug(pack: Option<&TurnPack>, fx: &Fixture) -> String {
    format!(
        "{:?}",
        turn_selection_request(
            pack,
            &fx.res,
            fx.derived_prefer_local,
            fx.preference,
            fx.caps.clone()
        )
    )
}

fn answer(value: u8, confidence: f32) -> Option<Answer> {
    Some(Answer {
        value,
        confidence,
        rule_id: "test.rule",
    })
}

fn assert_flag_off_matches_golden(i: usize) {
    let fx = fixture(i);
    assert_eq!(request_debug(None, &fx), fx.golden, "G{i}");
}

#[test]
fn flag_off_request_matches_golden_g1() {
    assert_flag_off_matches_golden(1);
}

#[test]
fn flag_off_request_matches_golden_g2() {
    assert_flag_off_matches_golden(2);
}

#[test]
fn flag_off_request_matches_golden_g3() {
    assert_flag_off_matches_golden(3);
}

#[test]
fn flag_off_request_matches_golden_g4() {
    assert_flag_off_matches_golden(4);
}

#[test]
fn flag_off_request_matches_golden_g5() {
    assert_flag_off_matches_golden(5);
}

fn research_pack() -> TurnPack {
    TurnPack {
        task_shape: answer(1, 0.95),
        ..Default::default()
    }
}

#[test]
fn flag_on_confident_task_patch_changes_request() {
    let fx = fixture(1);
    let pack = research_pack();
    let req = turn_selection_request(
        Some(&pack),
        &fx.res,
        fx.derived_prefer_local,
        fx.preference,
        fx.caps.clone(),
    );
    assert_eq!(req.intent.task, TaskCategory::Research);
    assert_ne!(format!("{req:?}"), GOLDEN_G1);
}

#[test]
fn task_set_by_caller_is_never_patched() {
    let mut fx = fixture(1);
    fx.res.task_unset = false;
    fx.res.task_category = TaskCategory::Testing;
    let pack = research_pack();
    let req = turn_selection_request(
        Some(&pack),
        &fx.res,
        fx.derived_prefer_local,
        fx.preference,
        fx.caps.clone(),
    );
    assert_eq!(req.intent.task, TaskCategory::Testing);
}

#[test]
fn pack_call_site_follows_source_policy_clutch_resolution() {
    let src = include_str!("resolve.rs");
    let clutch = src
        .find("res.clutch = Some(clutch);")
        .expect("source-policy clutch block present");
    let pack = src
        .find("turn_selection_request(")
        .expect("pack call site present");
    assert!(
        pack > clutch,
        "the pack must run after the source-policy clutch block"
    );
}

fn request(pack: &TurnPack, fx: &Fixture) -> vox_orchestrator::models::ModelSelectionRequest {
    turn_selection_request(
        Some(pack),
        &fx.res,
        fx.derived_prefer_local,
        fx.preference,
        fx.caps.clone(),
    )
}

#[test]
fn complexity_set_by_caller_is_never_patched() {
    let mut fx = fixture(3);
    fx.res.complexity_unset = false;
    let pack = TurnPack {
        complexity: answer(0, 0.95),
        ..Default::default()
    };
    assert_eq!(request(&pack, &fx).intent.complexity, 8);
}

#[test]
fn complexity_unset_is_patched_from_confident_score() {
    let fx = fixture(1);
    for (score, want) in [(0, 2), (1, 5), (2, 8)] {
        let pack = TurnPack {
            complexity: answer(score, 0.95),
            ..Default::default()
        };
        assert_eq!(request(&pack, &fx).intent.complexity, want, "score {score}");
    }
}

#[test]
fn prefer_local_not_patched_when_caller_set_task() {
    let mut fx = fixture(2);
    fx.res.task_unset = false;
    let pack = TurnPack {
        privacy_local: answer(1, 0.95),
        ..Default::default()
    };
    let req = request(&pack, &fx);
    assert_eq!(req.intent.task, TaskCategory::Research);
    assert!(!req.intent.prefer_local);
}

#[test]
fn prefer_local_recomputed_from_patched_task() {
    let fx = fixture(1);
    assert!(
        fx.derived_prefer_local,
        "the CodeGen placeholder derives prefer_local"
    );
    let pack = TurnPack {
        task_shape: answer(3, 0.95),
        ..Default::default()
    };
    let req = request(&pack, &fx);
    assert_eq!(req.intent.task, TaskCategory::Chat);
    assert!(!req.intent.prefer_local);
}

#[test]
fn privacy_local_raises_prefer_local_when_task_unset() {
    let fx = fixture(1);
    let pack = TurnPack {
        task_shape: answer(1, 0.95),
        privacy_local: answer(1, 0.95),
        ..Default::default()
    };
    let req = request(&pack, &fx);
    assert_eq!(req.intent.task, TaskCategory::Research);
    assert!(req.intent.prefer_local);
}

fn free_pack() -> TurnPack {
    TurnPack {
        allow_free_tier: answer(1, 0.95),
        ..Default::default()
    }
}

#[test]
fn allow_free_set_only_when_clutch_unset() {
    let fx = fixture(1);
    assert!(fx.res.clutch.is_none());
    assert!(
        request(&free_pack(), &fx)
            .intent
            .allow_free_in_performance_mode
    );
}

#[test]
fn allow_free_never_set_when_clutch_set() {
    for clutch in [
        ClutchProfile::Free,
        ClutchProfile::Efficiency,
        ClutchProfile::Balanced,
        ClutchProfile::Genius,
    ] {
        let mut fx = fixture(1);
        fx.res.clutch = Some(clutch);
        assert!(
            !request(&free_pack(), &fx)
                .intent
                .allow_free_in_performance_mode,
            "{clutch:?}"
        );
    }
}

#[test]
fn allow_free_blocked_by_high_stakes_or_privacy() {
    let fx = fixture(1);
    let stakes = TurnPack {
        stakes: answer(2, 0.95),
        ..free_pack()
    };
    assert!(!request(&stakes, &fx).intent.allow_free_in_performance_mode);
    let privacy = TurnPack {
        privacy_local: answer(1, 0.95),
        ..free_pack()
    };
    assert!(!request(&privacy, &fx).intent.allow_free_in_performance_mode);
}

#[test]
fn below_tau_never_patches() {
    let fx = fixture(1);
    let pack = TurnPack {
        task_shape: answer(1, TAU_APPLY - 0.01),
        complexity: answer(2, TAU_APPLY - 0.01),
        allow_free_tier: answer(1, TAU_FREE - 0.01),
        ..Default::default()
    };
    assert_eq!(format!("{:?}", request(&pack, &fx)), GOLDEN_G1);
}

#[test]
fn at_tau_patches() {
    let fx = fixture(1);
    let pack = TurnPack {
        task_shape: answer(1, TAU_APPLY),
        complexity: answer(2, TAU_APPLY),
        allow_free_tier: answer(1, TAU_FREE),
        ..Default::default()
    };
    let req = request(&pack, &fx);
    assert_eq!(req.intent.task, TaskCategory::Research);
    assert_eq!(req.intent.complexity, 8);
    assert!(req.intent.allow_free_in_performance_mode);
}

#[test]
fn pack_changes_only_the_four_inputs() {
    let pack = TurnPack {
        task_shape: answer(1, 0.95),
        complexity: answer(2, 0.95),
        stakes: answer(0, 0.95),
        allow_free_tier: answer(1, 0.95),
        privacy_local: answer(0, 0.95),
    };
    for i in 1..=5 {
        let fx = fixture(i);
        let patched = request(&pack, &fx);
        let mut expected = turn_selection_request(
            None,
            &fx.res,
            fx.derived_prefer_local,
            fx.preference,
            fx.caps.clone(),
        );
        assert_eq!(format!("{expected:?}"), fx.golden, "G{i}");
        expected.intent.task = patched.intent.task;
        expected.intent.complexity = patched.intent.complexity;
        expected.intent.prefer_local = patched.intent.prefer_local;
        expected.intent.allow_free_in_performance_mode =
            patched.intent.allow_free_in_performance_mode;
        assert_eq!(format!("{patched:?}"), format!("{expected:?}"), "G{i}");
    }
}

#[test]
fn uncalibrated_rules_never_patch() {
    let prompts = [
        "please fix the bug in the parser",
        "what is the latest research on this",
        "plan the roadmap for the next milestone",
        "thanks, hello there",
        "quick typo fix",
        "redesign the architecture across crates",
        "explain and show me the file",
        "delete the production credentials",
        "cheap free tier is fine, no rush",
        "use the best model, this is critical",
        "keep this offline and local only",
        "Refactor -- the CONCURRENCY race condition; deploy it locally!",
    ];
    for p in prompts {
        let pack = evaluate(p);
        assert_ne!(pack, TurnPack::default(), "{p:?} fires no rule");
        for i in 1..=5 {
            let fx = fixture(i);
            assert_eq!(
                format!("{:?}", request(&pack, &fx)),
                fx.golden,
                "{p:?} G{i}"
            );
        }
    }
}

const TIE_RESEARCH: Rule = Rule {
    id: "task_shape.1.1",
    question: Question::TaskShape,
    any_of: &["fix"],
    answer: 1,
    confidence: UNCALIBRATED,
};
const TIE_CODE: Rule = Rule {
    id: "task_shape.0.1",
    question: Question::TaskShape,
    any_of: &["fix"],
    answer: 0,
    confidence: UNCALIBRATED,
};

#[test]
fn ties_break_by_rule_id_not_table_order() {
    let forward = evaluate_with_rules("Fix it.", &[TIE_RESEARCH, TIE_CODE]);
    let reverse = evaluate_with_rules("Fix it.", &[TIE_CODE, TIE_RESEARCH]);
    assert_eq!(forward, reverse);
    assert_eq!(
        forward.task_shape.map(|a| a.rule_id),
        Some("task_shape.0.1")
    );
}
