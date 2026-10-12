# Phase 2: Wire Up & Reclassify Dormant Crates - Context

**Gathered:** 2026-09-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire up functionally-complete but never-adopted functionality into its intended call path (exec-policy risk classification, MCP tool-name validation), and correct two crate classification errors (vox-search, vox-doc-inventory: DEAD → CORE). Covers REQ-dead-crate-wire-up only. EXTRACT-TO-PLUGIN and MISPLACED dispositions (Phase 3) are explicitly out of boundary.

</domain>

<decisions>
## Implementation Decisions

### Exec-grammar wiring — verify before assuming
- **D-01:** `crates/vox-exec-grammar` does not exist as a standalone crate (confirmed during discussion — `find`/`glob` returned nothing). Exec-policy enforcement already lives in `vox-container-types::exec_grammar::policy` (`ExecPolicyV1`), consumed by `vox-cli/src/commands/runtime/shell/check_terminal.rs::run_check`. This may mean REQ-dead-crate-wire-up's exec-grammar item is already satisfied, matching Phase 1's pattern (PRD-described crate doesn't exist under that name; functionality landed elsewhere). **Do not presuppose either way** — research must verify whether `check_terminal.rs`'s existing implementation actually satisfies the ADR-026 contract in `contracts/terminal/exec-policy.v1.yaml`, or whether something genuinely remains unwired. — **Reversibility:** N/A (a verification decision, not an implementation choice).

### TOOL_REGISTRY validation behavior
- **D-02:** When vox-orchestrator validates/enumerates MCP tool names via `TOOL_REGISTRY` (confirmed today: zero references from vox-orchestrator to vox-mcp-registry — genuinely dormant), unrecognized tool names must be rejected (fail closed), not merely logged. Matches the security-conscious pattern already established elsewhere in this codebase (capability gating, exec-policy). — **Reversibility:** costly — loosening a fail-closed check to fail-open later is a behavior change on a trust boundary, not a pure revert.

### Reclassification target — deferred to research
- **D-03:** `vox-search` and `vox-doc-inventory` are not mentioned anywhere in `docs/src/architecture/classification-ssot-2026.md` (confirmed during discussion — zero matches). Their current DEAD classification lives in some other file not yet identified. Research must locate the actual source before any reclassification edit is planned — do not guess the target file.

### Test coverage depth
- **D-04:** Beyond the bare AGENTS.md Test-First Policy minimum (one adjacent test per new `pub fn`), add integration-style tests exercising the full wired path: the exec-policy gate end-to-end (if new wiring is needed per D-01's outcome), and TOOL_REGISTRY's fail-closed behavior under a bad/unknown tool name (per D-02). — **Reversibility:** reversible — additional test coverage, no behavior risk.

### Claude's Discretion
- Exact reclassification file/mechanism once research identifies it (D-03).
- Exact shape of any new exec-policy wiring code, if D-01's research finds something genuinely missing.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements source
- `.planning/REQUIREMENTS.md` — REQ-dead-crate-wire-up's locked acceptance criteria.
- `docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` — the source PRD, full per-item rationale.
- `.planning/PROJECT.md` — milestone-level constraints (crate dependency edges, test-first policy).

### Exec-policy (D-01)
- `contracts/terminal/exec-policy.v1.yaml` — the ADR-026 contract this wiring must satisfy.
- `crates/vox-container-types/src/exec_grammar/policy.rs` — existing policy evaluation (`ExecPolicyV1`), maps an `ExecAst` against the contract's rules.
- `crates/vox-cli/src/commands/runtime/shell/check_terminal.rs` — `run_check`/`run_check_for_ci`, the existing gate consuming the policy.
- `crates/vox-cli/src/commands/ci/exec_policy_contract.rs` — `vox ci exec-policy-contract`, validates the YAML against schema.

### TOOL_REGISTRY / MCP (D-02)
- `crates/vox-mcp-registry/src/lib.rs` — `TOOL_REGISTRY` definition, generated from `contracts/mcp/tool-registry.canonical.yaml` via `build.rs`.
- `contracts/mcp/tool-registry.canonical.yaml` — the SSOT the registry is generated from.

### Reclassification (D-03)
- `docs/src/architecture/classification-ssot-2026.md` — checked, does NOT mention either crate; not the target.
- `docs/src/architecture/where-things-live.md` — likely candidate, not yet confirmed.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ExecPolicyV1` / `vox_container_types::exec_grammar::policy` — already-built policy evaluation engine; if D-01 finds real work needed, this is what any new call site wires into.
- `TOOL_REGISTRY` (`crates/vox-mcp-registry`) — already-generated, tested const array (`semcov_wave46_tests.rs` covers uniqueness, non-empty names, tier validity); vox-orchestrator's job is to consume it, not rebuild it.

### Established Patterns
- Build-generated SSOT consts (`TOOL_REGISTRY` from `contracts/mcp/tool-registry.canonical.yaml` via `build.rs`) — the same SSOT-driven codegen pattern used throughout this repo (AGENTS.md).
- Test-First Policy — every new `pub fn` needs an adjacent `#[test]` in the same file.

### Integration Points
- `crates/vox-orchestrator` — where TOOL_REGISTRY validation must be added (currently zero references).
- `crates/vox-cli/src/commands/runtime/shell/check_terminal.rs` — candidate integration point for D-01 if wiring is genuinely needed.

</code_context>

<specifics>
## Specific Ideas

No specific implementation-style requirements beyond the four decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. EXTRACT-TO-PLUGIN and MISPLACED work (Phase 3) was not discussed here.

</deferred>

---

*Phase: 2-Wire Up & Reclassify Dormant Crates*
*Context gathered: 2026-09-23*
