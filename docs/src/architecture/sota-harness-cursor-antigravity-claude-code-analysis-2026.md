---
title: "SOTA Agent Harness Comparative Analysis & v3.4 Architecture (Vox vs. Cursor, Antigravity & Claude Code)"
description: "Empirical post-mortem and engineering architecture for Vox Milestone v3.4: prefix-locked two-tier tool selection, AST symbol-slice reads and skeleton folding, pre-edit cross-crate grounding, speculative streaming tool prefetch, and workspace test canonicalization."
category: "Architecture SSOTs"
status: "current"
topic: "model-routing"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
---

# SOTA Agent Harness Comparative Analysis & v3.4 Architecture (Vox vs. Cursor, Antigravity & Claude Code)

<!-- kb:summary -->
This document synthesizes an empirical trajectory post-mortem and architectural benchmark comparing the Vox Axis agent harness (`vox-orchestrator`, `vox-orchestrator-mcp`, `vox-gui`) against October 2026 frontier harnesses (Cursor, Antigravity, and Claude Code). It specifies and documents Milestone v3.4 (`Phases 178–182`), delivering prefix-locked two-tier tool selection (`FROZEN_PREFIX_CORE_TOOLS`), AST symbol-slice extraction and signature-preserving skeleton folding, `<2ms` pre-edit cross-crate `Cargo.toml` and repo-policy preflight guards, streaming `delta.tool_calls` read prefetch, workspace test canonicalization, and FIFO multi-call test doubles.
<!-- /kb:summary -->

## 1. Empirical Trajectory Post-Mortem (`v3.3` Execution Audit)

Analyzing a full multi-crate milestone implementation (`Phases 173–177`) exposed five concrete classes of latency and token waste common to Cursor, Antigravity, and Claude Code:

| Failure / Friction Class | What Happened in Standard Agent Loops | Measured Cost | Vox `v3.4` Deterministic Elimination |
| :--- | :--- | :--- | :--- |
| **1. Unverified Cross-Crate Imports & Signatures** | Edits introduced `use vox_telemetry::...` in `vox-gui` (`E0432`) and guessed struct/method signatures across crate boundaries (`E0599`, `E0609`), discovered only after a cold `cargo test` build. | **~70s wall-clock, ~4,200 tokens** | `preflight_check_write_guards` (`ast_slice_and_preflight.rs`) checks `use vox_*` imports against target `Cargo.toml` in `<1ms` inside `PreToolUse`. |
| **2. Per-Turn BM25 Tool Prefix Reshuffling** | Re-sorting top-20 tool schemas on every turn (`select_tools_for_turn_ranked`) mutates the leading `tools` JSON array in the request prefix, invalidating provider KV caches. | **3,000–6,000 uncached prefix tokens/turn** | `select_prefix_locked_tools_for_turn` freezes `FROZEN_PREFIX_CORE_TOOLS` (`12` core tools) in strictly invariant order at indices `0..12`, appending spoke-ranked tools after the frozen prefix. |
| **3. Line-Window File Reads & Mid-Line Character Truncation** | Inspecting 1,000–2,200 LoC files required multiple 200-line reads (~15,000 tokens), and `cap_and_summarize_tool_output` sliced mid-function. | **~15,000 input tokens** | `extract_symbol_slice` extracts exact `fn`/`struct`/`impl`/`interface` blocks with top-of-file imports; `fold_code_to_ast_skeleton` preserves all signatures while eliding function bodies. |
| **4. Workspace Test Runner CLI Traps** | `cargo test -p vox-gui --lib` failed on a binary-only crate; `pnpm test -- --run <file>` ran all 336 Vitest suites (`22.5s`) instead of 2 (`0.78s`). | **29x slower Vitest run (`22.5s` vs `0.78s`)** | `canonicalize_workspace_command` (`lifecycle_hooks.rs`) rewrites `-p vox-gui --lib` to `--bin vox-gui` and `test -- --run <file>` to `exec vitest run <file>`. |
| **5. Post-Hoc Repo Guard & God-Object Drift** | `noVersionedModelIds.test.ts` and `toestub arch/god_object` (`>500` non-blank LoC) only ran after implementation completed, requiring a second refactor turn. | **3 extra edit+test turns (~45s)** | Inline write-time preflight (`preflight_check_write_guards`) blocks hardcoded versioned model literals / banned crypto imports and warns/flags `>450`/`>500` non-blank LoC immediately. |

---

## 2. Head-to-Head Matrix: Vox `v3.4` vs. Cursor, Antigravity & Claude Code

| Capability Axis | Cursor (Oct 2026) | Antigravity (Oct 2026) | Claude Code (Oct 2026) | Vox Axis Harness (`v3.4`) |
| :--- | :--- | :--- | :--- | :--- |
| **Prompt KV-Cache Stability** | Static tool set per mode | Fixed tool declarations | Prompt-cached system + deferred `ToolSearch` | **Prefix-Locked Two-Tier (`FROZEN_PREFIX_CORE_TOOLS` + Spoke Tail + Deferred Catalog Hint)** |
| **Code Read Granularity** | File / line-range chunks + semantic search | 800-line `read_file` windows | Line-range `Read` + `Grep` | **AST Symbol-Slice (`extract_symbol_slice`) + Signature-Preserving Skeleton Folding (`fold_code_to_ast_skeleton`)** |
| **Pre-Edit Verification** | Post-edit shadow workspace LSP | Post-edit `run_command` / `cargo` | Post-edit `PreToolUse` / `PostToolUse` shell hooks | **In-Memory `<2ms` Pre-Edit Crate-Edge (`Cargo.toml`), `arch/god_object` LoC, `noVersionedModelIds` & Crypto Guard** |
| **Streaming Tool Execution** | Speculative fast-apply on single file | Batch tool execution after turn stream | Sequential/batch after `tool_use` block | **SSE `delta.tool_calls` Incremental Assembly + Read-Only Prefetch Readiness (`try_parse_streaming_read_prefetch`)** |
| **Model Routing & Learning** | Static cloud model picker + Auto | Static model inheritance (`inherit`/`pro`/`flash`) | Manual `/model` + `subagent` tier | **Empirical Bayes `CommunityIntelligenceBundleV1` + Online Bradley-Terry Arena + Local 5-Spoke MENS Fast-Apply** |

