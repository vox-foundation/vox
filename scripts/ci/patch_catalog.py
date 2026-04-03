with open("crates/vox-cli/src/commands/ci/operations_catalog.rs", "r") as f:
    text = f.read()

old_struct_tail = """    #[serde(default)]
    pub mens_planner_visible: Option<bool>,
    #[serde(default)]
    pub mcp: Option<McpProjection>,
    #[serde(default)]
    pub cli: Option<CliProjection>,
}"""
new_struct_tail = """    #[serde(default)]
    pub mens_planner_visible: Option<bool>,
    #[serde(default)]
    pub canonical_name: Option<String>,
    #[serde(default)]
    pub latin_aliases: Option<Vec<String>>,
    #[serde(default)]
    pub mcp: Option<McpProjection>,
    #[serde(default)]
    pub cli: Option<CliProjection>,
}"""

old_init_tail = """            human_takeover_friendly: None,
            mens_planner_visible: None,
            mcp: None,
            cli: None,
        });"""
new_init_tail = """            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: None,
            latin_aliases: None,
            mcp: None,
            cli: None,
        });"""

old_verify_block = """    if !seen_cli.contains(path) && !exempt_cli.contains(path) {
            return Err(anyhow!(
                "operations catalog orphan CLI path {:?}; add row or exemption",
                path
            ));
        }
    }
    verify_mcp_dispatch_coverage(repo_root, &seen_mcp)?;"""

new_verify_block = """    if !seen_cli.contains(path) && !exempt_cli.contains(path) {
            return Err(anyhow!(
                "operations catalog orphan CLI path {:?}; add row or exemption",
                path
            ));
        }
    }
    verify_catalog_nomenclature(&catalog)?;
    verify_mcp_dispatch_coverage(repo_root, &seen_mcp)?;"""

test_str = """
fn verify_catalog_nomenclature(catalog: &OperationsCatalog) -> Result<()> {
    let mut alias_to_id = BTreeMap::new();
    for row in &catalog.operations {
        let is_core_canonical = ["orchestrator", "skills", "forge", "database", "secrets", "speech", "ml", "gamification", "tutorial", "pm", "package_manager"]
            .contains(&row.canonical_name.as_deref().unwrap_or(""));
            
        let has_alias = row.latin_aliases.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
        if is_core_canonical && !has_alias {
            return Err(anyhow!("command-compliance T045: English canonical command '{}' must declare at least one Latin alias in `latin_aliases`", row.id));
        }

        if let Some(aliases) = &row.latin_aliases {
            for alias in aliases {
                // Check grammar T053: invalid alias grammar tags
                if !alias.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                    return Err(anyhow!("command-compliance T053: Latin alias '{}' has invalid grammar tag (must be lower-kebab-case)", alias));
                }

                if Some(alias.as_str()) == row.canonical_name.as_deref() {
                    return Err(anyhow!("command-compliance T046: Latin alias '{}' cannot be the canonical structural identifier for '{}'", alias, row.id));
                }
                
                if let Some(existing_id) = alias_to_id.insert(alias.clone(), row.id.clone()) {
                    if existing_id != row.id {
                         return Err(anyhow!("command-compliance T047: Latin alias collision for '{}' between '{}' and '{}'", alias, existing_id, row.id));
                    }
                }
            }
        }
        
        if let Some(c) = &row.cli {
            if c.status == "retired" && has_alias {
                return Err(anyhow!("command-compliance T050: retired command '{}' cannot have Latin aliases", row.id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_row(id: &str, canon: Option<&str>, aliases: Option<Vec<&str>>, status: &str) -> OperationRow {
        OperationRow {
            id: id.to_string(),
            title: "".to_string(),
            description: "".to_string(),
            description_human: None,
            product_lane: "platform".to_string(),
            intent_tags: vec![],
            side_effect_class: None,
            scope_kind: None,
            reversible: None,
            requires_repo: None,
            preferred_for_models: None,
            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: canon.map(|s| s.to_string()),
            latin_aliases: aliases.map(|a| a.into_iter().map(|s| s.to_string()).collect()),
            mcp: None,
            cli: Some(CliProjection {
                path: vec![id.to_string()],
                status: status.to_string(),
                latin_ns: None,
                handler_rust: None,
                feature_gate: None,
                catalog_group: None,
                ref_cli_required: None,
                reachability_required: None,
            }),
        }
    }

    #[test]
    fn test_t045_missing_alias() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("orchestrator", Some("orchestrator"), None, "active"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T045"));
    }

    #[test]
    fn test_t046_alias_cannot_be_canonical() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("test_op", Some("alias_name"), Some(vec!["alias_name"]), "active"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T046"));
    }

    #[test]
    fn test_t047_alias_collision() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("op1", Some("op1"), Some(vec!["shared_alias"]), "active"),
                create_row("op2", Some("op2"), Some(vec!["shared_alias"]), "active"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T047"));
    }

    #[test]
    fn test_t050_retired_command_aliases() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("op1", Some("op1"), Some(vec!["alias"]), "retired"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T050"));
    }

    #[test]
    fn test_t053_invalid_grammar() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("op1", Some("op1"), Some(vec!["UPPERCASE"]), "active"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("T053"));
    }

    #[test]
    fn test_t054_positive_parity_metadata() {
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![
                create_row("dei", Some("orchestrator"), Some(vec!["dei"]), "active"),
            ],
        };
        let res = verify_catalog_nomenclature(&cat);
        assert!(res.is_ok());
    }
}
"""

text = text.replace(old_struct_tail, new_struct_tail)
text = text.replace(old_init_tail, new_init_tail)
text = text.replace(old_verify_block, new_verify_block)

base_text = text.strip()

with open("crates/vox-cli/src/commands/ci/operations_catalog.rs", "w") as f:
    f.write(base_text + "\n" + test_str.strip() + "\n")
