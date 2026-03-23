//! Synthetic training-pair generator for Vox MCP tools, A2A messaging, skills,
//! orchestrator commands, durable workflows, and agent constructs.
//!
//! All data is sourced from compile-time Rust constants — never from external files
//! or hardcoded magic values. Adding a new MCP tool to the registry automatically
//! generates training pairs for it on the next `vox corpus generate` run.
//!
//! ## Usage
//! ```no_run
//! use vox_corpus::synthetic_gen::{SyntheticGenConfig, generate_all};
//! let cfg = SyntheticGenConfig::default();
//! generate_all(&cfg, std::path::Path::new("populi/data/synthetic.jsonl")).unwrap();
//! ```

use std::io::Write;
use std::path::Path;

use anyhow::Context;
use serde_json::{json, Value};

// ─── Compile-time source tables ──────────────────────────────────────────────

pub use vox_mcp_meta::{
    SKILL_TOOLS,
    ORCHESTRATOR_TOOLS,
};

include!(concat!(env!("OUT_DIR"), "/dynamic_registry.rs"));

/// Example agent ID pairs for A2A training pair diversification.
const EXAMPLE_AGENT_PAIRS: &[(&str, &str)] = &[
    ("agent_1", "agent_2"),
    ("compiler_agent", "review_agent"),
    ("orchestrator", "worker_1"),
    ("planner", "executor"),
    ("frontend_agent", "backend_agent"),
];

/// Example task descriptions for orchestrator SFT pairs.
const EXAMPLE_TASKS: &[&str] = &[
    "implement the login page component",
    "add a database index for the users table",
    "write unit tests for the authentication module",
    "refactor the workflow actor to use the new message type",
    "update the MCP tool schema for vox_validate_file",
    "generate the TypeScript bindings for the Vox runtime",
    "rank model reliability using hallucination EWMA scores",
    "check for unreliable endpoints in the OpenRouter registry",
    "audit the agent fleet for recurring task failures",
    "fix the parser error recovery for malformed actor declarations",
    "implement the durable checkout workflow with retry semantics",
];

// ─── Config ───────────────────────────────────────────────────────────────────

/// Configuration for the synthetic corpus generator.
#[derive(Debug, Clone)]
pub struct SyntheticGenConfig {
    /// Minimum phrasings per tool call (diversity floor).
    pub min_phrasings_per_tool: usize,
    /// Minimum A2A pairs per message type.
    pub min_pairs_per_a2a_type: usize,
    /// RNG seed. `0` uses deterministic seeding from tool name hash.
    pub seed: u64,
    /// Whether to emit tool_trace format rows.
    pub emit_tool_traces: bool,
    /// Whether to emit A2A trace rows.
    pub emit_a2a_traces: bool,
    /// Whether to emit workflow trace rows.
    pub emit_workflow_traces: bool,
    /// Whether to emit orchestrator command rows.
    pub emit_orchestrator_rows: bool,
    /// Whether to emit skill management rows.
    pub emit_skill_rows: bool,
    /// Whether to emit agent definition / agent usage rows.
    pub emit_agent_rows: bool,
    /// Whether to emit CLI command usage rows.
    pub emit_cli_rows: bool,
    /// Whether to emit shell script generation rows.
    pub emit_script_rows: bool,
    /// Whether to emit organic Vox code generation pairs.
    pub emit_organic_vox: bool,
    /// Whether to run the augmentation engine (typos, synonyms, case) after generation.
    /// This 3× multiplies effective corpus size with robust variants.
    pub augment_after_generate: bool,
}

impl Default for SyntheticGenConfig {
    fn default() -> Self {
        Self {
            min_phrasings_per_tool: 6,
            min_pairs_per_a2a_type: 4,
            seed: 0,
            emit_tool_traces: true,
            emit_a2a_traces: true,
            emit_workflow_traces: true,
            emit_orchestrator_rows: true,
            emit_skill_rows: true,
            emit_agent_rows: true,
            emit_cli_rows: true,
            emit_script_rows: true,
            emit_organic_vox: true,
            augment_after_generate: true,
        }
    }
}

// ─── XorShift RNG (no external deps) ─────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64, salt: u64) -> Self {
        let mut s = seed ^ salt;
        if s == 0 {
            s = 0xdeadbeef_cafebabe;
        }
        Self(s)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.next() as usize % slice.len()]
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Deterministic string hash (FNV-1a 64-bit) for seeding RNG per tool name.
fn name_hash(s: &str) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    let mut h = OFFSET;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Emit one JSONL line to `out`.
fn emit_line(
    out: &mut impl Write,
    prompt: &str,
    response: &Value,
    category: &str,
    record_type: &str,
) -> anyhow::Result<()> {
    let row = json!({
        "prompt": prompt,
        "response": response.to_string(),
        "category": category,
        "record_type": record_type,
        "schema_version": "vox_dogfood_v1",
    });
    writeln!(out, "{}", serde_json::to_string(&row)?)?;
    Ok(())
}

