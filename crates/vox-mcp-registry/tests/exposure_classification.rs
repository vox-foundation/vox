//! Exposure-classification invariants over the generated `TOOL_REGISTRY`.

use std::collections::{BTreeSet, HashSet};

use vox_mcp_registry::{
    Exposure, SideEffectClass, Subsystem, TOOL_REGISTRY, classify_tool_row, registry_entry,
};

/// Spec §3 initial `internal` set.
const SPEC_INTERNAL_PREFIXES: &[&str] = &["vox_agy_", "vox_openclaw_", "vox_gamify_"];
const SPEC_INTERNAL_SUFFIXES: &[&str] = &["_gateway_call"];
const SPEC_INTERNAL_EXACT: &[&str] = &[
    "vox_add_approval_allowlist_entry",
    "vox_emergency_stop",
    "vox_drain_agent",
    "vox_secrets_doctor",
    "vox_credentials_status",
];
/// Internal beyond the spec list: the native agent-gateway family, browser cookie transfer, and
/// the Axis `gui.*` preference ops (loopback-only by design).
const RECORDED_INTERNAL_EXTENSIONS: &[&str] = &[
    "vox_agent_list_remote",
    "vox_agent_notify",
    "vox_agent_subscribe",
    "vox_agent_subscriptions",
    "vox_agent_unsubscribe",
    "vox_browser_cookies_export",
    "vox_browser_cookies_import",
    "vox_gui_preference_get",
    "vox_gui_preference_set",
];

fn in_spec_internal_set(name: &str) -> bool {
    SPEC_INTERNAL_PREFIXES.iter().any(|p| name.starts_with(p))
        || SPEC_INTERNAL_SUFFIXES.iter().any(|s| name.ends_with(s))
        || SPEC_INTERNAL_EXACT.contains(&name)
}

#[test]
fn spec_internal_set_is_internal() {
    let matched: Vec<_> = TOOL_REGISTRY
        .iter()
        .filter(|e| in_spec_internal_set(e.name))
        .collect();
    assert!(
        matched.len() >= 32,
        "spec §3 patterns matched only {} tools",
        matched.len()
    );
    for e in matched {
        assert_eq!(
            e.exposure,
            Exposure::Internal,
            "{} must be internal",
            e.name
        );
    }
}

#[test]
fn internal_tools_are_exactly_spec_set_plus_recorded_extensions() {
    let actual: BTreeSet<&str> = TOOL_REGISTRY
        .iter()
        .filter(|e| e.exposure == Exposure::Internal)
        .map(|e| e.name)
        .collect();
    let expected: BTreeSet<&str> = TOOL_REGISTRY
        .iter()
        .map(|e| e.name)
        .filter(|n| in_spec_internal_set(n) || RECORDED_INTERNAL_EXTENSIONS.contains(n))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn classification_is_populated_not_defaulted() {
    let exposures: BTreeSet<Exposure> = TOOL_REGISTRY.iter().map(|e| e.exposure).collect();
    assert_eq!(
        exposures.len(),
        3,
        "all three exposure classes must be in use"
    );
    let side_effects: HashSet<SideEffectClass> =
        TOOL_REGISTRY.iter().map(|e| e.side_effect_class).collect();
    assert!(
        side_effects.len() >= 5,
        "only {} side-effect classes in use",
        side_effects.len()
    );
    let with_requires = TOOL_REGISTRY
        .iter()
        .filter(|e| !e.requires.is_empty())
        .count();
    assert!(
        with_requires * 3 >= TOOL_REGISTRY.len(),
        "only {with_requires} of {} tools declare requires",
        TOOL_REGISTRY.len()
    );
    for e in TOOL_REGISTRY {
        let req: Vec<String> = e.requires.iter().map(|s| s.as_str().to_string()).collect();
        classify_tool_row(
            e.http_read_role_eligible,
            Some(e.exposure.as_str()),
            Some(e.side_effect_class.as_str()),
            Some(req.as_slice()),
        )
        .unwrap_or_else(|errs| panic!("{}: {errs:?}", e.name));
    }
}

#[test]
fn anchor_tools_carry_expected_classification() {
    let cases: &[(&str, Exposure, SideEffectClass, &[Subsystem])] = &[
        (
            "vox_git_status",
            Exposure::Public,
            SideEffectClass::ReadOnly,
            &[],
        ),
        (
            "vox_list_models",
            Exposure::Public,
            SideEffectClass::ReadOnly,
            &[Subsystem::ModelRegistry],
        ),
        (
            "vox_kb_query",
            Exposure::Public,
            SideEffectClass::ReadOnly,
            &[Subsystem::SearchIndex],
        ),
        (
            "vox_snapshot_restore",
            Exposure::Operator,
            SideEffectClass::Destructive,
            &[Subsystem::Db],
        ),
        (
            "vox_browser_open",
            Exposure::Operator,
            SideEffectClass::Network,
            &[Subsystem::Browser],
        ),
        (
            "vox_agy_delegate",
            Exposure::Internal,
            SideEffectClass::SpawnsProcess,
            &[],
        ),
    ];
    for &(name, exposure, side_effect_class, requires) in cases {
        let e = registry_entry(name).unwrap_or_else(|| panic!("{name} missing"));
        assert_eq!(e.exposure, exposure, "{name}");
        assert_eq!(e.side_effect_class, side_effect_class, "{name}");
        assert_eq!(e.requires, requires, "{name}");
    }
}
