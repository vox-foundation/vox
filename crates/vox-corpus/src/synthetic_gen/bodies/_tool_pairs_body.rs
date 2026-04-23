pub(crate) fn tool_prompt_templates() -> &'static [String] {
    &TEMPLATES.tool_definitions
}

/// Generate tool-call SFT pairs for all entries in `registry`.
pub(crate) fn generate_tool_pairs(
    out: &mut impl Write,
    registry: &[&str],
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0usize;
    for &name in registry {
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        let desc = format!("{} action", name.replace("vox_", "").replace("_", " "));
        let desc_lower = desc.to_lowercase();
        // Example args are minimal but well-formed
        let example_args = example_args_for_tool(name, &mut rng);
        let templates = tool_prompt_templates();
        let n = cfg.min_phrasings_per_tool.max(templates.len());
        for i in 0..n {
            let tmpl = &templates[i % templates.len()];
            let prompt = tmpl
                .replace("{tool}", name)
                .replace("{desc}", &desc)
                .replace("{desc_lower}", &desc_lower);
            emit_tool_pair(out, name, &desc, &prompt, example_args.clone(), name, name)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Generate plausible example arguments for a given tool name.
/// Arguments are purposefully minimal and illustrative — the model learns the shape.
pub(crate) fn example_args_for_tool(tool: &str, rng: &mut Rng) -> Value {
    match tool {
        "vox_submit_task" => {
            let tasks = crate::synthetic_gen::example_tasks();
            let task = tasks[rng.next() as usize % tasks.len().max(1)].clone();
            json!({ "description": task, "files": ["src/main.vox"] })
        }
        "vox_task_status" | "vox_complete_task" | "vox_fail_task" | "vox_cancel_task" => {
            json!({ "task_id": "task-00000000-0000-0000-0000-000000000001" })
        }
        "vox_check_file_owner" | "vox_claim_file" | "vox_validate_file" => {
            json!({ "path": "src/components/login.vox" })
        }
        "vox_set_context" => json!({ "key": "current_phase", "value": "build", "ttl_secs": 300 }),
        "vox_get_context" | "vox_list_context" => json!({ "key": "current_phase" }),
        "vox_handoff_context" | "vox_agent_handoff" => {
            json!({ "from_agent_id": 1, "to_agent_id": 2, "summary": "Phase 1 complete. Continuing with tests." })
        }
        "vox_check_mood" | "vox_agent_status" | "vox_agent_continue" | "vox_agent_assess" => {
            json!({ "agent_id": 1 })
        }
        "vox_queue_status" | "vox_my_files" => json!({ "agent_id": 1 }),
        "vox_budget_status"
        | "vox_lock_status"
        | "vox_orchestrator_status"
        | "vox_test_all"
        | "vox_check_workspace"
        | "vox_file_graph"
        | "vox_config_get"
        | "vox_repo_index_status"
        | "vox_repo_index_refresh"
        | "vox_vcs_status"
        | "vox_session_list"
        | "vox_memory_list_keys"
        | "vox_session_cleanup"
        | "vox_lock_status2"
        | "vox_rebalance"
        | "vox_oratio_status"
        | "vox_chat_history"
        | "vox_get_active_model"
        | "vox_populi_local_status"
        | "vox_benchmark_list" => json!({}),
        "vox_run_tests" => json!({ "crate_name": "vox-cli", "filter": "training" }),
        "vox_build_crate" | "vox_lint_crate" | "vox_coverage_report" => {
            json!({ "crate_name": "vox-cli" })
        }
        "vox_transfer_file" => json!({ "path": "src/main.vox", "to_agent_id": 2 }),
        "vox_ask_agent" => {
            json!({ "agent_id": 2, "question": "Have you finished the auth module?" })
        }
        "vox_answer_question" => {
            json!({ "agent_id": 1, "question_id": 42, "answer": "Yes, auth is complete." })
        }
        "vox_pending_questions" => json!({ "agent_id": 1 }),
        "vox_broadcast" => json!({ "agent_id": 1, "message": "Phase 2 starting now." }),
        "vox_publish_message" => json!({ "message": "Build succeeded. Ready for review." }),
        "vox_memory_store" => json!({ "key": "last_refactor", "value": "extracted auth module" }),
        "vox_memory_recall" => json!({ "key": "last_refactor" }),
        "vox_memory_search" => json!({ "query": "auth module" }),
        "vox_memory_log" => json!({ "entry": "Completed route extraction" }),
        "vox_knowledge_query" => json!({ "query": "actor message passing" }),
        "vox_memory_save_db" => {
            json!({ "agent_id": 1, "key": "phase", "value": "testing", "memory_type": "fact" })
        }
        "vox_memory_recall_db" => json!({ "agent_id": 1, "key_prefix": "phase" }),
        "vox_skill_install" => {
            json!({ "bundle_json": "{\"id\":\"vox-lint-fixer\",\"version\":\"1.0.0\",\"description\":\"Auto-fix lint warnings\",\"handler\":\"fix_lint\"}" })
        }
        "vox_skill_uninstall" | "vox_skill_info" => json!({ "skill_id": "vox-lint-fixer" }),
        "vox_skill_search" => json!({ "query": "lint" }),
        "vox_skill_parse" => {
            json!({ "skill_md": "---\nname: vox-lint-fixer\nversion: 1.0.0\n---\nFixes lint warnings." })
        }
        "vox_compaction_status" => json!({ "agent_id": 1 }),
        "vox_session_create" => {
            json!({ "agent_id": 1, "model_id": "anthropic/claude-3-5-haiku", "system_prompt": "You are a Vox expert." })
        }
        "vox_session_reset" | "vox_session_info" | "vox_session_compact" => {
            json!({ "session_id": "sess-abc123" })
        }
        "vox_preference_get" => json!({ "key": "theme" }),
        "vox_preference_set" => json!({ "key": "theme", "value": "dark" }),
        "vox_preference_list" => json!({ "prefix": "" }),
        "vox_learn_pattern" => {
            json!({ "pattern": "agent writes tests before impl", "confidence": 0.85, "category": "development" })
        }
        "vox_behavior_record" => json!({ "event": "file_saved", "path": "src/auth.vox" }),
        "vox_behavior_summary" => json!({ "agent_id": 1, "lookback_hours": 24 }),
        "vox_reorder_task" => json!({ "task_id": "task-001", "priority": "high" }),
        "vox_drain_agent" => json!({ "agent_id": 2 }),
        "vox_cost_history" => json!({ "since_hours": 24 }),
        "vox_config_set" => {
            json!({ "max_agents": 4, "default_model": "anthropic/claude-3-5-haiku" })
        }
        "vox_map_agent_session" => {
            json!({ "session_id": "sess-abc123", "agent_id": 1 })
        }
        "vox_poll_events" => json!({ "since_ms": 0, "limit": 20 }),
        "vox_heartbeat" => json!({ "agent_id": 1, "session_id": "sess-abc123" }),
        "vox_record_cost" => {
            json!({ "agent_id": 1, "input_tokens": 1200, "output_tokens": 400, "model_id": "claude-3-5-haiku" })
        }
        "vox_git_log" => json!({ "max_commits": 10 }),
        "vox_git_diff" => json!({ "path": "src/main.vox" }),
        "vox_git_blame" => json!({ "path": "src/auth.vox" }),
        "vox_snapshot_list" => json!({ "agent_id": 1, "limit": 10 }),
        "vox_snapshot_diff" => json!({ "from_id": "snap_001", "to_id": "snap_002" }),
        "vox_snapshot_restore" => json!({ "snapshot_id": "snap_001" }),
        "vox_oplog" => json!({ "limit": 20 }),
        "vox_undo" => json!({ "op_id": "op-42" }),
        "vox_redo" => json!({ "op_id": "op-42" }),
        "vox_conflicts" => json!({}),
        "vox_resolve_conflict" => {
            json!({ "path": "src/auth.vox", "resolution": "ours" })
        }
        "vox_conflict_diff" => json!({ "path": "src/auth.vox" }),
        "vox_workspace_create" => json!({ "agent_id": 2, "base": "main" }),
        "vox_workspace_merge" => json!({ "agent_id": 2 }),
        "vox_workspace_status" => json!({ "agent_id": 2 }),
        "vox_change_create" => {
            json!({ "name": "auth-refactor", "description": "Refactor the auth module" })
        }
        "vox_change_log" => json!({ "change_id": "chg-001" }),
        "vox_a2a_send" => {
            json!({ "sender_id": 1, "receiver_id": 2, "msg_type": "plan_handoff", "payload": "{\"plan\":\"implement auth\"}" })
        }
        "vox_a2a_inbox" => json!({ "agent_id": 2 }),
        "vox_a2a_ack" => json!({ "agent_id": 2, "message_id": 42 }),
        "vox_a2a_broadcast" => {
            json!({ "sender_id": 1, "msg_type": "progress_update", "payload": "{\"done\":50}" })
        }
        "vox_a2a_history" => json!({ "since_ms": 0, "limit": 20 }),
        "vox_db_schema" | "vox_db_relationships" | "vox_db_data_flow" => json!({}),
        "vox_db_sample_data" => json!({ "table": "users", "limit": 5 }),
        "vox_db_explain_query" | "vox_db_suggest_query" => {
            json!({ "query": "users where email = 'foo@bar.com'" })
        }
        "vox_db_research_session_upsert" => {
            json!({ "session_key": "arch-review-2026-03", "repository_id": "", "title": "Architecture review" })
        }
        "vox_db_conversation_version_append" => {
            json!({ "conversation_id": "conv-001", "version": 1, "summary": "Initial analysis" })
        }
        "vox_db_research_metric_linked" => {
            json!({ "session_key": "arch-review-2026-03", "metric_name": "coverage_ratio", "value": 0.92 })
        }
        "vox_generate_code" => {
            json!({ "prompt": "Write a Vox actor that manages a counter with increment and reset messages" })
        }
        "vox_list_models" => json!({}),
        "vox_suggest_model" => json!({ "task": "codegen" }),
        "vox_set_model" => json!({ "agent_id": 1, "model_id": "anthropic/claude-3-5-haiku" }),
        "vox_set_active_model" => json!({ "model_id": "anthropic/claude-3-5-haiku" }),
        "vox_oratio_transcribe" => json!({ "path": "recordings/meeting.wav" }),
        "vox_chat_message" => {
            json!({ "message": "Generate a Vox actor for rate limiting", "session_id": "sess-abc123" })
        }
        "vox_inline_edit" => {
            json!({ "path": "src/auth.vox", "range": { "start": 10, "end": 25 }, "instruction": "Add error handling" })
        }
        "vox_plan" => {
            json!({ "goal": "Add authentication to the API", "write_to_disk": false })
        }
        "vox_replan" => {
            json!({ "session_id": "sess-abc123", "delta_hint": "User wants OAuth instead of basic auth" })
        }
        "vox_plan_status" => json!({ "session_id": "sess-abc123" }),
        "vox_schola_submit" => {
            json!({ "description": "Train Mens on the updated corpus", "require_cuda": true })
        }
        "vox_reliability_list" => json!({ "limit": 25 }),
        "vox_reliability_agents" => json!({}),
        _ => derive_args_from_description(tool),
    }
}

pub(crate) fn derive_args_from_description(tool: &str) -> Value {
    if tool.starts_with("vox_get_") {
        json!({ "id": "123" })
    } else if tool.starts_with("vox_set_") {
        json!({ "id": "123", "value": "test" })
    } else if tool.starts_with("vox_list_") {
        json!({ "limit": 10 })
    } else {
        json!({ "query": "example" })
    }
}
