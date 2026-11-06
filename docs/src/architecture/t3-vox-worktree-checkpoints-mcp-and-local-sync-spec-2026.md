---
title: "T3-to-Vox Domain 3 & 4 Implementation Spec: Junction-Safe Worktrees, Git Checkpoints, Cross-Provider Handoff, McpToolAccess & Branch Divergence UI (2026)"
description: "Detailed Rust and TypeScript implementation specification for Phases 221-223: Windows NTFS junction-safe worktree removal, WorkspaceLease, Git plumbing checkpoints, Muse multi-variant worktrees, ContextHandoffService, McpToolAccess/DPoP, and GUI branch divergence comparison."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3-to-Vox Domain 3 & 4 Implementation Spec: Junction-Safe Worktrees, Git Checkpoints, Cross-Provider Handoff, `McpToolAccess` & Branch Divergence UI (2026)

<!-- kb:summary -->
This document specifies the exact Rust and TypeScript implementation for `Phases 221–223` (`T3-WORKTREE-VCS-01..04`, `T3-HANDOFF-MCP-01..04`, and `T3-CLIENT-BRANCH-01..04`) of Milestone `v4.1`. It details how to replace `DelegationWorktree::cleanup` (`git worktree remove --force`) in `agy_worktree.rs` with Windows NTFS junction-safe worktree removal under `WorkspaceLease`, replace the 12-line `checkpoint_engine.rs` stub with `GitPlumbingCheckpointStore` (`refs/vox/checkpoints/`), add Muse-style `1..4` parallel worktree variants, implement `ContextHandoffService` (`16K` token / `64KB` cap) and `McpToolAccess` capability ceilings, and wire `CHUNK_SIZE = 16` lazy loading and `divergencePoint` branch comparison into `vox-gui`.
<!-- /kb:summary -->

## 1. Phase 221: Junction-Safe Worktrees, `WorkspaceLease`, Git Plumbing Checkpoints & Muse Pool (`T3-WORKTREE-VCS-01..04`)

### 1.1 Replace `DelegationWorktree` in `crates/vox-orchestrator-mcp/src/agy_worktree.rs`
- **Source Inspiration:** `t3code` `apps/server/src/worktreesDirectory.ts:12-97`, `apps/server/src/vcs/worktreeRemoval.ts:17-157`, `apps/server/src/workspace/workspaceLease.ts:1-23`, and `GitVcsDriverCore.ts` (`copyWorktreeEnvFiles`).
- **Legacy Code Replaced & Deleted:**
  - Delete raw `GitExec::new(repo_root).run(&["worktree", "remove", "--force", &path_s])` in `crates/vox-orchestrator-mcp/src/agy_worktree.rs:60-66` (which traverses NTFS directory junctions in Git for Windows `< 2.54`).
- **New `ManagedWorktreeDriver` Mechanics:**
  1. **2-Segment Managed Path Verification (`validate_managed_worktree_path`):** Resolves canonical paths (`std::fs::canonicalize`), rejects filesystem roots, and verifies that the worktree resides exactly 2 segments under `.vox/agy-worktrees/<repo>/<slug>`.
  2. **Keyed Per-Worktree Semaphore (`WorkspaceLeaseMap`):** Maintains `std::sync::Mutex<std::collections::HashMap<PathBuf, Arc<tokio::sync::Semaphore>>>` (1 permit per canonical worktree path, zero external crate dependencies). Every worktree creation, checkpoint capture/restore, and cleanup runs inside `with_workspace_lease(path, fut)`.
  3. **`.env*` Provisioning (`copy_worktree_env_files`):** On `DelegationWorktree::create`, copies untracked `.env*` files (excluding `.env.production*` unless opted in) from `repo_root` into the new worktree.
  4. **Junction-Safe `remove_linked_worktree`:**
     - Verifies `.git` admin linkage via `git rev-parse --absolute-git-dir` and `--git-common-dir`.
     - Checks that `<git-dir>/locked` does not exist.
     - Unlinks the worktree directory tree using junction-safe removal (unlinking symlinks/junctions before directory removal) and then runs `git worktree prune` in `repo_root`.

