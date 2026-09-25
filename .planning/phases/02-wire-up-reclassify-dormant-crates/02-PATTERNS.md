# Phase 2: Wire Up & Reclassify Dormant Crates - Pattern Map

**Mapped:** 2026-09-25
**Files analyzed:** 7 (4 modified Rust/config, 1 new test file, 1 modified contract JSON, 1 modified doc) + 1 optional
**Analogs found:** 7 / 7
**Tracked-source gate:** every path below was verified with `git ls-files` (all tracked; no gitignored mirror paths).

## Scope confirmation (against CONTEXT.md + RESEARCH.md)

The expected list is confirmed. SC#1 (exec-policy) needs NO file changes (verification only: `cargo run -p vox-cli -- ci exec-policy-contract`). Files to touch:

| # | File | Action | SC |
|---|------|--------|----|
| 1 | `crates/vox-orchestrator/src/tool_receipt.rs` | modify (new error enum, validation in `issue_intent`, propagate in `issue`, in-file `#[cfg(test)]`) | SC#2 |
| 2 | `crates/vox-orchestrator/src/orchestrator/safety.rs` | modify (`issue_tool_receipt` return type) | SC#2 |
| 3 | `crates/vox-orchestrator/Cargo.toml` | modify (+1 dep line) | SC#2 |
| 4 | `contracts/ci/crate-edges.allow.v1.json` | modify (+1 `exceptions` entry, user-authorized) | SC#2 |
| 5 | `crates/vox-orchestrator/tests/tool_receipt_registry_gate.rs` | NEW integration test | SC#2 / D-04 |
| 6 | `docs/src/architecture/crate-classification-2026-05-08.md` | modify (2 lines: row 86, row 140) | SC#3 |
| 7 | `crates/vox-orchestrator/src/lib.rs` (line 377 re-export) | modify, one token (`ToolReceiptError`) | SC#2 |

Optional/discretionary (not required by REQ): same doc's row 85 (`vox-mcp-registry | DEAD | 0 | ... orphaned`) and row 139 are now stale as well (registry is consumed by vox-orchestrator-mcp today and by vox-orchestrator after this phase). The tombstone banner at line 15 already says so.

## Signature-propagation audit (issue_intent / issue_tool_receipt)

Repo-wide `rg` (crates/, excluding target) found these touch points; changing `issue_intent` to return `Result` affects ONLY:

| Site | Kind | Impact |
|------|------|--------|
| `crates/vox-orchestrator/src/tool_receipt.rs:90` `issue_intent` | definition | change return type |
| `crates/vox-orchestrator/src/tool_receipt.rs:145-155` `issue()` | only in-repo caller of `issue_intent`; does `self.issue_intent(..)` then `.fulfill_intent(..).unwrap()` | must propagate with `?` (return `Result<ToolReceipt, ToolReceiptError>`); `issue()` itself has zero callers |
| `crates/vox-orchestrator/src/orchestrator/safety.rs:8-18` `issue_tool_receipt` | only caller of `issue_intent` outside the file | change return `String` -> `Result<String, ToolReceiptError>`; ZERO callers of `issue_tool_receipt` anywhere |
| `safety.rs:21` `fulfill_tool_receipt`, `:27` `verify_tool_receipt` | siblings | unchanged (zero callers; they do not take a tool name to validate) |
| `crates/vox-orchestrator/src/orch_daemon/mod.rs:1020` (`SAFETY_LEDGER`) | uses `tool_ledger_handle()` + `snapshot()` only | unaffected |
| `crates/vox-orchestrator/src/orchestrator/accessors.rs:521`, `core/mod.rs:198` (`from_config`), `orchestrator.rs:128` | construct/expose ledger | unaffected |
| `crates/vox-orchestrator/src/lib.rs:377` `pub use tool_receipt::{ReceiptValidationResult, ToolReceipt, ToolReceiptLedger};` | public re-export | add `ToolReceiptError` |
| `docs/superpowers/plans/2026-06-18-orchestrator-tool-call-record.md` | design doc for a FUTURE `ToolCallLedger` with a different 7-arg `issue_intent` | not a caller; ignore (do not edit; overlaps Phase 5 TRUST-01) |

