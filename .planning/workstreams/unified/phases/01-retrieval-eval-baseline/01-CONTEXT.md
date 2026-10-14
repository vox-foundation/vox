# Phase 1 — Context (user decisions)

Recorded 2026-10-04. Binding for executors.

## Decisions

- **D-01 Snapshot form:** `frozen-snapshot` (user, 2026-10-04, 01-04 Task 1).
- **D-02 Seed queries — delegated:** the user first chose to author ≥40 seed queries, then
  delegated ("solve these yourself"). Mining the user's past session prompts into this public
  repo was refused by the safety classifier, so seeds are **agent-authored** and labelled
  honestly with a new origin `agent-seed` — never `human`.
  - EVAL-02 is amended: ≥40 **seed** queries, where seed = `human` ∪ `agent-seed`.
  - The ratchet subset `dev/human` becomes `dev/seed` (human + agent-seed); `dev/all` unchanged.
  - Real human-authored queries may be added later as `origin: human` via a reviewed re-epoch.
  - Independence safeguard: the agent that authors seed wording must not be the agent that
    proposes qrels (01-08/01-09), and must not copy doc sentences.
- **D-03 All other Phase 1 assumptions (AP-01..AP-15):** accepted at their recommended defaults.