### 1.2 Replace Stub `checkpoint_engine.rs` with `GitPlumbingCheckpointStore` (`crates/vox-orchestrator/src/agentos/checkpoint_engine.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/checkpointing/CheckpointStore.ts:45-158`, `Diffs.ts:1-62`, and `GitVcsDriverCore.ts` (`VcsCheckpointOps`, `T3CODE_RESTORE_POINT_PREFIX`).
- **Legacy Code Replaced:** Expands the 12-line `crates/vox-orchestrator/src/agentos/checkpoint_engine.rs` stub into a complete Git plumbing checkpoint store while keeping `should_sparse_checkpoint(tool: &str) -> bool` as the pre-tool trigger predicate.
- **Mechanics:**
  - **`capture_checkpoint(worktree_path, thread_id, ordinal) -> Result<CheckpointReceipt>`:**
    - Uses a temporary `GIT_INDEX_FILE` copy of `.git/index`, runs `git add -A`, `git write-tree`, and `git commit-tree <tree_sha> -m "vox-checkpoint:<thread_id>:<ordinal>"`, and updates hidden ref `refs/vox/checkpoints/<thread_id>/<ordinal>` via `git update-ref`. Zero branch commits or working-tree mutations occur.
  - **`create_restore_point(worktree_path, label)` & `restore_checkpoint(worktree_path, ref_name)`:**
    - Before rolling back a worktree to `refs/vox/checkpoints/<thread_id>/<ordinal>`, saves the current dirty state under `refs/vox/restore-points/<timestamp>` (`vox-restore-point`), then runs `git read-tree --reset -u <checkpoint_sha>` and `git clean -fd` under `with_workspace_lease`.
  - **`diff_checkpoints(worktree_path, from_ref, to_ref) -> Vec<CheckpointFileStat>`:** Parses `git diff --numstat <from_ref> <to_ref>` into file-level additions/deletions.

### 1.3 Muse Multi-Variant Worktree Fan-Out (`crates/vox-orchestrator/src/workspace/muse_pool.rs`)
- **Source Inspiration:** `t3code` `packages/provider-muse` (`MuseDriver.ts`, `MuseWorktreePool.ts`).
- **Mechanics:** Provisions `variant_count` (`1..=4`) isolated worktrees under `.vox/agy-worktrees/muse/<thread_id>/variant-<i>`, dispatches the prompt in parallel across up to 4 selected models/providers, captures per-variant `CheckpointFileStat` diffs, and exposes `adopt_variant(variant_index)` to apply the chosen variant's branch to the parent workspace.

### 1.4 Adaptive PR Sync/Watch Reactors, AI Multi-Section PR Review Guides & `TextGeneration` (`crates/vox-orchestrator/src/vcs/pr_reactor.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/orchestration-v2/PullRequestSyncReactor.ts`, `PullRequestWatchReactor.ts`, `apps/server/src/pullRequestReview/PullRequestReviewGuideService.ts`, and `apps/server/src/git/Layers/TextGeneration.ts`.
- **Mechanics:**
  1. **Adaptive PR Polling (`PullRequestSyncReactor`):** Polls open PRs linked to active threads at adaptive intervals (`ACTIVE_SYNC_INTERVAL_MS = 60_000` (1m) when active within `ACTIVE_WINDOW_MS = 1_800_000` (30m); `QUIET_SYNC_INTERVAL_MS = 300_000` (5m) when quiet; `SLOW_SYNC_INTERVAL_MS = 900_000` (15m) for closed/merged PRs) and auto-links stacked PR siblings (`source = "stack"`).
  2. **CI/Review Wake Reactor (`PullRequestWatchReactor`):** Automatically wakes a thread when watched CI checks fail or new review comments arrive, guarded by `PULL_REQUEST_WATCH_WAKE_LIMIT = 10` wakes per watch arm and `PULL_REQUEST_WATCH_NO_CHECKS_MS = 900_000` (15m) timeout when no checks register.
  3. **Structured Multi-Section AI PR Review Guides (`PullRequestReviewGuideService`):** Generates a structured review walk-through (`summary`, `architecture_impact`, `ordered_file_groups`, `risk_hotspots`, `verification_steps`) from a branch or PR diff and caches it in `pull_request_review_guides` keyed by `(repo_root, pr_number, head_sha)`.
  4. **Provider-Agnostic Git `TextGeneration` (`generate_commit_message`, `generate_pr_content`, `generate_branch_name`, `generate_thread_title`):** Routes all four lightweight Git/thread text generation tasks through `vox_actor_runtime::llm::llm_chat` using structured JSON schemas, enriching `generate_thread_title` with linked PR and issue titles (`MAX_CONTEXT_TITLE_CHARS = 160`).

