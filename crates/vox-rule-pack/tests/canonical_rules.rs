//! Locks the canonical rules.v1.yaml against the current vox-rule-pack schema.
//!
//! If this test fails, either the YAML is malformed or a required rule was removed.

use std::path::PathBuf;
use vox_rule_pack::RulePack;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // crates/vox-rule-pack → workspace root
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn canonical_rules_yaml_parses() {
    let path = workspace_root()
        .join("contracts")
        .join("code-audit")
        .join("rules.v1.yaml");
    let pack = RulePack::load_from_path(&path)
        .unwrap_or_else(|e| panic!("rules.v1.yaml must parse: {e}"));
    assert!(
        pack.len() >= 43,
        "expected at least 43 rules (29 detector rules + 14 scaling metadata stubs), got {}",
        pack.len()
    );
    for needed in [
        "victory-claim/premature",
        "victory-claim/todo-leftover",
        "victory-claim/fixme",
        "victory-claim/hack",
        "ai-laziness/placeholder-return",
        "ai-laziness/conditional-stub",
        "stub/todo",
        "stub/unimplemented",
        "stub/placeholder",
        "deprecated-usage",
        "raw-jsx-leakage",
        "rust/unwrap-call",
        "stringly-typed-enum",
        "security/hardcoded-secret/generic",
        "security/hardcoded-secret/aws-key",
        "magic-value/port",
        "magic-value/db-conn",
    ] {
        assert!(
            pack.rule(needed).is_some(),
            "required rule '{needed}' must exist in rules.v1.yaml"
        );
    }
}

#[test]
fn all_rules_have_nonempty_patterns() {
    let path = workspace_root()
        .join("contracts")
        .join("code-audit")
        .join("rules.v1.yaml");
    let pack = RulePack::load_from_path(&path).unwrap();
    for rule in pack.rules() {
        assert!(
            !rule.message.is_empty(),
            "rule '{}' must have a non-empty message",
            rule.id
        );
        assert!(
            !rule.languages.is_empty(),
            "rule '{}' must apply to at least one language",
            rule.id
        );
    }
}