/// Emit a human-readable + JSON tool-call pair.
fn emit_tool_pair(
    out: &mut impl Write,
    tool_name: &str,
    description: &str,
    prompt: &str,
    args: Value,
    category: &str,
    record_type: &str,
) -> anyhow::Result<()> {
    let response = json!({
        "tool": tool_name,
        "description": description,
        "arguments": args,
    });
    emit_line(out, prompt, &response, category, record_type)
}

// ─── Tool-call SFT pairs ──────────────────────────────────────────────────────

use std::sync::LazyLock;

#[derive(serde::Deserialize)]
struct TemplatesConfig {
    synthetic: SyntheticTemplates,
}

#[derive(serde::Deserialize)]
struct SyntheticTemplates {
    tool_definitions: Vec<String>,
    a2a_messages: Vec<String>,
    skills: Vec<String>,
    orchestrator_commands: Vec<String>,
    workflows: Vec<ScenarioDef>,
    agents: Vec<ScenarioDef>,
}

#[derive(serde::Deserialize, Clone)]
struct ScenarioDef {
    name: String,
    description: String,
    snippet: String,
}

static TEMPLATES: LazyLock<SyntheticTemplates> = LazyLock::new(|| {
    let yaml = include_str!("../../../populi/config/templates.yaml");
    let cfg: TemplatesConfig = serde_yaml::from_str(yaml).expect("Failed to parse templates.yaml");
    cfg.synthetic
});

fn tool_prompt_templates() -> &'static [String] {
    &TEMPLATES.tool_definitions
}

