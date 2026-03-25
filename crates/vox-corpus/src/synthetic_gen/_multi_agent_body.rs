// ─── Multi-agent conversation pairs (Gap 8) ────────────────────────────────────

/// Multi-turn conversation traces where agents coordinate via A2A messages.
/// Teaches the model to reason about full agent-to-agent communication flows,
/// not just isolated tool calls.
pub fn generate_multi_agent_conversation_pairs(
    out: &mut impl Write,
    _cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    // Each scenario is (description, Vec<(speaker, content_json)>)
    let scenarios: &[(&str, &[(&str, serde_json::Value)])] = &[
        (
            "Orchestrator delegates auth task to worker, worker reports completion",
            &[
                (
                    "orchestrator→worker",
                    json!({"tool": "vox_a2a_send", "arguments": {"sender_id": 1, "receiver_id": 2, "msg_type": "task_assignment", "payload": "{\"task\": \"implement auth module\", \"file\": \"src/auth.vox\"}"}}),
                ),
                (
                    "worker acks",
                    json!({"tool": "vox_a2a_ack", "arguments": {"agent_id": 2, "message_id": 101}}),
                ),
                (
                    "worker claims file",
                    json!({"tool": "vox_claim_file", "arguments": {"path": "src/auth.vox"}}),
                ),
                (
                    "worker completes task",
                    json!({"tool": "vox_a2a_send", "arguments": {"sender_id": 2, "receiver_id": 1, "msg_type": "task_result", "payload": "{\"status\": \"complete\", \"artifact\": \"src/auth.vox\"}"}}),
                ),
            ],
        ),
        (
            "Planner creates plan and dispatches work to two parallel agents",
            &[
                (
                    "planner creates plan",
                    json!({"tool": "vox_plan", "arguments": {"goal": "Build user management system with auth and profile pages", "write_to_disk": true}}),
                ),
                (
                    "planner dispatches auth",
                    json!({"tool": "vox_submit_task", "arguments": {"description": "Implement auth.vox", "files": ["src/auth.vox"]}}),
                ),
                (
                    "planner dispatches profile",
                    json!({"tool": "vox_submit_task", "arguments": {"description": "Implement profile.vox", "files": ["src/profile.vox"]}}),
                ),
                (
                    "planner broadcasts phase start",
                    json!({"tool": "vox_a2a_broadcast", "arguments": {"sender_id": 1, "msg_type": "phase_start", "payload": "{\"phase\": 2, \"tasks\": 2}"}}),
                ),
            ],
        ),
        (
            "Agent asks peer for status, peer responds with progress",
            &[
                (
                    "agent checks peer status",
                    json!({"tool": "vox_agent_status", "arguments": {"agent_id": 3}}),
                ),
                (
                    "agent queries peer inbox",
                    json!({"tool": "vox_a2a_inbox", "arguments": {"agent_id": 1}}),
                ),
                (
                    "agent asks direct question",
                    json!({"tool": "vox_ask_agent", "arguments": {"agent_id": 3, "question": "Have you finished the database index?"}}),
                ),
            ],
        ),
    ];

    for (desc, turns) in scenarios {
        let turns_json: Vec<_> = turns
            .iter()
            .map(|(speaker, action)| json!({"speaker": speaker, "action": action}))
            .collect();
        let prompts = [
            format!("Show the complete multi-agent interaction for: {desc}"),
            format!("Walk through the agent coordination sequence for: {desc}"),
            format!("What tool calls are needed for this multi-agent flow: {desc}?"),
        ];
        for prompt in &prompts {
            let response = json!({
                "scenario": desc,
                "turns": turns_json,
                "pattern": "sequential_a2a",
            });
            emit_line(
                out,
                prompt,
                &response,
                "multi_agent_flow",
                "multi_agent_trace",
            )?;
            count += 1;
        }
    }
    Ok(count)
}
