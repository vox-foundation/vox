//! Compile-time-embedded copy of contracts/code-audit/rules.v1.yaml,
//! exposed as a process-wide singleton RulePack.

use std::sync::OnceLock;
use vox_rule_pack::RulePack;

const EMBEDDED_YAML: &str = include_str!("../../../contracts/code-audit/rules.v1.yaml");

static PACK: OnceLock<RulePack> = OnceLock::new();

/// Returns the process-wide compiled rule pack.
///
/// # Panics
///
/// Panics on the first call if the embedded YAML is malformed. This indicates
/// a build-time invariant violation and is intentionally fatal.
pub fn embedded_pack() -> &'static RulePack {
    PACK.get_or_init(|| {
        RulePack::load_from_str(EMBEDDED_YAML)
            .expect("embedded contracts/code-audit/rules.v1.yaml must parse")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_pack_loads() {
        let pack = embedded_pack();
        assert!(pack.len() >= 4, "expected at least 4 rules, got {}", pack.len());
    }

    #[test]
    fn victory_claim_rules_present() {
        let pack = embedded_pack();
        for id in [
            "victory-claim/premature",
            "victory-claim/todo-leftover",
            "victory-claim/fixme",
            "victory-claim/hack",
        ] {
            assert!(
                pack.rule(id).is_some(),
                "embedded pack missing required rule: {id}"
            );
        }
    }
}
