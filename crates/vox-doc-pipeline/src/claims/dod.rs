//! Docs Definition-of-Done (DoD) checker (DRIFT-07).
//! Deterministically verifies that changes to public CLI commands, env vars, or contracts
//! include corresponding documentation or generator updates.

use serde::{Deserialize, Serialize};

/// Deterministic warning emitted when a PR violates Docs Definition of Done.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DodWarning {
    pub rule_id: String,
    pub trigger_file: String,
    pub expected_updates: Vec<String>,
    pub message: String,
}

/// Check changed files against the Docs Definition of Done rules.
pub fn check_definition_of_done(changed_files: &[String]) -> Vec<DodWarning> {
    let mut warnings = Vec::new();
    let norm_files: Vec<String> = changed_files
        .iter()
        .map(|f| f.trim().replace('\\', "/"))
        .collect();

    // Rule 1: contracts/cli/command-registry.yaml
    let touches_cmd_registry = norm_files
        .iter()
        .any(|f| f == "contracts/cli/command-registry.yaml");

    if touches_cmd_registry {
        let has_doc_update = norm_files.iter().any(|f| {
            f.contains("docs/src/reference/cli-command-surface.generated.md")
                || f.contains("docs/src/reference/cli.md")
                || f.starts_with("docs/src/reference/commands/")
                || f.starts_with("docs/src/ref/commands/")
                || f.contains("crates/vox-cli/src/commands/ci/command_sync.rs")
        });

        if !has_doc_update {
            warnings.push(DodWarning {
                rule_id: "DOD-CLI-01".to_string(),
                trigger_file: "contracts/cli/command-registry.yaml".to_string(),
                expected_updates: vec![
                    "docs/src/reference/cli-command-surface.generated.md".to_string(),
                    "docs/src/reference/cli.md".to_string(),
                ],
                message: "CLI command registry was modified, but no CLI documentation or generator was updated. Run `cargo run -p vox-cli -- ci command-sync` or update command reference docs.".to_string(),
            });
        }
    }

    // Rule 2: contracts/config/env-vars.v1.yaml
    let touches_env_registry = norm_files
        .iter()
        .any(|f| f == "contracts/config/env-vars.v1.yaml");

    if touches_env_registry {
        let has_doc_update = norm_files.iter().any(|f| {
            f.contains("docs/src/reference/env-vars.md")
                || f.contains("docs/src/ref/env-vars.md")
                || f.contains("docs/src/reference/configuration.md")
        });

        if !has_doc_update {
            warnings.push(DodWarning {
                rule_id: "DOD-ENV-01".to_string(),
                trigger_file: "contracts/config/env-vars.v1.yaml".to_string(),
                expected_updates: vec![
                    "docs/src/reference/env-vars.md".to_string(),
                    "docs/src/ref/env-vars.md".to_string(),
                ],
                message: "Environment variable registry was modified, but no environment variable reference documentation was updated.".to_string(),
            });
        }
    }

    // Rule 3: Public contracts in contracts/ without corresponding docs or ADRs
    for file in &norm_files {
        if file.starts_with("contracts/")
            && (file.ends_with(".yaml") || file.ends_with(".json") || file.ends_with(".toml"))
            && !file.starts_with("contracts/reports/")
            && file != "contracts/cli/command-registry.yaml"
            && file != "contracts/config/env-vars.v1.yaml"
        {
            let has_any_doc_update = norm_files.iter().any(|f| f.starts_with("docs/"));
            if !has_any_doc_update {
                warnings.push(DodWarning {
                    rule_id: "DOD-CONTRACT-01".to_string(),
                    trigger_file: file.clone(),
                    expected_updates: vec!["docs/src/reference/".to_string(), "docs/src/adr/".to_string()],
                    message: format!(
                        "Public contract '{}' was modified without any accompanying documentation update under docs/.",
                        file
                    ),
                });
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dod_cli_registry_requires_doc_update() {
        // Without doc update -> triggers warning
        let files = vec!["contracts/cli/command-registry.yaml".to_string()];
        let warnings = check_definition_of_done(&files);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule_id, "DOD-CLI-01");

        // With command surface sync -> ok
        let files_with_doc = vec![
            "contracts/cli/command-registry.yaml".to_string(),
            "docs/src/reference/cli-command-surface.generated.md".to_string(),
        ];
        let warnings_ok = check_definition_of_done(&files_with_doc);
        assert!(warnings_ok.is_empty());
    }

    #[test]
    fn test_dod_env_vars_requires_doc_update() {
        let files = vec!["contracts/config/env-vars.v1.yaml".to_string()];
        let warnings = check_definition_of_done(&files);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule_id, "DOD-ENV-01");

        let files_with_doc = vec![
            "contracts/config/env-vars.v1.yaml".to_string(),
            "docs/src/reference/env-vars.md".to_string(),
        ];
        let warnings_ok = check_definition_of_done(&files_with_doc);
        assert!(warnings_ok.is_empty());
    }

    #[test]
    fn test_dod_public_contracts_require_docs() {
        let files = vec!["contracts/terminal/exec-policy.v1.yaml".to_string()];
        let warnings = check_definition_of_done(&files);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule_id, "DOD-CONTRACT-01");

        let files_with_doc = vec![
            "contracts/terminal/exec-policy.v1.yaml".to_string(),
            "docs/src/reference/terminal.md".to_string(),
        ];
        let warnings_ok = check_definition_of_done(&files_with_doc);
        assert!(warnings_ok.is_empty());
    }
}
