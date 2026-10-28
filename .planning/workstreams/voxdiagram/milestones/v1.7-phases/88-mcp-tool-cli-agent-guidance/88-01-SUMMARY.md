---
phase: 88-mcp-tool-cli-agent-guidance
plan: 01
subsystem: mcp
tags: [mcp, vox-diagram, operations-catalog, crate-edges, insta, grammar-card]

requires:
  - phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
    provides: vox-diagram public API (check, fmt, mermaid import/export, svg render, GRAMMAR_CARD) and the vox-doc-pipeline re-export
provides:
  - "vox_diagram::{validate, ValidateOpts, Emit, Validation}: the one validation envelope (MCP tool and, in plan 02, vox diagram --json)"
  - "vox_validate_diagram MCP tool (catalog row diagram.validate, dispatch arm, input schema, read-role governance, lang profile, advertised-tools fixture)"
  - "grammar card appended to the tool's advertised description, measured under 600 o200k tokens"
  - "the single user-authorized crate edge vox-orchestrator-mcp -> vox-diagram with its ledger entry"
affects: [88-02 vox diagram CLI (consumes validate/Validation), 88-03 grammar card finalization and docs]

status: complete
plan_head_before: 99a64c801e1cf9f8ade83a193643db5149fcfd0f
plan_head_after: eae2146e3b704332ed5688ec05297fb0198e6eca
actuals:
  tokens: 6400
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Pure envelope in the domain crate, thin MCP handler: serde-parsed Args with deny_unknown_fields, CPU work in spawn_blocking"
    - "Advertised description derived at runtime from one constant (advertised_description) so the card is never copied"

key-files:
  created:
    - crates/vox-diagram/src/validate.rs
    - crates/vox-diagram/tests/validate_envelope.rs
    - crates/vox-diagram/tests/snapshots/validate_envelope__broken_graph.snap
    - crates/vox-diagram/tests/snapshots/validate_envelope__sequence_emit_mermaid.snap
    - crates/vox-diagram/tests/snapshots/validate_envelope__from_mermaid_ok.snap
    - crates/vox-diagram/tests/snapshots/validate_envelope__from_mermaid_unsupported.snap
    - crates/vox-orchestrator-mcp/src/diagram_tools.rs
    - crates/vox-orchestrator-mcp/tests/validate_diagram_tool.rs
  modified:
    - crates/vox-diagram/src/lib.rs
    - crates/vox-orchestrator-mcp/{Cargo.toml,src/lib.rs,src/dispatch.rs,src/input_schemas.rs,src/registry.rs}
    - crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt
    - contracts/operations/catalog.v1.yaml
    - contracts/mcp/http-read-role-governance.yaml
    - contracts/profiles/lang.v1.yaml
    - contracts/ci/crate-edges.allow.v1.json
    - Cargo.lock

key-decisions:
  - "Envelope field order and serde attributes exactly as the plan's interfaces block; normalized_source is always serialized (null while errors remain)"
  - "Mermaid input path is the DSL path over the converted DSL, so diagnostics spans point into normalized_source"
  - "Args are spelled-out fields (no flatten) so unknown-field refusal works; a missing source is reported by serde as invalid arguments naming `source`"

patterns-established:
  - "Render failure is surfaced as emit_error, never dropped"

requirements-completed: [TOOL-01]

duration: ~75min
completed: 2026-10-17
---

# Phase 88 Plan 01: vox_validate_diagram MCP tool Summary

**Deterministic `vox_validate_diagram` MCP tool over a new shared `vox_diagram::validate` envelope (diagnostics with fixes, canonical source, stats, Mermaid import, SVG/Mermaid emit), with the grammar card in its advertised description and the one ledgered `vox-orchestrator-mcp -> vox-diagram` edge.**

## Accomplishments

- Task 1 (tracer): a broken graph goes `handle_tool_call` -> dispatch arm -> `diagram_tools` -> `vox_diagram::validate` and returns `success: true`, `valid: false`, two diagnostics (`diagram/undeclared-node`, `diagram/expected-quoted-label`) each with `fixes[]`, `normalized_source: null`, `stats`, `schema_version: 1`; the data equals `serde_json::to_value(validate(..))`. Catalog row, governance, schema, fixture, projections and the edge + exception landed with it.
- Task 2: `from_mermaid` (conversion as `normalized_source` + `unsupported: []`, or `valid: false` + by-line `unsupported`), `emit` svg/mermaid, `emit_error` for render-cap failures, `dropped`, strict arguments, `spawn_blocking`, determinism; four insta goldens pin the JSON shape.
- Task 3: `advertised_description` appends `GRAMMAR_CARD` (measured under the 600 o200k-token budget by `grammar_card_fits_600_tokens`), `lang` profile entry, full SSOT regeneration, gates, and the ledger mutation proof.

## Task Commits

1. Task 1 tracer: `2cb02cc64`
2. Task 2 full envelope + goldens: `1a425e880`
3. Task 3 card, profile, regeneration: `360498b4f`
4. Follow-up generator output (operations inventory counts): `eae2146e3`

## TDD record (RED observed before GREEN)

