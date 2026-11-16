# Phase 413: Pre-Push & CI Gate Zero-Red Convergence - Context

**Gathered:** 2026-11-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Restore 100% green `exit 0` infrastructure configuration and CI verification across `vox ci pre-push`, `vox-arch-check`, `vox ci fan-in-budget`, `vox ci secret-env-guard`, `vox ci crate-edges`, `vox ci ssot-drift`, and `vox ci cross-repo-parity`.

</domain>

<decisions>
## Implementation Decisions

### the agent's Discretion
- All implementation choices are at the agent's discretion — pure infrastructure phase (`HYGIENE-GATE-01..06`).
- Exclude vendored `patches/` directories from Edition-2024 `rustfmt` sweeps in `crates/vox-cli/src/commands/ci/pre_push.rs` (`step_fmt_scoped`) and `scripts/fmt.vox` (`skip_tombstone`).
- Replace direct `std::env::var("VOX_REPO_ROOT")` in `crates/vox-doc-inventory/src/cli_generate.rs` with explicit `--repo-root <dir>` CLI flag + CWD walk-up discovery so `secret-env-guard` passes with zero new crate edges.
- Annotate detector pattern literal in `crates/vox-code-audit/src/detectors/hardcoded_abs_dir.rs` with `// vox-arch-check: allow abs-path` + line-aligned `toestub-ignore` comment, and reconcile `layers.toml` `vox-secrets` `max_dependents` (`35`) + `fan-in-snapshot.v1.json` (`vox-ast: 4`, `vox-bounded-fs: 24`).
- Reconcile user-authorized downward baseline edges in `contracts/ci/crate-edges.allow.v1.json` and run `vox ci crate-edges --tighten` to purge all `39` stale edges.
- Regenerate `contracts/ci/crate-graph.v1.json` (`vox ci affected-crates --regen --out contracts/ci/crate-graph.v1.json`) and sync telemetry contracts with `vox-ops` (`vox ci cross-repo-parity --sync`). Note: `contracts/db/baseline-version-policy.yaml` is being updated concurrently by `v7.0` Phase 408 (`BASELINE_VERSION = 98`), so inspect before touching and defer any residual digest check to Phase 418 if Phase 408 is mid-edit.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-cli/src/commands/ci/pre_push.rs` (`step_fmt_scoped`, `check_fmt`)
- `crates/vox-cli-ci/src/crate_edges.rs` (`tighten`, `check`)
- `crates/vox-repository/src/repo_catalog/federation.rs` (`sync_federated_satellites`)

### Established Patterns
- Targeted contract updates without re-sorting unrelated rows (`00-core.md` Rule 11).
- Scoped `rustfmt --edition 2024 <changed .rs>` formatting (`00-core.md` Rule 7).

### Integration Points
- `contracts/ci/crate-edges.allow.v1.json`, `contracts/ci/fan-in-snapshot.v1.json`, `contracts/ci/crate-graph.v1.json`, `docs/src/architecture/layers.toml`.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
