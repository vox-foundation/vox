//! CI gate: every MCP operation in the operations catalog carries capability-profile exposure
//! metadata (`mcp.exposure`, `mcp.requires`, non-null `side_effect_class`).

use anyhow::{Result, anyhow};

use super::operations_catalog::{OPERATIONS_CATALOG_REL, OperationsCatalog};

/// One line per problem, prefixed with the op id and MCP tool name, so one run lists every gap.
pub(crate) fn mcp_classification_errors(catalog: &OperationsCatalog) -> Vec<String> {
    let mut out = Vec::new();
    for row in &catalog.operations {
        let Some(m) = &row.mcp else {
            continue;
        };
        if let Err(errs) = vox_mcp_registry::classify_tool_row(
            m.http_read_role_eligible,
            m.exposure.as_deref(),
            row.side_effect_class.as_deref(),
            m.requires.as_deref(),
        ) {
            for e in errs {
                out.push(format!("op `{}` (mcp `{}`): {e}", row.id, m.name));
            }
        }
    }
    out
}

/// Fails with every classification problem when any MCP op is unclassified or invalid.
pub(crate) fn ensure_mcp_classified(catalog: &OperationsCatalog) -> Result<()> {
    let errors = mcp_classification_errors(catalog);
    if errors.is_empty() {
        return Ok(());
    }
    Err(anyhow!(
        "{} MCP exposure-metadata problem(s) in {OPERATIONS_CATALOG_REL} (each `mcp:` op needs `mcp.exposure`, `mcp.requires`, and a non-null `side_effect_class`; vocabulary: vox_mcp_registry::{{Exposure, SideEffectClass, Subsystem}}):\n{}",
        errors.len(),
        errors.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ci::operations_catalog::{CliProjection, McpProjection, OperationRow};

    fn mcp(
        name: &str,
        read: bool,
        exposure: Option<&str>,
        requires: Option<&[&str]>,
    ) -> Option<McpProjection> {
        Some(McpProjection {
            name: name.to_string(),
            http_read_role_eligible: read,
            exposure: exposure.map(str::to_string),
            requires: requires.map(|r| r.iter().map(|s| s.to_string()).collect()),
        })
    }

    fn row(id: &str, side_effect_class: Option<&str>, mcp: Option<McpProjection>) -> OperationRow {
        OperationRow {
            id: id.to_string(),
            title: String::new(),
            description: String::new(),
            description_human: None,
            owner_crate: None,
            domain: None,
            product_lane: "ai".to_string(),
            intent_tags: Vec::new(),
            side_effect_class: side_effect_class.map(str::to_string),
            scope_kind: None,
            reversible: None,
            requires_repo: None,
            preferred_for_models: None,
            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: None,
            latin_aliases: None,
            cli: mcp.is_none().then(|| CliProjection {
                path: vec![id.to_string()],
                status: "active".to_string(),
                latin_ns: None,
                handler_rust: None,
                feature_gate: None,
                catalog_group: None,
                ref_cli_required: None,
                reachability_required: None,
            }),
            mcp,
        }
    }

    fn catalog(operations: Vec<OperationRow>) -> OperationsCatalog {
        OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            operations,
            exemptions: Default::default(),
        }
    }

    #[test]
    fn classification_gate_lists_every_missing_field() {
        let cat = catalog(vec![row(
            "x.unclassified",
            None,
            mcp("vox_x_unclassified", false, None, None),
        )]);
        let errs = mcp_classification_errors(&cat);
        assert_eq!(errs.len(), 3, "{errs:?}");
        assert!(
            errs.iter()
                .all(|e| e.starts_with("op `x.unclassified` (mcp `vox_x_unclassified`): ")),
            "{errs:?}"
        );
        assert!(errs.iter().any(|e| e.contains("missing `exposure`")));
        assert!(
            errs.iter()
                .any(|e| e.contains("missing `side_effect_class`"))
        );
        assert!(errs.iter().any(|e| e.contains("missing `requires`")));
    }

    #[test]
    fn classification_gate_rejects_unknown_values() {
        let cat = catalog(vec![row(
            "x.bad",
            Some("mixed"),
            mcp("vox_x_bad", false, Some("core"), Some(&["orchestrator"])),
        )]);
        assert_eq!(mcp_classification_errors(&cat).len(), 3);
    }

    #[test]
    fn classification_gate_enforces_read_role_invariant() {
        let cat = catalog(vec![row(
            "x.leaky",
            Some("writes_files"),
            mcp("vox_x_leaky", true, Some("public"), Some(&[])),
        )]);
        let errs = mcp_classification_errors(&cat);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].contains("http_read_role_eligible"), "{errs:?}");
    }

    #[test]
    fn classification_gate_ignores_cli_only_rows_and_accepts_classified_rows() {
        let cat = catalog(vec![
            row("cli.only", None, None),
            row(
                "x.ok",
                Some("read_only"),
                mcp("vox_x_ok", true, Some("public"), Some(&["db"])),
            ),
            row(
                "x.op",
                Some("writes_files"),
                mcp("vox_x_op", false, Some("operator"), Some(&[])),
            ),
        ]);
        assert!(mcp_classification_errors(&cat).is_empty());
        assert!(ensure_mcp_classified(&cat).is_ok());
    }

    #[test]
    fn ensure_reports_problem_count_and_catalog_path() {
        let cat = catalog(vec![row(
            "x.half",
            None,
            mcp("vox_x_half", false, Some("public"), None),
        )]);
        let err = ensure_mcp_classified(&cat).unwrap_err().to_string();
        assert!(
            err.starts_with(
                "2 MCP exposure-metadata problem(s) in contracts/operations/catalog.v1.yaml"
            ),
            "{err}"
        );
        assert!(err.contains("op `x.half` (mcp `vox_x_half`): missing `side_effect_class`"));
        assert!(err.contains("op `x.half` (mcp `vox_x_half`): missing `requires`"));
    }
}
