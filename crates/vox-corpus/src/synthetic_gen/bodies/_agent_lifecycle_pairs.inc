/// Generate agent lifecycle training pairs.
///
/// Covers create / deploy / health-check / shutdown flows for Vox agents,
/// teaching the model to reason about full agent lifecycle management.
pub fn generate_agent_lifecycle_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let lifecycle_flows: &[(&str, &str, &str)] = &[
        (
            "create and register a new Vox agent",
            "Define the agent with `@agent` annotation and call `vox_register_agent` via MCP to register it with the orchestrator.",
            "agent_lifecycle_create",
        ),
        (
            "deploy a Vox agent to the distributed mens",
            "After registering, call `vox_submit_task` with `task_type = 'deploy_agent'` and the agent's `agent_id`. The mens runtime handles placement.",
            "agent_lifecycle_deploy",
        ),
        (
            "check whether a Vox agent is healthy and responsive",
            "Call `vox_get_task_status` with the agent's active task id, or query the mens with `vox_populi_local_status` to inspect mailbox depth and last heartbeat.",
            "agent_lifecycle_health",
        ),
        (
            "gracefully shut down a running Vox agent",
            "Send a `shutdown` message via `vox_send_a2a_message` to the agent's address. The agent's `on_shutdown` handler runs before the process exits.",
            "agent_lifecycle_shutdown",
        ),
        (
            "view the reliability score for an agent over its last 100 tasks",
            "Reliability scores are stored in `agent_reliability` (Arca). Access via the `vox db stats --agent <id>` CLI command or query `VoxDb::list_agent_reliability()`.",
            "agent_lifecycle_reliability",
        ),
    ];

    let phrasings_formats = [
        "How do I {}?",
        "What is the correct way to {}?",
        "Walk me through how to {} in Vox.",
    ];

    let mut count = 0;
    let min = cfg.min_phrasings_per_tool.max(2);

    for (goal, guidance, category) in lifecycle_flows {
        for fmt in phrasings_formats.iter().take(min) {
            let prompt = fmt.replace("{}", goal);
            let response = json!({ "guidance": guidance, "category": category });
            emit_line(out, &prompt, &response, category, "agent_lifecycle")?;
            count += 1;
        }
    }
    Ok(count)
}

