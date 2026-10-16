// ─── Web construct SFT pairs ──────────────────────────────────────────────────

const WEB_CONSTRUCTS: &[(&str, &str, &str)] = &[
    (
        "component",
        "Navbar",
        "fn Navbar(active_tab: str) to Element {\n    div(class: \"navbar\", children: [text(active_tab)])\n}",
    ),
    (
        "page",
        "Dashboard",
        "routes {\n    \"/dash\" to Dashboard\n}\n\nfn Dashboard() to Element {\n    div(class: \"dashboard\", children: [h1(text: \"Dashboard\"), Counter()])\n}",
    ),
    (
        "query",
        "get_user",
        "query fn get_user(id: int) to Option[User] {\n    ret db.User.find(id)\n}",
    ),
    (
        "mutation",
        "update_user",
        "mutation fn update_user(id: int, name: str) to Result[Unit] {\n    db.User.update(id, { name: name })\n    ret Ok(())\n}",
    ),
    (
        "server fn",
        "submit_form",
        "server fn submit_form(payload: str) to Result[str] {\n    ret Ok(\"submitted: \" + payload)\n}",
    ),
    (
        "server fn",
        "generate_pdf",
        "server fn generate_pdf(report: str) to Result[str] {\n    ret Ok(\"pdf_ready: \" + report)\n}",
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
            "messages": [
                {"role": "user", "content": prompt},
                {"role": "assistant", "content": snippet}
            ],
            "category": format!("vox.web.{}", construct),
            "format": "vox_organic",
            "schema_version": "vox_dogfood_v1",
        });
        writeln!(out, "{}", serde_json::to_string(&line)?)?;
        count += 1;
    }
    Ok(count)
}

