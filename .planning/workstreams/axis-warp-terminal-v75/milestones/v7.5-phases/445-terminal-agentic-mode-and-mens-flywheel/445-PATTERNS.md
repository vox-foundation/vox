# Phase 445: Terminal Agentic Mode, Safety Tiers, and MENS Flywheel - Pattern Map

**Mapped:** 2026-11-15
**Files analyzed:** 35 (33 git-tracked analogs verified via `git ls-files --error-unmatch`; 1 untracked working-tree file flagged per `#3645`)
**Analogs found:** 26 / 26 planned targets

---

## Critical API Signature Verification (Anti-Hallucination Ledger)

Direct inspection of `HEAD` uncovered **7 exact signature/type differences** vs prose sketches in `445-RESEARCH.md`. Planners and executors MUST use the **Actual HEAD Signature** column below:

| Item | Sketch in `445-RESEARCH.md` | Actual HEAD Signature (Verified) | Source Location |
|---|---|---|---|
| `LlmConfig::direct` return type | `Result<LlmConfig, String>` | `pub fn direct(provider: impl Into<String>, model: impl Into<String>) -> Self` (infallible `Self`; egress/key validation runs inside `llm_stream_activity` / `llm_chat` via `vox_config::resolve_egress::resolve_egress`) | `crates/vox-actor-runtime/src/llm/types.rs:212-217` |
| `llm_stream_activity` args & stream trait | `llm_stream_activity(&config, &messages, &opts)` | `pub async fn llm_stream_activity(options: &ActivityOptions, messages: Vec<LlmChatMessage>, config: LlmConfig) -> ActivityResult<Pin<Box<dyn tokio_stream::Stream<Item = Result<String, String>> + Send>>>` — `options` is **arg 1**, `messages` & `config` are **owned**, and `vox-gui` does NOT depend on `tokio-stream` directly so consume via `futures_util::StreamExt::next(&mut stream)` without naming `tokio_stream::Stream` in type annotations | `crates/vox-actor-runtime/src/llm/stream.rs:131-135`, `crates/vox-gui/Cargo.toml:37` |
| `llm_chat` signature | `llm_chat(&config, &messages)` | `pub async fn llm_chat(options: &ActivityOptions, messages: Vec<LlmChatMessage>, config: LlmConfig) -> ActivityResult<Result<LlmResponse, String>>` (`LlmResponse.content: String`) | `crates/vox-actor-runtime/src/llm/chat.rs:105-109` |
| Local Clavis `SecretId` & provider probe | `SecretId::VoxLocalEndpoint` (does not exist) | `SecretId::VoxPopuliLocalOllamaUrl`, `SecretId::OllamaUrl`, `SecretId::VoxOrchestratorPopuliInferenceBaseUrl`, `SecretId::VoxModel`, plus `vox_orchestrator_mcp::gui_ops::models::provider_status().await` (`Vec<ProviderStatusDto>`) | `crates/vox-secrets/src/spec/ids.rs:18-20,99,213`, `crates/vox-orchestrator-mcp/src/gui_ops/models.rs:439-469` |
| `vox-gui` slash helper names | `get_context_budget_impl`, `get_llm_spend_snapshot_impl`, `get_memory_status_impl`, `mnemosyne_recall_impl` | `context_budget_impl(b, session_id)` (`context_budget.rs:76`), `get_llm_spend_impl(b, session_id)` (`llm_spend.rs:13`), `get_active_model_impl(b)` / `set_active_model_impl(b, id)` (`model_active.rs:35,47`), `memory_status_impl(b)` / `recall_impl(b, query, limit)` (`memory.rs:97,104`) | `crates/vox-gui/src/commands/{context_budget,llm_spend,model_active,memory}.rs` |
| `BlockKind`, `Stream`, `TranscriptKind` | `BlockKind::Meta`, `Stream::Meta`, `TranscriptKind::AgentTurn { prompt, reply }`, `SessionOpened` | `BlockKind`: `VoxNative \| Shell \| AgentTurn \| SlashCommand \| System` (`block.rs:7-13`); `Stream`: `Stdout \| Stderr \| Agent` (`block.rs:25-29`); `TranscriptKind`: `Submitted { intent, input } \| Output { stream, text } \| AgentTurn { text } \| ExitStatus { code } \| Accepted { block } \| Rejected { block } \| Corrected { from, to }` (`transcript.rs:13-41`) | `crates/vox-terminal-core/src/{block.rs,transcript.rs}` |
| `LocalHost` transcript/corpus wiring + approval test-id | Assumed `LocalHost` already wrote transcripts; research used `terminal-approval-card` | `LocalHost` (`host.rs:338-357`) currently has **no** `TranscriptSink` field (`Entry` uses `NullSink`!). Must add `with_transcript_sink` on `LocalHost` and wire a redacted sink in `vox_orchestrator_d.rs:287` + `pty.rs:135`. Locked `445-CONTEXT.md` D-04 test-id is `data-testid="term-approval-card"` | `crates/vox-terminal-core/src/host.rs:338-357`, `445-CONTEXT.md` D-03/D-04 |

