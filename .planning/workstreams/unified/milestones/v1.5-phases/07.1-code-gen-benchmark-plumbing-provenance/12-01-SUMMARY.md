---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 01
subsystem: llm-facade
tags: [llm-egress, seed, openai-wire, reproducibility, wiremock]
requires: []
provides:
  - "vox_llm_egress::ChatParams::seed: Option<u64>, serialized as top-level \"seed\" only when Some"
  - "vox_actor_runtime::llm::LlmConfig::seed: Option<u64>, forwarded by llm_chat and llm_stream"
affects: [12-02, 12-05]
status: complete
actuals:
  tokens: 1700
  tasks: 2
  commits: 2
plan_head_before: c4480404424c1bda322eb63febb59131d51dc978
tech-stack:
  added: []
  patterns:
    - "top_p-shaped optional field: Option + skip_serializing_if, so unset callers keep a byte-identical wire body"
key-files:
  created: []
  modified:
    - crates/vox-llm-egress/src/lib.rs
    - crates/vox-llm-egress/src/wire.rs
    - crates/vox-llm-egress/tests/wire_mock.rs
    - crates/vox-actor-runtime/src/llm/types.rs
    - crates/vox-actor-runtime/src/llm/chat.rs
    - crates/vox-actor-runtime/src/llm/stream.rs
    - crates/vox-actor-runtime/src/llm/cascade.rs
    - crates/vox-actor-runtime/src/model_resolution.rs
    - crates/vox-cli/src/commands/chat.rs
    - crates/vox-effort-audit/src/judge/mod.rs
    - crates/vox-effort-route/src/embed.rs
    - crates/vox-effort-route/src/route/mod.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/harness_issue_judge.rs
    - crates/vox-orchestrator/tests/context_compaction_wiring_test.rs
    - crates/vox-search/src/embedding_env.rs
key-decisions:
  - "No serde attribute on LlmConfig.seed: serde already maps an absent Option key to None, proven by llm_config_without_seed_key_still_deserializes"
requirements-completed: []
---

# Phase 12 Plan 01: Sampling seed on the LLM facade and OpenAI wire Summary

A benchmark run can now pass a sampling seed end to end: `LlmConfig.seed` -> `ChatParams.seed` -> top-level `"seed"` in the chat and stream HTTP bodies, and requests without a seed are byte-identical to before. BENCH-02 is not marked complete here (the orchestrator does that when 12-02 lands).

## Commits

| Task | Commit | What |
|---|---|---|
| 1 (tracer) | 3fa650d9e | `ChatParams.seed`, `OpenAiChatRequest.seed`, `build_request` forwarding, two wiremock tests |
| 2 | 8a9526c32 | `LlmConfig.seed`, forwarding at 3 `ChatParams` literals (chat.rs 1, stream.rs 2), `seed: None` at every `LlmConfig` literal, two facade tests |

## Mutation proof 3 (seed forwarding in build_request)

- Before: `grep -c 'seed: params.seed' crates/vox-llm-egress/src/wire.rs` = 1
- Mutated line to `seed: None,`: count = 0; `chat_once_sends_seed_when_set` FAILED (both nextest tries)
- Restored: count = 1; `git diff` of wire.rs showed only the intended 3 added lines; the 2 seed tests passed again (2 passed)

## Verification

- `cargo nextest run -p vox-llm-egress --test wire_mock -E 'test(seed)'`: 2 passed
- `cargo nextest run -p vox-actor-runtime --lib -E 'test(llm_chat_forwards_seed_to_the_wire) | test(llm_config_without_seed_key_still_deserializes)'`: 2 passed
- `cargo check --all-targets` on vox-actor-runtime, vox-llm-egress, vox-cli, vox-effort-audit, vox-effort-route, vox-orchestrator, vox-research-shim, vox-search: exit 0
- vox-orchestrator-mcp: `cargo check --lib` exit 0. See Deferred Issues for its `--all-targets` state.

## Deviations from Plan

**1. [Rule 2 - Test-first policy] Added a test to vox-search embedding_env.rs**
- **Found during:** Task 2 commit (lefthook `tdd-guard`)
- **Issue:** the mechanical `seed: None` edit touched a file whose pre-existing `pub fn embedding_config_from_env` had no `#[test]`, and the hook blocked the commit.
- **Fix:** added `embedding_config_never_carries_a_seed` (passes). No hook bypass.
- **Files modified:** crates/vox-search/src/embedding_env.rs
- **Commit:** 8a9526c32

**2. Three planned files needed no edit.** `crates/vox-orchestrator/src/models/registry.rs`, `crates/vox-research-shim/src/research/metering.rs` and `crates/vox-search/src/embeddings.rs` build configs through `LlmConfig::*` constructors or `..base` update syntax, so they compile unchanged. 13 of the 16 Task 2 files were touched.

**3. Test module placement.** The plan names "chat.rs test module"; the new facade tests live in the existing `infer_with_retry_latency_tests` module and reuse its `mock_config`.

## Deferred Issues

- `cargo check -p vox-orchestrator-mcp --all-targets` fails on 5 errors in code this plan did not touch and unrelated to `seed`: `INFERENCE_PROFILE_TEST_LOCK` is a private import (`llm_bridge/model_route_policy/resolve.rs:799`) and four `visus_review::types::CacheEntry` initializers miss `defects`/`ux_report` (`visus_review/types.rs:240`, `visus_review/mod.rs:119,1200,1238`). No error mentions `seed`, so the `agent_loop.rs` test literal edit is typechecked in the same pass without complaint, but it is not proven by a clean build. Out of scope; left alone.

## Known Stubs

None.

## Threat Flags

None. T-12-01-01 mitigated and proven by `chat_once_omits_seed_when_none`.

## Self-Check: PASSED

- 3fa650d9e and 8a9526c32 exist in `git log`; `git rev-list --count c44804044..HEAD` = 2
- Both commits contain only this plan's files (pathspec-scoped); pre-staged vox-crypto/Cargo.lock/orchestrator preregistration files remain staged and uncommitted
