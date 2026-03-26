// ─── Skill SFT pairs ──────────────────────────────────────────────────────────

const EXAMPLE_SKILLS: &[&str] = &[
    "vox-lint-fixer",
    "vox-docs-generator",
    "vox-test-writer",
    "vox-refactor-bot",
];

pub(crate) fn generate_skill_pairs(out: &mut impl Write, cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0;
    let skill_templates = &TEMPLATES.skills;

    for &skill in EXAMPLE_SKILLS {
        let _seed = cfg.seed; // Keep for deterministic iteration if needed later

        for tmpl in skill_templates {
            let prompt = tmpl.replace("{value}", skill);
            let response = json!({
                "tool": "vox_skill_install",
                "arguments": {
                    "bundle_json": format!(
                        r#"{{"id":"{skill}","version":"1.0.0","description":"Auto-generated skill","handler":"run"}}"#
                    )
                }
            });
            emit_line(out, &prompt, &response, "vox_skill_install", "tool_trace")?;
            count += 1;
        }
    }

    Ok(count)
}

// ─── Orchestrator command SFT pairs ──────────────────────────────────────────

pub(crate) fn orchestrator_prompt_templates() -> &'static [String] {
    if !TEMPLATES.orchestrator_commands.is_empty() {
        &TEMPLATES.orchestrator_commands
    } else {
        static FALLBACK: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
            vec![
                "The orchestrator needs to {desc_lower}. Write the tool call.".into(),
                "How does a Vox agent {desc_lower}?".into(),
                "Which orchestrator tool handles: {desc}".into(),
                "Show the JSON for {tool} with typical arguments.".into(),
                "Demonstrate {tool} being used in a Vox multi-agent session.".into(),
            ]
        });
        &FALLBACK
    }
}

pub(crate) fn generate_orchestrator_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;
    // Find all orchestrator tools in the slim registry
    let orch_tools: Vec<_> = TOOL_REGISTRY_SLIM
        .iter()
        .filter(|&name| {
            name.starts_with("vox_submit")
                || name.starts_with("vox_task")
                || name.starts_with("vox_orchestrator")
                || name.starts_with("vox_complete")
                || name.starts_with("vox_fail")
                || name.starts_with("vox_cancel")
                || name.starts_with("vox_rebalance")
                || name.starts_with("vox_reorder")
                || name.starts_with("vox_drain")
                || name.starts_with("vox_queue")
                || name.starts_with("vox_lock")
                || name.starts_with("vox_budget")
        })
        .copied()
        .collect();

    let prompts = orchestrator_prompt_templates();

    for &name in &orch_tools {
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        let args = example_args_for_tool(name, &mut rng);
        let desc = format!("{} action", name.replace("vox_", "").replace("_", " "));
        let desc_lower = desc.to_lowercase();
        for tmpl in prompts {
            let prompt = tmpl
                .replace("{tool}", name)
                .replace("{desc}", &desc)
                .replace("{desc_lower}", &desc_lower);
            let response = json!({
                "tool": name,
                "arguments": args,
                "description": desc,
            });
            emit_line(out, &prompt, &response, name, name)?;
            count += 1;
        }
    }

    // Multi-step orchestrator interaction scenarios
    let scenarios = [
        (
            "Submit a task, poll its status, then mark it complete when done.",
            vec![
                json!({"tool":"vox_submit_task","arguments":{"description":"implement login","files":["src/login.vox"]}}),
                json!({"tool":"vox_task_status","arguments":{"task_id":"task-001"}}),
                json!({"tool":"vox_complete_task","arguments":{"task_id":"task-001"}}),
            ],
            "success",
        ),
        (
            "Start the orchestrator, assign a file to an agent, then check locks.",
            vec![
                json!({"tool":"vox_orchestrator_start","arguments":{}}),
                json!({"tool":"vox_claim_file","arguments":{"path":"src/auth.vox"}}),
                json!({"tool":"vox_lock_status","arguments":{}}),
            ],
            "success",
        ),
        (
            "Submit a CUDA-required training task that fails capability checks, then cancel it.",
            vec![
                json!({"tool":"vox_schola_submit","arguments":{"description":"train qlora run","require_cuda":true,"min_vram_mb":16384,"trajectory_capture":true,"min_quality_score":4}}),
                json!({"tool":"vox_task_status","arguments":{"task_id":"task-qlora-001"}}),
                json!({"tool":"vox_cancel_task","arguments":{"task_id":"task-qlora-001"}}),
            ],
            "failure",
        ),
        (
            "Submit a training task, detect stale lock contention, recover by requeueing, then verify queued status.",
            vec![
                json!({"tool":"vox_schola_submit","arguments":{"description":"train trajectory eval","require_cuda":true,"min_vram_mb":12288,"trajectory_capture":true,"min_quality_score":3}}),
                json!({"tool":"vox_lock_status","arguments":{}}),
                json!({"tool":"vox_reorder_task","arguments":{"task_id":"task-qlora-002","priority":"background"}}),
                json!({"tool":"vox_task_status","arguments":{"task_id":"task-qlora-002"}}),
            ],
            "recovery",
        ),
    ];

    for (desc, steps, outcome) in &scenarios {
        let response = json!({ "multi_step": true, "steps": steps, "outcome": outcome });
        let category = match *outcome {
            "success" => "trajectory_success",
            "failure" => "trajectory_failure",
            "recovery" => "trajectory_recovery",
            _ => "tool_trace",
        };
        emit_line(out, desc, &response, "vox_submit_task", category)?;
        count += 1;
    }

    Ok(count)
}
