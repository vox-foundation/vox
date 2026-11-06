---
title: "T3-to-Vox Domain 1 Implementation Spec: Deterministic IDs, Wire Interning, Atomic Stream Resumption & Guest Share (2026)"
description: "Detailed Rust and TypeScript implementation specification for Phases 216-217: VoxIdAllocatorV2, ForwardCompatibleEnvelope, WireIdTableInterner, LiveBodyDeduper, atomic mid-stream multi-tab join, CommandOutputHub, Live Guest Share, ThreadRunQueue, promote_to_steer, and limit_recovery."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3-to-Vox Domain 1 Implementation Spec: Deterministic IDs, Wire Interning, Atomic Stream Resumption & Guest Share (2026)

<!-- kb:summary -->
This document specifies the exact Rust and TypeScript implementation for `Phases 216–217` (`T3-WIRE-ID-01..04` and `T3-STREAM-GUEST-01..04`) of Milestone `v4.1`. It details how to replace ad-hoc ID generation and uncompressed orchestrator snapshots with `VoxIdAllocatorV2` (`allocate` + `derive` colon-separated URI-encoded deterministic IDs), `ForwardCompatibleEnvelope<T>`, `WireIdTableInterner` (`orchestrationV2IdTable.ts`), `LiveBodyDeduper` (`orchestrationV2DedupedBodies.ts`), `omit_idle_subagents` cold-connect filtering, `NotT3Chat`-style `Mutex`-locked atomic mid-stream multi-tab resumption, `CommandOutputHub` `\r` folding, host-moderated Live Guest Share, and `ThreadRunQueue` with mid-turn `promote_to_steer` and rate-limit auto-recovery (`limit_recovery`).
<!-- /kb:summary -->

## 1. Phase 216: `VoxIdAllocatorV2`, `ForwardCompatibleEnvelope` & Wire Compression (`T3-WIRE-ID-01..04`)

### 1.1 `VoxIdAllocatorV2` (`crates/vox-orchestrator/src/session/id_allocator.rs`)
- **Source Inspiration:** `t3code` `packages/provider-core/src/server/IdAllocator.ts:183-433` (`IdAllocatorV2`).
- **Legacy Code Replaced:** Ad-hoc `uuid::Uuid::new_v4().to_string()` and monotonic string concatenation scattered across `crates/vox-orchestrator/src/session/state.rs` and `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`.
- **Zero-Dependency Implementation Design (Stdlib RFC 3986 Percent-Encoding — Zero `Cargo.lock` Changes):**
  ```rust
  pub struct VoxIdAllocatorV2;

  /// RFC 3986 unreserved character percent-encoding (`A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~`)
  /// implemented in pure `std` so zero external crate dependencies (`urlencoding`) are added.
  pub fn percent_encode_id_segment(input: &str) -> String {
      const HEX: &[u8; 16] = b"0123456789ABCDEF";
      let mut out = String::with_capacity(input.len());
      for &b in input.as_bytes() {
          if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
              out.push(b as char);
          } else {
              out.push('%');
              out.push(HEX[(b >> 4) as usize] as char);
              out.push(HEX[(b & 0x0f) as usize] as char);
          }
      }
      out
  }

  impl VoxIdAllocatorV2 {
      /// Mints a fresh random UUIDv4 with canonical entity prefix (`thread:<uuid>`, `run:<uuid>`, etc.).
      pub fn allocate(prefix: &str) -> String {
          format!("{prefix}:{}", uuid::Uuid::new_v4())
      }

      /// Deterministic URI-component-encoded colon-joined ID (`prefix:part1:part2...`).
      /// Replaying the same provider/actor event stream yields identical entity IDs without a lookup table.
      pub fn derive(prefix: &str, parts: &[(&str, &str)]) -> String {
          let mut out = String::from(prefix);
          for (k, v) in parts {
              out.push(':');
              out.push_str(k);
              out.push(':');
              out.push_str(&percent_encode_id_segment(v));
          }
          out
      }
  }
  ```
- **Required Derived Constructors:** `derive_run_from_thread_ordinal(thread_id, ordinal)`, `derive_run_attempt(run_id, attempt_ordinal)`, `derive_node_from_provider_item(driver, turn_id, native_item_id)`, `derive_turn_item_from_provider_item(driver, turn_id, native_item_id, segment)`, and `derive_subagent_from_task(driver, thread_id, native_task_id)`.

