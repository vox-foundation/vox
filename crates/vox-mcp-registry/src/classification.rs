//! Validation of one MCP tool's exposure metadata. Compiled into both the library and
//! `build.rs` (via `#[path]`), and called by `vox ci operations-sync`, so every gate applies
//! the same rules.

use crate::exposure::{Exposure, SideEffectClass};
use crate::subsystem::Subsystem;

/// Parsed, validated exposure metadata of one tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolClassification {
    pub exposure: Exposure,
    pub side_effect_class: SideEffectClass,
    pub requires: Vec<Subsystem>,
}

/// Parse and validate one registry/catalog row. Returns every problem, not just the first.
pub fn classify_tool_row(
    http_read_role_eligible: bool,
    exposure: Option<&str>,
    side_effect_class: Option<&str>,
    requires: Option<&[String]>,
) -> Result<ToolClassification, Vec<String>> {
    let mut errors = Vec::new();

    let exposure = match exposure {
        None => {
            errors.push("missing `exposure` (public|operator|internal)".to_string());
            None
        }
        Some(s) => {
            let parsed = Exposure::parse(s);
            if parsed.is_none() {
                errors.push(format!("unknown exposure `{s}`"));
            }
            parsed
        }
    };

    let side_effect = match side_effect_class {
        None => {
            errors.push(
                "missing `side_effect_class` (none|read_only|writes_files|spawns_process|network|remote_mutation|destructive)"
                    .to_string(),
            );
            None
        }
        Some(s) => {
            let parsed = SideEffectClass::parse(s);
            if parsed.is_none() {
                errors.push(format!("unknown side_effect_class `{s}`"));
            }
            parsed
        }
    };

    let mut subsystems = Vec::new();
    match requires {
        None => errors.push("missing `requires` (use [] when none)".to_string()),
        Some(list) => {
            for s in list {
                match Subsystem::parse(s) {
                    None => errors.push(format!("unknown subsystem `{s}` in requires")),
                    Some(x) if subsystems.contains(&x) => {
                        errors.push(format!("duplicate subsystem `{s}` in requires"));
                    }
                    Some(x) => subsystems.push(x),
                }
            }
        }
    }

    if let (Some(e), Some(c)) = (exposure, side_effect) {
        let side_effect_free = matches!(c, SideEffectClass::None | SideEffectClass::ReadOnly);
        if http_read_role_eligible && (!side_effect_free || e == Exposure::Operator) {
            errors.push(format!(
                "http_read_role_eligible tool must be side-effect free and not operator (got exposure `{}`, side_effect_class `{}`)",
                e.as_str(),
                c.as_str()
            ));
        }
        if e == Exposure::Operator && side_effect_free {
            errors.push(format!(
                "operator exposure requires a mutating side_effect_class (got `{}`)",
                c.as_str()
            ));
        }
        if e == Exposure::Public
            && matches!(
                c,
                SideEffectClass::SpawnsProcess
                    | SideEffectClass::RemoteMutation
                    | SideEffectClass::Destructive
            )
        {
            errors.push(format!(
                "public exposure cannot have side_effect_class `{}`",
                c.as_str()
            ));
        }
    }

    match (exposure, side_effect) {
        (Some(exposure), Some(side_effect_class)) if errors.is_empty() => Ok(ToolClassification {
            exposure,
            side_effect_class,
            requires: subsystems,
        }),
        _ => Err(errors),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_REQS: &[String] = &[];

    fn reqs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn valid_row_parses_in_declared_order() {
        let r = reqs(&["search_index", "db"]);
        let c = classify_tool_row(true, Some("public"), Some("read_only"), Some(r.as_slice()))
            .expect("valid row");
        assert_eq!(
            c,
            ToolClassification {
                exposure: Exposure::Public,
                side_effect_class: SideEffectClass::ReadOnly,
                requires: vec![Subsystem::SearchIndex, Subsystem::Db],
            }
        );
    }

    #[test]
    fn empty_requires_is_valid() {
        let c = classify_tool_row(false, Some("operator"), Some("writes_files"), Some(NO_REQS))
            .expect("valid row");
        assert!(c.requires.is_empty());
    }

    #[test]
    fn every_missing_field_is_reported() {
        let errs = classify_tool_row(false, None, None, None).unwrap_err();
        assert_eq!(errs.len(), 3, "{errs:?}");
        assert!(errs[0].contains("missing `exposure`"), "{errs:?}");
        assert!(errs[1].contains("missing `side_effect_class`"), "{errs:?}");
        assert!(errs[2].contains("missing `requires`"), "{errs:?}");
    }

    #[test]
    fn unknown_values_and_duplicates_are_reported() {
        let r = reqs(&["db", "orchestrator", "db"]);
        let errs =
            classify_tool_row(false, Some("core"), Some("mixed"), Some(r.as_slice())).unwrap_err();
        assert_eq!(
            errs,
            vec![
                "unknown exposure `core`".to_string(),
                "unknown side_effect_class `mixed`".to_string(),
                "unknown subsystem `orchestrator` in requires".to_string(),
                "duplicate subsystem `db` in requires".to_string(),
            ]
        );
    }

    #[test]
    fn read_role_tools_must_be_side_effect_free_and_not_operator() {
        assert!(
            classify_tool_row(true, Some("public"), Some("writes_files"), Some(NO_REQS)).is_err()
        );
        assert!(
            classify_tool_row(true, Some("internal"), Some("read_only"), Some(NO_REQS)).is_ok()
        );
        assert!(classify_tool_row(true, Some("public"), Some("none"), Some(NO_REQS)).is_ok());
    }

    #[test]
    fn operator_requires_a_mutating_side_effect() {
        let errs = classify_tool_row(false, Some("operator"), Some("read_only"), Some(NO_REQS))
            .unwrap_err();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].starts_with("operator exposure requires"),
            "{errs:?}"
        );
    }

    #[test]
    fn public_cannot_spawn_mutate_remote_or_destroy() {
        for c in ["spawns_process", "remote_mutation", "destructive"] {
            assert!(
                classify_tool_row(false, Some("public"), Some(c), Some(NO_REQS)).is_err(),
                "{c}"
            );
        }
        for c in ["none", "read_only", "writes_files", "network"] {
            assert!(
                classify_tool_row(false, Some("public"), Some(c), Some(NO_REQS)).is_ok(),
                "{c}"
            );
        }
    }
}