> **`#3645` Tracked-Source Gate Note:** `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` is currently untracked (`??`) in the working tree from an uncommitted prior phase. All pattern anchors below use **git-tracked** files (`TurnTrace.tsx`, `BlockCard.tsx`, `ApprovalsView.tsx`, `RomanSurfaceHeader.tsx`); treat `ChatTerminalBlockCard.tsx` conditionally (integrate if present on branch, never hard-assume git history).

---

## File Classification

Every created or modified source file MUST stay `< 450` non-blank LoC (`D-10`). Note `crates/vox-gui/src/commands/pty.rs` is currently 856 total lines (562 prod + 294 test lines); extracting `#[cfg(test)] mod tests;` into `crates/vox-gui/src/commands/pty/tests.rs` drops `pty.rs` below 450 non-blank LoC while preserving 3-segment `commands::pty::<cmd>` registration in `axis_invoke_handler!`.

| New / Modified File | Role | Data Flow | Est. Non-Blank LoC | Closest Tracked Analog | Match Quality |
|---|---|---|---|---|---|
| `crates/vox-terminal-core/src/session_agent.rs` *(new)* | `Session` methods for `begin_agent_block`, `append_agent_chunk`, `finish_agent_block`, `record_slash_output`, `clear_unpinned_blocks`, `compact_blocks`, `record_accepted`, `record_rejected`, `record_corrected` | Event-driven state + `TranscriptSink` | ~190 | `crates/vox-terminal-core/src/session.rs` | exact |
| `crates/vox-terminal-core/src/session.rs` *(mod)* | Declare `mod session_agent;` + expose `blocks_mut` / `emit` / `sink` helpers | State machine | ~420 (trim if needed) | `crates/vox-terminal-core/src/session.rs` | self |
| `crates/vox-terminal-core/src/host_agent.rs` *(new)* | `LocalHost` & `SessionHost` extension ops (`agent_block_*`, `clear_blocks`, `compact_blocks`, `record_feedback`, `PreemptedSummary`, `Autopilot -> Manual` preemption) | Sync mutex + broadcast | ~210 | `crates/vox-terminal-core/src/host.rs` | exact |
| `crates/vox-terminal-core/src/host.rs` *(mod)* | Add `transcript_sink: Option<Arc<dyn TranscriptSink>>` + `with_transcript_sink`, hook `set_control_mode` preemption, extend `SessionHost` trait | Host coordinator | ~430 (keep tests in submodule if >450) | `crates/vox-terminal-core/src/host.rs` | self |
| `crates/vox-terminal-core/src/daemon_host_ops.rs` *(new)* | `DaemonHost` RPC methods (`term.agent_block_begin/append/end`, `term.clear`, `term.compact`, `term.feedback`) | JSON-line RPC client | ~150 | `crates/vox-terminal-core/src/daemon_host.rs` | exact |
| `crates/vox-terminal-core/src/daemon_host.rs` *(mod)* | Add missing `Some("snapshot")` arm in `DaemonHost::route` (`L149-309`) + delegate `SessionHost` ops to `daemon_host_ops` | Stream router | ~410 | `crates/vox-terminal-core/src/daemon_host.rs` | self |
| `crates/vox-terminal-core/src/commands.rs` *(mod)* | Add `/clear` and `/compact` specs to `CommandRegistry::default()` while keeping standalone unit-test parity (`model_with_arg`) | Pure registry | ~215 | `crates/vox-terminal-core/src/commands.rs` | self |
| `crates/vox-terminal-core/src/corpus/redact.rs` *(mod)* | Add `redact_event_with(&TranscriptEvent, &dyn Fn(&str) -> String) -> TranscriptEvent` scrubbing ALL `TranscriptKind` variants (fixing `Accepted`/`Rejected` `block.{input,output,cwd,git_branch}` leak) | Pure transform | ~130 | `crates/vox-terminal-core/src/corpus/writer.rs` | exact |
| `crates/vox-terminal-core/src/corpus/writer.rs` *(mod)* | Route `write_event_with` through `redact_event_with`; add `DualCorpusSink` (`JournalSink` + curated `CorpusWriter<File>`) | File / JSONL I/O | ~160 | `crates/vox-terminal-core/src/transcript.rs` | exact |
| `crates/vox-terminal-core/src/corpus/curate.rs` *(mod)* | Retain non-empty `TranscriptKind::Rejected { block }` in `DefaultPolicy::decide` as negative DPO signal (`D-08`) | Pure policy | ~155 | `crates/vox-terminal-core/src/corpus/curate.rs` | self |
| `crates/vox-orchestrator-d/src/term_session_ops.rs` *(new)* | Daemon handlers for `term.agent_block_*`, `term.clear`, `term.compact`, `term.feedback` + `DualCorpusSink` factory with `vox_secrets::scrub_resolved_secrets` | Daemon RPC handler | ~180 | `crates/vox-orchestrator-d/src/term_dispatch.rs` | exact |
| `crates/vox-orchestrator-d/src/term_dispatch.rs` *(mod)* | Route new `term.*` methods to `term_session_ops`; wire `with_transcript_sink` in `vox_orchestrator_d.rs` | RPC dispatcher | ~430 (prod) | `crates/vox-orchestrator-d/src/term_dispatch.rs` | self |
| `crates/vox-orchestrator-mcp/src/pending_approvals.rs` *(mod)* | Extend `PendingApprovalInfo` with optional terminal fields (`session_id`, `command`, `control_mode`, `credential_prompt`, `floor_category`) + `PendingResolution` (`outcome`, `modified_command`, `steering_note`, `allow_pattern`) | Async oneshot registry | ~230 | `crates/vox-orchestrator-mcp/src/pending_approvals.rs` | self |
| `crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs` *(mod)* | Extend `vox_resolve_approval` JSON schema with optional `modified_command`, `steering_note`, `allow_pattern` (schema has `"additionalProperties": false`!) | JSON Schema | ~220 | `crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs` | self |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs` + `src/daemon_extra.rs` *(mod)* | Forward `PendingResolution` on **both** `vox_resolve_approval` MCP dispatch (`tasks_core.rs:123`) and `orch_daemon_method::RESOLVE_APPROVAL` (`daemon_extra.rs:261`) | Dual-path RPC | ~310 | `crates/vox-orchestrator-mcp/src/daemon_extra.rs` | self |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` *(mod)* | Populate terminal metadata on `PendingApprovals::request_with_meta`, re-validate `modified_command` against `allows_agent_command` + `classify_floor` (`Edit & Run`), persist `allow_pattern` for session, attach `steering_note` on reject | Tool gate + PTY bridge | ~440 | `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` | self |
| `crates/vox-gui/src/commands/pty/agent.rs` *(new)* | `term_agent_prompt_impl` & `term_agent_explain_fix_impl`: Clavis/provider resolution -> `LlmConfig::direct` -> `llm_stream_activity` -> scrubbed context -> `AgentTurn` block chunks + proposed fix extraction | Streaming Tauri IPC | ~260 | `crates/vox-gui/src/commands/console_a2a.rs` + `mens_serve.rs` | role-match |
| `crates/vox-gui/src/commands/pty/slash.rs` *(new)* | `term_slash_exec_impl`: live `/model`, `/budget`, `/memory`, `/skills`, `/context`, `/clear`, `/compact` backed by `BackendCalls` + `llm_chat` compaction | Request-response IPC | ~230 | `crates/vox-gui/src/commands/{model_active,context_budget,llm_spend,memory}.rs` | exact |
| `crates/vox-gui/src/commands/pty/approvals.rs` *(new)* | `term_block_feedback_impl` (accept/reject/correct) + optional terminal approval resolve helper | Request-response IPC | ~120 | `crates/vox-gui/src/commands/mission_control.rs` | exact |
| `crates/vox-gui/src/commands/pty/tests.rs` *(new)* + `pty.rs` *(mod)* | Move 294 test lines from `pty.rs` to `pty/tests.rs`; declare `pub mod agent; pub mod approvals; pub mod slash;` + thin `#[tauri::command]` wrappers registered in `main.rs` (`commands::pty::term_*`) | Tauri command surface | ~390 (`pty.rs`) | `crates/vox-gui/src/commands/pty.rs` | self |
| `crates/vox-gui/ui/src/transport.ts` *(mod)* | Add `termAgentPrompt`, `termAgentExplainFix`, `termSlashExec`, `termBlockFeedback` + extended `PendingApprovalRow` fields in `mcpToolResult.ts` | Typed IPC client | +55 lines | `crates/vox-gui/ui/src/transport.ts:1029-1075` | self |
| `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx` *(new)* | Renders `AgentTurn` / `Explain & Fix` diagnosis (`data-testid="term-explain-fix-block"`), Proposed Command chip (`term-proposed-fix-run`, `term-proposed-fix-edit`), and MENS feedback strip (`term-feedback-accept`, `term-feedback-reject`, `term-feedback-correct`) | React component | ~180 | `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx` *(new)* | Inline HITL card (`data-testid="term-approval-card"`) with `RomanWaxSealBadge`, command preview, `Approve & Run`, `Edit & Run`, `Allow Pattern for Session`, `Reject + Note` | React component | ~195 | `crates/vox-gui/ui/src/components/surfaces/Approvals/ApprovalsView.tsx` | role-match |
| `crates/vox-gui/ui/src/components/surfaces/Console/useTerminalPendingApprovals.ts` *(new)* | Polls `voxTransport.invokeMcpTool('vox_pending_approvals', {})`, filters by `session_id` or `vox_terminal_*` tool, resolves via `vox_resolve_approval` | React hook | ~110 | `crates/vox-gui/ui/src/components/surfaces/Approvals/ApprovalsView.tsx:77-130` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/{BlockCard,BlockStreamView,TerminalSessionPane,Console}.tsx` *(mod)* | Wire `Explain & Fix` (`Failed` blocks), `AgentTurnContent`, `TerminalApprovalCard`, `@` / `/ai` -> `termAgentPrompt`, live slash commands -> `termSlashExec`, `Autopilot -> Manual` toast | Surface wiring | < 420 each | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` | self |
| `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx` *(mod)* | Surface inline `TerminalApprovalCard` badge/state when `vox_terminal_*` tool call is awaiting approval | Chat <-> Console parity | ~145 | `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx` | self |