---

## 2. Phase 222: `ContextHandoffService`, `McpToolAccess` Capability Gates, DPoP & DOM Shedding (`T3-HANDOFF-MCP-01..04`)

### 2.1 Cross-Provider `ContextHandoffService` (`crates/vox-orchestrator/src/session/context_handoff.rs`)
- **Source Inspiration:** `t3code` `packages/provider-core/src/server/handoffBudget.ts` (`DEFAULT_HANDOFF_TOKEN_CAP = 16_000`, `HANDOFF_BYTE_CAP = 64_000`) & `apps/server/src/orchestration-v2/ContextHandoffService.ts`.
- **Mechanics:**
  - Supports 5 strategies: `DeltaSinceTargetLastSeen`, `ForkDeltaSummary`, `FullThreadSummary`, `CheckpointSummary`, and `ManualContext`.
  - Synthesizes a structured XML-tagged `<context_handoff covered_runs="from..to" source_model="..." strategy="...">` preamble bounded strictly by `DEFAULT_HANDOFF_TOKEN_CAP = 16_000` tokens and `HANDOFF_BYTE_CAP = 64_000` bytes (`compact_text(text, 240)` for older turns while preserving recent turns verbatim).

### 2.2 `McpToolAccess` Capability Combinators, DPoP & DOM Shedding (`crates/vox-orchestrator-mcp/src/mcp_tool_access.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/mcp/McpToolAccess.ts:31-188`, `OrchestratorMcpService.ts:679-725`, `EnvironmentAuth.ts`, `McpOAuth.ts`, and `McpHttpServer.ts` (`boundSnapshotMetadata`).
- **Legacy Code Replaced:** Scattered ad-hoc permission booleans across `crates/vox-orchestrator-mcp/src/mcp_context.rs` and tool handlers.
- **Mechanics:**
  1. **Sealed `VerifiedMcpToolHandler` (`McpToolAccess`):** Requires every MCP tool handler to be constructed via one of four access combinators:
     - `McpToolAccess::reads(handler)`
     - `McpToolAccess::writes(handler)` — rejects `"read-only"` callers with `McpAccessError::CapabilityDenied`.
     - `McpToolAccess::writes_with_capability(cap, handler)`
     - `McpToolAccess::starts_threads(requested_runtime_mode, requested_interaction_mode, handler)` — enforces `runtime_mode_rank` (`ReadOnly = 0 < Default = 1 < FullAccess = 2`) and `interaction_mode_rank` (`Plan = 0 < Default = 1`), returning `McpAccessError::RuntimeModeEscalationDenied` or `InteractionModeEscalationDenied` if a caller attempts privilege escalation.
  2. **Stateless BLAKE3 Keyed-MAC OAuth `client_id` & RFC 9449 DPoP Validator:** Encodes dynamic MCP OAuth client registrations into `vox_<hex(payload)>.<hex(vox_crypto::keyed_hash)>` using `vox_crypto::keyed_hash` and `SecretId::VoxMcpOAuthClientKey` (zero unauthenticated DB writes) and validates `dpop+jwt` proofs (`htm`, `htu`, `iat <= 300s`, `jti` 10,000-entry replay cache).
  3. **20 KB DOM Snapshot Shedding (`bound_snapshot_metadata`):** Any browser/DOM inspection tool result whose `snapshot` exceeds `20 * 1024` bytes (`20 KiB`) strips the inline DOM string and returns a structured hint instructing the agent to call `preview_find` or pass a narrower CSS `selector`.

