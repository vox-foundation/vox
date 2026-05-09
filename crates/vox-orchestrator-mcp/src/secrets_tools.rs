use crate::params::ToolResult;
use crate::server_state::ServerState;

pub async fn secrets_doctor(_state: &ServerState, args: serde_json::Value) -> String {
    let workflow_str = args
        .get("workflow")
        .and_then(|v| v.as_str())
        .unwrap_or("Chat");
    let profile_str = args
        .get("profile")
        .and_then(|v| v.as_str())
        .unwrap_or("Dev");

    let wf = match workflow_str {
        "Mcp" => vox_secrets::Workflow::Mcp,
        "Publish" => vox_secrets::Workflow::Publish,
        "Review" => vox_secrets::Workflow::Review,
        "DbRemote" => vox_secrets::Workflow::DbRemote,
        "MensMesh" => vox_secrets::Workflow::MensMesh,
        _ => vox_secrets::Workflow::Chat,
    };
    let profile = match profile_str {
        "Ci" => vox_secrets::Profile::Ci,
        "Prod" => vox_secrets::Profile::Prod,
        "Mobile" => vox_secrets::Profile::Mobile,
        _ => vox_secrets::Profile::Dev,
    };

    let mut secrets = Vec::new();

    // Membership mapping
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

    for spec in vox_secrets::all_specs() {
        let resolved = vox_secrets::resolve_secret(spec.id);
        let meta = spec.id.metadata();

        secrets.push(serde_json::json!({
            "id": format!("{:?}", spec.id),
            "canonical_env": spec.canonical_env,
            "status": format!("{:?}", resolved.status),
            "source": format!("{:?}", resolved.source),
            "class": format!("{:?}", meta.class),
            "material_kind": format!("{:?}", meta.material_kind),
            "capabilities": vox_secrets::capabilities_for_secret(spec.id).iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>(),
            "bundle_membership": ms.get(&spec.id).cloned().unwrap_or_default(),
            "is_present": resolved.is_present(),
            "remediation": if resolved.is_present() { None } else { Some(spec.remediation.to_string()) },
        }));
    }

    let report = serde_json::json!({
        "schema": "contracts/reports/secrets-doctor.v1.json",
        "generated_at_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64,
        "backend_mode": format!("{:?}", vox_secrets::BackendMode::from_env()),
        "vault_diagnostic": vox_secrets::backend::vox_vault::cloudless_vault_env_diagnostic(),
        "workflow": format!("{:?}", wf),
        "profile": format!("{:?}", profile),
        "rollout_flags": vox_config::rollout_flag_snapshot(),
        "secrets": secrets,
    });

    ToolResult::ok(report).to_json()
}
