# Phase 2: Wire Up & Reclassify Dormant Crates - Research

**Researched:** 2026-09-23
**Domain:** In-repo Rust workspace wiring + architecture-doc correction (no new external dependencies)
**Confidence:** HIGH — every finding below was verified by reading the actual source/contract files this session (not training-data recall). Package-legitimacy tooling was not invoked because this phase introduces zero external packages.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01 — Exec-grammar wiring, verify before assuming:** `crates/vox-exec-grammar` does not exist as a standalone crate. Exec-policy enforcement already lives in `vox-container-types::exec_grammar::policy` (`ExecPolicyV1`), consumed by `vox-cli/src/commands/runtime/shell/check_terminal.rs::run_check`. This may mean REQ-dead-crate-wire-up's exec-grammar item is already satisfied, matching Phase 1's pattern. Do not presuppose either way — research must verify whether `check_terminal.rs`'s existing implementation actually satisfies the ADR-026 contract in `contracts/terminal/exec-policy.v1.yaml`, or whether something genuinely remains unwired. Reversibility: N/A (a verification decision, not an implementation choice).

**D-02 — TOOL_REGISTRY validation behavior:** When vox-orchestrator validates/enumerates MCP tool names via `TOOL_REGISTRY` (confirmed: zero references from vox-orchestrator to vox-mcp-registry — genuinely dormant), unrecognized tool names must be rejected (fail closed), not merely logged. Matches the security-conscious pattern already established elsewhere in this codebase (capability gating, exec-policy). Reversibility: costly — loosening a fail-closed check to fail-open later is a behavior change on a trust boundary, not a pure revert.

**D-03 — Reclassification target, deferred to research:** `vox-search` and `vox-doc-inventory` are not mentioned anywhere in `docs/src/architecture/classification-ssot-2026.md` (confirmed — zero matches). Their current DEAD classification lives in some other file not yet identified. Research must locate the actual source before any reclassification edit is planned — do not guess the target file.

