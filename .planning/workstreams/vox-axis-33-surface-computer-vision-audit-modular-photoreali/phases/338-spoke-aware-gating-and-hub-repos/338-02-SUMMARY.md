# Phase 338-02 Summary: Versioned Gate Receipts & Spoke Gate State

## Completed Scope
1. **`vox-eval::gate_state`**:
   - Implemented `canonical_json` key sorting and deterministic serialization.
   - Added `policy_hash_input` capturing parsed policy YAML values and code constants (`PAIRED_RULE`, `GRADER_ID`, `SLICE_SIZE`).
   - Implemented `GateState` (`Passed`, `Failed`, `NeedsReGate`, `NotEvaluated`) and `gate_state(...)`.
   - Added `SpokeGateRow` and `spoke_gates_json` schema helpers.
   - Defined constants `RECEIPT_SCHEMA = "vox_mens_gate_receipt_v2"` and `SPOKE_GATES_SCHEMA = "vox_mens_spoke_gates_v1"`.

2. **Versioned JSON Schemas**:
   - `contracts/mens/spoke-gate-state.v1.schema.json`: Strict draft-07 contract governing `spoke_gates.json` across run dirs and staged publish bundles.
   - `contracts/mens/gate-receipt.v2.schema.json`: Strict draft-07 contract governing schema 2 receipts with mandatory `policy_blake3` and `inputs`.
   - Registered both schemas in `contracts/documentation/canonical-map.v1.yaml` under `mens-training` `spec_paths`.
   - Both schemas serve as the contract interface consumed by Phase 339 (Hub Cards v2) and Phase 340 (Local Model Selector).

3. **`vox-populi` & Hub Sync**:
   - `vox_populi::mens::gate_policy::spoke_gate_rows` parses gate profiles and returns per-spoke rows.
   - Precedence: ladder row evaluates first; collateral report only feeds ladder row (D-04); domain profiles lacking runs yield `NotEvaluated` (D-02).
   - In `hub_card/facts.rs`, `RunRecords` parses `spoke_gates` and `gate_of` evaluates the ladder row state.
   - In `hub_sync.rs`, `stage_publish_bundle` generates `spoke_gates.json` via `spoke_gate_rows` and `sanitize_json_paths`.
   - Added `spoke_gates.json` to `OPTIONAL_BUNDLE_FILES`.

4. **`vox-ml-cli` Writer & Receipts**:
   - `run_eval_gate` computes `policy_blake3` and writes schema 2 `gate_receipt.json`.
   - Writes `spoke_gates.json` alongside `gate_receipt.json`.
   - Added schema validation tests verifying that generated receipts and spoke gate manifests validate against draft-07 schemas.

## Verification
- `cargo test -p vox-eval --lib -- gate_state policy_hash`: 9 passed, 0 failed.
- `cargo test -p vox-populi --features mens-hf-hub --lib gate_policy`: 2 passed, 0 failed.
- `cargo test -p vox-populi --features mens-hf-hub --lib hub_card`: 113 passed, 0 failed.
- `cargo test -p vox-populi --features mens-hf-hub --lib hub_sync`: 14 passed, 0 failed.
- `cargo test -p vox-ml-cli --features gpu --lib -- receipt_v2 policy_hash_parity spoke_gates_schema receipt_schema`: passed.
- `jq empty contracts/mens/spoke-gate-state.v1.schema.json contracts/mens/gate-receipt.v2.schema.json`: valid JSON.