Recommendation (root-cause, one guard): put the `TOOL_REGISTRY` check inside `ToolReceiptLedger::issue_intent` so both `issue_intent` and `issue` and `Orchestrator::issue_tool_receipt` are all covered by the same fail-closed guard, instead of guarding only the Orchestrator wrapper (a wrapper-only guard leaves `ToolReceiptLedger::issue*` open, and `ToolReceiptLedger` is `pub` + re-exported).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/vox-orchestrator/src/tool_receipt.rs` (error enum + guard) | service (ledger) / validation gate | request-response (sync, in-memory) | `crates/vox-orchestrator/src/a2a/dispatch/lease_gate.rs` (thiserror enum + gate returning `Result<(), Enum>`); `crates/vox-orchestrator/src/security.rs:115-130` (default-deny) | role-match |
| `crates/vox-orchestrator/src/tool_receipt.rs` (in-file tests) | test | request-response | `crates/vox-orchestrator/src/security.rs:362-395` (`#[cfg(test)] mod tests { use super::*; ... }`) | exact |
| `crates/vox-orchestrator/src/orchestrator/safety.rs` | controller-ish facade (Orchestrator method) | request-response | itself; sibling `acquire_resource_lock` returns `bool`, `OrchestratorError` (`orchestrator/types.rs`) for fallible ops | role-match |
| `crates/vox-orchestrator/Cargo.toml` | config | n/a | `crates/vox-orchestrator-mcp/Cargo.toml:70` (`vox-mcp-registry.workspace = true`) | exact |
| `contracts/ci/crate-edges.allow.v1.json` | config (ledger) | n/a | existing `exceptions[0..5]` (2026-08-02 group) | exact |
| `crates/vox-orchestrator/tests/tool_receipt_registry_gate.rs` (NEW) | test (integration) | request-response | `crates/vox-orchestrator/tests/agentos_mcp_policy_wiring.rs` (wiring test on `Orchestrator`) + `research_gate.rs` (`matches!` on a refusal) | exact |
| `docs/src/architecture/crate-classification-2026-05-08.md` | docs | n/a | row 78 (`vox-search | CORE | many | ...`) in the same table | exact |

## Pattern Assignments

### `crates/vox-orchestrator/src/tool_receipt.rs` (service, request-response)

**Current signature (lines 89-90):**
```rust
    /// Issue a new receipt for a tool execution intent.
    pub fn issue_intent(&self, agent_id: AgentId, tool_name: &str, args_json: &str) -> ToolReceipt {
```
It returns a bare `ToolReceipt` (infallible, ZERO validation of `tool_name`). The file has NO existing `#[cfg(test)]` block and imports only (lines 1-6):
```rust
use crate::types::AgentId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
```
Neighbouring ledger methods in this same file use `Result<_, &'static str>` (`fulfill_intent` line 124, `verify` line 158; `validate_agent_claims` even string-matches on `Err("Receipt not found in ledger")`). Do NOT add another `&'static str`: `validate_agent_claims`'s string-match makes that convention brittle, and the crate's dominant convention for rejection is a `thiserror` enum (17 of them; see below).

**Error convention to copy** - `crates/vox-orchestrator/src/a2a/dispatch/lease_gate.rs:3-16` (gate that rejects; struct-variant carries the offending value):
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LeaseGateError {
    #[error("scope `{scope_key}` is held by remote node `{holder_node_id}` until {expires_at}ms")]
    HeldByRemote {
        scope_key: String,
        holder_node_id: String,
        expires_at: i64,
    },
    #[error("vox-db error: {0}")]
    Db(String),
}
```
Also see `orchestrator/types.rs:3-71` (`OrchestratorError`, `#[derive(Debug, thiserror::Error)]`, `/// doc` on every variant, e.g. `ScopeDenied(String)` `#[error("Scope denied: {0}")]`). `thiserror` is already a dependency of vox-orchestrator (`Cargo.toml`, `thiserror = { workspace = true }`), no new dep. Prefer a NEW small enum in `tool_receipt.rs` over adding a variant to `OrchestratorError` (the ledger is usable standalone via `ToolReceiptLedger`, and `OrchestratorError` is matched elsewhere):
```rust
/// Rejection reasons when issuing a tool receipt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ToolReceiptError {
    /// `tool_name` is not in the canonical MCP `TOOL_REGISTRY` (fail closed, D-02).
    #[error("Unknown tool: {tool_name}")]
    UnknownTool { tool_name: String },
}
```
(`Unknown tool: {name}` deliberately mirrors the message at `crates/vox-orchestrator-mcp/src/dispatch.rs:1852`, `Err(anyhow::anyhow!("Unknown tool: {}", name))`.) `handoff.rs:21` (`#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]`) is the precedent for the derive set with `PartialEq` for test `assert_eq!`.

