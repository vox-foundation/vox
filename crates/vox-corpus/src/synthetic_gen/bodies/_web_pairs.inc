// ─── Web construct SFT pairs ──────────────────────────────────────────────────

const WEB_CONSTRUCTS: &[(&str, &str, &str)] = &[
    (
        "component",
        "Navbar",
        "component Navbar() {\n  view: <nav><ul><li>Home</li></ul></nav>\n}",
    ),
    (
        "island",
        "Counter",
        "component Counter() {\n  state count: int = 0\n  view: <button on:click={|| count = count + 1}>{count}</button>\n}",
    ),
    (
        "page",
        "Dashboard",
        "@route(\"/dash\")\ncomponent Dashboard() {\n  view: <main><h1>Dashboard</h1><Counter /></main>\n}",
    ),
    (
        "@query",
        "get_user",
        "@query\nfn get_user(id: int) -> Option[User] {\n  ret VoxDb::query(\"SELECT * FROM users WHERE id = ?1\", [id]).first()\n}",
    ),
    (
        "@mutation",
        "update_user",
        "@mutation\nfn update_user(id: int, name: str) -> Result[Unit] {\n  VoxDb::execute(\"UPDATE users SET name = ?2 WHERE id = ?1\", [id, name])\n  ret Ok(())\n}",
    ),
    (
        "@action",
        "submit_form",
        "@action\nfn submit_form(data: FormData) -> Result[Unit] {\n  // form logic\n  ret Ok(())\n}",
    ),
    (
        "@server",
        "generate_pdf",
        "@server\nfn generate_pdf(report: str) -> bytes {\n  // server side only logic\n  ret b\"...\"\n}",
    ),
];

pub(crate) fn generate_web_construct_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;
    let prompts = [
        "Implement a {name} {construct} in Vox.",
        "Show me how to write a {construct} named {name}.",
        "Provide a Vox {construct} definition for {name}.",
        "Create a {name} that acts as a {construct}.",
        "Write the {name} {construct} in Vox syntax.",
    ];

    for (construct, name, snippet) in WEB_CONSTRUCTS {
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        for (j, tmpl) in prompts.iter().enumerate() {
            let prompt = tmpl
                .replace("{name}", name)
                .replace("{construct}", construct);
            let response = json!({
                "construct": construct,
                "name": name,
                "vox_snippet": snippet,
            });
            let _ = (j, &mut rng); // prevent warning
            emit_line(out, &prompt, &response, construct, "web_construct_trace")?;
            count += 1;
        }
    }
    // Boost these with raw code responses
    for (construct, name, snippet) in WEB_CONSTRUCTS {
        let prompt = format!("Write a Vox {construct} called `{name}`");
        let line = json!({
            "prompt": prompt,
            "response": snippet,
            "category": format!("vox.web.{}", construct),
            "format": "vox_organic",
            "schema_version": "vox_dogfood_v1",
        });
        writeln!(out, "{}", serde_json::to_string(&line)?)?;
        count += 1;
    }
    Ok(count)
}

