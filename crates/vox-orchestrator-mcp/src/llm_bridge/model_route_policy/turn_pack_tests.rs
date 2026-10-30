//! DEC-01 / DEC-03 tests for the turn decision pack (phase 322-01).
//!
//! `GOLDEN_G*` are `format!("{:?}")` strings of `build_selection_request`,
//! captured from the unmodified code before the pack existed
//! (`.planning/phases/322-*/evidence/322-01-goldens-before.txt`).

use vox_orchestrator::config::CostPreference::{self, Economy, Performance};
use vox_orchestrator::mode::{ClutchProfile, RiskPosture};
use vox_orchestrator::models::Capability;
use vox_orchestrator::types::TaskCategory;

use super::turn_pack::{Answer, TurnPack, turn_selection_request};
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
