// ─── A2A SFT pairs ────────────────────────────────────────────────────────────

pub(crate) fn a2a_prompt_templates() -> &'static [String] {
    if !TEMPLATES.a2a_messages.is_empty() {
        &TEMPLATES.a2a_messages
    } else {
        // Fallback static array wrapped in LazyLock or just vector
        static FALLBACK: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
            vec![
                "Send a {msg_type} message from {from} to {to}. Use the appropriate Vox A2A tool."
                    .into(),
                "Agent {from} needs to inform agent {to} about a {msg_type} event. How?".into(),
                "Use vox_a2a_send to deliver a {msg_type} from {from} to {to}.".into(),
                "Broadcast a {msg_type} to all agents except {from}.".into(),
                "Read the inbox of agent {to} and acknowledge any {msg_type} messages.".into(),
                "What is the correct tool call to send a {msg_type} A2A message in Vox?".into(),
                "Show the vox_a2a_send call for a {msg_type} from {from} to {to}.".into(),
                "Agent {from} completed its work and wants to tell {to}. Use {msg_type}.".into(),
            ]
        });
        &FALLBACK
    }
}

pub(crate) fn generate_a2a_pairs(out: &mut impl Write, cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0usize;
    let prompts = a2a_prompt_templates();
    for &msg_type in A2A_MESSAGE_TYPES {
        let mut rng = Rng::new(cfg.seed, name_hash(msg_type));
        let n = cfg.min_pairs_per_a2a_type.max(prompts.len());
        for i in 0..n {
            let pairs = crate::synthetic_gen::example_agent_pairs();
            let (from, to) = pairs[rng.next() as usize % pairs.len().max(1)].clone();
            let tmpl = &prompts[i % prompts.len()];
            let prompt = tmpl
                .replace("{msg_type}", msg_type)
                .replace("{from}", &from)
                .replace("{to}", &to);

            // Decide which tool to use based on template
            let (tool, args) = if tmpl.contains("Broadcast") || tmpl.contains("broadcast") {
                (
                    "vox_a2a_broadcast",
                    json!({
                        "sender_id": 1,
                        "msg_type": msg_type,
                        "payload": json!({ "event": msg_type }).to_string(),
                    }),
                )
            } else if tmpl.contains("inbox") || tmpl.contains("acknowledge") {
                ("vox_a2a_inbox", json!({ "agent_id": 2 }))
            } else {
                (
                    "vox_a2a_send",
                    json!({
                        "sender_id": 1,
                        "receiver_id": 2,
                        "msg_type": msg_type,
                        "payload": json!({ "event": msg_type, "from": from, "to": to }).to_string(),
                    }),
                )
            };

            let response = json!({
                "tool": tool,
                "arguments": args,
                "note": format!("Use {} for {} coordination", tool, msg_type),
            });
            emit_line(out, &prompt, &response, msg_type, "a2a_trace")?;
            count += 1;
        }
    }
    Ok(count)
}

