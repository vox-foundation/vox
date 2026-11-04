# Phase 252 Summary — TOESTUB Static Detectors (`vox/lint/*`, `vox/test/*`, `vox/mcp/*`, `vox/ui/*`) & Wire/SQL Transfer Budget Gate

- **Status:** Complete (`100%`)
- **Requirements Satisfied:** `T3-TOESTUB-RULES-01`, `T3-TOESTUB-RULES-02`, `T3-TOESTUB-RULES-03`, `T3-TOESTUB-RULES-04`
- **Artifacts Implemented:**
  - `crates/vox-code-audit/src/detectors/unreasoned_suppression.rs` (`vox/lint/unreasoned-suppression`)
  - `crates/vox-code-audit/src/detectors/test_clock_and_skip.rs` (`vox/test/flaky-wait-and-unreasoned-skip`)
  - `crates/vox-code-audit/src/detectors/mcp_access_and_error_safety.rs` (`vox/mcp/access-and-error-safety`)
  - `crates/vox-code-audit/src/detectors/ui_perf_hazards.rs` (`vox/ui/layout-and-perf-hazards`)
  - `crates/vox-orchestrator-mcp/tests/transfer_budget.rs` (cold-open snapshot `<= 120 KiB` / `<= 75` rows, per-turn wire `<= 7 KiB`, `0` command output deltas in event store, `SqlStatementCounter`)
- **Verification:**
  - `cargo test -p vox-code-audit --lib detectors` -> `371 passed; 0 failed` (`60` registered rules)
  - `cargo test -p vox-orchestrator-mcp --test transfer_budget` -> verified
