//! Phase 80 (`REAL-01`, `REAL-02`, `REAL-03`): Hermetic real-backend integration harness.
//!
//! Boots a real `ServerState` backed by an isolated `VoxDb` (`DbConfig::Memory`),
//! exercises `/api/v2/*` HTTP gateway handlers (`api_v2_get` / `api_v2_post`),
//! `vox_chat_session_*`, `gui_preference_*`, `SessionHost` (`LocalHost`), and
//! `vox_cli::command_catalog::build_catalog()` without any browser mock stubs.

use std::sync::Arc;

use serde_json::{Value, json};
use vox_db::{DbConfig, VoxDb};
use vox_orchestrator_mcp::gui_ops::chat::{
    ChatSessionAppendParams, ChatSessionCreateParams, chat_session_append, chat_session_create,
};
use vox_orchestrator_mcp::gui_ops::inproc::{api_v2_get, api_v2_post};
use vox_orchestrator_mcp::gui_ops::prefs::{
    GuiPreferenceGetParams, GuiPreferenceSetParams, gui_preference_get, gui_preference_set,
};
use vox_orchestrator_mcp::server::tool_json_envelope_is_error;
use vox_orchestrator_mcp::{ServerState, handle_tool_call, load_config};
use vox_terminal_core::{BlockOrigin, LocalHost, OpenSpec, TermControlMode, TermSecurityLevel};

async fn boot_hermetic_backend() -> (ServerState, Arc<VoxDb>) {
    let db = Arc::new(
        VoxDb::connect(DbConfig::Memory)
            .await
            .expect("connect isolated in-memory VoxDb"),
    );
    let state = ServerState::new_full(load_config())
        .with_db_initialized(Arc::clone(&db))
        .await;
    (state, db)
}