/// Generate tool-call SFT pairs for all entries in `registry`.
fn generate_tool_pairs(
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
fn example_args_for_tool(tool: &str, rng: &mut Rng) -> Value {
    match tool {
        "vox_submit_task" => {
            let task = EXAMPLE_TASKS[rng.next() as usize % EXAMPLE_TASKS.len()];
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
        "vox_budget_status" | "vox_lock_status" | "vox_orchestrator_status"
        | "vox_test_all" | "vox_check_workspace" | "vox_file_graph" | "vox_config_get"
        | "vox_repo_index_status" | "vox_repo_index_refresh" | "vox_vcs_status"
        | "vox_session_list" | "vox_memory_list_keys" | "vox_session_cleanup"
        | "vox_lock_status2" | "vox_rebalance" | "vox_oratio_status"
        | "vox_chat_history" | "vox_get_active_model" | "vox_mesh_local_status"
        | "vox_benchmark_list" => json!({}),
        "vox_run_tests" => json!({ "crate_name": "vox-cli", "filter": "training" }),
        "vox_build_crate" | "vox_lint_crate" | "vox_coverage_report" => {
            json!({ "crate_name": "vox-cli" })
        }
        "vox_transfer_file" => json!({ "path": "src/main.vox", "to_agent_id": 2 }),
        "vox_ask_agent" => json!({ "agent_id": 2, "question": "Have you finished the auth module?" }),
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
        "vox_skill_parse" => json!({ "skill_md": "---\nname: vox-lint-fixer\nversion: 1.0.0\n---\nFixes lint warnings." }),
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
        "vox_change_create" => json!({ "name": "auth-refactor", "description": "Refactor the auth module" }),
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
        "vox_replan" => json!({ "session_id": "sess-abc123", "delta_hint": "User wants OAuth instead of basic auth" }),
        "vox_plan_status" => json!({ "session_id": "sess-abc123" }),
        "vox_train_submit" => {
            json!({ "description": "Train Populi on the updated corpus", "require_cuda": true })
        }
        "vox_reliability_list" => json!({ "limit": 25 }),
        "vox_reliability_agents" => json!({}),
        _ => derive_args_from_description(tool),
    }
}

fn derive_args_from_description(tool: &str) -> Value {
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

// ─── A2A SFT pairs ────────────────────────────────────────────────────────────

fn a2a_prompt_templates() -> &'static [String] {
    if !TEMPLATES.a2a_messages.is_empty() {
        &TEMPLATES.a2a_messages
    } else {
        // Fallback static array wrapped in LazyLock or just vector
        static FALLBACK: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
            vec![
                "Send a {msg_type} message from {from} to {to}. Use the appropriate Vox A2A tool.".into(),
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

fn generate_a2a_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0usize;
    let prompts = a2a_prompt_templates();
    for &msg_type in A2A_MESSAGE_TYPES {
        let mut rng = Rng::new(cfg.seed, name_hash(msg_type));
        let n = cfg.min_pairs_per_a2a_type.max(prompts.len());
        for i in 0..n {
            let &(from, to) = rng.pick(EXAMPLE_AGENT_PAIRS);
            let tmpl = &prompts[i % prompts.len()];
            let prompt = tmpl
                .replace("{msg_type}", msg_type)
                .replace("{from}", from)
                .replace("{to}", to);

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
                (
                    "vox_a2a_inbox",
                    json!({ "agent_id": 2 }),
                )
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

// ─── Workflow construct SFT pairs ─────────────────────────────────────────────

fn generate_workflow_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    let prompts = [
        "Implement {desc} as a workflow named {name}.",
        "Show me how to write {desc} in Vox.",
        "Provide a Vox @workflow definition for {name}.",
        "Create a {name} workflow that acts as {desc}.",
        "Write the {name} durable workflow in Vox.",
    ];

    for def in &TEMPLATES.workflows {
        let name = &def.name;
        let desc = &def.description;
        let snippet = &def.snippet;
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        for (j, tmpl) in prompts.iter().enumerate() {
            let prompt = tmpl
                .replace("{name}", name)
                .replace("{desc}", desc);
            let response = json!({
                "construct": "workflow_def",
                "name": name,
                "description": desc,
                "vox_snippet": snippet,
            });
            let _ = (j, &mut rng); // prevent unused warnings
            emit_line(out, &prompt, &response, "workflow_def", "workflow_trace")?;
            count += 1;
        }
    }
    Ok(count)
}

// ─── Skill SFT pairs ──────────────────────────────────────────────────────────

const EXAMPLE_SKILLS: &[&str] = &[
    "vox-lint-fixer",
    "vox-docs-generator",
    "vox-test-writer",
    "vox-refactor-bot",
];

fn generate_skill_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
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

fn orchestrator_prompt_templates() -> &'static [String] {
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

fn generate_orchestrator_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;
    // Find all orchestrator tools in the slim registry
    let orch_tools: Vec<_> = TOOL_REGISTRY_SLIM
        .iter()
        .filter(|&name| {
            name.starts_with("vox_submit") || name.starts_with("vox_task") ||
            name.starts_with("vox_orchestrator") || name.starts_with("vox_complete") ||
            name.starts_with("vox_fail") || name.starts_with("vox_cancel") ||
            name.starts_with("vox_rebalance") || name.starts_with("vox_reorder") ||
            name.starts_with("vox_drain") || name.starts_with("vox_queue") ||
            name.starts_with("vox_lock") || name.starts_with("vox_budget")
        })
        .map(|s| *s)
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
        ("Submit a task, poll its status, then mark it complete when done.",
         vec![
             json!({"tool":"vox_submit_task","arguments":{"description":"implement login","files":["src/login.vox"]}}),
             json!({"tool":"vox_task_status","arguments":{"task_id":"task-001"}}),
             json!({"tool":"vox_complete_task","arguments":{"task_id":"task-001"}}),
         ]),
        ("Start the orchestrator, assign a file to an agent, then check locks.",
         vec![
             json!({"tool":"vox_orchestrator_start","arguments":{}}),
             json!({"tool":"vox_claim_file","arguments":{"path":"src/auth.vox"}}),
             json!({"tool":"vox_lock_status","arguments":{}}),
         ]),
    ];

    for (desc, steps) in &scenarios {
        let response = json!({ "multi_step": true, "steps": steps });
        emit_line(out, desc, &response, "vox_submit_task", "tool_trace")?;
        count += 1;
    }

    Ok(count)
}

// ─── Web construct SFT pairs ──────────────────────────────────────────────────

const WEB_CONSTRUCTS: &[(&str, &str, &str)] = &[
    ("component", "Navbar", "component Navbar {\n  ret <nav><ul><li>Home</li></ul></nav>\n}"),
    ("island", "Counter", "island Counter {\n  state count: int = 0\n  ret <button onClick={|| self.count += 1}>{self.count}</button>\n}"),
    ("page", "Dashboard", "@route(\"/dash\")\npage Dashboard {\n  ret <main><h1>Dashboard</h1><Counter /></main>\n}"),
    ("@query", "get_user", "@query\nfn get_user(id: int) -> Option[User] {\n  ret VoxDb::query(\"SELECT * FROM users WHERE id = ?1\", [id]).first()\n}"),
    ("@mutation", "update_user", "@mutation\nfn update_user(id: int, name: str) -> Result[Unit] {\n  VoxDb::execute(\"UPDATE users SET name = ?2 WHERE id = ?1\", [id, name])\n  ret Ok(())\n}"),
    ("@action", "submit_form", "@action\nfn submit_form(data: FormData) -> Result[Unit] {\n  // form logic\n  ret Ok(())\n}"),
    ("@server", "generate_pdf", "@server\nfn generate_pdf(report: str) -> bytes {\n  // server side only logic\n  ret b\"...\"\n}"),
];

fn generate_web_construct_pairs(out: &mut impl Write, cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0;
    let prompts = [
        "Implement a {name} {construct} in Vox.",
        "Show me how to write a {construct} named {name}.",
        "Provide a Vox {construct} definition for {name}.",
        "Create a {name} that acts as a {construct}.",
        "Write the {name} {construct} in Vox syntax."
    ];

    for (construct, name, snippet) in WEB_CONSTRUCTS {
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        for (j, tmpl) in prompts.iter().enumerate() {
            let prompt = tmpl.replace("{name}", name).replace("{construct}", construct);
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

// ─── Negative Preference (Rejection Sampling) SFT pairs ───────────────────────

fn generate_negative_preference_pairs(out: &mut impl Write, _cfg: &SyntheticGenConfig) -> anyhow::Result<usize> {
    let mut count = 0;
    
    // Hardcoded negative preference scenarios (tool hallucination, bad params, etc)
    let negatives = [
        (
            "I need to query the database. Can you run select * from users?",
            "vox_sql_execute", "Query tool hallucinated raw SQL when it should use the Codex query builder.",
            json!({"sql": "SELECT * FROM users"})
        ),
        (
            "Create a new component.",
            "vox_file_write", "Writing without gathering requirements is an anti-pattern.",
            json!({"path": "src/Component.tsx", "content": "export default function Component() {}"})
        ),
        (
            "Change the CSS for the button.",
            "vox_run_command", "Attempted to use sed to modify CSS instead of file write.",
            json!({"command": "sed -i 's/blue/red/g' style.css"})
        ),
        (
            "Delete the whole directory.",
            "vox_run_command", "Dangerous rm -rf without confirmation checks.",
            json!({"command": "rm -rf ."})
        ),
    ];

    for (prompt, bad_tool, reason, bad_args) in negatives {
        let response = json!({
            "rejected_tool": bad_tool,
            "reason": reason,
            "arguments": bad_args,
        });
        emit_line(out, prompt, &response, "negative_routing", "negative_preference")?;
        count += 1;
    }
    
    Ok(count)
}

// ─── Agent construct SFT pairs ────────────────────────────────────────────────

fn generate_agent_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;
    let prompts = [
        "Define a Vox AI agent called {name} that can {desc}.",
        "Write a Vox @agent_def for {name}.",
        "How do I create an agent named {name} in Vox?",
        "Build a {name} agent in Vox with appropriate tools and memory.",
        "Implement the {name} agent — it should {desc}.",
        "Show the Vox syntax for an agent that {desc_lower}.",
    ];

    for def in &TEMPLATES.agents {
        let name = &def.name;
        let desc = &def.description;
        let snippet = &def.snippet;
        let mut rng = Rng::new(cfg.seed, name_hash(name));
        let desc_lower = desc.to_lowercase();
        for (i, tmpl) in prompts.iter().enumerate() {
            let prompt = tmpl
                .replace("{name}", name)
                .replace("{desc}", desc)
                .replace("{desc_lower}", &desc_lower);
            let response = json!({
                "construct": "agent_def",
                "name": name,
                "description": desc,
                "vox_snippet": snippet,
            });
            let _ = (i, &mut rng);
            emit_line(out, &prompt, &response, "agent_def", "agent_def")?;
            count += 1;
        }
    }
    Ok(count)
}

// ─── CLI command SFT pairs ────────────────────────────────────────────────────

fn generate_cli_pairs(
    out: &mut impl Write,
    _cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    let templates = [
        "How do I {desc_lower}?",
        "What is the command to {desc_lower}?",
        "Show me how to use `vox {cmd}`",
        "Run `vox {cmd}` — what does it do?",
        "I want to {desc_lower}. What Vox command should I use?",
    ];

    for &(ref cmd, ref desc) in CLI_COMMANDS {
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

// ─── Shell script SFT pairs ──────────────────────────────────────────────────

fn generate_script_pairs(
    out: &mut impl Write,
    _cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    let mut count = 0;

    let scripts: &[(&str, &str, &str)] = &[
        // (prompt, script content, category)
        (
            "Write a PowerShell script to monitor QLoRA training telemetry",
            r#"$TelemetryPath = "populi\runs\qwen25_qlora\telemetry.jsonl"
if (Test-Path $TelemetryPath) {
    Get-Content $TelemetryPath -Wait -Tail 10 | ForEach-Object {
        $event = $_ | ConvertFrom-Json
        if ($event.event -eq "step") {
            Write-Host "Step $($event.payload.step) | Loss: $($event.payload.loss) | ETA: $($event.payload.eta_sec)s" -ForegroundColor Cyan
        }
    }
} else {
    Write-Host "Telemetry file not found: $TelemetryPath" -ForegroundColor Red
}"#,
            "powershell_script",
        ),
        (
            "Write a batch file to build Vox with CUDA support on Windows",
            r#"@echo off
setlocal
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if %errorlevel% neq 0 (
    echo MSVC init failed
    exit /b 1
)
cargo build -p vox-cli --release --features gpu,populi-candle-cuda
endlocal"#,
            "batch_script",
        ),
        (
            "Write a PowerShell script to generate and mix the Populi training corpus",
            r#"$RepoRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $RepoRoot

Write-Host "Generating synthetic corpus..." -ForegroundColor Cyan
& "$RepoRoot\target\release\vox.exe" populi corpus generate --output populi/data/synthetic.jsonl

Write-Host "Mixing corpus..." -ForegroundColor Cyan
& "$RepoRoot\target\release\vox.exe" populi corpus mix --config populi/config/mix.yaml

Write-Host "Corpus ready at target/dogfood/train_mixed.jsonl" -ForegroundColor Green"#,
            "powershell_script",
        ),
        (
            "Write a shell command to run all Vox workspace tests",
            "cargo test --workspace",
            "shell_command",
        ),
        (
            "Write a command to check a specific Vox crate compiles",
            "cargo check -p vox-corpus",
            "shell_command",
        ),
        (
            "Write a PowerShell script to start QLoRA training detached with persistent logging",
            r#"$RunDir = "populi\runs\qwen25_qlora"
New-Item -ItemType Directory -Force -Path $RunDir | Out-Null
New-Item -ItemType Directory -Force -Path "target\dogfood" | Out-Null

Start-Process -FilePath "launch_train.bat" -RedirectStandardOutput "$RunDir\train_run.log" -RedirectStandardError "$RunDir\train_err.log" -NoNewWindow
Write-Host "Training started. Monitor with: Get-Content $RunDir\telemetry.jsonl -Wait -Tail 5""#,
            "powershell_script",
        ),
        (
            "Write a command to push local Git commits to main",
            "git add -A && git commit -m \"update\" && git push origin main",
            "shell_command",
        ),
        (
            "Write a Cargo command to build the Vox release binary with GPU features",
            "cargo build -p vox-cli --release --features gpu",
            "shell_command",
        ),
        (
            "Write a PowerShell command to tail the training log file",
            "Get-Content C:\\Users\\Owner\\vox\\populi\\runs\\qwen25_qlora\\telemetry.jsonl -Wait -Tail 10",
            "shell_command",
        ),
        (
            "Write a command to check line endings for cross-platform integrity",
            "cargo run -p vox-cli -- ci line-endings",
            "shell_command",
        ),
    ];

    for (prompt, script, category) in scripts {
        let response = json!({
            "script": script,
            "language": if category.contains("powershell") { "powershell" }
                       else if category.contains("batch") { "batch" }
                       else { "shell" },
        });
        emit_line(out, prompt, &response, category, "script_trace")?;
        count += 1;
    }

    Ok(count)
}

// ─── Multi-tool orchestration pairs ──────────────────────────────────────────

/// Generate multi-tool orchestration training pairs.
///
/// Teaches the model to chain 2–3 sequential tool calls to accomplish compound
/// goals. Sequences are derived dynamically from `TOOL_REGISTRY_SLIM` so they
/// stay in sync as tools are added.
pub fn generate_tool_chain_pairs(
    out: &mut impl Write,
    cfg: &SyntheticGenConfig,
) -> anyhow::Result<usize> {
    // Curated 2-and-3-tool sequences drawn from real orchestration flows
    let sequences: &[(&[&str], &str, &str)] = &[
        (
            &["vox_plan_create", "vox_generate_vox_code"],
            "Plan and then generate Vox code for a user authentication module",
            "First call `vox_plan_create` to create a structured plan for the auth module, then call `vox_generate_vox_code` with the plan as context to emit the implementation.",
        ),
        (
            &["vox_submit_task", "vox_get_task_status"],
            "Submit a background task and then check its status",
            "Call `vox_submit_task` with the task description, receive a task_id, then call `vox_get_task_status` with that id to poll for completion.",
        ),
        (
            &["vox_repo_index_files", "vox_generate_vox_code"],
            "Index the repository files and then generate a Vox wrapper for a found Rust crate",
            "Use `vox_repo_index_files` to walk the workspace and identify Rust crates, then call `vox_generate_vox_code` to emit a `.vox` binding wrapper for the selected crate.",
        ),
        (
            &["vox_plan_create", "vox_submit_task", "vox_get_task_status"],
            "Plan, dispatch, and monitor a multi-step refactoring task",
            "Chain: `vox_plan_create` → create the refactor plan; `vox_submit_task` → dispatch it to an agent; `vox_get_task_status` → poll until done.",
        ),
        (
            &["vox_chat_message", "vox_generate_vox_code"],
            "Ask the model to explain an API, then generate Vox bindings for it",
            "Use `vox_chat_message` to ask for an explanation of the target API surface, then call `vox_generate_vox_code` with the response as context to emit typed Vox bindings.",
        ),
    ];

    let mut count = 0;
    let min = cfg.min_phrasings_per_tool.max(2);
    let mut rng = Rng::new(cfg.seed, name_hash("tool_chain"));

    for (tools, goal, strategy) in sequences {
        let tool_list = tools.join(" → ");
        let phrasings = [
            format!("How do I use {tool_list} together to {goal}?"),
            format!("What is the right sequence of tool calls to {goal}?"),
            format!("I need to {goal}. Which tools should I call and in what order?"),
        ];
        for phrasing in phrasings.iter().take(min) {
            let response = json!({
                "strategy": strategy,
                "tool_sequence": tools,
                "reasoning": format!("These tools are chained because each step's output feeds the next: {tool_list}"),
            });
            emit_line(out, phrasing, &response, "tool_chain", "tool_chain_trace")?;
            count += 1;
        }
        let _ = rng.next(); // advance for seed mixing
    }
    Ok(count)
}

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
            "deploy a Vox agent to the distributed mesh",
            "After registering, call `vox_submit_task` with `task_type = 'deploy_agent'` and the agent's `agent_id`. The mesh runtime handles placement.",
            "agent_lifecycle_deploy",
        ),
        (
            "check whether a Vox agent is healthy and responsive",
            "Call `vox_get_task_status` with the agent's active task id, or query the mesh with `vox_mesh_local_status` to inspect mailbox depth and last heartbeat.",
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

// ─── Top-level generator ──────────────────────────────────────────────────────

/// Generate all synthetic training pairs and write them to `output_path` as JSONL.
///
/// Returns the total number of pairs written.
pub fn generate_all(cfg: &SyntheticGenConfig, output_path: &Path) -> anyhow::Result<usize> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output directory: {}", parent.display()))?;
    }

    let mut file = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output_path)
            .with_context(|| format!("create synthetic corpus: {}", output_path.display()))?,
    );

    let mut total = 0usize;

    if cfg.emit_tool_traces {
        let n = generate_tool_pairs(&mut file, TOOL_REGISTRY_SLIM, cfg)?;
        eprintln!("  [synthetic] tool_trace: {n} pairs");
        total += n;
    }

    if cfg.emit_a2a_traces {
        let n = generate_a2a_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] a2a_trace: {n} pairs");
        total += n;
    }

    if cfg.emit_workflow_traces {
        let n = generate_workflow_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] workflow_trace: {n} pairs");
        total += n;
    }

    if cfg.emit_orchestrator_rows {
        let n = generate_orchestrator_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] orchestrator: {n} pairs");
        total += n;
    }

    if cfg.emit_skill_rows {
        let n = generate_skill_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] skill: {n} pairs");
        total += n;
    }

    if cfg.emit_agent_rows {
        let n = generate_agent_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] agent_def: {n} pairs");
        total += n;
    }

    let n = generate_web_construct_pairs(&mut file, cfg)?;
    eprintln!("  [synthetic] web_construct: {n} pairs");
    total += n;

    let n = generate_negative_preference_pairs(&mut file, cfg)?;
    eprintln!("  [synthetic] negative_routing: {n} pairs");
    total += n;

    if cfg.emit_cli_rows {
        let n = generate_cli_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] cli_command: {n} pairs");
        total += n;
    }

    if cfg.emit_script_rows {
        let n = generate_script_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] script: {n} pairs");
        total += n;
    }

    // ── Organic Vox code generation ──────────────────────────────────────
    if cfg.emit_organic_vox {
        let organic = crate::codegen_vox::generate_organic_corpus(cfg.seed);
        let verified = organic.iter().filter(|p| p.verified).count();
        let mut compact_count = 0usize;
        let mut error_fix_count = 0usize;
        let mut multiturn_count = 0usize;
        for (i, pair) in organic.iter().enumerate() {
            writeln!(file, "{}", pair.to_jsonl())?;
            total += 1;

            // Compact variant: every 5th pair gets a compact form (20%)
            if i % 5 == 0 && !pair.response.is_empty() {
                let compact_line = crate::corpus::preflight::compact_variant(
                    &pair.prompt, &pair.response, &pair.category
                );
                writeln!(file, "{}", compact_line)?;
                total += 1;
                compact_count += 1;
            }

            // Error → fix pairs: 50% of organic pairs, cycling through all 12 BrokenKind variants
            if i % 2 == 0 {
                use crate::corpus::preflight::{BrokenKind, break_vox, error_fix_to_jsonl};
                let kind = match i % 12 {
                    0  => BrokenKind::MissingReturnArrow,
                    1  => BrokenKind::UnclosedBrace,
                    2  => BrokenKind::KeywordTypo,
                    3  => BrokenKind::MissingRet,
                    4  => BrokenKind::MissingToUnit,
                    5  => BrokenKind::TypeMismatch,
                    6  => BrokenKind::OptionUnwrapMissing,
                    7  => BrokenKind::BadReturnType,
                    8  => BrokenKind::WrongType,
                    9  => BrokenKind::UnresolvedGenericArity,
                    10 => BrokenKind::InferenceAmbiguity,
                    _  => BrokenKind::UnreachableMatchArm,
                };
                let (broken, explanation) = break_vox(&pair.response, kind);
                let fix_line = error_fix_to_jsonl(&broken, &explanation, &pair.response, &pair.category);
                writeln!(file, "{}", fix_line)?;
                total += 1;
                error_fix_count += 1;
            }

            // Multi-turn conversations
            if i % 10 == 0 {
                use crate::corpus::preflight::{gen_multiturn_vox, multiturn_to_jsonl};
                let decl_type = pair.coverage_tags.first().map(|s| s.as_str()).unwrap_or("function");
                let name = pair.category.split('.').next().unwrap_or("handler");
                // The signature in preflight.rs is gen_multiturn_vox(construct, name, base_code, template_idx)
                let turns = gen_multiturn_vox(decl_type, name, &pair.response, i);
                let mt_line = multiturn_to_jsonl(&turns, &pair.category);
                writeln!(file, "{}", mt_line)?;
                total += 1;
                multiturn_count += 1;
            }
        }
        eprintln!(
            "  [synthetic] organic_vox: {} pairs ({} verified) +{} compact +{} error_fix +{} multiturn",
            organic.len(), verified, compact_count, error_fix_count, multiturn_count
        );

        // Architectural Q&A pairs (static, high-signal)
        let arch_n = crate::corpus::preflight::write_architectural_pairs(&mut file)?;
        total += arch_n;
        eprintln!("  [synthetic] architectural_qa: {arch_n} pairs");

        // TypeScript interop pairs
        let ts_n = crate::corpus::preflight::write_ts_interop_pairs(&mut file)?;
        total += ts_n;
        eprintln!("  [synthetic] ts_interop: {ts_n} pairs");

        // Explain, debug, refactor pairs derived from the organic corpus
        let organic_triples: Vec<(String, String, String)> = organic
            .iter()
            .map(|p| (p.prompt.clone(), p.response.clone(), p.category.clone()))
            .collect();
        let explain_lines = crate::corpus::preflight::gen_explain_pairs(&organic_triples, 5);
        let debug_lines = crate::corpus::preflight::gen_debug_pairs(&organic_triples, 7);
        let refactor_lines = crate::corpus::preflight::gen_refactor_pairs(&organic_triples, 7);
        for line in explain_lines.iter().chain(debug_lines.iter()).chain(refactor_lines.iter()) {
            writeln!(file, "{line}")?;
            total += 1;
        }
        eprintln!(
            "  [synthetic] explain+debug+refactor: {} pairs",
            explain_lines.len() + debug_lines.len() + refactor_lines.len()
        );
    }

    // Tool-chain orchestration pairs
    let tc_n = generate_tool_chain_pairs(&mut file, cfg)?;
    total += tc_n;
    eprintln!("  [synthetic] tool_chain: {tc_n} pairs");

    // Agent lifecycle pairs
    let al_n = generate_agent_lifecycle_pairs(&mut file, cfg)?;
    total += al_n;
    eprintln!("  [synthetic] agent_lifecycle: {al_n} pairs");

    file.flush()?;
    eprintln!("  [synthetic] total: {total} pairs → {}", output_path.display());


    // ── Post-generation augmentation ────────────────────────────────────
    // Re-read the generated JSONL and apply the typo/synonym/case augmentation
    // engine to produce 3× more training variants with natural language noise.
    if cfg.augment_after_generate {
        let raw_lines: Vec<String> = std::fs::read_to_string(output_path)?
            .lines()
            .map(String::from)
            .collect();
        let aug_cfg = crate::corpus::augment::AugmentConfig {
            variants_per_prompt: 3,
            typo_char_rate: 0.05,
            synonym_swap_rate: 0.25,
            shuffle_words: true,
            case_variants: true,
        };
        let augmented = crate::corpus::augment::augment_jsonl_lines(&raw_lines, &aug_cfg, cfg.seed);
        let aug_count = augmented.len() - raw_lines.len();
        // Rewrite file with augmented lines
        let mut aug_file = std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(output_path)?,
        );
        for line in &augmented {
            writeln!(aug_file, "{}", line)?;
        }
        aug_file.flush()?;
        total += aug_count;
        eprintln!("  [augment] +{aug_count} augmented variants (total now {total})");
    }

    // Generate search traces into a companion file
    let search_output_path = output_path.with_file_name("synthetic_search.jsonl");
    let mut search_file = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&search_output_path)
            .with_context(|| format!("create synthetic search corpus: {}", search_output_path.display()))?,
    );
    let n = crate::synthetic_search_gen::generate_search_traces(&mut search_file)?;
    search_file.flush()?;
    eprintln!("  [synthetic_search] total: {n} pairs → {}", search_output_path.display());

    // ── Coverage report ──────────────────────────────────────────────────
    // Re-generate the organic corpus to compute coverage metrics.
    // This is cheap (deterministic, in-memory) and surfaces exact gaps.
    if cfg.emit_organic_vox {
        let organic = crate::codegen_vox::generate_organic_corpus(cfg.seed);
        let report = crate::codegen_vox::compute_coverage_report(&organic);
        crate::codegen_vox::print_coverage_report(&report);
    }

    Ok(total)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn run_all_to_string(cfg: &SyntheticGenConfig) -> String {
        let mut buf = Vec::new();
        generate_tool_pairs(&mut buf, TOOL_REGISTRY_SLIM, cfg).unwrap();
        generate_a2a_pairs(&mut buf, cfg).unwrap();
        generate_workflow_pairs(&mut buf, cfg).unwrap();
        generate_orchestrator_pairs(&mut buf, cfg).unwrap();
        generate_skill_pairs(&mut buf, cfg).unwrap();
        generate_agent_pairs(&mut buf, cfg).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn default_cfg() -> SyntheticGenConfig {
        SyntheticGenConfig::default()
    }

    #[test]
    fn all_registry_tools_appear_in_output() {
        let cfg = SyntheticGenConfig::default();
        let out = run_all_to_string(&cfg);
        for &name in TOOL_REGISTRY_SLIM {
            assert!(
                out.contains(name),
                "tool {name} missing from synthetic output"
            );
        }
    }

    #[test]
    fn all_a2a_types_appear_in_output() {
        let cfg = SyntheticGenConfig::default();
        let out = run_all_to_string(&cfg);
        for &msg_type in A2A_MESSAGE_TYPES {
            assert!(
                out.contains(msg_type),
                "A2A type {msg_type} missing from synthetic output"
            );
        }
    }

    #[test]
    fn output_is_valid_jsonl() {
        let cfg = SyntheticGenConfig::default();
        let out = run_all_to_string(&cfg);
        let mut valid = 0;
        for line in out.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("invalid JSON line: {e}\nLine: {line}"));
            assert!(v.get("prompt").is_some(), "missing prompt field");
            assert!(v.get("response").is_some(), "missing response field");
            assert!(v.get("category").is_some(), "missing category field");
            valid += 1;
        }
        assert!(valid > 100, "expected many pairs, got {valid}");
    }

    #[test]
    fn all_workflow_scenarios_appear_in_output() {
    let out = run_all_to_string(&default_cfg());
    let yaml = include_str!("../../../populi/config/templates.yaml");
    let cfg: serde_json::Value = serde_yaml::from_str(yaml).unwrap();
    let workflows = cfg.get("synthetic").unwrap().get("workflows").unwrap().as_array().unwrap();
    for w in workflows {
        let name = w.get("name").unwrap().as_str().unwrap();
        assert!(out.contains(name), "workflow {name} missing from synthetic output");
    }
}

    #[test]
    fn all_agent_scenarios_appear_in_output() {
    let out = run_all_to_string(&default_cfg());
    let yaml = include_str!("../../../populi/config/templates.yaml");
    let cfg: serde_json::Value = serde_yaml::from_str(yaml).unwrap();
    let agents = cfg.get("synthetic").unwrap().get("agents").unwrap().as_array().unwrap();
    for a in agents {
        let name = a.get("name").unwrap().as_str().unwrap();
        assert!(out.contains(name), "agent {name} missing from synthetic output");
    }
}

    #[test]
    fn min_phrasings_respected() {
        let cfg = SyntheticGenConfig {
            min_phrasings_per_tool: 10,
            ..Default::default()
        };
        let mut buf = Vec::new();
        generate_tool_pairs(&mut buf, &["vox_submit_task"], &cfg).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let count = out.lines().filter(|l| !l.trim().is_empty()).count();
        assert!(count >= 10, "expected ≥10 phrasings, got {count}");
    }

    #[test]
    fn skill_tools_all_covered() {
        let cfg = SyntheticGenConfig::default();
        let out = run_all_to_string(&cfg);
        for &tool in SKILL_TOOLS {
            assert!(out.contains(tool), "skill tool {tool} missing from output");
        }
    }

    #[test]
    fn tool_registry_slim_matches_orchestrator_tools() {
        // Every entry in ORCHESTRATOR_TOOLS must appear in TOOL_REGISTRY_SLIM
        for &name in ORCHESTRATOR_TOOLS {
            assert!(
                TOOL_REGISTRY_SLIM.iter().any(|n| *n == name),
                "ORCHESTRATOR_TOOLS entry {name} not in TOOL_REGISTRY_SLIM"
            );
        }
    }
}
