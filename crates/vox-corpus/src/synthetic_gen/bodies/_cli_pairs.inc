
// ─── CLI command SFT pairs ────────────────────────────────────────────────────

pub(crate) fn generate_cli_pairs(out: &mut impl Write, _cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0;

    let templates = [
        "How do I {desc_lower}?",
        "What is the command to {desc_lower}?",
        "Show me how to use `vox {cmd}`",
        "Run `vox {cmd}` — what does it do?",
        "I want to {desc_lower}. What Vox command should I use?",
    ];

    for (cmd, desc) in CLI_COMMANDS {
        let desc_lower = desc.to_lowercase();
        for (i, tmpl) in templates.iter().enumerate() {
            let prompt = tmpl
                .replace("{cmd}", cmd)
                .replace("{desc}", desc)
                .replace("{desc_lower}", &desc_lower);
            let response = json!({
                "command": format!("vox {}", cmd),
                "description": desc,
                "usage": format!("vox {} [options]", cmd),
            });
            let _ = i;
            emit_line(out, &prompt, &response, "cli_command", "cli_trace")?;
            count += 1;
        }
    }

    Ok(count)
}
