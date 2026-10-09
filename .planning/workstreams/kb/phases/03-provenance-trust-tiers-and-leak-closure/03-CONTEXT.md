# Phase 3 — Context (user decisions)

Recorded 2026-10-04. Binding for executors.

## Decisions

- **D-01 Frozen-core schema sign-off (03-03 Task 1):** APPROVED. Delegated by the user
  (brbrainerd) on 2026-10-04 ("solve these yourself"); baseline bump to Phase 2's integer + 1
  (expected 96) and re-tiering existing rows as planned. Signer recorded as
  "brbrainerd (delegated to agent, 2026-10-04)".
- **D-02 Legacy tool retirement (03-10 Task 1):** APPROVED — retire the 12 legacy `vox_kb_*`
  tools with no aliases; CHANGELOG notes the break.
- **D-03 Existing unreviewed rows:** re-tier Untrusted and exclude from the auto preamble
  (not purged).
- **D-04 `persist_web_hits`:** stays default-on, meaning inbox capture with dedupe.
- **D-05 Interim regression (Phases 3→7):** accepted (Untrusted dropped from preamble, `@kb`
  inert, GUI/CLI research web hits unpersisted, GUI findings via toasts + `vox kb status`).
- **D-06 All other assumptions (taint rule, DPO-only MENS signals, sanitizer caps, snapshot
  location, etc.):** accepted at their recommended defaults.
- **D-07 Agent-written facts are Untrusted (gap closure, 2026-10-08, user decision):** model- or
  web-derived rows written through the model-callable memory store (`MemoryManager::persist_fact`
  and anything else reachable from a model tool call) must map to `ContextTrustTier::Untrusted`,
  never Advisory, so they cannot pass `exclude_untrusted_hits`. Narrows D-06: `fact`/`concept`
  stay Advisory only for non-agent writers. Resolves review finding WR-10.
- **D-08 Trusted means git-tracked and unmodified (gap closure, 2026-10-08, user decision):** the
  Library reindex stamps a `docs/src` file `Trusted` only if it is tracked by git and has no
  working-tree modifications; untracked or modified files are indexed `Untrusted`. Enforces the
  TRUST-01 "merged to git" definition. Resolves review finding WR-09.