---

## Pattern Assignments

### 1. Model-Agnostic LLM Streaming & Clavis Provider Resolution (`crates/vox-gui/src/commands/pty/agent.rs`)

**Analogs:**
- `crates/vox-actor-runtime/src/llm/types.rs:148-225` (`LlmChatMessage`, `LlmConfig::direct`)
- `crates/vox-actor-runtime/src/llm/stream.rs:131-145` (`llm_stream_activity`)
- `crates/vox-actor-runtime/src/llm/chat.rs:105-115` (`llm_chat`)
- `crates/vox-orchestrator-mcp/src/gui_ops/models.rs:439-469` (`provider_status`)
- `crates/vox-gui/src/commands/model_active.rs:27-45` (`get_active_model_impl`, `SecretId::VoxModel`)
- `crates/vox-gui/src/commands/console_a2a.rs:28-33` (`scrub_for_agent_inbox`)

**Exact Imports & Signatures Pattern:**
```rust
use futures_util::StreamExt;
use vox_actor_runtime::ActivityOptions;
use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, llm_chat, llm_stream_activity};
use vox_secrets::{SecretId, resolve_secret};

use crate::commands::console_a2a::scrub_for_agent_inbox;
use crate::commands::model_active::get_active_model_impl;
use crate::remote::calls::BackendCalls;
```