**Core guard pattern** - registry lookup copied from `crates/vox-mcp-registry/src/semcov_wave46_tests.rs:15-20`:
```rust
let found = TOOL_REGISTRY.iter().find(|e| e.name == target);
```
Apply at the top of `issue_intent` (before the UUID / hashing, so nothing is inserted into `self.receipts` on rejection):
```rust
pub fn issue_intent(&self, agent_id: AgentId, tool_name: &str, args_json: &str)
    -> Result<ToolReceipt, ToolReceiptError>
{
    if !vox_mcp_registry::TOOL_REGISTRY.iter().any(|e| e.name == tool_name) {
        return Err(ToolReceiptError::UnknownTool { tool_name: tool_name.to_string() });
    }
    // ... existing body unchanged (lines 91-116) ...
    Ok(receipt)   // was: receipt
}
```
`McpToolRegistryEntry { name: &'static str, .. }` is `pub` (`crates/vox-mcp-registry/src/lib.rs:6-12`), `TOOL_REGISTRY` is generated at `include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"))`. Do NOT hand-roll a name list (RESEARCH "Don't Hand-Roll").

**`issue()` propagation** (lines 144-155): existing body ends `.fulfill_intent(&receipt.receipt_id, result_json).unwrap()`. Change to return `Result<ToolReceipt, ToolReceiptError>` and use `let receipt = self.issue_intent(agent_id, tool_name, args_json)?;`; keep the `.unwrap()` on `fulfill_intent` (cannot fail: the receipt was just inserted) wrapped in `Ok(...)`.

**Test-First policy (AGENTS.md):** the new `pub enum`/changed `pub fn` in a file over 30 non-blank lines needs a `#[cfg(test)]` block in the SAME file. Copy the block shape from `crates/vox-orchestrator/src/security.rs:362-372`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_default_deny() {
        let p = SecurityPolicy::new("agent-1");
        let result = p.check(&SecurityAction::ExecShell);
        assert!(result.is_err());
    }
```
Tests to write in-file (sync, no tokio; ledger needs no config): `ToolReceiptLedger::new([7u8; 32])`; unknown name -> `Err(ToolReceiptError::UnknownTool{..})` AND `ledger.is_empty()` (guard runs before insertion); known name (`"vox_submit_task"`, asserted present in `semcov_wave46_tests.rs:18-20`; `"vox_git_status"` is also in the registry, `contracts/mcp/tool-registry.canonical.yaml:686`) -> `Ok`, `ledger.verify(&r.receipt_id).is_ok()`; `issue()` with unknown name -> `Err`. `AgentId(1)` is a tuple struct (`pub struct AgentId(pub u64)`, `vox-orchestrator-types/src/agent_types/ids.rs:32`).

Note `vox_run_shell` (used in `agentos_mcp_policy_wiring.rs`) was NOT found in the registry YAML by name grep; do not use it as a "known good" fixture.

---

### `crates/vox-orchestrator/src/orchestrator/safety.rs` (Orchestrator facade, request-response)

**Analog:** itself, lines 1-18 (current):
```rust
use crate::budget::DriftDecision;
use crate::orchestrator::Orchestrator;
use crate::types::{AgentId, TaskId};

