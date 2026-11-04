//! ChatARC shortlist, Choice, discovery and precedence tests (phase 322-03).

use super::*;
use vox_mcp_registry::{Exposure, SideEffectClass};
use vox_plugin_api::skill::LoadedSkill;
use vox_skills::new_registry_arc;

fn cand(kind: ActionKind, id: &str, text: &str) -> ActionCandidate {
    ActionCandidate {
        kind,
        id: id.to_string(),
        text: text.to_string(),
    }
}

fn action(id: &str, score: u64) -> ChatArcOption {
    ChatArcOption::Action {
        candidate: cand(ActionKind::Tool, id, id),
        score,
    }
}

fn install(reg: &SkillRegistry, id: &str, description: &str) {
    reg.install(LoadedSkill {
        plugin_id: id.to_string(),
        format_version: 1,
        manifest: SkillManifest {
            id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
            ..Default::default()
        },
        body: format!("body of {id}"),
        exposed_tools: vec![],
    });
}

fn entry(name: &'static str, lane: &'static str) -> McpToolRegistryEntry {
    McpToolRegistryEntry {
        name,
        description: "synthetic tool for discovery tests",
        product_lane: lane,
        http_read_role_eligible: true,
        exposure: Exposure::Public,
        side_effect_class: SideEffectClass::ReadOnly,
        requires: &[],
    }
}

fn leak(entries: Vec<McpToolRegistryEntry>) -> &'static [McpToolRegistryEntry] {
    Box::leak(entries.into_boxed_slice())
}

fn ids(cands: &[ActionCandidate]) -> Vec<&str> {
    cands.iter().map(|c| c.id.as_str()).collect()
}

fn top_id(options: &[ChatArcOption]) -> Option<&str> {
    match options.first()? {
        ChatArcOption::Action { candidate, .. } => Some(candidate.id.as_str()),
        ChatArcOption::None => None,
    }
}

#[test]
fn shortlist_is_at_most_k_and_ends_with_none() {
    let cands: Vec<ActionCandidate> = (0..30)
        .map(|i| {
            cand(
                ActionKind::Tool,
                &format!("t{i}"),
                &format!("deploy the service {i}"),
            )
        })
        .collect();
    let options = shortlist(&cands, "deploy service");
    assert_eq!(options.len(), K_MAX, "{options:?}");
    assert_eq!(options.last(), Some(&ChatArcOption::None));
    assert!(
        options[..K_MAX - 1]
            .iter()
            .all(|o| matches!(o, ChatArcOption::Action { .. }))
    );
    assert_eq!(shortlist(&cands, "zebra"), vec![ChatArcOption::None]);
}

#[test]
fn choice_below_tau_is_none() {
    // margin (100 - 50) / 100 = 0.5 lands in the second bin
    let options = vec![action("a", 100), action("b", 50), ChatArcOption::None];
    assert_eq!(choose_with(&options, &[(0.0, 0.1), (0.4, 0.79)], 0.8), None);
    let pick = choose_with(&options, &[(0.0, 0.1), (0.4, 0.80)], 0.8).expect("at tau");
    assert_eq!((pick.id.as_str(), pick.confidence), ("a", 0.80));
    assert_eq!(
        choose_with(&[ChatArcOption::None], &[(0.0, 1.0)], 0.0),
        None
    );
}

#[test]
fn decide_if_enabled_off_is_none_and_gathers_nothing() {
    let pick = decide_if_enabled(
        false,
        || -> Vec<ActionCandidate> { panic!("gathered candidates while ChatARC is off") },
        "run the tests",
        &[(0.0, 1.0)],
        0.0,
    );
    assert_eq!(pick, None);
    let on = decide_if_enabled(
        true,
        || vec![cand(ActionKind::Tool, "vox_run_tests", "run tests")],
        "run the tests",
        &[(0.0, 1.0)],
        0.0,
    );
    assert_eq!(on.map(|p| p.id), Some("vox_run_tests".to_string()));
}