**Clavis-Backed Provider Cascade (`D-01` — Zero Hardcoded Vendor URLs):**
`vox_config::resolve_egress::resolve_egress` (called inside `llm_stream_activity` and `llm_chat`) accepts provider slugs `"ollama"` (used for local OpenAI-compatible/Ollama endpoints including `MensServeState` via `base_url`), `"openrouter"`, `"anthropic"`, `"openai"`, `"gemini"`, `"groq"`, `"mistral"`, `"deepseek"`.
```rust
pub struct ResolvedTerminalModel {
    pub config: LlmConfig,
    pub display_label: String,
}

pub async fn resolve_terminal_llm_config<B: BackendCalls>(
    backend: &B,
    mens_port: Option<u16>,
) -> Result<ResolvedTerminalModel, String> {
    let preferred_model = get_active_model_impl(backend).await.ok().flatten();

    // 1. Running local MENS serve supervised by Axis GUI (`MensServeState`)
    if let Some(port) = mens_port {
        let model = preferred_model.unwrap_or_else(|| "mens-default".to_string());
        let mut cfg = LlmConfig::direct("ollama", model.clone())
            .with_max_tokens(1024)
            .with_temperature(0.2);
        cfg.base_url = Some(format!("http://127.0.0.1:{port}/v1/chat/completions"));
        return Ok(ResolvedTerminalModel {
            config: cfg,
            display_label: format!("local:mens:{model}"),
        });
    }

    // 2. Local Populi / vox-ml-cli / Ollama probe + Clavis cloud fallback via shared MCP ops
    let statuses = vox_orchestrator_mcp::gui_ops::models::provider_status().await;
    for st in &statuses {
        if st.is_local && st.local_reachable == Some(true) {
            let model = preferred_model
                .clone()
                .or_else(|| st.local_models.first().cloned())
                .unwrap_or_else(|| "qwen2.5-coder:7b".to_string());
            let cfg = LlmConfig::direct("ollama", model.clone())
                .with_max_tokens(1024)
                .with_temperature(0.2);
            return Ok(ResolvedTerminalModel {
                config: cfg,
                display_label: format!("local:{}:{model}", st.provider.to_lowercase()),
            });
        }
    }

    // 3. Cloud providers with Clavis keys present (`resolve_egress` resolves Clavis SecretId internally)
    let cloud_candidates: &[(&str, SecretId, &str)] = &[
        ("openrouter", SecretId::OpenRouterApiKey, "anthropic/claude-3.5-haiku"),
        ("anthropic", SecretId::AnthropicApiKey, "claude-3-5-haiku-latest"),
        ("gemini", SecretId::GeminiApiKey, "gemini-2.5-flash"),
        ("openai", SecretId::OpenaiApiKey, "gpt-4o-mini"),
        ("groq", SecretId::GroqApiKey, "llama-3.3-70b-versatile"),
    ];
    for (provider_slug, secret_id, default_model) in cloud_candidates {
        if resolve_secret(*secret_id).is_present() {
            let model = preferred_model
                .clone()
                .unwrap_or_else(|| (*default_model).to_string());
            let cfg = LlmConfig::direct(*provider_slug, model.clone())
                .with_max_tokens(1024)
                .with_temperature(0.2);
            return Ok(ResolvedTerminalModel {
                config: cfg,
                display_label: format!("{provider_slug}:{model}"),
            });
        }
    }

    Err("No LLM provider configured. Start local MENS (`vox mens serve`) or configure an API key in Clavis (`vox clavis set`).".to_string())
}
```