#[tokio::test]
async fn real_backend_chat_loquela_and_orchestrator_roundtrip_persists_in_voxdb() {
    let (state, _db) = boot_hermetic_backend().await;

    // 1. Create a real chat session in VoxDb
    let created_raw = chat_session_create(
        &state,
        ChatSessionCreateParams {
            title: Some("Phase 80 Real E2E Session".to_string()),
        },
    )
    .await;
    let created: Value = serde_json::from_str(&created_raw).expect("valid JSON");
    assert_eq!(created["success"], true, "create session failed: {created}");
    let session_id = created["data"]["session_id"]
        .as_str()
        .expect("session_id string")
        .to_string();

    // 2. Append user prompt + assistant reply with TurnCostReceipt metadata
    let user_raw = chat_session_append(
        &state,
        ChatSessionAppendParams {
            session_id: session_id.clone(),
            role: "user".to_string(),
            content: "@kb Summarize the repository architecture".to_string(),
            metadata_json: Some(r#"{"already_submitted":true,"kb_scope":true}"#.to_string()),
        },
    )
    .await;
    let user_res: Value = serde_json::from_str(&user_raw).expect("valid JSON");
    assert_eq!(user_res["success"], true, "append user failed: {user_res}");

    let receipt_meta = json!({
        "model_id": "anthropic/claude-sonnet",
        "latency_ms": 184,
        "cost_usd": 0.0014,
        "tokens_in": 412,
        "tokens_out": 128
    })
    .to_string();
    let assistant_raw = chat_session_append(
        &state,
        ChatSessionAppendParams {
            session_id: session_id.clone(),
            role: "assistant".to_string(),
            content: "Vox uses a 5-layer architecture enforced by vox-arch-check.".to_string(),
            metadata_json: Some(receipt_meta),
        },
    )
    .await;
    let assistant_res: Value = serde_json::from_str(&assistant_raw).expect("valid JSON");
    assert_eq!(
        assistant_res["success"], true,
        "append assistant failed: {assistant_res}"
    );

    // 3. Query /api/v2/chat/sessions and /api/v2/chat/sessions/{id}/messages over HTTP router
    let sessions_resp = api_v2_get(&state, "/chat/sessions?limit=50")
        .await
        .expect("GET /api/v2/chat/sessions");
    let sessions = sessions_resp["data"].as_array().expect("sessions array");
    assert!(
        sessions.iter().any(|s| s["session_id"] == session_id),
        "session {session_id} should be persisted in VoxDb: {sessions_resp}"
    );

    let msgs_resp = api_v2_get(
        &state,
        &format!("/chat/sessions/{session_id}/messages?limit=50"),
    )
    .await
    .expect("GET /api/v2/chat/sessions/{id}/messages");
    let msgs = msgs_resp["data"].as_array().expect("messages array");
    assert_eq!(msgs.len(), 2, "expected 2 persisted messages: {msgs_resp}");
    assert_eq!(msgs[0]["role"], "user");
    assert_eq!(msgs[1]["role"], "assistant");
    assert_eq!(msgs[1]["model_id"], "anthropic/claude-sonnet");

    // 4. Verify Loquela oversight level (1..5) and @kb scope preferences persist in VoxDb
    let set_oversight: Value = serde_json::from_str(
        &gui_preference_set(
            &state,
            GuiPreferenceSetParams {
                key: "gui.oversight_level".to_string(),
                value: "4".to_string(),
            },
        )
        .await,
    )
    .unwrap();
    assert_eq!(set_oversight["success"], true);

    let get_oversight: Value = serde_json::from_str(
        &gui_preference_get(
            &state,
            GuiPreferenceGetParams {
                key: "gui.oversight_level".to_string(),
            },
        )
        .await,
    )
    .unwrap();
    assert_eq!(get_oversight["data"], "4");

    // 5. Submit a hopper item over /api/v2/hopper/submit and verify /api/v2/orchestrator/tasks & /subagent-tree
    let submit_resp = api_v2_post(
        &state,
        "/hopper/submit",
        &json!({
            "intent": "Verify real-backend hopper persistence",
            "priority": 1
        }),
    )
    .await
    .expect("POST /api/v2/hopper/submit");
    assert!(
        submit_resp["data"]["item_id"].is_string(),
        "expected item_id in response: {submit_resp}"
    );

    let tasks_resp = api_v2_get(&state, "/orchestrator/tasks")
        .await
        .expect("GET /api/v2/orchestrator/tasks");
    assert!(
        tasks_resp["data"]["tasks"].is_array() || tasks_resp["data"].is_array(),
        "expected tasks payload: {tasks_resp}"
    );

    let tree_resp = api_v2_get(&state, "/orchestrator/subagent-tree")
        .await
        .expect("GET /api/v2/orchestrator/subagent-tree");
    assert!(
        tree_resp["data"].is_object() || tree_resp["data"].is_array(),
        "expected subagent-tree payload: {tree_resp}"
    );
}

#[tokio::test]
async fn real_backend_terminal_session_host_and_command_catalog_roundtrip() {
    let (state, _db) = boot_hermetic_backend().await;

    // 1. Terminal SessionHost: open -> submit -> attach -> secret redaction (REAL-03)
    let host = LocalHost::default();
    let sid = host
        .open(OpenSpec::default())
        .await
        .expect("open PTY session");
    host.set_security_level(&sid, TermSecurityLevel::L3StructuredDefault)
        .expect("set security level");
    host.set_control_mode(&sid, TermControlMode::Autopilot)
        .expect("set autopilot");

    // User submit automatically drops Autopilot -> Manual
    let b1 = host
        .submit(&sid, "!echo real-terminal-ok")
        .expect("submit echo");
    assert_eq!(host.control_mode(&sid).unwrap(), TermControlMode::Manual);

    // Submit command containing a secret bearer token and verify context redaction
    let _b2 = host
        .submit_with_origin(
            &sid,
            "!echo Authorization: Bearer sk-abcdefghijklmnopqrstuvwxyz123456",
            BlockOrigin::Agent {
                agent_id: "agent-real-e2e".to_string(),
            },
        )
        .expect("submit agent block");

    let attached = host.attach(&sid).expect("attach session");
    assert_eq!(attached.blocks.len(), 2);
    assert_eq!(attached.blocks[0].id, b1);
    assert!(
        attached.blocks[0]
            .plain_output()
            .contains("real-terminal-ok")
    );

    let redacted = host
        .redacted_context_bundle(&sid, 10)
        .expect("redacted context bundle");
    assert_eq!(redacted.len(), 2);
    assert!(
        !redacted[1]
            .output
            .contains("sk-abcdefghijklmnopqrstuvwxyz123456"),
        "secret token must be scrubbed from terminal context bundle"
    );
    assert!(redacted[1].output.contains("[REDACTED_KEY]"));
    host.close(&sid).expect("close session");

    // 2. Command Catalog (⌘K Palette + Explorer) & live read-only MCP tool execution (REAL-03)
    let catalog = vox_cli::command_catalog::build_catalog();
    assert!(
        catalog.entries.len() > 20,
        "expected populated CLI command catalog, got {}",
        catalog.entries.len()
    );
    assert!(
        catalog.entries.iter().any(|e| e.command == "vox check"),
        "catalog must include `vox check`"
    );

    let git_status_raw = handle_tool_call(&state, "vox_git_status", json!({}))
        .await
        .expect("dispatch vox_git_status");
    assert!(
        !tool_json_envelope_is_error(&git_status_raw),
        "expected non-error vox_git_status envelope: {git_status_raw}"
    );
}

#[tokio::test]
async fn real_backend_operational_surfaces_settings_models_approvals_policies_subagents_roundtrip()
{
    let (state, _db) = boot_hermetic_backend().await;

    // 1. OPS-01: Settings / Clavis secrets redaction + .env dry-run import + Oratio/LLM preferences + spend
    let secret_rows = vox_secrets::list_secret_status();
    assert!(
        !secret_rows.is_empty(),
        "expected managed secret specs from Clavis"
    );
    for row in &secret_rows {
        assert!(
            !row.canonical_env.is_empty(),
            "canonical_env must be populated"
        );
        assert!(
            row.redacted == "(missing)" || row.redacted.contains("redacted"),
            "secret preview must be redacted or (missing), got: {}",
            row.redacted
        );
    }

    let tmp_dir = tempfile::tempdir().expect("tempdir for .env dry-run");
    let env_path = tmp_dir.path().join(".env");
    std::fs::write(
        &env_path,
        "OPENROUTER_API_KEY=sk-or-v1-1234567890abcdef1234567890abcdef\n",
    )
    .expect("write temp .env");
    let dry_import =
        vox_secrets::import_env_from_path(&env_path, false).expect("dry-run import_env_from_path");
    assert!(!dry_import.applied);
    assert_eq!(dry_import.count(), 1);
    assert_eq!(dry_import.entries[0].canonical_env, "OPENROUTER_API_KEY");
    assert!(
        !dry_import.entries[0]
            .redacted
            .contains("1234567890abcdef1234567890abcdef"),
        "raw secret value must never leak in import preview"
    );

    // Oratio STT preference round-trip in VoxDb
    let set_stt: Value = serde_json::from_str(
        &gui_preference_set(
            &state,
            GuiPreferenceSetParams {
                key: "gui.stt_engine".to_string(),
                value: "sherpa".to_string(),
            },
        )
        .await,
    )
    .unwrap();
    assert_eq!(set_stt["success"], true);
    let get_stt: Value = serde_json::from_str(
        &gui_preference_get(
            &state,
            GuiPreferenceGetParams {
                key: "gui.stt_engine".to_string(),
            },
        )
        .await,
    )
    .unwrap();
    assert_eq!(get_stt["data"], "sherpa");

    let spend_resp = api_v2_get(&state, "/llm/spend")
        .await
        .expect("GET /api/v2/llm/spend");
    assert!(
        spend_resp["data"]["dailyBudgetUsd"].is_number(),
        "expected numeric dailyBudgetUsd: {spend_resp}"
    );

    // 2. OPS-02: ModelsView / Mens routing preferences + snapshot + provider status
    let set_routing_raw = vox_orchestrator_mcp::gui_ops::models::models_preference_set(
        &state,
        vox_orchestrator_mcp::gui_ops::models::ModelsPreferenceSetParams {
            key: "routing_priority".to_string(),
            value: "efficiency=80,precision=90".to_string(),
        },
    )
    .await;
    let set_routing: Value = serde_json::from_str(&set_routing_raw).unwrap();
    assert_eq!(
        set_routing["success"], true,
        "models_preference_set failed: {set_routing}"
    );

    let prefs_resp = api_v2_get(&state, "/models/preferences")
        .await
        .expect("GET /api/v2/models/preferences");
    assert_eq!(prefs_resp["data"]["routing_priority"]["efficiency"], 80);
    assert_eq!(prefs_resp["data"]["routing_priority"]["precision"], 90);

    let snapshot_resp = api_v2_get(&state, "/models/snapshot?limit=10")
        .await
        .expect("GET /api/v2/models/snapshot");
    assert!(
        snapshot_resp["data"]["cards"].is_array(),
        "expected cards array in /models/snapshot: {snapshot_resp}"
    );
    assert!(
        snapshot_resp["data"]["providers"].is_array(),
        "expected providers array in /models/snapshot: {snapshot_resp}"
    );

    let routing_summary_resp = api_v2_get(&state, "/models/routing-summary")
        .await
        .expect("GET /api/v2/models/routing-summary");
    assert!(
        routing_summary_resp["data"].is_object(),
        "expected object in /models/routing-summary: {routing_summary_resp}"
    );

    // 3. OPS-03: Approvals, Hopper queue mutations, Policies registry, SubAgents tree
    let hop_submit_raw = vox_orchestrator_mcp::gui_ops::orchestrator::hopper_submit(
        &state,
        vox_orchestrator_mcp::gui_ops::orchestrator::HopperSubmitParams {
            intent: "OPS-03 Hopper Lifecycle Verification".to_string(),
            affinity: vec!["rust".to_string()],
        },
    )
    .await;
    let hop_submit: Value = serde_json::from_str(&hop_submit_raw).unwrap();
    assert_eq!(hop_submit["success"], true, "{hop_submit}");
    let item_id = hop_submit["data"]["item_id"]
        .as_str()
        .expect("item_id")
        .to_string();

    let hop_reprio_raw = vox_orchestrator_mcp::gui_ops::orchestrator::hopper_reprioritize(
        &state,
        vox_orchestrator_mcp::gui_ops::orchestrator::HopperReprioritizeParams {
            item_id: item_id.clone(),
            priority: 2,
        },
    )
    .await;
    let hop_reprio: Value = serde_json::from_str(&hop_reprio_raw).unwrap();
    assert_eq!(hop_reprio["success"], true, "{hop_reprio}");
    assert_eq!(hop_reprio["data"]["priority"], 2);

    let hop_done_raw = vox_orchestrator_mcp::gui_ops::orchestrator::hopper_mark_done(
        &state,
        vox_orchestrator_mcp::gui_ops::orchestrator::HopperItemParams {
            item_id: item_id.clone(),
        },
    )
    .await;
    let hop_done: Value = serde_json::from_str(&hop_done_raw).unwrap();
    assert_eq!(hop_done["success"], true, "{hop_done}");
    assert_eq!(hop_done["data"]["state"], "done");

    let hop_list = api_v2_get(&state, "/hopper/list")
        .await
        .expect("GET /api/v2/hopper/list");
    assert!(
        hop_list["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["item_id"] == item_id && row["state"] == "done"),
        "completed item must appear in /hopper/list recent window: {hop_list}"
    );

    let approvals_resp = api_v2_get(&state, "/orchestrator/approvals")
        .await
        .expect("GET /api/v2/orchestrator/approvals");
    assert!(
        approvals_resp["data"]["approvals"].is_array(),
        "expected approvals array: {approvals_resp}"
    );

    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let policy_reg =
        vox_config::load_policy_registry(&repo_root).expect("load_policy_registry on repo_root");
    assert!(
        !policy_reg.policies.is_empty(),
        "policy registry must contain real policy rules"
    );
}

#[tokio::test]
async fn real_backend_data_surfaces_kb_research_scientia_atlas_mesh_and_telemetry_roundtrip() {
    let (state, db) = boot_hermetic_backend().await;
    vox_gamify::db::apply_ludus_migrations(&db)
        .await
        .expect("apply ludus migrations");

    // 1. DATA-01: Knowledge Base (MemoryView / Library) ingest -> nodes -> health -> recall -> delete
    let ingest_raw = vox_orchestrator_mcp::gui_ops::knowledge::kb_ingest_text(
        &state,
        vox_orchestrator_mcp::gui_ops::knowledge::KbIngestTextParams {
            title: "Phase 83 Architecture Note".to_string(),
            content: "Vox enforces a strict 5-layer crate dependency architecture.".to_string(),
            source_url: Some("https://vox.dev/docs/architecture".to_string()),
        },
    )
    .await;
    let ingest_res: Value = serde_json::from_str(&ingest_raw).unwrap();
    assert_eq!(ingest_res["success"], true, "{ingest_res}");
    let node_id = ingest_res["data"].as_str().expect("node_id").to_string();

    let kb_nodes = api_v2_get(&state, "/knowledge/nodes?limit=20")
        .await
        .expect("GET /api/v2/knowledge/nodes");
    assert!(
        kb_nodes["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["id"] == node_id),
        "ingested KB node must appear in /knowledge/nodes: {kb_nodes}"
    );

    let kb_health = api_v2_get(&state, "/knowledge/health")
        .await
        .expect("GET /api/v2/knowledge/health");
    assert!(
        kb_health["data"]["node_count"].as_u64().unwrap_or(0) >= 1,
        "expected node_count >= 1: {kb_health}"
    );

    let kb_counts = api_v2_get(&state, "/knowledge/corpus-counts")
        .await
        .expect("GET /api/v2/knowledge/corpus-counts");
    assert!(kb_counts["data"].is_object(), "{kb_counts}");

    let kb_del_raw = vox_orchestrator_mcp::gui_ops::knowledge::kb_node_delete(
        &state,
        vox_orchestrator_mcp::gui_ops::knowledge::KbNodeDeleteParams { id: node_id },
    )
    .await;
    let kb_del: Value = serde_json::from_str(&kb_del_raw).unwrap();
    assert_eq!(kb_del["success"], true, "{kb_del}");

    // 2. DATA-02: Deep Research session + detail + engine-status
    let r_sid = db
        .create_research_session("phase83-research", "How does Vox verify claims?")
        .await
        .expect("create_research_session");
    let r_sessions = api_v2_get(&state, "/research/sessions?limit=10")
        .await
        .expect("GET /api/v2/research/sessions");
    assert!(
        r_sessions["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == r_sid),
        "expected research session {r_sid} in list: {r_sessions}"
    );

    let r_detail = api_v2_get(&state, &format!("/research/sessions/{r_sid}"))
        .await
        .expect("GET /api/v2/research/sessions/{id}");
    assert_eq!(
        r_detail["data"]["session"]["query_text"],
        "How does Vox verify claims?"
    );

    let r_engine = api_v2_get(&state, "/research/engine-status")
        .await
        .expect("GET /api/v2/research/engine-status");
    assert!(
        r_engine["data"]["providers"].is_array(),
        "expected providers array in /research/engine-status: {r_engine}"
    );

    // 3. DATA-03: Scientia discovery inbox + manifests + dashboard + queue + cost
    let disc_id = db
        .insert_discovery_inbox(
            "pub-phase83",
            1_700_000_000,
            "strong_candidate",
            r#"["research_pipeline.supported_claims"]"#,
        )
        .await
        .expect("insert_discovery_inbox");
    let discoveries = api_v2_get(&state, "/scientia/discoveries")
        .await
        .expect("GET /api/v2/scientia/discoveries");
    assert!(
        discoveries["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["publication_id"] == "pub-phase83"),
        "expected pub-phase83 in /scientia/discoveries: {discoveries}"
    );

    let ack_raw = vox_orchestrator_mcp::gui_ops::scientia::discovery_ack(
        &state,
        vox_orchestrator_mcp::gui_ops::scientia::DiscoveryAckParams { id: disc_id },
    )
    .await;
    let ack: Value = serde_json::from_str(&ack_raw).unwrap();
    assert_eq!(ack["success"], true, "{ack}");

    let sci_dash = api_v2_get(&state, "/scientia/dashboard")
        .await
        .expect("GET /api/v2/scientia/dashboard");
    assert!(sci_dash["data"]["queue"].is_object(), "{sci_dash}");
    assert!(sci_dash["data"]["cost"].is_object(), "{sci_dash}");

    // 4. DATA-04: Code Atlas graphify contract + Mesh topology + VCS isolation
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let graph_reg =
        vox_config::graphify::load_graphify_corpora(&repo_root).expect("load_graphify_corpora");
    assert_eq!(graph_reg.default_corpus_id, "repo-code-graph");
    assert!(!graph_reg.corpora.is_empty());

    let mesh_resp = api_v2_get(&state, "/mesh/nodes")
        .await
        .expect("GET /api/v2/mesh/nodes");
    assert!(mesh_resp["data"]["nodes"].is_array(), "{mesh_resp}");

    let vcs_resp = api_v2_get(&state, "/vcs/isolation")
        .await
        .expect("GET /api/v2/vcs/isolation");
    assert_eq!(vcs_resp["data"]["strategy_default"], "shared_branch");

    // 5. DATA-05: Telemetry / Runs / Activity / Gamify live persistence
    let runs_resp = api_v2_get(&state, "/runs/gui")
        .await
        .expect("GET /api/v2/runs/gui");
    assert!(runs_resp["data"].is_array(), "{runs_resp}");

    let activity_resp = api_v2_get(&state, "/activity")
        .await
        .expect("GET /api/v2/activity");
    assert!(activity_resp["data"].is_array(), "{activity_resp}");

    let gamify_ev_raw = vox_orchestrator_mcp::gui_ops::gamify::gamify_record_event(
        &state,
        vox_orchestrator_mcp::gui_ops::gamify::GamifyRecordEventParams {
            event: json!({ "type": "task_completed", "source": "gui" }),
        },
    )
    .await;
    let gamify_ev: Value = serde_json::from_str(&gamify_ev_raw).unwrap();
    assert_eq!(gamify_ev["success"], true, "{gamify_ev}");

    let gamify_prof_raw = vox_orchestrator_mcp::gui_ops::gamify::gamify_profile(
        &state,
        vox_orchestrator_mcp::gui_ops::gamify::GamifyProfileParams {},
    )
    .await;
    let gamify_prof: Value = serde_json::from_str(&gamify_prof_raw).unwrap();
    assert_eq!(gamify_prof["success"], true, "{gamify_prof}");
    assert!(
        gamify_prof["data"]["total_xp_earned"].as_u64().unwrap_or(0) > 0,
        "expected positive XP after recording task_completed: {gamify_prof}"
    );
}
