---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 04
subsystem: search
tags: [vox-search, kb_findings, capture-sink, trust-tier, redaction, snapshot, fail-closed]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "SanitizedText/ContentHash/FindingOrigin (03-02), insert_pending_finding / kb_findings_list (03-03)"
provides:
  - "vox_search::capture::{WebFindingCapture, CapturedWebHit} and SearchRuntimeContext::web_capture / with_web_capture"
  - "vox_orchestrator_mcp::kb_tools::findings::{CaptureInput, CaptureOutcome, capture_finding, extract_quotes, snapshot_relpath, McpFindingCapture, web_capture_for_state} + size caps"
  - "Web leg (execute_search_plan) and Tavily CRAG branch (bundle.rs) feed only the capture sink; no corpus write path remains"
  - "persist_text_document_chunk deleted"
affects: [03-05, 03-06, 03-08, 03-11, 03-13, phase-09]
tech-stack:
  added: []
  patterns: ["injected fire-and-forget sink (mirrors LexicalMemoryFallback) so vox-search needs no vox-crypto edge", "redact before split/truncate, then sanitize", "content-addressed snapshot, hash-only filename"]
key-files:
  created:
    - crates/vox-search/src/capture.rs
    - crates/vox-search/tests/web_capture_test.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/findings.rs
    - crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs
  modified:
    - crates/vox-search/src/lib.rs
    - crates/vox-search/src/context.rs
    - crates/vox-search/src/execution.rs
    - crates/vox-search/src/bundle.rs
    - crates/vox-search/src/ingest.rs
    - crates/vox-search/src/policy.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/mod.rs
    - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
    - crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs
key-decisions:
  - "Redact the whole summary source (bounded to 2 MiB) before extracting quotes or truncating, so a secret is never split on '.' or cut in half"
  - "source_url and title go through redact+sanitize like the summary; an IPv4 literal in a URL is therefore stored as [REDACTED_IP] (redact_owned design)"
  - "Tavily capture passes snippet empty and full_text = content; capture derives the summary from the full_text prefix"
  - "Dropped the advertised tavily:<url> DurableArtifact entries: nothing is persisted at that URI after the mirror removal"
requirements-completed: [TRUST-02, TRUST-04, TRUST-05]
status: complete
commits: 2
plan_head_before: 16263d1cf31c0cc9e4c9dc846eb256039700acc8
actuals:
  tokens: 30000
  tasks: 2
  commits: 2
duration: 80min
completed: 2026-10-06
---

# Phase 3 Plan 04: Web capture chokepoint Summary

Web hits from the SearXNG/DDG web leg and the Tavily CRAG branch now reach only an injected `WebFindingCapture` sink; the MCP sink turns each into a redacted, sanitized, BLAKE3-hashed, pending Untrusted `kb_findings` row with a content-addressed snapshot under `.vox/cache/kb-findings/`, and nothing enters `search_documents`.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | Web leg to pending Untrusted finding end to end, zero corpus rows | 2a6a24162 |
| 2 | Harden capture at ingest, reroute Tavily CRAG mirror, delete mirror helper | 1d4fec63d |

## What was verified

- `cargo nextest run -p vox-search capture_` and `-E 'test(/capture_|crag|ingest/)'`: 19 passed.
- `cargo nextest run -p vox-orchestrator-mcp capture_ findings`: 16 passed (includes `capture_redteam_neutralized_at_ingest` over all 13 red-team cases, `capture_concurrent_same_hit_one_row`, `capture_web_leg_end_to_end`).
- `cargo check -p vox-orchestrator-mcp -p vox-cli -p vox-research-shim -p vox-cli-research`: clean.
- `VOX_SKIP_FRESHNESS_CHECK=1 cargo run -p vox-cli -- ci crate-edges`: `OK (689 live in-tree edges within baseline)`; no Cargo.toml touched. (It also prints a pre-existing stale-baseline warning for `vox-gui -> vox-db-types`, not from this plan.)
- Acceptance greps: no `persist_text_document_chunk` anywhere in `crates`; no `"tavily:` in bundle.rs; no `web-ingest:` in execution.rs; no `hash_fast_hex|blake3::` or `let _ =` in findings.rs; `web_capture_for_state` appears once in retrieval.rs and twice in handlers_memory.rs; `git check-ignore` on a snapshot path exits 0.

## Mutation proofs

Presence counts are `grep -c` before / during mutation / after restore.

1. No-sink guard (Task 1): re-added a `persist_text_document_chunk` spawn in the `None =>` branch of the web leg in execution.rs (count 0 -> 1 -> 0). `capture_no_sink_no_writes` went red on `assertion left == right: left 1, right 0` (the row-count assertion); restored, green.
2. Redaction (Task 2): removed both `vox_redact::redact_owned(` calls in findings.rs (count 2 -> 0 -> 2). `capture_secrets_are_redacted_in_summary_and_quotes` went red; restored, green.
3. Hash-only filename (Task 2): replaced `snapshot_relpath(&hex)` with a URL-derived name (count 1 -> 0, `scope.replace` 0 -> 1 -> restored 1 / 0). `capture_snapshot_filename_is_hash_only` went red; restored, green.

## Deviations from Plan

**1. [Rule 1 - Bug] Removed the `tavily:<url>` DurableArtifact entries in bundle.rs**
- **Found during:** Task 2 acceptance (`rg '"tavily:' bundle.rs` must print nothing).
- **Issue:** the CRAG branch advertised a durable artifact with `uri = tavily:<url>` for the chunk it mirrored into the corpus. With the mirror gone that URI points at nothing, so the A2A response would carry a dangling reference.
- **Fix:** dropped the artifact loop (comment left in place); the test asserts `durable_artifacts` is empty. The only consumers just serialize the vector (`a2a_contract.rs`).
- **Files:** crates/vox-search/src/bundle.rs. **Commit:** 1d4fec63d.

**2. [Test adaptation] End-to-end `source_url` assertion**
- The wiremock SearXNG URL is `http://127.0.0.1:<port>/...`; capture redacts IPv4 literals by design, so the e2e test compares against `redact_owned(hit_url)`. A public-host URL equality (`https://e.x/a`) is asserted in `capture_empty_input_is_rejected_without_writes`.

Otherwise the plan executed as written. TDD note: the vox-search and MCP tests were written alongside the implementation in one pass, so there is no separate RED commit; red behavior is demonstrated by the three mutation runs above.

## Known issues

- The web-leg tests take about 7 s each (same baseline as the existing `web_leg_toggle_test`) and `capture_no_sink_no_writes` / `capture_sink_receives_hit_and_corpus_stays_empty` each flaked once with 0 web requests/hits under parallel build load, passing on nextest retry. Run alone they pass 3 of 3.
- D-05 interim regression (accepted): GUI in-process search, CLI research and orchestrator goal planning have no sink, so their web hits are persisted nowhere until 03-06/03-08 inject one.

## Known Stubs

None.

## Threat Flags

None. The only new filesystem write is the snapshot under the repo-root `.vox/cache/kb-findings/`, already in the threat register (T-03-09, T-03-42). Snapshots are written unredacted by design (evidence for the content hash) and are never read by retrieval code; the "never sent to a model" half of the privacy prohibition has no automated check and is flagged unverified.

## Self-Check: PASSED

Created files exist (capture.rs, findings.rs, both test files); commits 2a6a24162 and 1d4fec63d present in `git log`.
