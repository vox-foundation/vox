---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 07
subsystem: chat
tags: [kb-leak, trust-03, ret-02, grow-06, chat-message, retrieval-bundle, legacy-voxkb]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "vox_db::LEGACY_VOXKB_DDL, VoxDb::{legacy_kb_counts, kb_findings_counts} (03-03)"
provides:
  - "chat turn no longer writes model output into the knowledge base (post-turn spawn + 3 signal adapters deleted)"
  - "RetrievalBundle without kb_lines; no legacy KB text or @kb dump in any chat prompt"
  - "parse_kb_mentions kept (doc-marked for the Phase 7 Library-scope remap, D-05), no consumer"
  - "MemoryManager::sync_verified_research_findings and its test deleted (GROW-06 MEMORY sink)"
  - "message_kb_leak_tests.rs: seeded legacy-KB chat-turn regression tests (kb_leak_*)"
affects: [03-09, 03-10, 03-12, 03-13, phase-07]
tech-stack:
  added: []
  patterns: ["leak tests seed the legacy tables AND a row the retired path would have matched, so a regression cannot pass vacuously"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message_kb_leak_tests.rs
  modified:
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
    - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/mod.rs
    - crates/vox-orchestrator/src/memory/manager.rs
  deleted:
    - crates/vox-orchestrator-mcp/src/kb_tools/signal_chat.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/signal_research.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/signal_scientia.rs
    - crates/vox-orchestrator/tests/memory_research_sync_test.rs
key-decisions:
  - "D-05 applied: @name still parses, injects nothing until Phase 7 (accepted regression, CHANGELOG in 03-13)"
  - "D-06 applied: MEMORY sink deleted, not rerouted"
  - "Prompt-leak seed embeds the canonicalized prompt in a legacy entry: the retired enrichment searched with a substring match on the whole canonical prompt, so a plain keyword seed never matched and the first draft of the test passed before deletion (vacuous)"
patterns-established:
  - "test-first red evidence for a deletion: keep the failing assertion run in the log before removing the code"
requirements-completed: [TRUST-03, RET-02, GROW-06]
duration: long (build host heavily loaded; one rustc hang and two nextest stalls, see Issues)
completed: 2026-10-07
status: complete
commits: 2
plan_head_before: 451cd8999033a9c751377dbd620eddc5be1ee4d1
actuals:
  tokens: 6100
  tasks: 2
  commits: 2
---

# Phase 3 Plan 07: Chat-turn KB leak closure Summary

A chat turn can no longer write an accepted legacy KB row, and legacy KB text (keyword enrichment or `@name` dump) can no longer reach a chat prompt; the dead MEMORY.md research sink is gone.

## Commits

- `a2aa0f0f4` fix(03-07): close the chat-turn KB writeback loop and delete the signal adapters (Task 1, tracer)
- `e98a1235a` fix(03-07): keep legacy KB text out of chat prompts and delete the dead MEMORY sink (Task 2)

## What was built

- **Task 1 (tracer).** Deleted the fire-and-forget `ingest_chat_turn` spawn in `chat_message` and the three signal modules (`signal_chat`, `signal_research` with its `max(0.95)` floor, `signal_scientia` with 0.98) plus their `pub mod` lines. `message_kb_leak_tests.rs` (registered like `turn_trace_tests`) seeds `LEGACY_VOXKB_DDL`, a KB `mykb`, a keyword rule `widgets` and entries, drives `chat_message` against a wiremock OpenRouter that replies with a rule-matching sentence, waits 2 s, and asserts `legacy_kb_counts().kb_entries` unchanged and `kb_findings_counts().total == 0` (and that the turn succeeded and the model was called).
- **Task 2.** `RetrievalBundle.kb_lines`, its `KbStore::search_entries` fill and constructor field removed from `retrieval.rs`; the `[AUTONOMOUS RESEARCH — KNOWLEDGE BASE]` push and the whole `kb_mention_lines` / `[MENTIONED KNOWLEDGE BASES]` block removed from `message.rs`. `parse_kb_mentions` and its tests untouched apart from a doc line (kept for Phase 7 INJ-09, no consumer). `MemoryManager::sync_verified_research_findings` and `tests/memory_research_sync_test.rs` deleted. Two prompt tests (`kb_leak_keyword_prompt_carries_no_legacy_kb_text`, `kb_leak_at_mention_injects_nothing`) read the request bodies wiremock received and assert neither the marker string nor either header is present.

## Evidence

**Red before deletion (Task 1).** `cargo nextest run -p vox-orchestrator-mcp kb_leak_` against the unmodified spawn: `kb_leak_chat_turn_writes_no_kb_entry_or_finding` FAILED, `assertion left == right failed: a chat turn must not write a legacy kb_entries row, left: 2, right: 1` (3 tries). After deletion: PASSED.

**Red before deletion (Task 2).** First draft of the keyword test PASSED before deletion (vacuous: `kb_search_entries` is `lower(content) LIKE %<whole canonical prompt>%`, so the plain keyword seed never matched). Fixed by seeding entry `e2` whose content embeds the canonicalized prompt; re-run: `kb_leak_keyword_prompt_carries_no_legacy_kb_text` and `kb_leak_at_mention_injects_nothing` both FAILED with `legacy KB text reached the model prompt (ZX9-LEGACY-KB-MARKER-4471)` (3 tries each). After deletion: both PASS.

**Mutation proofs (grep -c presence counts, before / mutated / restored).**

| Guard | Mutation | Counts | Result |
|-------|----------|--------|--------|
| TRUST-03 writeback (T-03-11) | raw `INSERT INTO kb_entries` through `state.db` at the old spawn site in `message.rs` | `INSERT INTO kb_entries`: 0 / 1 / 0 | `kb_leak_chat_turn_writes_no_kb_entry_or_finding` FAILED (`left: 2, right: 1`) |
| RET-02 prompt (T-03-26) | re-added a `[MENTIONED KNOWLEDGE BASES]` context part pushing every legacy entry (via `legacy_kb_entries_page`) before `all_context_files` | `MENTIONED KNOWLEDGE BASES`: 0 / 1 / 0 | both prompt tests FAILED on the marker; restored and re-verified 0 |

**Single-definition counts** (`grep -c`, after both tasks): `pub async fn chat_message` in message.rs = 1; `pub fn parse_kb_mentions` in retrieval.rs = 1; `kb_leak_tests` module registered once.

**Verification.** `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_leak_|parse_kb_mentions|retrieval_bundle/)'`: 9 passed. `cargo nextest run -p vox-orchestrator memory`: 27 passed (run before the new manager.rs unit test was added). `cargo test -p vox-orchestrator --lib for_account_scopes`: 1 passed. `vox ci sql-surface-guard --all` (from source): OK. `rustfmt --check` clean on touched files. Acceptance greps: no `signal_chat|signal_research|signal_scientia|ingest_chat_turn`, no `max(0.95)|0.98` in kb_tools, no `kb_lines`, no `sync_verified_research_findings`, no `knowledge_base::store|KbStore` in `chat_tools`/`memory_tools` (all `rg` empty); `pub fn parse_kb_mentions` one line.

## Deviations from Plan

**1. [Rule 3 - Blocking] manager.rs needed a real unit test to commit.** Staging `manager.rs` tripped the pre-commit `tdd-guard` (`skeleton/untested-pub-api`, whole-file scan: 27 public fns, no `#[test]`; the deleted integration test lived in `tests/`, so it was never visible to the guard). Added `for_account_scopes_the_manager_and_starts_with_no_keys` rather than suppressing. Not `--no-verify`.
- **Files:** crates/vox-orchestrator/src/memory/manager.rs. **Commit:** e98a1235a.

**2. [Rule 1 - Bug in own test] vacuous keyword test.** See Evidence; fixed in the same task before the deletion.

**3. [Rule 1 - Bug in own test] sql-surface-guard tripped on a comment.** A code comment in the new test file spelled the banned `connection().query(` token; the guard scans raw text. Reworded.

Otherwise none; plan executed as written (tracer gate: verified end-to-end before expansion).

## Issues Encountered

- Build host heavily loaded by other sessions (load average 12-19). One `rustc` for `vox-orchestrator-mcp` hung until the 3000 s timeout: killed, deleted only `target/debug/incremental/vox_orchestrator_mcp-*` (gitignored), rebuilt fine. Two `cargo nextest run` invocations (`-p vox-orchestrator`) stalled for the full timeout after a successful build, with no child test process; the same test was then run directly with `cargo test -p vox-orchestrator --lib`. One mutation run reported the writeback test as flaky (passed on a retry) with 140 s per try under that load; the mutation still failed the two prompt tests as intended. No cargo/rustc processes started by this plan remain.
- `main is 94 commits ahead of origin/main` and nightly-failure banners are printed by the pre-commit hook; unrelated.

## Known Stubs

None.

## Threat Flags

None. Net effect removes surface (a model-output writer, a prompt injection path, a file sink).

## Self-Check: PASSED

- Created/modified files exist: message_kb_leak_tests.rs FOUND; message.rs, retrieval.rs, kb_tools/mod.rs, manager.rs FOUND; four deleted files confirmed absent.
- Commits a2aa0f0f4 and e98a1235a FOUND in `git log`; `git rev-list --count 451cd8999..HEAD` = 2 at the time of the code commits.
