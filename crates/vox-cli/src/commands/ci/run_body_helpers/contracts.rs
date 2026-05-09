use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(serde::Serialize)]
struct ContractRow {
    id: String,
    canonical_env: String,
    aliases: Vec<String>,
    deprecated_aliases: Vec<String>,
    class: String,
    material_kind: String,
    capabilities: Vec<String>,
}

#[derive(serde::Serialize)]
struct EnvNamesManifest {
    schema: &'static str,
    generated_at_ms: i64,
    generator: &'static str,
    secrets: Vec<ContractRow>,
    operator_tuning_envs: Vec<String>,
}

#[derive(serde::Serialize)]
struct CapabilityRow {
    id: String,
    canonical_env: String,
    aliases: Vec<String>,
    class: String,
    auth_registry: Option<String>,
    capabilities: Vec<String>,
    bundles: Vec<String>,
}

#[derive(serde::Serialize)]
struct CapabilitiesManifest {
    schema: &'static str,
    generated_at_ms: i64,
    secrets: Vec<CapabilityRow>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn run_secrets_contracts(root: &Path) -> Result<()> {
    let mut ms: std::collections::BTreeMap<vox_secrets::SecretId, Vec<&'static str>> =
        std::collections::BTreeMap::new();
    for spec in vox_secrets::all_specs() {
        ms.insert(spec.id, Vec::new());
    }

    for &b in vox_secrets::SecretBundle::variants() {
        let reqs = vox_secrets::requirements_for_bundle(b);
        let b_name = b.doc_name();
        let mut ids = std::collections::BTreeSet::new();
        for r in &reqs.blocking {
            match r {
                vox_secrets::RequirementSet::AllOf(list)
                | vox_secrets::RequirementSet::AnyOf(list) => {
                    for &id in *list {
                        ids.insert(id);
                    }
                }
            }
        }
        for &id in &reqs.optional {
            ids.insert(id);
        }
        for id in ids {
            if let Some(list) = ms.get_mut(&id) {
                list.push(b_name);
            }
        }
    }

    let specs: Vec<_> = vox_secrets::all_specs()
        .iter()
        .map(|s| ContractRow {
            id: format!("{:?}", s.id),
            canonical_env: s.canonical_env.to_string(),
            aliases: s.aliases.iter().map(|a| a.to_string()).collect(),
            deprecated_aliases: s.deprecated_aliases.iter().map(|a| a.to_string()).collect(),
            class: format!("{:?}", s.id.metadata().class),
            material_kind: format!("{:?}", s.id.metadata().material_kind),
            capabilities: vox_secrets::capabilities_for_secret(s.id)
                .iter()
                .map(|c| format!("{c:?}"))
                .collect(),
        })
        .collect();

    let cap_rows: Vec<_> = vox_secrets::all_specs()
        .iter()
        .map(|s| CapabilityRow {
            id: format!("{:?}", s.id),
            canonical_env: s.canonical_env.to_string(),
            aliases: s.aliases.iter().map(|a| a.to_string()).collect(),
            class: format!("{:?}", s.id.metadata().class),
            auth_registry: s.auth_registry.map(|x| x.to_string()),
            capabilities: vox_secrets::capabilities_for_secret(s.id)
                .iter()
                .map(|c| format!("{c:?}"))
                .collect(),
            bundles: ms
                .get(&s.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
        })
        .collect();

    let mut all_operator_envs: std::collections::BTreeSet<String> =
        vox_secrets::OPERATOR_TUNING_ENVS
            .iter()
            .map(|s| s.to_string())
            .collect();
    all_operator_envs.extend(
        vox_config::operator_registry::all_operator_env_names()
            .into_iter()
            .map(|s| s.to_string()),
    );

    let manifest = EnvNamesManifest {
        schema: "contracts/secrets/managed-env-names.v1.json",
        generated_at_ms: now_ms(),
        generator: "vox ci secrets-contracts",
        secrets: specs,
        operator_tuning_envs: all_operator_envs.into_iter().collect(),
    };

    let cap_manifest = CapabilitiesManifest {
        schema: "contracts/secrets/secret-capabilities.v1.json",
        generated_at_ms: now_ms(),
        secrets: cap_rows,
    };

    let out = root.join("contracts/secrets/managed-env-names.v1.json");
    if let Some(p) = out.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(&out, serde_json::to_string_pretty(&manifest)?)?;

    let cap_out = root.join("contracts/secrets/secret-capabilities.v1.json");
    fs::write(&cap_out, serde_json::to_string_pretty(&cap_manifest)?)?;

    let mut md_lines = Vec::new();
    let mut names: Vec<String> = vox_secrets::managed_secret_env_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    names.sort();
    names.dedup();
    for name in names {
        let is_deprecated = vox_secrets::all_specs()
            .iter()
            .any(|s| s.deprecated_aliases.contains(&name.as_str()));
        if is_deprecated {
            let canon = vox_secrets::all_specs()
                .iter()
                .find(|s| s.deprecated_aliases.contains(&name.as_str()))
                .unwrap()
                .canonical_env;
            md_lines.push(format!("- `{name}` *(DEPRECATED — use {canon})*"));
        } else {
            md_lines.push(format!("- `{name}`"));
        }
    }

    md_lines.push("\n### Operator Tuning Variables (Non-Secrets)\n".to_string());
    for name in manifest.operator_tuning_envs {
        md_lines.push(format!("- `{name}`"));
    }
    let md_out = root.join("contracts/secrets/managed-env-names.md");
    fs::write(&md_out, md_lines.join("\n") + "\n")?;

    println!("secrets-contracts OK: {} secrets", manifest.secrets.len());
    Ok(())
}
