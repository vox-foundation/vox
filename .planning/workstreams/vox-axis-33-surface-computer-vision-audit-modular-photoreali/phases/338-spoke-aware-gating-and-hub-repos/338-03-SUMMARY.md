# Phase 338-03 Summary: Purge Paper Keys and Wire Supervised Ratio

## Completed Scope
1. **Purged All Paper Keys from Policy & Configs (D-03, SPOKE-02)**:
   - Deleted dead paper keys from YAML configs (`mens/config/eval-gates-ladder.yaml`, `mens/config/eval-gates.yaml`, `mens/config/eval-gates-rust.yaml`, `mens/config/eval-gates-agents.yaml`, `mens/config/eval-gates-post-train.yaml`): `truncation`, `throughput`, `eval_local`, `per_context`, `modal_mix`, `mcp_tool_schema`, `review_recurrence`, `version`, `perplexity`, `rust_compile_rate`, `clippy_clean_rate`, `tool_call_valid_json_rate`, `tool_name_exists_rate`, `bfcl_accuracy`.
   - Deleted dead `eval-gates-bfcl.yaml`.
   - Deleted dead `crates/vox-ml-cli/src/commands/mens/eval_gate/bfcl.rs`.
   - Cleaned `baseline.rs`: replaced `bfcl_results.json` and `bfcl_accuracy` with general `eval_results.json` and `accuracy`. Preserved `BaselineEntry`, `BaselineReport`, `beat_base`, and `wilson_ci` for `parity_report.rs`.
   - Zero occurrences of `bfcl` remain under `crates/vox-ml-cli/src/commands/mens/eval_gate`.

2. **Strict Deserialization (`#[serde(deny_unknown_fields)]`)**:
   - Added `#[serde(deny_unknown_fields)]` to `EvalGatePolicy` and sub-structs (`SupervisedRatioGate`, `PassAtKGate`, `AntiStubGate`, `CollateralDamageGate`, `SequentialBlock`, `SlicePin`).
   - Ensures no paper keys or misspellings can be silently reintroduced.

3. **Wired Real `supervised_ratio` Reader**:
   - `check_run.rs` reads `supervised_ratio_pct` directly from the nested `payload` of real training telemetry step lines emitted by `vox-populi`.
   - Fallback to top-level key and token count calculation maintained for legacy runs.
   - Added `supervised_ratio_reads_nested_payload` unit test proving parsing on real telemetry shapes.

4. **Policy & Collateral Report Verification**:
   - Fixed `CollateralDamageGate::to_requirement` to output canonical `vox_eval::reasoning_slice::PAIRED_RULE` string matching `check_collateral_report`.
   - Fixed `validate_collateral_rule` to enforce schedule size checks on active sequential rules while allowing inactive ones.
   - Added `gate_keys_inventory` test verifying exact mapping of remaining gate keys to producer tests.
   - Added `deleted_gate_key_fails_to_load` test verifying `deny_unknown_fields`.

## Verification
- `test ! -e crates/vox-ml-cli/src/commands/mens/eval_gate/bfcl.rs`: OK (deleted).
- `test ! -e mens/config/eval-gates-bfcl.yaml`: OK (deleted).
- `rg -n 'bfcl' crates/vox-ml-cli/src/commands/mens/eval_gate`: 0 occurrences (clean).
- `cargo test -p vox-ml-cli --features gpu --lib -- eval_gate parity_report`: 87 passed, 0 failed.
- `cargo run -p vox-cli -- ci crate-edges`: OK (703 live in-tree edges within baseline).
