// ─── Tests ────────────────────────────────────────────────────────────────────

use super::{
    A2A_MESSAGE_TYPES, ORCHESTRATOR_TOOLS, SKILL_TOOLS, SyntheticGenConfig, TOOL_REGISTRY_SLIM,
    a2a_pairs::generate_a2a_pairs,
    agent_pairs::generate_agent_pairs,
    orchestrator_pairs::{generate_orchestrator_pairs, generate_skill_pairs},
    tool_pairs::generate_tool_pairs,
    workflow_pairs::generate_workflow_pairs,
};

fn run_all_to_string(cfg: &SyntheticGenConfig) -> String {
    let mut buf = Vec::new();
    generate_tool_pairs(&mut buf, TOOL_REGISTRY_SLIM, cfg).unwrap();
    generate_a2a_pairs(&mut buf, cfg).unwrap();
    generate_workflow_pairs(&mut buf, cfg).unwrap();
    generate_orchestrator_pairs(&mut buf, cfg).unwrap();
    generate_skill_pairs(&mut buf, cfg).unwrap();
    generate_agent_pairs(&mut buf, cfg).unwrap();
    String::from_utf8(buf).unwrap()
}

fn default_cfg() -> SyntheticGenConfig {
    SyntheticGenConfig::default()
}

#[test]
fn all_registry_tools_appear_in_output() {
    let cfg = SyntheticGenConfig::default();
    let out = run_all_to_string(&cfg);
    for &name in TOOL_REGISTRY_SLIM {
        assert!(
            out.contains(name),
            "tool {name} missing from synthetic output"
        );
    }
}

#[test]
fn all_a2a_types_appear_in_output() {
    let cfg = SyntheticGenConfig::default();
    let out = run_all_to_string(&cfg);
    for &msg_type in A2A_MESSAGE_TYPES {
        assert!(
            out.contains(msg_type),
            "A2A type {msg_type} missing from synthetic output"
        );
    }
}

#[test]
fn output_is_valid_jsonl() {
    let cfg = SyntheticGenConfig::default();
    let out = run_all_to_string(&cfg);
    let mut valid = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("invalid JSON line: {e}\nLine: {line}"));
        assert!(v.get("prompt").is_some(), "missing prompt field");
        assert!(v.get("response").is_some(), "missing response field");
        assert!(v.get("category").is_some(), "missing category field");
        valid += 1;
    }
    assert!(valid > 100, "expected many pairs, got {valid}");
}

#[test]
fn all_workflow_scenarios_appear_in_output() {
    let out = run_all_to_string(&default_cfg());
    let yaml = include_str!("../../../../mens/config/templates.yaml");
    let cfg: serde_json::Value = serde_yaml::from_str(yaml).unwrap();
    let workflows = cfg
        .get("synthetic")
        .unwrap()
        .get("workflows")
        .unwrap()
        .as_array()
        .unwrap();
    for w in workflows {
        let name = w.get("name").unwrap().as_str().unwrap();
        assert!(
            out.contains(name),
            "workflow {name} missing from synthetic output"
        );
    }
}

#[test]
fn all_agent_scenarios_appear_in_output() {
    let out = run_all_to_string(&default_cfg());
    let yaml = include_str!("../../../../mens/config/templates.yaml");
    let cfg: serde_json::Value = serde_yaml::from_str(yaml).unwrap();
    let agents = cfg
        .get("synthetic")
        .unwrap()
        .get("agents")
        .unwrap()
        .as_array()
        .unwrap();
    for a in agents {
        let name = a.get("name").unwrap().as_str().unwrap();
        assert!(
            out.contains(name),
            "agent {name} missing from synthetic output"
        );
    }
}

#[test]
fn min_phrasings_respected() {
    let cfg = SyntheticGenConfig {
        min_phrasings_per_tool: 10,
        ..Default::default()
    };
    let mut buf = Vec::new();
    generate_tool_pairs(&mut buf, &["vox_submit_task"], &cfg).unwrap();
    let out = String::from_utf8(buf).unwrap();
    let count = out.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(count >= 10, "expected ≥10 phrasings, got {count}");
}

#[test]
fn skill_tools_all_covered() {
    let cfg = SyntheticGenConfig::default();
    let out = run_all_to_string(&cfg);
    for &tool in SKILL_TOOLS {
        assert!(out.contains(tool), "skill tool {tool} missing from output");
    }
}

#[test]
fn tool_registry_slim_matches_orchestrator_tools() {
    // Every entry in ORCHESTRATOR_TOOLS must appear in TOOL_REGISTRY_SLIM
    for &name in ORCHESTRATOR_TOOLS {
        assert!(
            TOOL_REGISTRY_SLIM.contains(&name),
            "ORCHESTRATOR_TOOLS entry {name} not in TOOL_REGISTRY_SLIM"
        );
    }
}