**D-04 — Test coverage depth:** Beyond the bare AGENTS.md Test-First Policy minimum (one adjacent test per new `pub fn`), add integration-style tests exercising the full wired path: the exec-policy gate end-to-end (if new wiring is needed per D-01's outcome), and TOOL_REGISTRY's fail-closed behavior under a bad/unknown tool name (per D-02). Reversibility: reversible — additional test coverage, no behavior risk.

### Claude's Discretion
- Exact reclassification file/mechanism once research identifies it (D-03).
- Exact shape of any new exec-policy wiring code, if D-01's research finds something genuinely missing.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope. EXTRACT-TO-PLUGIN and MISPLACED work (Phase 3) was not discussed here.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-dead-crate-wire-up | Wire up functionally-complete but never-adopted crates into their intended call path instead of deleting working code. Acceptance: WIRE-UP exec-grammar risk classification into the exec-policy gate per ADR-026/`exec-policy.v1.yaml`; WIRE-UP vox-mcp-registry into vox-orchestrator with fail-closed TOOL_REGISTRY validation, confirm vox-mcp-meta already deleted; RECLASSIFY vox-search and vox-doc-inventory from DEAD to CORE in the crate catalog. | See §SC#1 (exec-policy — already satisfied, verification only), §SC#2 (vox-orchestrator wiring — genuinely needed, exact integration point identified), §SC#3 (reclassification — target file identified, one of two crates already correct). |
</phase_requirements>

## Summary

Phase 2 covers three independently-verifiable success criteria. Deep source investigation this session found they are **not symmetric in effort**: SC#1 requires no new production code at all (it was already fully wired and CI-tested before this milestone began — Phase 1's "PRD-described crate landed elsewhere" pattern repeats exactly); SC#2 requires a small, well-scoped code change at a precisely identified integration point, gated by a crate-dependency-edge authorization step that the plan must not skip; SC#3 requires a one-file documentation correction, and even that file is only half wrong (vox-search's entry is already correct).

**Primary recommendation:** Plan Phase 2 as three narrow, independent tasks — (1) a verification-and-doc-cleanup task for exec-policy (no functional code changes), (2) a small `vox-orchestrator` change adding TOOL_REGISTRY-backed fail-closed validation to the currently-zero-caller `Orchestrator::issue_tool_receipt` path, gated behind an explicit human-authorized crate-edge exception, and (3) a single-file fix to `crate-classification-2026-05-08.md`'s `vox-doc-inventory` row. Do not let SC#2's wiring expand into rewiring the live MCP dispatch chain (`vox-orchestrator-mcp`) — that risks duplicating Phase 5's TRUST-01 scope (HMAC tool receipts).

## Architectural Responsibility Map

This project is a CLI/orchestrator/compiler tool, not a browser/frontend-server/CDN application — the standard client/server tier table does not map cleanly. Tiers below are adapted to this codebase's actual architecture (CLI process / orchestrator runtime / static docs).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Exec-policy risk classification | CLI process (`vox-cli` invoking `vox-container` → `vox-container-types::exec_grammar`) | — | Enforced synchronously inside `vox shell check` before any command reaches a shell; no network/DB involved. |
| MCP tool-name validation (fail-closed) | Orchestrator runtime (`vox-orchestrator`, L3) | MCP registry data (`vox-mcp-registry`, L0, compile-time generated from SSOT YAML) | The orchestrator is the trust boundary that issues tool-execution receipts to agents; the registry is a pure compile-time data crate it should consult, not duplicate. |
| Crate classification (CORE/DEAD/etc.) | Docs (no runtime tier) | — | Pure documentation accuracy fix; zero runtime code path. |

## Standard Stack

**N/A — this phase introduces zero new external dependencies.** All three success criteria are satisfied using crates and files that already exist in the workspace (`vox-container-types::exec_grammar`, `vox-mcp-registry`, `docs/src/architecture/crate-classification-2026-05-08.md`). The only new artifact is a first-party, in-workspace Cargo dependency edge (`vox-orchestrator` → `vox-mcp-registry`, both first-party crates, zero transitive risk — see Package Legitimacy Audit).

## Package Legitimacy Audit

**N/A — no external (crates.io) packages are introduced by this phase.** The one new dependency edge added by SC#2 (`vox-orchestrator` → `vox-mcp-registry`) is a first-party, in-workspace crate that already exists and is already a dependency of `vox-cli`, `vox-orchestrator-mcp`, `vox-corpus`, and others (verified: `contracts/ci/crate-edges.allow.v1.json` lists `vox-mcp-registry`'s only own dependency as `workspace-hack`). No `npm view`/`pip index`/`cargo search` legitimacy check applies to intra-workspace crates.

## Architecture Patterns

### System Architecture Diagram

```
SC#1 — exec-policy (already wired, verified this session):

  vox shell check <payload>
        │
        ▼
  vox-cli::commands::runtime::shell::check_terminal::run_check[_for_ci]
        │  loads + schema-validates contracts/terminal/exec-policy.v1.yaml
        ▼
  pwsh available? ──no──► run_check_rust_fallback(payload, policy)
        │ yes                    │
        ▼                        │  vox_container::exec_grammar::parse_pipeline()
  PowerShell AST path            │  vox_container::exec_grammar::risk::classify() ◄── THE CALL
  (full-fidelity, Windows)       │  vox_container::exec_grammar::ExecPolicy::evaluate()
        │                        │
        └───────────┬────────────┘
                     ▼
         Ok(()) / Err(PolicyViolation) — fail closed on unknown cmd, blocked param, disallowed domain
                     │
                     ▼
         vox ci exec-policy-contract  (CI gate — exercises BOTH paths against SMOKE_PAYLOADS + REJECT_PAYLOADS)


SC#2 — MCP tool-name validation (genuinely dormant, needs wiring):

  Agent claims a tool call
        │
        ▼
  Orchestrator::issue_tool_receipt(agent_id, tool_name, args_json)   ◄── ZERO CALLERS TODAY (confirmed)
        │  crates/vox-orchestrator/src/orchestrator/safety.rs:8
        ▼
  ToolReceiptLedger::issue_intent(...)                                ◄── accepts ANY tool_name string today
        │  crates/vox-orchestrator/src/tool_receipt.rs:90
        ▼
  [NEW] validate tool_name against vox_mcp_registry::TOOL_REGISTRY   ◄── does not exist yet — THE GAP
        │  fail closed: reject unrecognized names (per D-02)
        ▼
  HMAC receipt issued (BLAKE3-keyed) or Err returned

  Separately — the LIVE, already-wired MCP dispatch chain (different crate, not in this phase's scope):
  vox-orchestrator-mcp::dispatch::handle_tool_call
        │  already depends on vox-mcp-registry (registry.rs: `use super::TOOL_REGISTRY`)
        │  already fail-closed via exhaustive match + "Unknown tool: {name}" Err arm (dispatch.rs:1852)
        ▼
  Orchestrator::record_agentos_mcp_tool → AgentosPolicyLedger::record_mcp_tool
        (POST-dispatch risk/mutation classification — NOT a gate, cannot reject)
```

### Pattern 1: Fail-closed registry validation (established elsewhere in this codebase)
**What:** Reject unrecognized identifiers rather than silently accepting or merely logging them.
**When to use:** Any trust-boundary input (tool names, exec commands, policy identifiers).
**Example already in this codebase** (`vox-orchestrator-mcp/src/dispatch.rs:1852`, read this session):
```rust
// vox:skip — excerpt from an out-of-file dispatch match; inlining would need the full 1800+ line match
Err(anyhow::anyhow!("Unknown tool: {}", name))
```
This is the exact pattern D-02 asks the new `vox-orchestrator` validation to follow: an `Err` on the unrecognized branch, not a log line.

### Anti-Patterns to Avoid
- **Wiring `issue_tool_receipt` into the live dispatch chain as part of this phase:** `Orchestrator::issue_tool_receipt`/`fulfill_tool_receipt`/`verify_tool_receipt` (all in `orchestrator/safety.rs`) have **zero callers anywhere in the repository** (verified via repo-wide grep, not just within `vox-orchestrator`). Making them reachable from the live MCP dispatch path is a bigger, security-relevant change than this phase's REQ text asks for, and it substantially overlaps TRUST-01 (Phase 5: "Agent tool calls emit cryptographically verifiable HMAC receipts... two-tier formal-intent verification system defined in ADR-029"). Keep Phase 2's change scoped to adding the dependency + validation function; leave "wire the receipt ledger into the live dispatch loop" to Phase 5.
- **Hand-editing `contracts/ci/crate-edges.allow.v1.json`'s `edges` array:** That array's own doc comment states `"Frozen baseline, sorted [from, to] pairs. Machine-tightened only."` — `vox ci crate-edges --tighten` is **removal-only** (it `bail!`s if any live edge isn't already in the prior baseline ∪ exceptions — see `crate_edges.rs:217-227`). The new `vox-orchestrator → vox-mcp-registry` edge must go through the `exceptions` array instead, and that array is explicitly **USER-AUTHORIZED-ONLY** per both the file's embedded `HEAL` text and AGENTS.md §Dependency Discipline: *"Never write one yourself; never regenerate baselines to admit your own edge."*

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| "Is this a known MCP tool name?" | A new hardcoded list/const array of valid tool names inside `vox-orchestrator` | `vox_mcp_registry::TOOL_REGISTRY.iter().any(\|e\| e.name == tool_name)` | `TOOL_REGISTRY` is already build.rs-generated from the single canonical SSOT (`contracts/mcp/tool-registry.canonical.yaml`) and already has 24 dedicated correctness tests (`semcov_wave46_tests.rs`, read this session) covering uniqueness, non-empty fields, and naming convention. A second hand-rolled list is exactly the kind of drift already found live in this repo (see Common Pitfalls, `vox-corpus/src/mcp_meta.rs`). |
| "Classify this shell command's risk" | A second risk-scoring function inside `vox-orchestrator` or anywhere else | `vox_container_types::exec_grammar::risk::classify` (re-exported as `vox_container::exec_grammar::risk::classify`) | Already implements the ADR-026 clean-room-verified classifier (Safe/Elevated/Blocked), already policy-driven from `exec-policy.v1.yaml`, already CI-tested via `vox ci exec-policy-contract`. |

**Key insight:** Both dormant-crate items in this phase are not "missing features" — they are existing, tested, single-purpose components that a *different* piece of code needs to be pointed at. The temptation in a fresh implementation session is to write a fresh check inline; resist it and import the existing symbol.

## Common Pitfalls

### Pitfall 1: Assuming SC#1 needs new production code
**What goes wrong:** A plan spends a wave adding a `vox_exec_grammar::risk::classify` call to `vox-container` or `vox-cli-core`, duplicating logic that already runs today.
**Why it happens:** REQUIREMENTS.md's acceptance text was written against the original PRD (`dead-crate-fate-plan-2026-05-08.md`), which predates the actual implementation landing as a module inside `vox-container-types` rather than a standalone `vox-exec-grammar` crate. The PRD's crate name traces back to ADR-026 itself, which speaks of a "Future `vox-exec-grammar` (AST command validator)" (`docs/src/adr/026-third-party-code-provenance.md:95`, read this session) — that name never became a real crate; the functionality landed as `vox-container-types::exec_grammar` instead, already calling `risk::classify` from `check_terminal.rs::run_check_rust_fallback` (line 438) and `run_check_for_ci` (always exercises the rust-fallback path, line 348), with CI coverage via `vox ci exec-policy-contract` (`crates/vox-cli/src/commands/ci/exec_policy_contract.rs`, confirmed with reject-payload assertions at lines 87-94).
**How to avoid:** Task 1 of the plan should be a verification task (read the four files above, confirm the call, run `vox ci exec-policy-contract` locally), not an implementation task. If D-04's "integration-style test" ask needs satisfying, extend `SMOKE_PAYLOADS`/`REJECT_PAYLOADS` or the optional `contracts/terminal/exec-policy.test-corpus.yaml` disk corpus rather than writing new call-site code.
**Warning signs:** A task titled "wire risk::classify into the exec-policy gate" without first citing `check_terminal.rs:438` as already doing so.

### Pitfall 2: Picking the wrong integration point for SC#2
**What goes wrong:** Wiring `vox-mcp-registry` into `AgentosPolicyLedger::record_mcp_tool` (`crates/vox-orchestrator/src/agentos/policy_runtime.rs:46`) because it's the one live call path already reachable from `vox-orchestrator-mcp::dispatch.rs:390` in production.
**Why it happens:** It's the only "MCP tool name" touchpoint in `vox-orchestrator` that's actually called today, so it looks like the natural wiring target.
**How to avoid:** `record_mcp_tool` runs **after** a tool call completes (its own doc comment: "Record classification after an MCP tool completes") — it feeds a risk/mutation-kind overlay, not an admission gate. By the time it runs, the tool has already executed; there is nothing left to "fail closed" against. D-02 requires rejecting unrecognized names *before* the call proceeds. The correct integration point is `Orchestrator::issue_tool_receipt` → `ToolReceiptLedger::issue_intent`, which is called *before* a tool executes (it issues an "intent" receipt) and currently performs zero validation on `tool_name: &str`.
**Warning signs:** A plan task that touches `policy_runtime.rs`/`mutation_classifier.rs` for tool-name rejection.

### Pitfall 3: Missing the crate-edge authorization gate
**What goes wrong:** The plan adds `vox-mcp-registry = { workspace = true }` to `vox-orchestrator/Cargo.toml`, writes the validation code, and the very next `vox ci crate-edges` (part of the fast pre-push tier) fails with `NEW EDGE not in baseline: vox-orchestrator -> vox-mcp-registry`, blocking the phase.
**Why it happens:** `contracts/ci/crate-edges.allow.v1.json` is an exact-edge-set ratchet (verified this session: 549 edges, `["vox-orchestrator","vox-mcp-registry"]` absent). `--tighten` only removes stale edges, never adds new ones (`crate_edges.rs:217-227`, `bail!("--tighten would ADD ... (tighten is removal-only)")`).
**How to avoid:** The plan must include an explicit step that proposes an `exceptions` entry (matching the existing entries' shape — `from`, `to`, `reason`, `date`, `authorized_by`) and surfaces it to the user for authorization rather than writing it autonomously — per the tool's own embedded `HEAL` text: *"PROPOSE an `exceptions` entry in your PR description and STOP... Never write one yourself."* Concretely: a `checkpoint:human-verify`-style task before the code that introduces the dependency is merged, or at minimum before `vox ci pre-push` is expected to pass.
**Warning signs:** No task in the plan that touches `contracts/ci/crate-edges.allow.v1.json`, or a task that edits it without a preceding confirmation step.

### Pitfall 4: Editing `crate-classification-2026-05-08.md` and expecting it to "count" as a live SSOT
**What goes wrong:** Treating the fix as done once the `vox-doc-inventory` row is corrected, without noting that the file itself carries `status: "deprecated"` and a `training_eligible: false` banner reading *"TOMBSTONE — do not use for contraction or architecture decisions... For current crate health use `where-things-live.md`, `layers.toml`, and `cargo run -p vox-arch-check`."*
**Why it happens:** It is nonetheless the exact file the source PRD names as the edit target (`dead-crate-fate-plan-2026-05-08.md:120,325`: "Update `crate-classification-2026-05-08.md`"), and it is the **only** file anywhere in the repo using the CORE/DEAD/PLUGIN/SHARED/MISPLACED vocabulary for these two crates — `where-things-live.md` has no classification column (it's a concept→crate lookup table) and `classification-ssot-2026.md` documents unrelated domain-prefix naming (confirmed: neither mentions `vox-search` or `vox-doc-inventory`, matching CONTEXT.md D-03's own finding).
**How to avoid:** Fix the tombstoned file's factual error (it is not itself an "architecture decision," it's a false "0 consumers" claim contradicted by `vox-cli/Cargo.toml:204`) as the primary, literal satisfaction of the locked acceptance criterion. Optionally also confirm/enrich the existing correct rows in `where-things-live.md` (lines 99, 168, already describe both crates accurately without needing a classification-label flip) so a future reader who follows the tombstone's own redirect still finds accurate information. Do not spend effort trying to invent a brand-new "current" classification doc — that's out of this phase's boundary (a real un-deprecation of the taxonomy is an architecture decision, not a doc fix).
**Warning signs:** A task that creates a new classification file, or a task that skips the edit entirely because "the file says not to use it."

### Pitfall 5: Conflating `crates/vox-corpus/src/mcp_meta.rs` with the REQ's vox-mcp-meta item
**What goes wrong:** Treating `crates/vox-corpus/src/mcp_meta.rs` (which duplicates `A2A_MESSAGE_TYPES`, `SKILL_TOOLS`, `ORCHESTRATOR_TOOLS` with a *smaller, drifted* subset vs. `vox-mcp-registry`'s canonical versions) as unfinished vox-mcp-meta migration work this phase must finish.
**Why it happens:** Phase 1's research flagged this file as "a live duplicate in vox-corpus to reconcile" and pointed at Phase 2.
**What we found this session:** The file is **not actually compiled** — `crates/vox-corpus/src/lib.rs` declares 14 `mod`/`pub mod` items and `mcp_meta` is not among them (verified by reading the full module list). It is an orphaned source file, dead at the Rust-module level (not reachable via `cargo tree` or any `use` path), separate from — and less urgent than — the REQ's actual `vox-orchestrator` wiring gap.
**How to avoid:** This phase's REQ text scopes the vox-mcp-meta item entirely to confirming its deletion (done, commit `72bde3718`, cross-checked against Phase 1's `01-RESEARCH.md`) and to `vox-orchestrator`'s wiring — not to `vox-corpus`. Treat the orphaned file as an optional discretionary cleanup (delete the dead file, or leave it) rather than a required task; flag it for the planner's discretion rather than mandating removal, since REQ-dead-crate-wire-up's acceptance criteria name only `vox-orchestrator` and vox-mcp-meta itself.

## Code Examples

### The exec-policy call already in place (verified, no new code needed)
```rust
// Source: crates/vox-cli/src/commands/runtime/shell/check_terminal.rs:418-448 (read in full this session)
fn run_check_rust_fallback(payload: &str, policy: &ExecPolicyV1) -> Result<()> {
    use vox_container::exec_grammar::{ExecPolicy, risk};

    let asts = vox_container::exec_grammar::parse_pipeline(payload)
        .map_err(|e| anyhow!("parse error (rust fallback): {e}"))?;

    let grammar_policy = ExecPolicy {
        allowed_cmdlets: policy.allowed_cmdlets.clone(),
        allowed_binaries: policy.allowed_binaries.clone(),
        blocked_parameters: policy.blocked_parameters.clone(),
        network_fetch_commands: policy.network_fetch_commands.clone(),
        network_fetch_domains: policy.network_fetch_domains.clone(),
    };

    for mut ast in asts {
        risk::classify(&mut ast, &grammar_policy);   // <-- THE call SC#1 asks for, already present
        let violations = grammar_policy.evaluate(&ast);
        if !violations.is_empty() {
            return Err(anyhow!("exec-policy violation (rust fallback): {}", /* ... */ ""));
        }
        // ... network-domain enforcement ...
    }
    Ok(())
}
```

### Recommended shape for SC#2's new validation (illustrative — not yet in the tree)
```rust
// vox:skip — illustrative sketch for the planner; exact error type/signature is Claude's Discretion.
// Target file: crates/vox-orchestrator/src/orchestrator/safety.rs (or tool_receipt.rs directly)
pub fn issue_tool_receipt(
    &self,
    agent_id: AgentId,
    tool_name: &str,
    args_json: &str,
) -> Result<String, UnknownToolError> {
    if !vox_mcp_registry::TOOL_REGISTRY.iter().any(|e| e.name == tool_name) {
        return Err(UnknownToolError { tool_name: tool_name.to_string() });
    }
    let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
    Ok(ledger.issue_intent(agent_id, tool_name, args_json).receipt_id)
}
```
Signature change is safe: `issue_tool_receipt` has zero callers anywhere in the repo today (verified via repo-wide grep for `issue_tool_receipt`), so changing its return type from `String` to `Result<String, _>` cannot break any existing call site.

### The already-existing fail-closed pattern to mirror (different crate, for reference only)
```rust
// vox:skip — excerpt from an 1800+ line match in a different crate; not to be modified this phase.
// Source: crates/vox-orchestrator-mcp/src/dispatch.rs:1852 (grep-confirmed, not modified this phase)
Err(anyhow::anyhow!("Unknown tool: {}", name))
```

## State of the Art

| Old Approach (PRD-era, 2026-05-08) | Current Approach (verified 2026-09-23) | When Changed | Impact |
|--------------------------------------|------------------------------------------|---------------|--------|
| `vox-exec-grammar` as a standalone crate (per ADR-026's forward-looking language) | `vox-container-types::exec_grammar` module, re-exported via `vox-container` | Landed before this milestone (exact commit not dated this session; predates Phase 1's cleanup) | The PRD's SC#1 acceptance text is stale on crate name/location but the underlying requirement (risk classification wired into the exec-policy gate) is already met. |
| `vox-mcp-meta` crate owning `A2A_MESSAGE_TYPES` | Deleted; constants now live in `vox-mcp-registry::A2A_MESSAGE_TYPES` | `72bde3718`, 2026-05-08 (per Phase 1's `01-RESEARCH.md`, cross-checked) | REQ's "delete the now-redundant vox-mcp-meta wrapper" sub-item is already satisfied — no action needed in Phase 2. |
| `crate-classification-2026-05-08.md` as the crate-health SSOT | Superseded by `where-things-live.md` + `layers.toml` + `vox-arch-check`; classification doc tombstoned 2026-06-10 | 2026-06-10 (per the doc's own frontmatter `training_rationale`) | The reclassification fix still targets the tombstoned file (it's the only place the labels exist), but future crate-health questions should route through the live SSOTs, not this one. |

**Deprecated/outdated:**
- `crate-classification-2026-05-08.md`: superseded per its own frontmatter (`superseded_by: 2026-05-08-workspace-reorg-outcome.md, 2026-05-08-crate-org-followup-design.md`). Still the correct edit target for this narrow factual fix per the PRD's explicit instruction; not the correct place to look for any *other* crate-health question.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The recommended integration point for SC#2 (`Orchestrator::issue_tool_receipt`) is the "intended call path" the REQ means, rather than some other not-yet-discovered site. | Architecture Patterns / Pitfall 2 | If wrong, the fail-closed check is added somewhere agents never actually call, and SC#2's spirit ("active in its intended call path") is only partially met even though the literal acceptance text (dependency + TOOL_REGISTRY-backed validation) is satisfied. This is flagged as the primary open question below — verified as the best fit found after exhaustively searching `vox-orchestrator` for every `tool_name`-touching function, but no code in the repo currently calls `issue_tool_receipt`, so its status as "the" intended call path is inferred from doc comments and design fit, not from an existing caller. |
| A2 | The `crate-classification-2026-05-08.md` fix, despite the file's deprecated status, is the correct way to satisfy SC#3's literal acceptance text ("the crate catalog lists vox-search and vox-doc-inventory as CORE"). | Common Pitfalls / Pitfall 4 | If the planner/user instead wants a "real," non-deprecated classification record created, that is materially more work (a new or resurrected SSOT doc) and should be raised as a discussion point before planning, not discovered mid-execution. |

## Open Questions

1. **Should `Orchestrator::issue_tool_receipt` also be wired into the live MCP dispatch chain (`vox-orchestrator-mcp::dispatch.rs`), or is adding TOOL_REGISTRY validation to the currently-uncalled function sufficient for this phase?**
   - What we know: The literal REQ/SC#2 acceptance text only asks for "vox-orchestrator depends on vox-mcp-registry and validates/enumerates MCP tool names via TOOL_REGISTRY" — it does not explicitly require the validated path to be reachable from production traffic today. `issue_tool_receipt` has zero callers anywhere in the repo.
   - What's unclear: Whether "functionally-complete crates... are active in their intended call path" (the phase's stated Goal, not just the SC text) implicitly requires making `issue_tool_receipt` reachable from somewhere real.
   - Recommendation: Satisfy the literal SC#2 text only (add the dependency + fail-closed validation inside the existing function) and explicitly flag the "is it reachable from production" question as a discretion boundary against TRUST-01/Phase 5's overlapping scope (HMAC receipts, ADR-029's two-tier verification). Wiring it into live dispatch is a bigger, security-relevant change that a later phase already owns.

2. **Does fixing `vox-doc-inventory`'s row in the tombstoned classification doc need a companion note pointing to it from `where-things-live.md`, or is the single-file fix sufficient?**
   - What we know: `where-things-live.md` already has accurate, correctly-worded rows for both crates (lines 99, 168) — it just has no CORE/DEAD label column to begin with.
   - What's unclear: Whether the user considers "the crate catalog" (SC#3's wording) to mean specifically the classification-labeled doc, or the broader concept of "wherever a reader would look."
   - Recommendation: Fix the one file that actually carries the wrong label (minimum viable fix, matches the PRD's literal instruction); treat any broader documentation enrichment as optional/discretionary.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `pwsh`/`powershell` | `check_terminal.rs::run_check`'s high-fidelity PowerShell AST path | ✗ (checked this session: `pwsh not found`) | — | Rust fallback path (`run_check_rust_fallback`) — the one that calls `risk::classify` — is exercised automatically and is also the path `vox ci exec-policy-contract`'s `run_check_for_ci` always uses regardless of `pwsh` availability. No phase work is blocked by pwsh's absence. |
| `cargo` (via build broker) | All Rust wiring/tests in this phase | ✓ | 1.98.1 | — |
| `cargo-nextest` | Fast test iteration (`.config/nextest.toml` present) | Not directly probed this session; standard for this repo per AGENTS.md §Local CI Gate Tiers | — | `cargo test -p <crate>` as a fallback if nextest isn't on PATH in the execution environment. |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** `pwsh` — the phase's exec-policy work exercises the Rust-fallback path exclusively in this environment, which is the path that matters for SC#1's verification anyway.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest (`.config/nextest.toml` present, `[profile.default]`/`[profile.ci]`) |
| Config file | `.config/nextest.toml` |
| Quick run command | `cargo test -p vox-orchestrator --lib` (scoped; use `cargo nextest run -p vox-orchestrator` if nextest is on PATH) |
| Full suite command | `vox ci pre-push --full` (per AGENTS.md §Local CI Gate Tiers) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-dead-crate-wire-up (SC#1) | Exec-policy rust-fallback path calls `risk::classify` and rejects disallowed commands | integration | `cargo run -p vox-cli -- ci exec-policy-contract` | ✅ already exists (`crates/vox-cli/src/commands/ci/exec_policy_contract.rs`, exercises `SMOKE_PAYLOADS`/`REJECT_PAYLOADS` against both paths) |
| REQ-dead-crate-wire-up (SC#2) | Unrecognized MCP tool name is rejected fail-closed by the new validation | unit | `cargo test -p vox-orchestrator --lib tool_receipt` or `-- safety::` once tests are added | ❌ Wave 0 — no such test exists yet; `tool_receipt.rs` currently has zero `#[cfg(test)]` coverage in-file |
| REQ-dead-crate-wire-up (SC#2, D-04 integration depth) | A known-good tool name (e.g. any `vox_mcp_registry::TOOL_REGISTRY` entry) is still accepted after the validation is added | unit/integration | new test in `crates/vox-orchestrator/tests/` (no existing file covers `ToolReceiptLedger`/`issue_tool_receipt`) | ❌ Wave 0 |
| REQ-dead-crate-wire-up (SC#3) | `crate-classification-2026-05-08.md` no longer lists `vox-doc-inventory` as DEAD | manual review | none — this is a documentation fix; the file is explicitly tombstoned and outside any automated doc-accuracy gate found this session | ❌ N/A — manual verification only, no automated gate covers this file's content |

### Sampling Rate
- **Per task commit:** `cargo test -p vox-orchestrator --lib` (SC#2) and `cargo run -p vox-cli -- ci exec-policy-contract` (SC#1 verification).
- **Per wave merge:** `vox ci pre-push --complete` (includes clippy — required since this touches Rust source; the default fast tier omits clippy per AGENTS.md's Perennial Bug Patterns note) plus `vox ci crate-edges` (will fail until the exceptions entry is authorized — see Pitfall 3).
- **Phase gate:** Full suite green before `/gsd-verify-work`, and `vox ci crate-edges` green (post-authorization) before merge.

### Wave 0 Gaps
- [ ] `crates/vox-orchestrator/src/tool_receipt.rs` or `orchestrator/safety.rs` — add `#[cfg(test)]` coverage for the new fail-closed validation (currently zero in-file tests for `ToolReceiptLedger`).
- [ ] `crates/vox-orchestrator/tests/` — no existing integration test file touches `ToolReceiptLedger`/`issue_tool_receipt`; D-04 asks for integration-style coverage, so a new test file (or an addition to an existing orchestrator integration test file) is needed.
- [ ] Framework install: none — `cargo`/nextest already present.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V4 Access Control | yes | Fail-closed rejection of unrecognized MCP tool names before a receipt is issued (D-02) — an unrecognized "tool" must not be treated as an authorized action. |
| V5 Input Validation | yes | `tool_name: &str` is currently unvalidated free-form input at a trust boundary (`ToolReceiptLedger::issue_intent`); validating against the canonical `TOOL_REGISTRY` allow-list is the standard control already used elsewhere in this codebase (`ExecPolicy`'s allow-list-of-binaries pattern in `vox-container-types::exec_grammar::policy`). |
| V6 Cryptography | no (unchanged) | `ToolReceipt`'s BLAKE3-keyed HMAC tag generation is pre-existing, untouched by this phase's scope, and already routes through the standard primitives per AGENTS.md's Cryptography Policy (BLAKE3 is on the approved list). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Agent claims a tool call using a name that doesn't correspond to any real MCP tool (typo, hallucination, or deliberate spoofing of a not-yet-existent "future" tool) | Spoofing / Elevation of Privilege | Fail-closed `TOOL_REGISTRY` membership check before receipt issuance (this phase's SC#2 work) — exactly the fix D-02 locks in. |
| A new crate dependency edge silently expands one crate's blast radius without review | Tampering (of the dependency graph / build) | `vox ci crate-edges`'s exact-edge-set ratchet + human-authorized `exceptions` ledger (Pitfall 3) — already enforced tooling, this phase must not bypass it. |

## Sources

### Primary (HIGH confidence — read in full or targeted-read this session)
- `contracts/terminal/exec-policy.v1.yaml` — the ADR-026 runtime policy contract.
- `crates/vox-container-types/src/exec_grammar/{mod.rs,policy.rs,risk.rs}` — the existing risk classifier and its tests.
- `crates/vox-cli/src/commands/runtime/shell/check_terminal.rs` — `run_check`, `run_check_for_ci`, `run_check_rust_fallback` (lines 1-411, 413-500 read in full).
- `crates/vox-cli/src/commands/ci/exec_policy_contract.rs` — full file read; confirms CI coverage of both paths.
- `crates/vox-container/src/lib.rs` — confirms `vox-cli`'s actual call path to `exec_grammar`.
- `docs/src/adr/026-third-party-code-provenance.md` — confirms the "Future `vox-exec-grammar`" language that seeded the PRD's stale crate-name assumption.
- `crates/vox-mcp-registry/src/lib.rs`, `crates/vox-mcp-registry/src/semcov_wave46_tests.rs` — TOOL_REGISTRY's public shape and existing correctness coverage.
- `crates/vox-orchestrator/src/tool_receipt.rs` (full file), `crates/vox-orchestrator/src/orchestrator/safety.rs` (read) — the identified integration point and its current zero-validation state.
- `crates/vox-orchestrator/src/agentos/policy_runtime.rs` (full file) — the rejected alternate integration point and why it doesn't fit.
- `crates/vox-orchestrator-mcp/src/registry.rs` (full file), targeted reads of `dispatch.rs` — confirms the live, already-wired MCP dispatch chain in a different crate.
- `crates/vox-corpus/src/mcp_meta.rs` (full file) and `crates/vox-corpus/src/lib.rs` module list — confirms the file is orphaned/uncompiled.
- `contracts/ci/crate-edges.allow.v1.json`, `contracts/ci/crate-layers.v1.json`, `crates/vox-cli-ci/src/crate_edges.rs` (full file) — the crate-edge ratchet mechanics and exceptions authorization requirement.
- `docs/src/architecture/crate-classification-2026-05-08.md` (targeted read, lines 1-160) — confirms tombstone status and the actual current per-crate rows.
- `docs/src/architecture/where-things-live.md`, `docs/src/architecture/layers.toml`, `docs/src/architecture/classification-ssot-2026.md` — confirms none of these carry the CORE/DEAD taxonomy for these crates.
- `docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` — the source PRD, cross-checked against current repo state.
- `.planning/phases/01-dead-crate-cleanup-remove-confirm/01-RESEARCH.md` — cross-checked vox-mcp-meta deletion commit and the vox-corpus duplicate flag.
- `crates/vox-cli/Cargo.toml`, `crates/vox-doc-inventory/Cargo.toml`, `crates/vox-orchestrator/Cargo.toml`, `crates/vox-mcp-registry/Cargo.toml` — dependency verification.

### Secondary (MEDIUM confidence)
None used — all package/dependency claims in this research were verified directly against source files, not web search, since this phase involves zero external packages.

### Tertiary (LOW confidence)
None.

## Metadata

**Confidence breakdown:**
- SC#1 (exec-policy): HIGH — fully read the call chain and its CI test; the "already satisfied" finding is a direct code read, not an inference.
- SC#2 (MCP wiring): HIGH on the gap and rejected-alternative reasoning (all read directly); MEDIUM on "is `issue_tool_receipt` THE intended call path" — flagged explicitly as Assumption A1 / Open Question 1, since no caller exists to confirm intent.
- SC#3 (reclassification): HIGH — the target file, its tombstone status, and both crates' current rows were read directly; the one remaining edit (vox-doc-inventory) is unambiguous.

**Research date:** 2026-09-23
**Valid until:** Effectively indefinite for the exec-policy/MCP findings (they describe code as of a specific commit and won't silently go stale — re-verify only if `main` moves significantly before planning starts). Treat as stale if more than ~7 days pass before planning, per this repo's fast-moving-crate-graph norm (`crate-edges.allow.v1.json` changes routinely).
