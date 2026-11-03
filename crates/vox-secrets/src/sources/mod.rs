//! Secret sources (env, auth.json compatibility, Populi mesh env, external secret managers).

pub mod auth_json;
pub mod env;
pub mod populi_env;

use crate::spec::{
    Profile, ProviderManifestEntry, RequirementSet, SecretId, Workflow, all_specs,
    provider_manifest_for, requirements_for_profile,
};

/// Supported external secret manager import sources (`CLAVIS-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalSecretManager {
    Dotenv,
    OnePassword,
    Doppler,
    Bitwarden,
    Infisical,
}

impl ExternalSecretManager {
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Dotenv => "dotenv",
            Self::OnePassword => "1password",
            Self::Doppler => "doppler",
            Self::Bitwarden => "bitwarden",
            Self::Infisical => "infisical",
        }
    }
}

/// Parse exported key-value pairs from an external secret manager (`dotenv`, `1password` op:// JSON,
/// `doppler` JSON/env, `bitwarden` JSON, `infisical` JSON) and filter to recognized Clavis [`SecretId`]s.
///
/// Returns `(SecretId, source_key_name, value)` tuples for recognized managed secrets.
#[must_use]
pub fn parse_external_manager_export(
    _manager: ExternalSecretManager,
    payload: &str,
) -> Vec<(SecretId, String, String)> {
    let specs = all_specs();
    let match_spec = |key: &str| {
        specs
            .iter()
            .find(|s| {
                s.canonical_env == key
                    || s.aliases.contains(&key)
                    || s.deprecated_aliases.contains(&key)
            })
            .map(|s| s.id)
    };

    let trimmed = payload.trim();
    let mut out = Vec::new();

    // 1. Try JSON object (`{"OPENROUTER_API_KEY": "..."}` or `{"fields": [{"label": "...", "value": "..."}]}`)
    if trimmed.starts_with('{')
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed)
        && let Some(obj) = val.as_object()
    {
        if let Some(fields) = obj.get("fields").and_then(|f| f.as_array()) {
            for field in fields {
                let k = field
                    .get("label")
                    .or_else(|| field.get("id"))
                    .or_else(|| field.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let v = field.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if !k.is_empty()
                    && !v.is_empty()
                    && let Some(id) = match_spec(k)
                {
                    out.push((id, k.to_string(), v.to_string()));
                }
            }
            return out;
        }
        for (k, v) in obj {
            let str_val = v
                .as_str()
                .or_else(|| v.get("computed").and_then(|c| c.as_str()))
                .unwrap_or("");
            if !str_val.is_empty()
                && let Some(id) = match_spec(k)
            {
                out.push((id, k.clone(), str_val.to_string()));
            }
        }
        return out;
    }

    // 2. Fallback: KEY=VALUE lines (dotenv / doppler / infisical env format)
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
        if !v.is_empty()
            && let Some(id) = match_spec(k)
        {
            out.push((id, k.to_string(), v.to_string()));
        }
    }
    out
}

/// Compute missing-key acquisition recipes for a workflow and profile (`CLAVIS-04`).
#[must_use]
pub fn compute_missing_key_recipes(
    workflow: Workflow,
    profile: Profile,
) -> Vec<ProviderManifestEntry> {
    let reqs = requirements_for_profile(workflow, profile);
    let mut missing = Vec::new();
    for set in reqs.blocking {
        match set {
            RequirementSet::AllOf(ids) => {
                for id in ids {
                    if !crate::resolve_secret(*id).is_present() {
                        missing.push(provider_manifest_for(*id));
                    }
                }
            }
            RequirementSet::AnyOf(ids) => {
                if !ids.iter().any(|id| crate::resolve_secret(*id).is_present()) {
                    for id in ids {
                        missing.push(provider_manifest_for(*id));
                    }
                }
            }
        }
    }
    missing
}

#[cfg(test)]
mod external_import_tests {
    use super::*;

    #[test]
    fn parses_doppler_and_1password_json_exports() {
        let doppler_json =
            r#"{"OPENROUTER_API_KEY": {"computed": "sk-or-v1-test123456"}, "UNMANAGED": "ignore"}"#;
        let parsed = parse_external_manager_export(ExternalSecretManager::Doppler, doppler_json);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, SecretId::OpenRouterApiKey);
        assert_eq!(parsed[0].2, "sk-or-v1-test123456");

        let op_json =
            r#"{"fields": [{"label": "GEMINI_API_KEY", "value": "AIzaSyTestKey123456"}]}"#;
        let parsed_op = parse_external_manager_export(ExternalSecretManager::OnePassword, op_json);
        assert_eq!(parsed_op.len(), 1);
        assert_eq!(parsed_op[0].0, SecretId::GeminiApiKey);
    }

    #[test]
    fn compute_missing_key_recipes_returns_manifest_entries_with_urls() {
        let recipes = compute_missing_key_recipes(Workflow::Publish, Profile::Ci);
        // Either ForgeToken is present in the test env or it's listed with its GitHub acquisition URL.
        if let Some(first) = recipes.first() {
            assert_eq!(first.secret_id, SecretId::ForgeToken);
            assert_eq!(
                first.acquisition_url,
                Some("https://github.com/settings/tokens")
            );
        }
    }
}