### 1.2 `ForwardCompatibleEnvelope<T>` (`crates/vox-orchestrator/src/models/wire_envelope.rs`)
- **Source Inspiration:** `t3code` `packages/shared/src/ForwardCompatibleUnion.ts`.
- **Implementation Design:**
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[serde(untagged)]
  pub enum ForwardCompatibleEnvelope<T> {
      Known(T),
      Unknown(serde_json::Value),
  }
  ```
  Provides `into_known_or_raw()` so older GUI/MCP clients never fail deserialization when a newer `vox-orchestrator` emits a newly added turn-event or turn-item variant.

### 1.3 `WireIdTableInterner` & `LiveBodyDeduper` (`crates/vox-orchestrator-mcp/src/dei_tools/wire_compression.rs`)
- **Source Inspiration:** `t3code` `packages/contracts/src/orchestrationV2IdTable.ts` (`80455376`), `packages/shared/src/orchestrationV2DedupedBodies.ts` (`e19fe083`), and `apps/server/src/orchestration-v2/WireProjection.ts`.
- **Legacy Code Replaced:** Uncompressed snapshot serialization in `crates/vox-orchestrator-mcp/src/dei_tools/orchestrator_snapshot.rs` and `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs`.
- **Mechanics:**
  1. **`intern_record_ids(payload: &mut Value) -> Vec<String>`:** Walks top-level arrays and nested `item` objects; for every key equal to `"id"` or ending with `"_id"` / `"Id"` (or string array ending with `"_ids"` / `"Ids"`), interns the string into `ids: Vec<String>` and replaces the field value with its integer index `u32`. `expand_record_ids(payload: &mut Value, ids: &[String])` restores the original strings losslessly.
  2. **`LiveBodyDeduper` (`LIVE_DEDUPE_MEMORY = 32`):** Tracks the last 32 transmitted `(item_id, vox_crypto::fast_hash(body.as_bytes()))` pairs (`XXH3-64` via `vox_crypto::fast_hash`) per subscription for `user_message.text`, `assistant_message.text`, `checkpoint.files`, and `subagent.{prompt, result}`. When an identical body is re-emitted on `turn-item.updated` or in a bounded snapshot where `messages` already carries the text, the field is cleared and `text_omitted: true` / `files_omitted: true` / `prompt_omitted: true` / `result_omitted: true` is set.
  3. **Cold-Connect Idle Subagent Filter (`omit_idle_subagents: bool`):** Inspired by `t3code` `packages/client-runtime/src/state/threadFamilies.ts` (`67c3afbd`), shell snapshot queries with `omit_idle_subagents = true` exclude child subagent threads whose status is `Completed | Cancelled | Failed | Interrupted`, reducing cold-connect thread-list payloads by ~33% while exposing `get_thread_family(root_thread_id)` on demand.

---

## 2. Phase 217: Atomic Mid-Stream Multi-Tab Join, `CommandOutputHub`, Live Guest Share & `ThreadRunQueue` (`T3-STREAM-GUEST-01..04`)

### 2.1 Atomic Mid-Stream Multi-Tab Join (`crates/vox-orchestrator/src/models/spec.rs` & `salvage_and_stream.rs`)
- **Source Inspiration:** `shaltielshmid/NotT3Chat` `backend/Program.cs:623-707, 750-795` (`StreamingMessage` with `SemaphoreSlim(1)` lock).
- **Legacy Code Replaced:** Unsynchronized `AgentEventKind::TokenStreamed` emission in `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs:90-95`.
- **Implementation Design:**
  - Upgrade `ResumableTurnStreamRegistry` to hold a per-turn `Arc<tokio::sync::Mutex<ActiveTurnStreamState>>` containing:
    - `chunks: Vec<SequencedTurnChunk>`
    - `accumulated_text: String`
    - `accumulated_reasoning: String`
    - `subscribers: Vec<tokio::sync::mpsc::UnboundedSender<SequencedTurnChunk>>`
  - **`subscribe_or_replay(turn_id, after_seq)`:** Acquires the per-turn `Mutex`, snapshots all chunks with `seq > after_seq`, registers a new `mpsc::UnboundedSender` in `subscribers`, and only then releases the `Mutex`. Because `append_chunk` acquires the exact same per-turn `Mutex` when appending a token and fanning out to `subscribers`, a tab refreshing or joining mid-stream is mathematically guaranteed **zero dropped tokens and zero duplicated tokens** at the join boundary.

### 2.2 `CommandOutputHub` & `\r` Progress-Bar Folding (`crates/vox-orchestrator/src/terminal_output.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/orchestration-v2/CommandOutputHub.ts` & `packages/shared/src/terminalOutput.ts:206` (`normalizeTerminalOutput`).
- **Mechanics:**
  - Constants: `MAX_CHUNKS_PER_STREAM = 64`, `MAX_BYTES_PER_STREAM = 262_144` (256 KiB), `TERMINAL_OUTPUT_TAIL_CHARS = 65_536` (64 KiB), `LIVE_LINES = 64`.
  - `normalize_terminal_output(raw: &str) -> String`: Resolves bare carriage returns (`\r` not followed by `\n`) by overwriting the current line from column 0, collapsing cargo/npm/pip progress-bar spam into clean final lines before persisting tool output into the transcript.

### 2.3 Host-Moderated Live Guest Share (`crates/vox-orchestrator-mcp/src/guest_share.rs`)
- **Source Inspiration:** `t3code` `packages/contracts/src/guestShare.ts`, `apps/server/src/guest/GuestShareService.ts`, and `apps/server/src/orchestration-v2/GuestShareDecider.ts` (`fb76594d`).
- **Crypto & Secret Compliance (`AGENTS.md`):**
  - Uses Clavis secret `SecretId::VoxGuestShareSigningKey` resolved via `vox_secrets::resolve_secret(SecretId::VoxGuestShareSigningKey, None, None)`.
  - Derives a 32-byte key via `vox_crypto::secure_hash(secret.as_bytes())` and signs `<share_id>` using `vox_crypto::keyed_hash(&key_32, share_id.as_bytes())` (BLAKE3 MAC), encoding the first 16 bytes (`128-bit` tag) with `vox_crypto::hex_encode(&mac[..16])` (invite token format: `<share_id>.<hex_mac_128bit>`).
- **Mechanics:**
  - Constants: `GUEST_SHARE_TTL_MS = 86_400_000` (24h), `GUEST_MESSAGE_MAX_LENGTH = 4_000`, `GUEST_NAME_MAX_LENGTH = 40`, `GUEST_PENDING_MESSAGE_LIMIT = 3`, `GUEST_RESOLVED_MESSAGE_LIMIT = 20`, `SNAPSHOT_ROW_LIMIT = 600`, `SNAPSHOT_USER_TURN_LIMIT = 20`.
  - Single-browser redemption (`redeem_guest_share`) binds `guest_name`; subsequent redemption attempts with a different session cookie fail with `GuestShareError::AlreadyRedeemed`.
  - Guest message submissions enter `pending` status (`GuestMessageStatus::Pending`). The host owner either **Approves** (`approve_guest_message`, which dispatches `"Message from guest {name}, sent by the owner:\n\n{text}"`) or **Dismisses** (`dismiss_guest_message`, which purges any pending guest attachments).

### 2.4 `ThreadRunQueue`, Mid-Turn `promote_to_steer`, Rate-Limit Auto-Recovery & `awaited` Background Settlement (`crates/vox-orchestrator/src/session/run_queue.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/orchestration-v2/Decider.ts` (`queued-message.enqueue`, `queued-message.promote-to-steer`, `limit.auto-resume`, `limit.snooze`) & `packages/provider-core/src/server/ThreadToolkit.ts` (`t3_background_wait`).
- **Legacy Code Replaced:** Single-flight blocking turn rejection in `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` when a session already has an active turn running.
- **Mechanics:**
  1. **Queued Runs (`queued_runs: VecDeque<QueuedRunEntry>`):** When a user submits a prompt while `ThreadRuntimeStatus::Running`, the message is appended to `queued_runs` with full editing, reordering, and cancellation support (`vox_chat_queue_edit`, `vox_chat_queue_reorder`, `vox_chat_queue_cancel`).
  2. **Mid-Turn Promote-to-Steer (`promote_to_steer(thread_id, queued_run_id)`):** Removes the queued message from `queued_runs` and injects it immediately into the active `agent_loop` iteration as a live steering user message before the next tool-batch or LLM round-trip, without aborting the running turn.
  3. **Rate-Limit Auto-Recovery (`LimitRecoveryState`):** When a provider returns a `429` or quota exhaustion with a parsable `reset_at` timestamp, the thread transitions to `WaitingRateLimit { run_id, reset_at_ms, auto_resume: true, snoozed_until_ms: None }` and schedules an automatic turn resume timer (with user toggles for `auto_resume` and `snooze`).
  4. **Selective Background Work Settlement (`awaited: bool`):** Subagents and background shells spawned during a turn carry `awaited: bool`. The turn state machine enters `SettlingBackgroundWork` at turn end **only** for children with `awaited == true`; fire-and-forget daemons (`awaited == false`) continue running in the background without blocking turn completion.

---

## 3. Verification Gates (`Phases 216–217`)
- `timeout 900s cargo test -p vox-orchestrator --lib session::id_allocator` (verifying deterministic `derive` idempotency and `percent_encode_id_segment` across 1,000 replayed events).
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib dei_tools::wire_compression` (verifying round-trip `intern_record_ids` / `expand_record_ids` and `LiveBodyDeduper`).
- `timeout 900s cargo test -p vox-orchestrator --lib models::spec::tests::atomic_mid_stream_multi_tab_join` (verifying zero dropped/duplicated tokens under concurrent producer + mid-stream subscriber join).
- `timeout 900s cargo test -p vox-orchestrator --lib session::run_queue::tests` (verifying `promote_to_steer`, `LimitRecoveryState` auto-resume, and `awaited` background work settlement).
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib guest_share::tests` (verifying BLAKE3 `keyed_hash` token verification, single redemption, 3-message pending cap, and owner approve/dismiss transitions).
