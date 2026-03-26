// ─── Agent construct SFT pairs ────────────────────────────────────────────────

pub(crate) fn generate_agent_pairs(out: &mut impl Write, cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0;
    let prompts = [
        "Define a Vox AI agent called {name} that can {desc}.",
        "Write a Vox @agent_def for {name}.",
        "How do I create an agent named {name} in Vox?",
        "Build a {name} agent in Vox with appropriate tools and memory.",
        "Implement the {name} agent — it should {desc}.",
        "Show the Vox syntax for an agent that {desc_lower}.",
    ];

    for def in &TEMPLATES.agents {
        let name = &def.name;
        let desc = &def.description;
        let snippet = &def.snippet;
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        let desc_lower = desc.to_lowercase();
        for (i, tmpl) in prompts.iter().enumerate() {
            let prompt = tmpl
                .replace("{name}", name)
                .replace("{desc}", desc)
                .replace("{desc_lower}", &desc_lower);
            let response = json!({
                "construct": "agent_def",
                "name": name,
                "description": desc,
                "vox_snippet": snippet,
            });
            let _ = (i, &mut rng);
            emit_line(out, &prompt, &response, "agent_def", "agent_def")?;
            count += 1;
        }
    }
    Ok(count)
}