---

## 3. Milestone `v3.4` Implementation Architecture (`Phases 178–182`)

### 3.1 Phase 178: Prefix-Locked Two-Tier Tool Selection (`CACHE-PREFIX-01..04`)
- **Module**: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/ast_slice_and_preflight.rs`
- **Mechanism**:
  - `FROZEN_PREFIX_CORE_TOOLS` defines the 12 highest-frequency workspace tools in a fixed canonical sequence.
  - `select_prefix_locked_tools_for_turn` guarantees that indices `0..frozen_prefix_count` are identical across turns regardless of user prompt wording, while the remaining slots up to `max_tools` (`20`) are populated from the spoke-conditioned BM25 ranking (`select_tools_for_turn_ranked`).
  - Overflow tools (`rank 20..28`) are summarized into `PrefixLockedToolSelection::deferred_catalog_hint` (`[DEFERRED MCP TOOLS AVAILABLE ON DEMAND: ...]`) and recorded in turn telemetry (`frozen_prefix_tools_count`, `deferred_tool_catalog_hint`).

### 3.2 Phase 179: AST Symbol-Slice Read & Skeleton Folding (`AST-SLICE-01..04`)
- **Module**: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/ast_slice_and_preflight.rs`
- **Mechanism**:
  - `extract_symbol_slice(source, symbol_name)` scans Rust, TypeScript/TSX, and `.vox` source for `fn`, `struct`, `enum`, `trait`, `impl`, `interface`, `type`, `const`, or `function` declarations, includes preceding attributes/doc-comments (`#[...]`, `///`, `@...`) and top-of-file `use`/`import` headers, and extracts the balanced brace/statement span with 1-based line numbers.
  - `fold_code_to_ast_skeleton(source, max_chars)` preserves all top-level and member signatures (`pub fn`, `pub struct`, `export interface`, `use`, `import`) while folding implementation bodies into `/* ... N body line(s) elided ... */`.

### 3.3 Phase 180: Pre-Edit Cross-Crate Grounding & Write-Time Repo Guards (`PREFLIGHT-GROUND-01..04`)
- **Modules**: `ast_slice_and_preflight.rs` & `lifecycle_hooks.rs`
- **Mechanism**:
  - `preflight_check_write_guards(path, content, cargo_toml_content)` inspects write payloads in `<2ms`:
    1. Detects `use vox_<crate>::...` imports missing from the target crate's `Cargo.toml` (`unlinked_crates`).
    2. Computes non-blank LoC (`non_blank_lines`), setting `god_object_warning` at `450..=500` lines and `god_object_exceeded` above `500` lines.
    3. Flags banned direct crypto imports (`chacha20poly1305`, `ed25519_dalek`, `x25519_dalek`, `sha2`, `sha3`, `md5`) outside `crates/vox-crypto/` and hardcoded versioned model IDs (`claude-sonnet-4.6`, etc.) in `.ts`/`.tsx` files.
  - Wired directly into `evaluate_pre_tool_use_hook` so write tools block banned literals/imports immediately and inject LoC/dependency advisories into `additional_context`.

### 3.4 Phase 181: Speculative Streaming Tool Prefetch (`STREAM-EXEC-01..04`)
- **Module**: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs`
- **Mechanism**:
  - `try_parse_streaming_read_prefetch(tool_name, arguments_raw)` checks in-flight `delta.tool_calls` accumulators as chunks arrive and validates read-only (`is_read_only_memoizable`) JSON arguments as soon as their outer `{...}` closes, enabling zero-fallback single-pass SSE tool turns.

### 3.5 Phase 182: Workspace Test Canonicalizer, FIFO `FakeBackend` & MENS Fast-Apply (`FAST-APPLY-01..04`)
- **Modules**: `lifecycle_hooks.rs` & `crates/vox-gui/src/remote/calls.rs`
- **Mechanism**:
  - `canonicalize_workspace_command` rewrites `cargo fmt --all` → `vox run scripts/fmt.vox -- --all`, `-p vox-gui --lib` → `-p vox-gui --bin vox-gui`, and `test -- --run <file>` → `exec vitest run <file>`.
  - `resolve_mens_fast_apply_target` selects the lowest-latency local 5-spoke MENS model (`SelectionAxes::LATENCY_FIRST`, `complexity = 3`, `prefer_local = true`) for fast code-edit application.
  - `FakeBackend` in `crates/vox-gui/src/remote/calls.rs` uses `VecDeque<Result<Value, String>>` per tool/GET key so multi-call tools (such as 2-arm `chat_arena_compare`) dequeue distinct responses in FIFO order while retaining the final entry for single-response callers.