impl Orchestrator {
    /// Issues a cryptographic tool receipt for an agent to perform a specific tool call.
    /// This prevents agents from hallucinating tool outputs that were never executed.
    pub fn issue_tool_receipt(
        &self,
        agent_id: AgentId,
        tool_name: &str,
        args_json: &str,
    ) -> String {
        let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
        ledger
            .issue_intent(agent_id, tool_name, args_json)
            .receipt_id
    }
```
Target shape (one guard lives in the ledger; the facade just propagates):
```rust
    ) -> Result<String, crate::tool_receipt::ToolReceiptError> {
        let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
        ledger
            .issue_intent(agent_id, tool_name, args_json)
            .map(|r| r.receipt_id)
    }
```
Lock convention: `crate::sync_lock::rw_read(&*self.tool_ledger)` (`sync_lock` is `pub use vox_orchestrator_queue::sync_lock;` at `lib.rs:46`); keep it. Update the doc comment to add `/// Returns `Err(ToolReceiptError::UnknownTool)` for names absent from `vox_mcp_registry::TOOL_REGISTRY`.` Callers: none in repo, so no ripple. Sibling `fulfill_tool_receipt`/`verify_tool_receipt` return `bool` (`.is_ok()`): leave untouched.

Do NOT wire this into `vox-orchestrator-mcp` dispatch or `AgentosPolicyLedger::record_mcp_tool` (`agentos/policy_runtime.rs:46`, post-execution, cannot reject; RESEARCH Pitfall 2) - Phase 5 (TRUST-01) owns that.

---

### `crates/vox-orchestrator/Cargo.toml` (config)

**Analog:** `crates/vox-orchestrator-mcp/Cargo.toml:70` uses `vox-mcp-registry.workspace = true`; workspace root defines it at `Cargo.toml:111` (`vox-mcp-registry = { path = "crates/vox-mcp-registry" }`, so `workspace = true` resolves). In `crates/vox-orchestrator/Cargo.toml` `[dependencies]` (lines ~93-102 are the vox-* block, e.g.):
```toml
vox-orchestrator-types.workspace = true
vox-orchestrator-queue.workspace = true
vox-config.workspace = true
vox-repository.workspace = true
```
Add one line in that block (non-optional, no feature gate, so it also builds under `--no-default-features`):
```toml
# Canonical MCP tool-name allow-list (generated from contracts/mcp/tool-registry.canonical.yaml).
# Used by tool_receipt.rs to fail closed on unknown tool names. Edge is ledgered in
# contracts/ci/crate-edges.allow.v1.json `exceptions` (user-authorized).
vox-mcp-registry.workspace = true
```
Layers (`contracts/ci/crate-layers.v1.json`): `vox-mcp-registry` = 0 (line 55), `vox-orchestrator` = 3 (line 63) -> downward edge, so only the NewEdge rule fires (not UpwardEdge). `vox-mcp-registry`'s only dependency is `workspace-hack` (exempt). Cargo.lock will change (already dirty in the tree from unrelated work; do not regenerate broadly, only the one edge).

---

### `contracts/ci/crate-edges.allow.v1.json` (config / human-gated ledger)

