---
phase: 338-spoke-aware-gating-and-hub-repos
plan: "07"
subsystem: mens-eval-suites
tags: [agent-episodes, rust-bench, eval-spoke, suite-gates]
requires:
  - phase: 338-05
    provides: "SuiteReport, Episode runner, and 72-episode MCP catalog"
provides:
  - "Per-suite gate schema SuiteGate and EvalGatePolicy.suites map"
  - "check_run enforcement of policy.suites against suite_report.<suite>.json and paired base controls"
  - "eval_spoke::{schema_validator, run_agent_suite, run_rust_bench_suite}"
  - "Suite gate configuration blocks in mens/config/{eval-gates,eval-gates-rust,eval-gates-agents}.yaml"
affects: [vox-ml-cli, mens/config]
tech-stack:
  added: []
  patterns: [paired suite verdicts in eval-gate]
key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/eval_spoke.rs
  modified:
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/eval_gate/policy.rs
    - crates/vox-ml-cli/src/commands/mens/eval_gate/check_run.rs
    - crates/vox-ml-cli/src/commands/mens/eval_gate/tests.rs
    - mens/config/eval-gates.yaml
    - mens/config/eval-gates-rust.yaml
    - mens/config/eval-gates-agents.yaml
key-decisions:
  - "Keep suites block: false, require_report: false in shipped configs while enforcing SuiteFloors and McNemar improvement whenever a suite_report.<name>.json file is present"
  - "Return paired (base, candidate) SuiteReport structs from run_agent_suite and run_rust_bench_suite matching vox_eval::suite_report::suite_verdict"
requirements-completed: [SPOKE-07, SPOKE-08]
completed: 2026-11-13
---

# Phase 338 Plan 07: Behavioral & Functional Suite Runners and Gate Enforcement Summary

**Wired `suite_report` verification into `vox mens eval-gate` and added paired `agent_episodes` / `rust_bench` runners.**

## Key Changes
- Added `SuiteGate` and `EvalGatePolicy.suites` in `eval_gate/policy.rs`, evaluated in `eval_gate/check_run.rs` via `vox_eval::suite_report::suite_verdict`.
- Created `crates/vox-ml-cli/src/commands/mens/eval_spoke.rs` exposing `schema_validator`, `run_agent_suite`, and `run_rust_bench_suite`, with `agent_suite_reports_oracle_gain_over_never_call` asserting 72/72 oracle pass over `never_call` (`p < 0.05`).
- Added `suites:` sections in `mens/config/eval-gates.yaml`, `mens/config/eval-gates-rust.yaml`, and `mens/config/eval-gates-agents.yaml`, and updated `gate_keys_inventory` in `eval_gate/tests.rs`.

## Verification
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_spoke hub train_arm`: `99 passed; 0 failed; 0 ignored` (including `commands::mens::eval_spoke::tests::agent_suite_reports_oracle_gain_over_never_call` and `commands::mens::eval_gate::tests::gate_keys_inventory`).
