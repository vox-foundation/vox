---
phase: 88-mcp-tool-cli-agent-guidance
verified: 2026-10-17T22:30:00Z
status: passed
score: 4/4 must-haves verified
covered_files:
  - ".planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/88-mcp-tool-cli-agent-guidance/88-03-SUMMARY.md"
  - "AGENTS.md"
  - "contracts/ci/crate-edges.allow.v1.json"
  - "contracts/operations/catalog.v1.yaml"
  - "contracts/profiles/lang.v1.yaml"
  - "crates/vox-cli/src/cli_dispatch/mod.rs"
  - "crates/vox-cli/src/commands/diagram.rs"
  - "crates/vox-cli/src/commands/diagram/write.rs"
  - "crates/vox-diagram/grammar-card.txt"
  - "crates/vox-diagram/src/validate.rs"
  - "crates/vox-orchestrator-mcp/Cargo.toml"
  - "crates/vox-orchestrator-mcp/src/diagram_tools.rs"
  - "crates/vox-orchestrator-mcp/src/dispatch.rs"
  - "crates/vox-orchestrator-mcp/src/registry.rs"
  - "docs/src/reference/cli.md"
  - "docs/src/reference/vox-diagram.md"
covered_digest: "v3:sha256:2b5f76cc2134b4026868d3868ebbaace19e68edc5264c94a5a996f8974a1854b"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 88: MCP Tool, CLI & Agent Guidance Verification Report

**Phase Goal:** Agents and developers can validate, format, render, export, and import diagrams through supported surfaces, and agents are told to author VoxDiagram.
**Verified:** 2026-10-17
**Status:** passed (with classified pre-existing red items, see Warnings)
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `vox_validate_diagram` returns every diagnostic, `fixes[]`, `normalized_source`, `stats`; `from_mermaid` gives conversion or by-line `unsupported`; deterministic, no LLM | VERIFIED | `crates/vox-diagram/src/validate.rs` is a pure function over `check`/`fmt`/`mermaid::import`/`svg::render`; handler `diagram_tools.rs` calls it via `spawn_blocking`, `deny_unknown_fields`, no network or model code. `cargo nextest run -p vox-orchestrator-mcp --test validate_diagram_tool`: 10/10 pass (from_mermaid ok and gantt-unsupported, emit svg/mermaid parity with the crate, unknown field and `emit:"png"` rejected, byte-identical repeat calls). `cargo nextest run -p vox-diagram`: 579 passed, 3 skipped (incl. golden `validate_envelope` snapshots). CLI `--json` on a two-mistake file printed both diagnostics with `fixes[]`, `normalized_source: null`. |
| 2 | Tool registered via the operations catalog (dispatch, schema, fixtures, generated artifacts, GUI coverage, no new exemption); profile-parity question resolved; `command-sync`/`ssot-drift` pass; only new edge `vox-orchestrator-mcp -> vox-diagram` citing 2026-10-16 authorization | VERIFIED | Catalog row `diagram.validate` (public, `side_effect_class: none`, read-role). `dispatch.rs` arm, `input_schemas.rs`, `registry.rs` (advertised description), `lang.v1.yaml`, `http-read-role-governance.yaml`, `tool-registry.canonical.yaml`, advertised-tools fixture, `mcp-tools.generated.md` all contain it. Fresh `target/debug/vox`: `ci ssot-drift` exit 0, `ci command-sync` OK, `ci command-compliance` OK (1033 ops), `ci gui-surface-coverage` and `ci gui-surface-registry` up to date, `ci crate-edges` OK (702 edges), `ci check-links` OK, `ci crate-api-sync` OK. `crate-edges.allow.v1.json` diff is exactly +7 lines, one `exceptions` entry, `date 2026-10-16`, `authorized_by "user (explicit chat approval, 2026-10-16)"`; no `edges` or fan-in snapshot change. No exemption/baseline file touched except the `command_catalog_paths_baseline.txt` additions (new paths). |
| 3 | `vox diagram check/fmt/render/export/import` on files or stdin, exit codes reflect diagnostics; catalog rows, needles, `cli.md`; no `vox-cli -> vox-diagram` edge | VERIFIED | Ran the built binary on scratch files: `check broken.vd` prints `file:line:col: error: code: msg`, exit 1; ok file exit 0; stdin `-` reports `<stdin>`; `fmt` prints canonical, `fmt --check` non-canonical exit 1; `fmt --write -` exit 2; `render` emits SVG, `export` emits Mermaid, `import` of a clean flowchart prints canonical DSL, of one with `classDef` prints `bad.mmd:3: unsupported: styling:classDef` exit 1; missing file exit 2; `--json` with `-o` exit 2; `render -o <input>` refused exit 2; unknown subcommand exit 2. `cargo nextest run -p vox-cli --test diagram_cli --test diagram_cli_files`: 27/27 pass. Six catalog rows (`diagram`, `diagram.{check,fmt,render,export,import}`), `cli.md` section, surface-registry row `cli_group: diagram` tier `none`. `crates/vox-cli/Cargo.toml` has no diff and no `vox-diagram`; `diagram.rs` imports only `vox_doc_pipeline::diagram`. |
| 4 | Grammar card (<=~600 tokens measured, examples pass `check`) in agent guidance; AGENTS.md directs new docs diagrams to `vox-diagram` | VERIFIED | `grammar-card.txt` 2189 bytes; `grammar_card_fits_600_tokens` measures with `tiktoken_rs` o200k and passes; `grammar_card` tests (examples pass `check`, are `fmt` fixed points, Mermaid-isms flagged, page equals constant) pass. `registry.rs` appends the card to the tool description (`tool_list_advertises_the_card` passes). `docs/src/reference/vox-diagram.md` publishes it; scoped doc lint "no hard errors". `AGENTS.md` adds a 7-line "Diagrams in docs (VoxDiagram)" subsection pointing at the page, the tool and `vox --json diagram check`, and forbidding new `mermaid` fences. |