**Analog:** the five `exceptions` entries dated 2026-08-02 (commit `344b5847d`, "register vox-oauth-pkce in crate-edges/layers ledgers"). Exact shape (2-space JSON indentation, keys in this order, all five required by `ExceptionEntry` in `crates/vox-cli-ci/src/crate_edges.rs:44-50`, no other fields):
```json
    {
      "from": "vox-cli",
      "to": "vox-oauth-pkce",
      "reason": "vox secrets login --oauth --provider openrouter (RFC 8252 loopback PKCE flow), free-tier onboarding plan",
      "date": "2026-08-02",
      "authorized_by": "user (explicit chat approval, 2026-08-02)"
    },
```
Conventions observed across all 34 existing entries:
- `date`: ISO `YYYY-MM-DD` string.
- `authorized_by`: either `"user (explicit chat approval, YYYY-MM-DD)"` (the user-authorized form; use this) or `"brbrainerd (bootstrap grandfather, plan-approved)"` (only the 2026-07-03 bootstrap; do not reuse).
- `reason`: one line, states the concrete need + the plan name (e.g. "..., free-tier onboarding plan"). Suggested: `"vox-orchestrator ToolReceiptLedger fails closed on tool names absent from vox_mcp_registry::TOOL_REGISTRY (REQ-dead-crate-wire-up SC#2, dead-crate wire-up phase 2)"`.
- Placement: newer user-authorized entries sit at the TOP of the array (the five 2026-08-02 entries precede the 29 bootstrap 2026-07-03 entries; each group is sorted by `(from, to)`). Insert the new entry as the first element of `exceptions` (or, if the executor prefers, at the end of the newest group; the tool does not care about order - `check()` builds a `BTreeSet` of pairs).
- Exceptions are DISJOINT from `edges` (verified: 0/34 exception pairs appear in `edges`); do NOT also add `["vox-orchestrator","vox-mcp-registry"]` to the `edges` array (that array is "Frozen baseline ... Machine-tightened only", `crate_edges.rs:37-38`). `vox-orchestrator` currently has 21 `edges` entries and 8 exceptions (all grandfathered `2026-07-03`), and no entry for `vox-mcp-registry`.
- `--tighten` (`crate_edges.rs:217-227`) is removal-only, so it cannot be used to add this.

