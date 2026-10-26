# Phase 301: Change-Gated CI Path Map & `vox ci check-changed` - Context

**Gathered:** 2026-10-25
**Status:** Ready for planning
**Mode:** Autonomous smart-discuss (auto-answered from ROADMAP + t3code audit)

<domain>
## Phase Boundary

Port t3code `check:changed` + INERT/EXTRA_OWNERS ownership mapping into a Vox contract and CLI so local/CI runs only the gates owned by the diff.

Reuse existing `vox_cli_ci::affected` / `affected-crates` / `check-targets.v1.yaml` — do not fork a second affected-crate engine. Add an **owner map** (inert prefixes + path→gate owners) and a **`vox ci check-changed`** front-end that plans/runs gates for the merge-base diff and exits 0 with zero cargo invocations on inert-only diffs.
</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
1. Contract path: `contracts/ci/changed-path-owners.v1.yaml` (schema_version 1).
2. CLI: `vox ci check-changed [--base <ref>] [--dry-run]` registered in `vox-cli-ci` `CiCmd`.
3. Inert prefixes (planning/docs/media that never force cargo): `.planning/**`, `docs/src/archive/**`, `*.md` under docs that are not SSOT-gated — keep SSOT/doc-lint paths non-inert when they affect gates.
4. EXTRA_OWNERS map path globs → check-target IDs from `check-targets.v1.yaml` (or logical groups: `rust`, `gui`, `docs`, `contracts`, `workflows`).
5. Inert-only → print summary, exit 0, never spawn cargo.
6. Docs: extend `docs/src/ci/affected-crate-selective-ci.md` Local parity + Future work F3 closure note in `runner-contract.md` briefly.
</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-cli-ci/src/affected.rs` — `compute_affected`, sentinels, docs-only → None
- `crates/vox-cli-ci/src/affected_cmd.rs` — `vox ci affected-crates`, `partition_crates` (weighted shards already exist — Phase 302)
- `contracts/ci/check-targets.v1.yaml` — gate registry with `pr_scope` / `rust_only`
- Docs already list Future work **F3: `vox ci path-gate-plan`** — this phase ships that under the name `check-changed`
</code_context>

<specifics>
## Specific Ideas

Mirror t3code `scripts/check-changed.ts` UX: compare merge-base with `origin/main` (or `--base`), list changed files, decide owners, run or dry-run mapped gates.
</specifics>

<deferred>
## Deferred Ideas

Full parallel execution of all check-targets commands in check-changed (beyond dry-run + inert short-circuit) can land as a follow-up once the plan JSON is stable — Phase 301 success criteria require inert exit-0 + docs + unit fixture; running a small allowlisted gate set on non-inert diffs is enough.
</deferred>