**Streaming Consumption Pattern (No `tokio_stream` Direct Dependency Required):**
```rust
let opts = ActivityOptions::new().with_timeout_secs(45);
let mut stream = match llm_stream_activity(&opts, messages, resolved.config).await {
    vox_actor_runtime::ActivityResult::Ok(s) => s,
    other => return Err(format!("terminal agent stream failed: {other:?}")),
};

let mut full_reply = String::new();
while let Some(item) = stream.next().await {
    let delta = item.map_err(|e| format!("stream chunk error: {e}"))?;
    if !delta.is_empty() {
        let scrubbed_delta = scrub_for_agent_inbox(&delta);
        full_reply.push_str(&scrubbed_delta);
        host.agent_block_append(&session_id, block_id, &scrubbed_delta).await?;
    }
}
host.agent_block_end(&session_id, block_id, true).await?;
```

---

### 2. Live Slash Commands & `/compact` (`crates/vox-gui/src/commands/pty/slash.rs`)

**Analogs (Exact `pub(crate)` Helpers in `crates/vox-gui/src/commands/`):**
- `crates/vox-gui/src/commands/model_active.rs:35-56`:
  ```rust
  pub(crate) async fn get_active_model_impl<B: BackendCalls>(b: &B) -> Result<Option<String>, String>
  pub(crate) async fn set_active_model_impl<B: BackendCalls>(b: &B, model_id: String) -> Result<(), String>
  ```
- `crates/vox-gui/src/commands/context_budget.rs:76-94`:
  ```rust
  pub(crate) async fn context_budget_impl<B: BackendCalls>(b: &B, session_id: Option<String>) -> Result<ContextBudgetPayload, String>
  ```
- `crates/vox-gui/src/commands/llm_spend.rs:13-29`:
  ```rust
  pub(crate) async fn get_llm_spend_impl<B: BackendCalls>(b: &B, session_id: Option<String>) -> Result<LlmSpendDto, String>
  ```
- `crates/vox-gui/src/commands/memory.rs:97-115`:
  ```rust
  pub(crate) async fn memory_status_impl<B: BackendCalls>(b: &B) -> Result<MemoryStatusPayload, String>
  pub(crate) async fn recall_impl<B: BackendCalls>(b: &B, query: &str, limit: usize) -> Result<Vec<UiHitResult>, String>
  ```
- `crates/vox-gui/src/commands/mcp.rs:19-40` (or `b.call_mcp_tool("vox_skill_list", json!({}))` / `b.call_mcp_tool("vox_skill_search", json!({"query": q}))` via `BackendCalls`).
- Unit test mock backend: `use crate::remote::calls::fake::FakeBackend;` (`crates/vox-gui/src/commands/model_active.rs:85`).

**`axis_invoke_handler!` 3-Segment Registration Rule (`crates/vox-gui/src/main.rs:9-18`):**
`axis_invoke_handler!` enforces `commands :: $mod:ident :: $fn:ident` and checks `$mod` against `contracts/gui/command-backends.v1.yaml` (`"pty"` is already listed at line 65). Keep thin `#[tauri::command]` wrappers in `crates/vox-gui/src/commands/pty.rs`:
```rust
pub mod agent;
pub mod approvals;
pub mod slash;
#[cfg(test)]
mod tests;

#[tauri::command]
pub async fn term_agent_prompt(
    host: State<'_, SessionHostState>,
    backend: State<'_, GuiBackendState>,
    mens: State<'_, crate::commands::mens_serve::MensServeState>,
    session_id: String,
    prompt: String,
) -> Result<TermAgentTurnDto, String> {
    agent::term_agent_prompt_impl(&host, backend.inner(), &mens, session_id, prompt).await
}
```
And register in `crates/vox-gui/src/main.rs` alongside `commands::pty::term_session_promote_bg`:
```rust
commands::pty::term_agent_prompt,
commands::pty::term_agent_explain_fix,
commands::pty::term_slash_exec,
commands::pty::term_block_feedback,
```

---

### 3. `vox-terminal-core` Session, Host, Daemon RPC, and MENS Flywheel (`Scope 2`)

**Analogs:**
- `crates/vox-terminal-core/src/session.rs:226-340` (`submit_with_origin`, `emit`, `sink.append`)
- `crates/vox-terminal-core/src/daemon_host.rs:149-309` (`DaemonHost::route`)
- `crates/vox-orchestrator-d/src/term_dispatch.rs:179-215, 352-371` (`SessionEvent::SnapshotResync`, `"term.security"`, `"term.control"`)
- `crates/vox-terminal-core/src/corpus/{redact,writer,curate}.rs`