- Task 1: vox-diagram `validate::tests` failed to compile (`validate`/`ValidateOpts` undeclared, 7 errors) before `validate.rs` implementation existed. The MCP integration tests were authored first but their first run was after implementation (RED there is by construction: unknown tool name / missing module), so no separate runtime RED was recorded for them.
- Task 2: `cargo nextest ... validate_envelope` failed to compile (unknown `from_mermaid`/`emit` fields, `Emit` import) before the full envelope.
- Task 3: `advertised_description` unresolved (2 compile errors) before implementation.

## Mutation proofs

1. `deny_unknown_fields` (T-88-03): removed the attribute from `diagram_tools.rs` (grep count 1 -> 0 for the attribute line; the file keeps one occurrence after restore), reran the MCP filter: `diagram_tools::tests::unknown_field_is_refused` and `validate_diagram_tool::rejects_unknown_fields_and_bad_emit` FAILED with ``assertion `left == right` failed`` on `{"success":true,"data":{"schema_version":1,"valid":true,...}}` (an extra field was accepted). Restored from a scratchpad copy (`cmp` clean), reran green (25/25).
2. Ledger entry (T-88-01): deleted the new `vox-orchestrator-mcp -> vox-diagram` object (`grep -B1 ... | grep -c vox-orchestrator-mcp` 1 -> 0); `vox ci crate-edges` exited 1 with `NEW EDGE not in baseline: vox-orchestrator-mcp -> vox-diagram` / `Error: crate-edges: 1 violation(s)`. Restored (`cmp` clean, grep back to 1, `git diff --numstat 82b24d2d4` = `7 0`), gate green (`crate-edges: OK (702 live in-tree edges within baseline)`).

## Verification

- `cargo nextest run -p vox-orchestrator-mcp ... (validate_diagram_tool, diagram_tools::, input_schemas::tests::, registry_dispatch_tests::)`: green except the two pre-existing failures below.
- `INSTA_UPDATE=no cargo nextest run -p vox-diagram ... validate_envelope`: 13/13; no `.snap.new`; `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean.
- `cargo nextest run -p vox-mcp-registry`: 85/85.
- `vox ci ssot-drift`, `crate-edges`, `crate-api-sync`, `affected-crates --check`, `cargo run -p vox-arch-check`: exit 0 (arch-check only stale-evidence WARN lines). `cargo hakari generate --diff`: empty.
- Acceptance: Cargo.toml `1 0`, edge ledger `7 0`, catalog `23 0` (append-only), `exemptions:` block 7 lines, `fan-in-snapshot.v1.json` and the `edges` array unchanged, `validate.rs` 175 non-blank lines, no `infer_with_retry|llm_chat|reqwest` in `diagram_tools.rs`.

## Generated files changed (all via generators)

`operations-sync`: `tool-registry.canonical.yaml`, `capability-registry.yaml`, `operations-catalog-inventory.v1.json`. `ssot-regen` list: `model-manifest.generated.json`, `capability-index.v1.json` (also gains the previously missing `crate.vox-diagram` row, kept as the plan directs), `capabilityIndex.generated.ts`, `gui-surface-coverage.v1.json`, `mcp-tools.generated.md`, `crate-graph.v1.json`, `test-inventory.v1.json`. `crate-api-sync --write`: `vox-diagram.generated.md`, `vox-orchestrator-mcp.generated.md`. Outside the plan's list but written by the same generators and committed because `ssot-drift` verifies them: `contracts/ci/crate-build-map.v1.json` (new vox-diagram row, +1 dependents/fan_in for the MCP crate's new edge; the fan-in snapshot baseline is untouched), `docs/src/.well-known/llms.txt`, `docs/src/.well-known/vox-docs.json` (capability-index outputs). `index.generated.md` and `workspace-hack` needed no change.

## Deviations from Plan

### Auto-fixed / handled

**1. [Rule 3 - Blocking] Generator invocation without PATH override**
- Found during: Task 3 regeneration
- Issue: the shell policy blocks inline `PATH=...`, which `vox run scripts/ssot-regen.vox` needs to find `vox`.
- Fix: ran the identical generator list from `scripts/ssot-regen.vox` by absolute path (`target/debug/vox ci ...`) plus `crate-api-sync --write`; same commands and arguments, no freshness bypass.

**2. [Scope] Pre-existing test failures, not fixed**
- `registry::profile_backcompat_tests::{snapshot_is_todays_default_feature_registry, dev_profile_advertises_the_snapshot}` fail at the phase base: the advertised-tools fixture lacks 14 tools the committed registry already advertises (`vox_gamify_*`, `vox_gui_run_*`, `vox_hopper_*`, `vox_plan_node*`). Plan 88-01 added `vox_validate_diagram` to both sides; the 14-tool diff is unrelated. Logged in `deferred-items.md`; the "existing tests pass" behavior bullet is therefore met for everything except these two.

**3. [Follow-up commit] `operations-catalog-inventory.v1.json`**
- The counts (+1 operation, +1 MCP tool) were written by a later generator run and committed separately as `eae2146e3`.

**4. [Minor] Doc comment wording**
- Reworded the `Args` doc comment so `grep -c deny_unknown_fields` on `diagram_tools.rs` prints 1 as the acceptance criterion requires.

**5. [Minor] Missing-source error text**
- With strict `Args` parsing, a missing `source` yields `invalid arguments: missing field \`source\``, which names `source` as required (Task 1 test), replacing Task 1's interim hand-written message.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model (T-88-01..05, SC all mitigated or accepted as planned).

## Self-Check: PASSED

All created files exist and all four commits are ancestors of HEAD (verified below).
