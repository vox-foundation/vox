// ─── Negative Preference (Rejection Sampling) SFT pairs ───────────────────────

pub(crate) fn generate_negative_preference_pairs(
    out: &mut impl Write,
    _cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    // Hardcoded negative preference scenarios (tool hallucination, bad params, etc)
    let negatives = [
        (
            "I need to query the database. Can you run select * from users?",
            "vox_sql_execute",
            "Query tool hallucinated raw SQL when it should use the Codex query builder.",
            json!({"sql": "SELECT * FROM users"}),
        ),
        (
            "Create a new component.",
            "vox_file_write",
            "Writing without gathering requirements is an anti-pattern.",
            json!({"path": "src/Component.tsx", "content": "export default function Component() {}"}),
        ),
        (
            "Change the CSS for the button.",
            "vox_run_command",
            "Attempted to use sed to modify CSS instead of file write.",
            json!({"command": "sed -i 's/blue/red/g' style.css"}),
        ),
        (
            "Delete the whole directory.",
            "vox_run_command",
            "Dangerous rm -rf without confirmation checks.",
            json!({"command": "rm -rf ."}),
        ),
    ];

    for (prompt, bad_tool, reason, bad_args) in negatives {
        let response = json!({
            "rejected_tool": bad_tool,
            "reason": reason,
            "arguments": bad_args,
        });
        emit_line(
            out,
            prompt,
            &response,
            "negative_routing",
            "negative_preference",
        )?;
        count += 1;
    }

    Ok(count)
}