#### 3a. Fix Missing `"snapshot"` Frame in `DaemonHost::route` (`crates/vox-terminal-core/src/daemon_host.rs:149`)
`term_dispatch.rs:179` serializes `SessionEvent::SnapshotResync` as `{"type": "snapshot", "blocks": ..., "resync_required": ..., "alt_screen": ..., "cwd": ..., "git_branch": ...}`. Add to `DaemonHost::route` (needed by `/clear` and `/compact` over daemon RPC!):
```rust
Some("snapshot") => {
    let blocks: Vec<Block> = v
        .get("blocks")
        .cloned()
        .and_then(|b| serde_json::from_value(b).ok())
        .unwrap_or_default();
    let resync_required = v.get("resync_required").and_then(Value::as_bool).unwrap_or(false);
    let alt_screen = v.get("alt_screen").and_then(Value::as_bool).unwrap_or(false);
    let cwd = v.get("cwd").and_then(Value::as_str).map(String::from);
    let git_branch = v.get("git_branch").and_then(Value::as_str).map(String::from);
    let _ = events.send(SessionEvent::SnapshotResync {
        blocks,
        resync_required,
        alt_screen,
        cwd,
        git_branch,
    });
}
```

#### 3b. `Autopilot -> Manual` Immediate Preemption (`D-05`, `crates/vox-terminal-core/src/host.rs`)
In `LocalHost::set_control_mode` (`host.rs:640-655`), detect `prev == TermControlMode::Autopilot && mode == TermControlMode::Manual`:
1. Send `TermSignal::Interrupt` (`\x03`) to the PTY ONLY if the active running block has `matches!(b.origin, BlockOrigin::Agent { .. })` (so a running user shell command is NOT killed unless agent-originated, or interrupt active PTY and mark running `BlockOrigin::Agent` block `BlockStatus::Cancelled`).
2. Emit `SessionEvent::ControlModeChanged { control_mode: mode }` so attached UIs show the preemption toast immediately.

#### 3c. Unified 3-Stage Redaction + `DefaultPolicy` + `LocalHost::with_transcript_sink` (`D-08` / `TERM-AGENT-04`)
Look at `crates/vox-terminal-core/src/corpus/writer.rs:51-65` today:
```rust
// CURRENT BUG AT writer.rs:62 — Accepted/Rejected leak unredacted Block fields!
other => other.clone(),
```
Fix in `crates/vox-terminal-core/src/corpus/redact.rs`:
```rust
use crate::block::{Block, OutputChunk};
use crate::transcript::{TranscriptEvent, TranscriptKind};

pub fn redact_block_with(block: &Block, extra: &dyn Fn(&str) -> String) -> Block {
    let scrub = |s: &str| redact_owned(&extra(s));
    Block {
        id: block.id,
        kind: block.kind,
        origin: block.origin.clone(),
        pinned: block.pinned,
        credential_prompt: block.credential_prompt,
        input: scrub(&block.input),
        output: block
            .output
            .iter()
            .map(|c| OutputChunk {
                stream: c.stream,
                text: scrub(&c.text),
                ts_ms: c.ts_ms,
            })
            .collect(),
        status: block.status,
        exit_code: block.exit_code,
        cwd: block.cwd.as_deref().map(scrub),
        git_branch: block.git_branch.as_deref().map(scrub),
    }
}

pub fn redact_event_with(
    event: &TranscriptEvent,
    extra: &dyn Fn(&str) -> String,
) -> TranscriptEvent {
    let scrub = |s: &str| redact_owned(&extra(s));
    let kind = match &event.kind {
        TranscriptKind::Submitted { intent, input } => TranscriptKind::Submitted {
            intent: intent.clone(),
            input: scrub(input),
        },
        TranscriptKind::Output { stream, text } => TranscriptKind::Output {
            stream: stream.clone(),
            text: scrub(text),
        },
        TranscriptKind::AgentTurn { text } => TranscriptKind::AgentTurn { text: scrub(text) },
        TranscriptKind::Corrected { from, to } => TranscriptKind::Corrected {
            from: scrub(from),
            to: scrub(to),
        },
        TranscriptKind::Accepted { block } => TranscriptKind::Accepted {
            block: redact_block_with(block, extra),
        },
        TranscriptKind::Rejected { block } => TranscriptKind::Rejected {
            block: redact_block_with(block, extra),
        },
        TranscriptKind::ExitStatus { code } => TranscriptKind::ExitStatus { code: *code },
    };
    TranscriptEvent {
        ts_ms: event.ts_ms,
        session_id: event.session_id.clone(),
        block_id: event.block_id,
        kind,
    }
}
```
And in `crates/vox-terminal-core/src/corpus/curate.rs:59-61`:
```rust
TranscriptKind::ExitStatus { .. } => Decision::Drop,
TranscriptKind::Rejected { block } => {
    if block.input.trim().is_empty() && block.output.is_empty() {
        Decision::Drop
    } else {
        Decision::Keep
    }
}
```
And wire `LocalHost::with_transcript_sink(sink)` in **both**:
1. `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:287` (passing `vox_secrets::scrub_resolved_secrets` + ANSI strip into a `DualCorpusTranscriptSink` writing `~/.vox/sessions/terminal/<yyyy-mm-dd>.jsonl` + `~/.vox/corpus/terminal/<yyyy-mm-dd>.jsonl`).
2. `crates/vox-gui/src/commands/pty.rs:135` (`SessionHostState::default()` fallback `LocalHost`).
Keep `LocalHost::default()` with `transcript_sink: None` so unit tests stay hermetic and never write to `~/.vox/`.

