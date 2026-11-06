// ─── Web construct SFT pairs ──────────────────────────────────────────────────

const WEB_CONSTRUCTS: &[(&str, &str, &str)] = &[
    (
        "component",
        "Navbar",
        "component Navbar(active_tab: str) {\n    view {\n        div(class: \"navbar\", children: [text(active_tab)])\n    }\n}",
    ),
    (
        "page",
        "Dashboard",
        "routes {\n    \"/dash\" to Dashboard\n}\n\ncomponent Dashboard() {\n    view {\n        div(class: \"dashboard\", children: [h1(text: \"Dashboard\"), Counter()])\n    }\n}",
    ),
    (
        "query",
        "get_user",
        "query get_user(id: Id[User]) to Option[User] {\n    return db.User.find(id)\n}",
    ),
    (
        "mutation",
        "update_user",
        "mutation update_user(id: Id[User], name: str) to Result[Unit, UserError] {\n    db.User.update(id, { name: name })\n    return Ok(())\n}",
    ),
    (
        "server",
        "submit_form",
        "server submit_form(payload: str) to Result[str, FormError] {\n    return Ok(\"submitted: \" + payload)\n}",
    ),
    (
        "server",
        "generate_pdf",
        "server generate_pdf(report: str) to Result[str, PdfError] {\n    return Ok(\"pdf_ready: \" + report)\n}",
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

