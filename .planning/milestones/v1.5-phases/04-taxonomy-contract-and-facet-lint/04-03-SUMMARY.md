---
phase: 04-taxonomy-contract-and-facet-lint
plan: 03
subsystem: kb-facets
tags: [kb, json-schema, codegen, drift-guard, gui]
requires: ["04-01"]
provides:
  - "contracts/documentation/kb-facets.v1.schema.json: committed facet JSON Schema under a golden test"
  - "vox_cli_ci::kb_facets_ts::{SCHEMA_REL, OUT_REL, render_kb_facets_ts, run}"
  - "vox ci config-gui-codegen --kb-facets [--check]"
  - "crates/vox-gui/ui/src/generated/kbFacets.generated.ts: KbFacets, Source, enum unions, *_VALUES, KB_FACET_KEYS"
affects: [04-08, 06-library-gui]
tech-stack:
  added: []
  patterns: ["closed-shape JSON-Schema-to-TS renderer that bails on unsupported shapes", "split-needle dispatch guard test for ssot-drift"]
key-files:
  created:
    - crates/vox-kb-facets/tests/facet_schema_golden.rs
    - contracts/documentation/kb-facets.v1.schema.json
    - crates/vox-cli-ci/src/kb_facets_ts.rs
    - crates/vox-gui/ui/src/generated/kbFacets.generated.ts
    - crates/vox-gui/ui/src/__tests__/kbFacets.generated.test.ts
  modified:
    - crates/vox-cli-ci/src/lib.rs
    - crates/vox-cli-ci/src/cmd_enums.rs
    - crates/vox-cli/src/commands/ci/run_body.rs
    - crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs
    - scripts/ssot-regen.vox
key-decisions:
  - "Renderer reads the committed JSON file, so vox-cli-ci takes no vox-kb-facets edge"
  - "Every generated property is optional (`?:`) and nullable fields are `T | null`, matching the schema (all properties defaulted, none required)"
  - "Schema property order relies on serde_json preserve_order, which workspace-hack pins for every crate"
requirements-completed: [TAX-02]
duration: ~35min
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: d5f31ff0c474fc802b9b9949481ce589f1b5a57a
actuals:
  tokens: 7600
  tasks: 2
  commits: 2
---

# Phase 4 Plan 03: Facet schema golden and generated GUI type Summary

The facet schema now exists once (the Rust `Facets` type), is committed as `kb-facets.v1.schema.json` under a golden test, and the GUI `KbFacets` TypeScript type is rendered from that committed file, with drift checks on the Rust side, the ssot-drift side, and the TS side.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: Rust Facets -> committed JSON Schema (golden) -> generated TS type | 9feb8286c |
| 2 | Wire TS drift check into ssot-drift, CLI, regen bot; vitest/typecheck guard | cdde1baa9 |

Tracer verified end-to-end (golden + 4 renderer tests green) before wiring.

## Tests

- `cargo test -p vox-kb-facets --test facet_schema_golden`: 1 passed (`facet_schema_matches_committed_contract`; failure text names `UPDATE_GOLDEN=1 cargo nextest run -p vox-kb-facets --test facet_schema_golden`)
- `cargo test -p vox-cli-ci kb_facets_ts`: 4 passed (`renders_small_schema_exactly`, `unsupported_shape_names_the_property`, `committed_schema_render_is_deterministic_and_ordered`, `committed_kb_facets_ts_matches_schema`)
- `cargo test -p vox-cli run_ssot_drift_dispatches`: both dispatch guards passed (`..._kb_facets_ts` new)
- vitest `kbFacets.generated.test.ts`: 9 passed; `pnpm typecheck` clean (the `satisfies KbFacets` fixture compiles)
- `cargo run -p vox-cli -- ci config-gui-codegen --kb-facets --check`: exit 0, "kb-facets TS OK"
- `vox check scripts/ssot-regen.vox`: passed with 1 pre-existing warning (line 16, `!=`, not from this plan)
- Suite runner was `cargo test` (not nextest) with a shared target dir; same tests.

## Mutation proofs (all with before/after presence checks)

1. Golden test: `grep -c mutation_probe lib.rs` 0 -> added `pub mutation_probe: Option<String>` to `Facets` (count 1) -> `facet_schema_matches_committed_contract` FAILED with the regenerate message -> removed (count 0) -> green.
2. TS drift: hand-edited `"llm-assisted"` to `"llm-assistedX"` in the generated TS (count 1 after) -> `committed_kb_facets_ts_matches_schema` FAILED naming the regenerate command -> restored (count 0) -> green.
3. ssot-drift wiring: `grep -c '^    ds!("kb_facets_ts"'` 1 -> deleted the line (0) -> `run_ssot_drift_dispatches_kb_facets_ts` FAILED -> restored (1) -> green.

## Deviations from Plan

None - plan executed as written. The plan's `nextest` commands were run as `cargo test` equivalents; the plan's `--check` CLI run used `cargo run -p vox-cli` (installed `vox` is stale).

## Known Stubs

None.

## Threat Flags

None. T-04-07 (golden + `--check` in ssot-drift + vitest/typecheck) and T-04-15 (renderer bails on unsupported shapes, including inline enums that would otherwise degrade to `string`) are mitigated.

## Self-Check: PASSED

- FOUND: contracts/documentation/kb-facets.v1.schema.json, crates/vox-cli-ci/src/kb_facets_ts.rs, crates/vox-gui/ui/src/generated/kbFacets.generated.ts, crates/vox-gui/ui/src/__tests__/kbFacets.generated.test.ts, crates/vox-kb-facets/tests/facet_schema_golden.rs
- FOUND commits: 9feb8286c, cdde1baa9
- No `vox-kb-facets` in crates/vox-cli-ci/Cargo.toml; no `: any`/`: unknown` in the generated TS
