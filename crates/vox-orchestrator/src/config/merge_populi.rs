use super::orchestrator_fields::OrchestratorConfig;

pub(crate) fn apply_vox_populi_toml(
    config: &mut OrchestratorConfig,
    mesh: &vox_repository::MeshToml,
) {
    if let Some(url) = mesh
        .control_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        config.populi_control_url = Some(url.to_string());
    }
    if let Some(sid) = mesh
        .scope_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        config.populi_scope_id = Some(sid.to_string());
    }
    if let Some(labels) = mesh.labels.as_ref() {
        for lab in labels {
            let lab = lab.trim();
            if lab.is_empty() {
                continue;
            }
            let s = lab.to_string();
            if !config.default_agent_capabilities.labels.contains(&s) {
                config.default_agent_capabilities.labels.push(s);
            }
        }
    }
    if mesh.advertise_gpu == Some(true) {
        config.default_agent_capabilities.gpu_cuda = true;
    }
}
