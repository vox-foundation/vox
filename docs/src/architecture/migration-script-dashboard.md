---
title: "Hybrid migration — script / CI metrics"
category: architecture
last_updated: 2026-03-21
---

# Migration metrics (script → `vox ci`)

| Metric | Baseline (2026-03-21) | Current (2026-03-21 QA recovery) |
|--------|------------------------|---------|
| GitHub `ci.yml` bash `scripts/*` invocations | 9 | 0 (Rust `vox ci` / `cargo run -p vox-cli -- ci …`) |
| Python doc-inventory in CI | 1 | 0 |
| Populi matrix steps (sequential) | 18 | 1 (`ci populi-gate --profile ci_full`) |
| `vox-cli` CI feature matrix includes `script-execution` | 0 | 1 (plain + `stub-check` mix) |
| `vox-compilerd` `run` RPC carries `RunMode` | no | yes (`mode` JSON field) |
| Stale ref scan (retired Python / shell gates in `docs/src` + workflows) | no | yes (`check-docs-ssot`) |
| Dogfood Populi orchestration in PS1 | ~60 lines | thin delegate → `vox populi pipeline` |
| ML workflow (`ml_data_extraction.yml`) Python one-liner for eval summary | 1 | 0 (`vox corpus eval --print-summary`) |
| GitLab inline `grep`/`find` repo guards | 3 blocks | `vox ci repo-guards` (in `vox-ci-guards` job) |

Source: [`docs/agents/baseline-script-metrics.json`](../../agents/baseline-script-metrics.json), [`docs/agents/script-registry.json`](../../agents/script-registry.json).
