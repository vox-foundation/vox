---
title: "T3→Vox CI/TOESTUB enforcement closure (v5.4)"
description: "Scorecard anchors for Phases 301–306: change-gated CI, transfer budgets, knip/PR hygiene, agent-ops, thin-transport detectors, fingerprint/cache/graph closure."
category: "Architecture SSOTs"
status: "current"
topic: "ci-guards"
domain: "platform"
authorship: "llm-generated"
authorship_model: "unknown"
---

# T3→Vox CI/TOESTUB enforcement closure (v5.4)

<!-- kb:summary -->
Closure note for milestone v5.4 (Phases 301–306), which ported T3 CI and TOESTUB enforcement
ideas into Vox. It links the machine-readable enforcement scorecard and maps each shipped gem to
its anchor: change-gated `vox ci check-changed`, transfer-budget reports, weighted test shards,
PR size labels, the evidence-asset ban, the kill-safety triad, the thin-transport detector and
the GUI sidecar fingerprint workflow. It also records what stayed partial or deferred, such as
live budget deltas in PR comments and the remaining oxlint rules, and points to the GUI cache
policy.
<!-- /kb:summary -->

Machine-readable scorecard: [`contracts/reports/t3-vox-ci-enforcement-scorecard.v1.json`](../../../contracts/reports/t3-vox-ci-enforcement-scorecard.v1.json).

## Shipped highlights

| Gem | Anchor |
|---|---|
| `check-changed` | `vox ci check-changed` + `contracts/ci/changed-path-owners.v1.yaml` |
| Transfer-budget artifact | `vox ci transfer-budget-report` |
| Weighted shards | `contracts/ci/shard-weights.v1.json` |
| PR size labels | `.github/workflows/pr-size.yml` |
| Evidence ban | `vox ci evidence-asset-ban` |
| Kill-safety triad | `AGENTS.md` + `docs/agents/kill-safety-and-hit-every-surface.md` |
| Thin-transport detector | `vox/transport/thin-transport-bypass` |
| Sidecar fingerprint | `.github/workflows/gui-sidecar-fingerprint.yml` (always green + label) |

## Partial / deferred

- Trusted budget PR comments post the **committed baseline** summary (full live-artifact delta like t3code Node reporter deferred).
- Knip runs via local/`pnpm dlx`; lockfile pin into `ui` job deferred.
- Surface graph SHAs refreshed from `~/dev` clones; `scripts/t3-surface-inventory.vox` full regen deferred if compile blockers remain.
- Remaining oxlint rules (`no-low-value-comments`, frame-waits, polling loops, disposable-render) listed as **deferred** in `contracts/reports/t3-oxlint-toestub-mapping-ledger.v1.json`.

## GUI cache

See [gui-cache-layers.md](../ci/gui-cache-layers.md) — main-only save, PR restore only (`vox ci cache-key-lint`).
