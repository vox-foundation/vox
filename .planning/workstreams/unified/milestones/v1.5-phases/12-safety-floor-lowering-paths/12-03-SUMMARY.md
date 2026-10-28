# Phase 12 Plan 03 Summary: Closing Preference Lowering Path (FLOOR-03)

**Execution Mode:** Autonomous (`/gsd-autonomous`)  
**Status:** Completed & Verified  

---

## 1. Summary of Changes

Closed the MCP preference self-lowering vulnerability where `vox_preference_set` could be used by callers to mutate oversight-critical parameters:
1. **Oversight Key Prefix Table in `handlers_preferences.rs`:**
   - Defined `const OVERSIGHT_PREF_PREFIXES: &[&str]` covering `socrates_gate_enforce` (and `...d`), `attention_`, `oversight.`, `oversight_`, `approval`, `trust_`, `permission`, and `tier_gate`.
   - Added `is_oversight_preference_key(&str) -> bool` trimming and lowercasing keys before checking prefixes.
2. **Refusal Guard at Top of `preference_set`:**
   - First statement in `preference_set`: `if is_oversight_preference_key(&params.key) { ... }`.
   - Immediately returns `ToolResult::<String>::err_with_remediation` with `OVERSIGHT_PREF_REFUSED: preference '<key>' affects oversight and cannot be set through MCP` before any database write or config mutation.
   - Points human operators to Vox.toml, the GUI settings, or the `vox attention` CLI.
3. **Dead Branch & Lock Removal:**
   - Removed the unreachable live config-mutation branches (`socrates_gate_enforced`, `attention_enabled`, `attention_budget_ms`, `attention_alert_threshold`).
   - Removed unused `const REM_LOCK_CFG`.
4. **Verification & Tests:**
   - Added unit test `oversight_key_predicate_table` verifying positive matches (including whitespace/case variants) and negative near-misses (`attentive_mode`, `trusted_hosts`).
   - Added integration test `preference_set_refuses_oversight_keys` verifying refusal across all oversight keys and confirming orchestrator config remains unchanged.
   - Added integration test `preference_set_allows_non_policy_keys` verifying non-policy keys (`ui_theme`, `editor.font_size`) proceed to database handling without refusal.

---

## 2. Acceptance Criteria Verification

- `grep -cF 'if is_oversight_preference_key(&params.key) {' crates/vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs` == 1 (verified)
- `grep -c 'OVERSIGHT_PREF_REFUSED' crates/vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs` >= 2 (found 4, verified)
- `grep -c 'fn preference_set_allows_non_policy_keys' crates/vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs` == 1 (verified)
- `grep -cF 'cfg.socrates_gate_enforce = enforce' crates/vox-orchestrator-mcp/src/memory_tools/handlers_preferences.rs` == 0 (verified)
- `cargo clippy --no-deps -p vox-orchestrator-mcp --all-targets -- -D warnings` exits 0 (verified)
- Test suite: `cargo test -p vox-orchestrator-mcp --lib -- memory_tools::handlers_preferences::tests` (3 passed, 0 failed, verified)