**Score:** 4/4 truths verified (0 present, behavior-unverified).

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| TOOL-01 | 88-01, 88-03 | SATISFIED | Truths 1, 2 |
| TOOL-02 | 88-02, 88-03 | SATISFIED | Truth 3 |
| TOOL-03 | 88-01, 88-03 | SATISFIED | Truth 4 |

No orphaned requirements: REQUIREMENTS.md maps only TOOL-01..03 to Phase 88, all claimed by plans.

### Anti-Patterns

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK` in the phase's new or edited source, tests or docs. No stubs: handler, envelope and CLI are substantive and exercised by tests; all review findings WR-01..06, IN-01, IN-02, IN-04 were fixed in `4be9a44bd` and `d36720b86` and their regression tests are in the passing suites above.

### Warnings (pre-existing, not caused by this phase; none blocks the goal)

1. `vox ci profile-parity` exits 1: `lean: forbidden crate vox-gamify present in lean build`. Independent check: `cargo tree -p vox-cli --no-default-features -i vox-gamify` shows it enters via `vox-cli-core` and `vox-lsp`; `vox-diagram` is a leaf with no gamify path, and the phase's only manifest change is the mcp -> diagram edge. It is the sole violation reported (no crate-count budget breach). The workflow runs it in `nightly.yml` only. The tool's profile placement (`lang`, next to `vox_validate_vuv`) is resolved.
2. `vox-orchestrator-mcp` lib tests `profile_backcompat_tests::{snapshot_is_todays_default_feature_registry,dev_profile_advertises_the_snapshot}` fail at the phase base: I confirmed `vox_hopper_submit` is in `tool-registry.canonical.yaml` but absent from the fixture at `82b24d2d4` (0 matches). The phase added only `vox_validate_diagram` to the fixture. Not run by me; the evidence is the fixture diff. These tests run in the hosted `tests` shard for the affected crate, so the owner should expect them red there until the 14-tool fixture gap is fixed (tracked in `deferred-items.md`).
3. Not exercised here by instruction: hosted CI and GUI typecheck (deferred to the hosted `ui` job and the milestone PR). `crate-edges` prints a pre-existing stale-baseline warning for `vox-gui -> vox-db-types`, unrelated.
4. Doc-accuracy note from the fix report (WR-06 wording of "bare envelope = MCP `data` field") was read in `cli.md` line 209 and matches the code and the MCP test (`data == validate()`).

### Human Verification Required

None.

## Gaps Summary

No gaps. All four ROADMAP success criteria hold on the committed tree, verified against code and by running the built `vox` binary and the targeted test suites. The two red items above predate the phase and are recorded in `deferred-items.md`.

---

_Verified: 2026-10-17_
_Verifier: Claude (gsd-verifier)_
