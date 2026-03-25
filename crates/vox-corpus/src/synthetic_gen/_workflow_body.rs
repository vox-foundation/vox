// ─── Workflow construct SFT pairs ─────────────────────────────────────────────

pub(crate) fn generate_workflow_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    let prompts = [
        "Implement {desc} as a workflow named {name}.",
        "Show me how to write {desc} in Vox.",
        "Provide a Vox @workflow definition for {name}.",
        "Create a {name} workflow that acts as {desc}.",
        "Write the {name} durable workflow in Vox.",
    ];

    for def in &TEMPLATES.workflows {
        let name = &def.name;
        let desc = &def.description;
        let snippet = &def.snippet;
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        for (j, tmpl) in prompts.iter().enumerate() {
            let prompt = tmpl.replace("{name}", name).replace("{desc}", desc);
            let response = json!({
                "construct": "workflow_def",
                "name": name,
                "description": desc,
                "vox_snippet": snippet,
            });
            let _ = (j, &mut rng); // prevent unused warnings
            emit_line(out, &prompt, &response, "workflow_def", "workflow_trace")?;
            count += 1;
        }
    }
    Ok(count)
}