---

### 4. MCP HITL Approval Dual-Path & Safety Re-Check (`Scope 3`)

**Analogs:**
- `crates/vox-orchestrator-mcp/src/pending_approvals.rs:19-95` (`ApprovalOutcome`, `PendingApprovalInfo`, `PendingApprovals`)
- `crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs:157-172` (`vox_resolve_approval` schema — **has `"additionalProperties": false`**!)
- `crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs:123-155` (`vox_resolve_approval` direct MCP handler)
- `crates/vox-orchestrator-mcp/src/daemon_extra.rs:261-285` (`orch_daemon_method::RESOLVE_APPROVAL` daemon RPC handler)
- `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:160-245` (`vox_terminal_exec` / `vox_terminal_send_input` approval gate)
- `crates/vox-orchestrator/src/oversight/exec_policy.rs:344-420` (AST compound-command splitter — enforces per-segment allowlist so `cargo test && rm -rf /` never slips past `"cargo test*"`)

**Critical Dual-Path Rule:**
In Axis GUI, `voxTransport.invokeMcpTool('vox_resolve_approval', ...)` routes through `crates/vox-gui/src/commands/mcp.rs` -> `BackendCalls::call_mcp_tool` -> daemon `McpExtraDispatch` (`daemon_extra.rs` / `tasks_core.rs`). **Both** `tasks_core.rs:123` AND `daemon_extra.rs:261` MUST deserialize and pass `PendingResolution`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingResolution {
    pub outcome: ApprovalOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steering_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalInfo {
    pub approval_id: String,
    pub tool: String,
    pub summary: String,
    pub requested_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_prompt: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_category: Option<String>,
}
```
**Mandatory `Edit & Run` Re-Validation (`terminal_tools.rs`):**
When `resolution.modified_command` is `Some(edited)`:
1. Re-run `classify_floor(&FloorInput::tool("vox_run_shell", &json!({"command": edited}), &cwd))` — if it hits a floor category that wasn't already approved or violates `L1ReadOnly`, reject or re-check!
2. Re-run `sec_level.allows_agent_command(&edited)` when `sec_level == TermSecurityLevel::L1ReadOnly` (never execute mutating edits on an L1 read-only session).
3. When `resolution.allow_pattern` is `Some(pat)`, store `(session_id, pat)` in a session-scoped `PrRwLock<HashMap<String, Vec<String>>>` on `ServerState` (cleared on session close / `Autopilot -> Manual`), and match **every AST simple-command segment** via `vox_orchestrator::oversight::exec_policy` so compound shell chains (`&&`, `;`, `|`, `$()`) cannot piggyback on an allowed prefix!
4. When `resolution.outcome == ApprovalOutcome::Rejected` and `steering_note` is `Some(note)`, include `steering_note` in the `ToolResult::err_with_remediation` message returned to the agent loop so the agent adapts immediately without retrying the rejected command.

---

### 5. Frontend Console & Chat Components (`Scope 4`)

**Analogs (All Git-Tracked):**
- `crates/vox-gui/ui/src/components/ui/RomanSurfaceHeader.tsx:309-330` (`RomanWaxSealBadge`)
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx:1-275` (`BlockCard` action bar + badges)
- `crates/vox-gui/ui/src/components/surfaces/Approvals/ApprovalsView.tsx:77-130` (`invokeMcpTool('vox_pending_approvals')` / `invokeMcpTool('vox_resolve_approval')`)
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:220-435` (`submit` handler, `InputEditor` / `ScriptoriumPromptEditor` wiring)
- `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx:1-126` (inline terminal tool event trace)

**Exact `RomanWaxSealBadge` Usage (`RomanSurfaceHeader.tsx:309`):**
```tsx
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';

// Valid tones: 'gold' | 'crimson' | 'emerald'
<RomanWaxSealBadge
  label={approval.floor_category ? `SAFETY FLOOR: ${approval.floor_category}` : 'APPROVAL REQUIRED'}
  tone={approval.floor_category || approval.credential_prompt ? 'crimson' : 'gold'}
/>
```

**Locked `data-testid` Contracts (`445-CONTEXT.md` D-03, D-04 & §4):**
- `BlockCard.tsx`: When `block.status === 'Failed'` (or `(block.exit_code ?? 0) !== 0`), render `<Button variant="ghost" size="sm" onClick={() => onExplainFix?.(block)}>Explain &amp; Fix</Button>` alongside `Ask Agent`.
- `AgentTurnContent.tsx`:
  - Root container for diagnosis cards: `data-testid="term-explain-fix-block"` (when diagnosis) or `data-testid="term-agent-turn-block"`.
  - Proposed command actions: `data-testid="term-proposed-fix-run"` (`Run`) and `data-testid="term-proposed-fix-edit"` (`Edit in Prompt` -> populates prompt draft without executing!).
  - MENS feedback buttons: `data-testid="term-feedback-accept"`, `data-testid="term-feedback-reject"`, `data-testid="term-feedback-correct"`.
- `TerminalApprovalCard.tsx`:
  - Root card: `data-testid="term-approval-card"` (locked in `445-CONTEXT.md` D-04).
  - Use `<Button>` from `../../ui/Button` and `<Input>` from `../../ui/Input` — NEVER raw `<button>`, `<input>`, `<select>`, or `<textarea>` (`noRawNativeControlsGuard`!).

**Vitest Mock Harness Pattern (`Console.test.tsx:7-40` & `BlockStreamView.test.tsx:1-46`):**
When adding new exports to `crates/vox-gui/ui/src/transport.ts`, update explicit `vi.mock('../../../transport', ...)` factories in `Console.test.tsx` (lines 11–40) or use `importOriginal` (`ChatTerminalBlockCard.test.tsx:26-38`) so existing Console tests do not fail with missing mock exports:
```tsx
termAgentPrompt: vi.fn().mockResolvedValue({ block_id: 99, model: 'local:mens', proposed_command: null }),
termAgentExplainFix: vi.fn().mockResolvedValue({ block_id: 100, model: 'local:mens', proposed_command: 'cargo check' }),
termSlashExec: vi.fn().mockResolvedValue({ handled: true, block_id: 101, message: 'ok' }),
termBlockFeedback: vi.fn().mockResolvedValue(undefined),
```

---

## Shared Patterns & Guardrails

1. **3-Stage Secret & ANSI Scrubbing (`D-02`, `D-08`):**
   - Outbound to LLM (`pty/agent.rs`, `pty/slash.rs` `/compact`): reuse `crate::commands::console_a2a::scrub_for_agent_inbox(raw)` (`strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_terminal_core::corpus::redact::redact_owned`) AND skip any block where `block.credential_prompt == true`.
   - Disk persistence (`JournalSink` + `CorpusWriter`): route through `redact_event_with(event, &scrub_fn)` covering `Submitted`, `Output`, `AgentTurn`, `Corrected`, `Accepted`, AND `Rejected`.
2. **Model-Agnostic LLM Boundary (`AGENTS.md` + `vox-code-audit` `llm_provider_call`):**
   - Never hardcode `api.anthropic.com`, `api.openai.com`, `generativelanguage.googleapis.com`, or `openrouter.ai` in `vox-gui` or `vox-terminal-core`. Only `vox_actor_runtime::llm::{llm_stream_activity, llm_chat, LlmConfig}` is permitted.
3. **Static UI Architecture Guards (`crates/vox-gui/ui/src/architecture/`):**
   - `transportIpcGuard.test.ts`: Frontend never imports `@tauri-apps/api/core` directly; all IPC goes through `crates/vox-gui/ui/src/transport.ts` (`safeInvoke`).
   - `zeroEmojiAndDingbat.test.ts`: Zero Unicode emojis or dingbats (`✓`, `✗`, `⚠`, `👍`, `👎`, `✨`, `🤖`) anywhere in JSX/strings — use `Icon` (`../../ui/Icons`), `RomanWaxSealBadge`, or plain Latin text (`Accept`, `Reject`, `Correct`).
   - `noRawNativeControlsGuard.test.ts`: Zero raw `<button>`, `<input>`, `<select>`, `<textarea>` in surface components — use `Button`, `Input`, `Segment` from `components/ui/` (`ScriptoriumPromptEditor` is the sole sanctioned composer exception, and default Console mount must keep a **single** `role="textbox"` until `Edit & Run` or `Correct` is clicked so `Console.test.tsx:93` `screen.getByRole('textbox')` stays unique!).
   - `toastBodyGuard.test.ts`: Every `pushToast` error call MUST wrap `err` with `sanitizeErrorForToast(err)` (`import { sanitizeErrorForToast } from '../../../lib/backendGuard'`).
   - Typography & Spacing (`UI-SPEC`): Max 4 font sizes (`text-xs`, `text-sm`, `text-base`, `text-xl`), max 2 font weights (`font-normal`, `font-semibold`), zero fractional `.5` Tailwind spacing (`p-1`, `p-2`, `px-2`, `py-1`, `gap-2` only — never `py-1.5` or `gap-1.5`).