### 2.3 `McpAppModelContext` Persistent Interactive Widget State (`crates/vox-orchestrator-mcp/src/mcp_app_context.rs`)
- **Source Inspiration:** `t3code` `packages/shared/src/mcpAppModelContext.ts` (`MAX_MCP_APP_MODEL_CONTEXT_BYTES = 16_384`).
- **Mechanics:** Allows interactive MCP UI widgets rendered inside a chat turn item to persist up to `16 KiB` of structured JSON/text state (`McpAppModelContext`) attached to `turn_item_id`, automatically cloning that state across thread forks (`thread.fork`) and injecting a compact `<mcp_app_context>` block into subsequent model turns when the user interacts with the widget.

---

## 3. Phase 223: Local-First Chunked Lazy Loading, Write Suppression & Tree Branch `divergencePoint` UI (`T3-CLIENT-BRANCH-01..04`)

### 3.1 Chunked Thread Loading & Streaming Write Suppression (`crates/vox-gui/ui/src/`)
- **Source Inspiration:** `helioschat/app` `src/lib/stores/chat.ts:50-157`.
- **Legacy Code Replaced:** Eager loading of all historical sessions and unthrottled local-storage/persistence writes on every streamed token in `crates/vox-gui/ui`.
- **Mechanics:**
  - Startup loads the first `CHUNK_SIZE = 16` threads immediately and schedules remaining chunks at `50ms` intervals with a per-thread `loadMutex` deduplicator.
  - Persistence subscribers skip storage writes while `isStreaming === true` or `temporary === true` (`!chat.temporary && !isStreaming`), executing a single persistence flush in `finally` when the stream completes.

### 3.2 Tree-Structured Message Branching & Side-by-Side `divergencePoint` Comparison (`crates/vox-gui/ui/src/components/surfaces/Chat/`)
- **Source Inspiration:** `guitaripod/omnichat` `src/services/branching/branch-manager.ts` & `src/components/chat/branch-comparison.tsx`.
- **Mechanics:**
  - `buildMessageTree(messages)` constructs a `MessageNode` DAG from `parent_id`, assigning hierarchical branch IDs (`main`, `main-<msgId>-0`, `main-<msgId>-1`).
  - `computeDivergencePoint(branches)` finds the deepest common message index across any `2..4` selected branches and renders a side-by-side comparison modal highlighting post-divergence responses.
  - Code-block syntax highlighting in the composer uses deferred highlighting thresholds (`MAX_HIGHLIGHTED_CODE_BLOCK_LENGTH = 20_000`, `DEFERRED_HIGHLIGHT_CODE_BLOCK_LENGTH = 2_000`, `DEFERRED_HIGHLIGHT_DELAY_MS = 300` from `t3code` `composerCodeBlockHighlight.ts`).

---

## 4. Verification Gates (`Phases 221–223`)
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib agy_worktree::tests` (verifying 2-segment path validation, `WorkspaceLeaseMap`, `.env*` copying, and junction-safe worktree removal).
- `timeout 900s cargo test -p vox-orchestrator --lib agentos::checkpoint_engine::tests` (verifying `refs/vox/checkpoints/` plumbing capture, restore points, and `diff_checkpoints` without working-tree mutation).
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib mcp_tool_access::tests` (verifying `McpToolAccess::writes` read-only rejection, `starts_threads` rank ceiling enforcement, and 20 KB DOM snapshot shedding).