/// WR-09: a threshold above 1.0 (Choice disabled) can never act, so ChatARC
/// gathers nothing: no skill list, no tool selection, no command catalog, no
/// BM25 build.
#[test]
fn decide_if_enabled_disabled_tau_gathers_nothing() {
    let pick = decide_if_enabled(
        true,
        || -> Vec<ActionCandidate> { panic!("gathered candidates while Choice is disabled") },
        "run the tests",
        &[(0.0, 1.0)],
        1.01,
    );
    assert_eq!(pick, None);
}

#[test]
fn new_skill_appears_on_next_discovery_pass() {
    let reg = new_registry_arc();
    install(&reg, "a", "alpha workflow");
    install(&reg, "b", "beta workflow");
    assert!(!ids(&skill_candidates(&reg.list(None))).contains(&"c"));
    install(&reg, "c", "quokka grooming ritual");
    let cands = skill_candidates(&reg.list(None));
    assert!(ids(&cands).contains(&"c"));
    assert_eq!(top_id(&shortlist(&cands, "quokka ritual")), Some("c"));
}

#[test]
fn new_tool_appears_on_next_discovery_pass() {
    let skills = new_registry_arc();
    let before = leak(vec![entry("vox_alpha", "ai")]);
    assert!(!ids(&gather_tool_candidates(before, &skills)).contains(&"vox_beta"));
    let after = leak(vec![entry("vox_alpha", "ai"), entry("vox_beta", "app")]);
    assert!(ids(&gather_tool_candidates(after, &skills)).contains(&"vox_beta"));
}

#[test]
fn new_command_appears_on_next_discovery_pass() {
    let hint = |invocation: &str| CommandHint {
        invocation: invocation.to_string(),
        about: format!("about {invocation}"),
    };
    assert!(!ids(&command_candidates(&[hint("vox x")])).contains(&"vox y"));
    assert!(ids(&command_candidates(&[hint("vox x"), hint("vox y")])).contains(&"vox y"));
}

#[test]
fn tool_candidates_respect_select_tools_for_turn_filters() {
    let registry = leak(vec![
        entry("vox_ok", "ai"),
        entry("vox_platform_only", "platform"),
        entry("vox_chat_reenter", "ai"),
    ]);
    assert_eq!(
        ids(&gather_tool_candidates(registry, &new_registry_arc())),
        ["vox_ok"]
    );
}

#[test]
fn real_registry_shortlist_is_bounded() {
    let cands = gather_tool_candidates(TOOL_REGISTRY, &new_registry_arc());
    assert!(cands.len() > crate::llm_bridge::tool_selection::DEFAULT_MAX_TOOLS);
    assert!(!ids(&cands).iter().any(|n| n.starts_with("vox_chat_")));
    let options = shortlist(&cands, "spawn a sub agent to run the tests");
    assert!(options.len() <= K_MAX, "{}", options.len());
    assert!(options.len() > 1, "expected at least one action");
    assert_eq!(options.last(), Some(&ChatArcOption::None));
}

#[test]
fn explicit_pin_wins_over_chatarc_skill() {
    assert_eq!(skill_to_inject(Some("x"), Some("y")), Some(("x", "pinned")));
    assert_eq!(skill_to_inject(None, Some("y")), Some(("y", "chatarc")));
    assert_eq!(skill_to_inject(None, None), None);
}

#[test]
fn only_skill_picks_name_a_skill() {
    let pick = |kind| ChatArcPick {
        kind,
        id: "x".to_string(),
        text: String::new(),
        confidence: 1.0,
    };
    assert_eq!(pick(ActionKind::Skill).skill_id(), Some("x"));
    assert_eq!(pick(ActionKind::Tool).skill_id(), None);
    assert_eq!(pick(ActionKind::Command).skill_id(), None);
}