**Authorization:** AGENTS.md "Dependency Discipline" rule 2 makes `exceptions` entries USER-AUTHORIZED-ONLY. The spawning prompt states the user authorized exactly this one edge (`vox-orchestrator -> vox-mcp-registry`) in this session; the planner should record that in `authorized_by` as `"user (explicit chat approval, 2026-09-25)"` (today's date per session context) and add NO other edges/exceptions. Never regenerate baselines. Verify with `cargo run -q -p vox-cli -- ci crate-edges` (should be green after Cargo.toml + entry both land).

---

### `crates/vox-orchestrator/tests/tool_receipt_registry_gate.rs` (NEW; test, request-response, sync)

**Naming/structure conventions** (50 files in `crates/vox-orchestrator/tests/`, flat, `snake_case.rs`, each auto-discovered; only `skill_runtime_inproc` and `mesh_ai_fixture_relay` have `[[test]]` entries because of features - do NOT add a `[[test]]` entry). Name choice mirrors `agentos_mcp_policy_wiring.rs` / `research_gate.rs` (`<subject>_<what>`).

**Primary analog:** `crates/vox-orchestrator/tests/agentos_mcp_policy_wiring.rs` (full file, 21 lines):
```rust
//! Integration-style wiring: MCP dispatch records tools via [`Orchestrator::record_agentos_mcp_tool`];
//! policy evaluation reads the latest `mutation_kind` through [`Orchestrator::evaluate_orchestrator_policy_for_agent`].

use vox_orchestrator::{Orchestrator, OrchestratorConfig};

#[test]
fn agentos_mcp_policy_wiring_risk_shift_for_shell_vs_read() {
    let orch = Orchestrator::new(OrchestratorConfig::default());
    orch.record_agentos_mcp_tool(Some(42), "vox_git_status");
    ...
    assert!(
        r_ext > r_read,
        "external/shell mutation should raise risk vs read-only git status (read={r_read} ext={r_ext})"
    );
}
```
Conventions to copy: crate-level `//!` doc explaining what is wired; `use vox_orchestrator::{Orchestrator, OrchestratorConfig};` (both re-exported at crate root); plain sync `#[test]` (no `#[tokio::test]` needed - `Orchestrator::new` is sync here, and `issue_tool_receipt` is sync); `Orchestrator::new(OrchestratorConfig::default())` (this test) or `OrchestratorConfig::for_testing()` (small limits / fast timeouts, `config/impl_env.rs:947`, `pub`, used by e.g. `a2a_integration_test.rs`, `task_lifecycle_durable_oplog.rs`). Prefer `for_testing()` for a new test. `AgentId` import path: `use vox_orchestrator::types::AgentId;` (as in `chat_round_trip.rs:21`) or root re-export `vox_orchestrator::AgentId` (`lib.rs:382`).

**Refusal-assert idiom** - `crates/vox-orchestrator/tests/research_gate.rs:6-20`:
```rust
use vox_orchestrator::research_gate::{GateResult, PreregGate, check_campaign_prereg};

/// A campaign with no prereg and no signature must be refused.
#[test]
fn refused_when_no_prereg_provided() {
    let result = check_campaign_prereg(None, None);
    assert!(
        matches!(result, GateResult::Refused { .. }),
        "check_campaign_prereg(None, None) must return Refused"
    );
```
Skeleton for the new file (fail-closed end to end through the `Orchestrator` facade, per D-04):
```rust
use vox_orchestrator::{AgentId, Orchestrator, OrchestratorConfig, ToolReceiptError};

#[test]
fn issue_tool_receipt_rejects_unknown_tool_fail_closed() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let err = orch.issue_tool_receipt(AgentId(1), "definitely_not_a_registered_tool", "{}").unwrap_err();
    assert_eq!(err, ToolReceiptError::UnknownTool { tool_name: "definitely_not_a_registered_tool".into() });
    // no receipt leaked into the ledger
    assert!(orch.tool_ledger_handle().read().unwrap().is_empty());
}

#[test]
fn issue_tool_receipt_accepts_every_registry_entry() { // loop vox_mcp_registry::TOOL_REGISTRY
    ...
    assert!(orch.verify_tool_receipt(&id));
}
```
Ledger read pattern for asserting no leak: `orch.tool_ledger_handle()` returns `Arc<std::sync::RwLock<ToolReceiptLedger>>` (`accessors.rs:521-525`), consumed as `.read().unwrap()` in `orch_daemon/mod.rs:1020-1022`. Iterating `vox_mcp_registry::TOOL_REGISTRY` from the test needs `vox-mcp-registry` visible to the test target: integration tests can use the crate's normal `[dependencies]`, so after the Cargo.toml edit `vox_mcp_registry::TOOL_REGISTRY` resolves with no dev-dependency entry. Also test the empty string and a near-miss (`"VOX_SUBMIT_TASK"` / trailing space) to pin exact-match semantics.

Test-tier notes: quick loop `cargo test -p vox-orchestrator --test tool_receipt_registry_gate` and `cargo test -p vox-orchestrator --lib tool_receipt`. `vox-orchestrator` default features are heavy (jj, runtime, corpus-db); first build is slow.

---

### `docs/src/architecture/crate-classification-2026-05-08.md` (docs)

**Analog:** the already-correct `vox-search` row (line 78) in the same table:
```markdown
| vox-search | CORE | many | agent retrieval execution (`execute_search_plan`, MCP); see [search-retrieval-ssot-2026.md](search-retrieval-ssot-2026.md) |
```
`vox-search` needs NO edit (already CORE). Two edits for `vox-doc-inventory`:

1. Line 86 (in the main table, currently `| vox-doc-inventory | DEAD | 0 | doc inventory; no consumers |`) -> flip to the same 4-column shape:
   `| vox-doc-inventory | CORE | 1 (vox-cli) | doc inventory; direct dep of vox-cli (`crates/vox-cli/Cargo.toml:204`, `vox-doc-inventory = { workspace = true }`) |`
   (Column style precedent: line 65 `vox-ssg | CORE (direct dep) | 1 (vox-cli)`, line 66-70 `| CORE | 1 | ...`.)
2. Line 140 (in the "DEAD crates - Deletion Candidates" reason table, `| `vox-doc-inventory` | No consumers |`) -> delete that row (it is a false claim and the table's header text says "zero consumers in Cargo.toml files"). Leave the neighbouring rows byte-identical.

Frontmatter / tombstone: file carries `status: "deprecated"` and `training_eligible: false` (lines 1-11); do not touch frontmatter or the TOMBSTONE banner at line 15 (do not add `last_updated`, the pipeline derives it). No new doc file, no `where-things-live.md` edit is required (rows 99/168 there already describe both crates accurately; optional). Doc lint if run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/crate-classification-2026-05-08.md`. Note: file has no fenced `vox` blocks in the edited region.

Optional (planner discretion, not in REQ): also flip row 85 `vox-mcp-registry | DEAD | 0 | MCP registry stub; orphaned` and delete row 139, consistent with banner line 15.

---

## Shared Patterns

### Fail-closed on unknown identifiers (trust boundary)
**Source:** `crates/vox-orchestrator-mcp/src/dispatch.rs:1852` (`Err(anyhow::anyhow!("Unknown tool: {}", name))`); `crates/vox-orchestrator/src/security.rs:125-129` (`"no matching rule (default deny)"`).
**Apply to:** `tool_receipt.rs` guard. Reject with `Err`, never log-and-continue (D-02, costly to reverse).

### Registry-as-single-source
**Source:** `crates/vox-mcp-registry/src/lib.rs` (`TOOL_REGISTRY`, build.rs-generated from `contracts/mcp/tool-registry.canonical.yaml`), 24 tests in `semcov_wave46_tests.rs`.
**Apply to:** `tool_receipt.rs` and the new integration test (iterate `TOOL_REGISTRY`; no hard-coded second list).

### Error enums via thiserror, doc-commented variants
**Source:** `orchestrator/types.rs:3-71`, `a2a/dispatch/lease_gate.rs:6-16`.
**Apply to:** the new `ToolReceiptError`; re-export from `lib.rs:377` alongside `ToolReceipt`, `ToolReceiptLedger`.

### Lock helper
**Source:** `crate::sync_lock::rw_read(&*self.tool_ledger)` (`safety.rs:14`). Keep for the facade; the ledger's own `receipts` uses `parking_lot::RwLock` internally (no change).

### Test-First policy
**Source:** AGENTS.md "Test-First Policy". Every changed/new `pub fn` in `crates/*/src/**` (files > 30 non-blank lines) needs an in-file `#[test]`. `tool_receipt.rs` has none today (Wave 0 gap in RESEARCH); the new in-file `mod tests` closes it. Detector `skeleton/untested-pub-api`.

### Crate-edge discipline
**Source:** AGENTS.md "Dependency Discipline" + `crates/vox-cli-ci/src/crate_edges.rs`. Land the `Cargo.toml` edge and the `exceptions` entry in the SAME commit so `vox ci crate-edges` (in the fast pre-push tier) stays green. Do not write any second exception. Run `vox ci pre-push --complete` (clippy) for Rust changes; `cargo` goes through the build broker (plain `cargo` on PATH).

### Broader constraints from AGENTS.md
- Crypto: the ledger already uses `blake3` keyed hashing (approved primitive); do not touch it.
- No LLM vendor hostnames, no `.sh/.py` glue, no `cargo fmt --all` (use `vox run scripts/fmt.vox`).

## No Analog Found

None. (All seven files have same-role analogs. The only judgment call: `ToolReceiptError` is a new type; its shape follows `LeaseGateError` / `OrchestratorError`.)

## Out of scope (do not plan)

- SC#1 exec-policy: verification-only (`cargo run -p vox-cli -- ci exec-policy-contract`, source `crates/vox-cli/src/commands/runtime/shell/check_terminal.rs:418-448`). Any D-04 integration coverage for it should extend `SMOKE_PAYLOADS`/`REJECT_PAYLOADS` in `crates/vox-cli/src/commands/ci/exec_policy_contract.rs`, not new call-site code.
- Wiring `issue_tool_receipt` into live MCP dispatch (`vox-orchestrator-mcp`) - Phase 5 (TRUST-01).
- `crates/vox-corpus/src/mcp_meta.rs` orphan file - discretionary cleanup only.
- `docs/superpowers/plans/2026-06-18-orchestrator-tool-call-record.md` (future `ToolCallLedger` redesign) - not a caller of the current API.

## Metadata

**Analog search scope:** `crates/vox-orchestrator/{src,tests}`, `crates/vox-orchestrator-mcp/src`, `crates/vox-mcp-registry`, `crates/vox-cli-ci/src/crate_edges.rs`, `contracts/ci/`, `docs/src/architecture/`.
**Files scanned:** ~25 read/grepped (repo-wide `rg` for `issue_intent|issue_tool_receipt|ToolReceiptLedger|tool_ledger`).
**Pattern extraction date:** 2026-09-25
