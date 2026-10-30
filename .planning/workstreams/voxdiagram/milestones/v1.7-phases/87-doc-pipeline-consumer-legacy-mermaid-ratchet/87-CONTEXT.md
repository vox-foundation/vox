# Phase 87: Doc-Pipeline Consumer & Legacy-Mermaid Ratchet - Context

**Gathered:** 2026-10-17
**Status:** Ready for planning

<domain>
## Phase Boundary

`crates/vox-doc-pipeline` becomes the first consumer of `vox-diagram`: doc lint validates every ```` ```vox-diagram ```` fence under `docs/src`, and a shrink-only legacy-Mermaid ratchet stops the count of ```` ```mermaid ```` fences from growing — before any migration (Phase 90). No SVG pre-render / Astro wiring (Phase 89), no CLI/MCP (Phase 88), no docs migration (Phase 90).

Requirements: DOCS-01, DOCS-02.

</domain>

<decisions>
## Implementation Decisions

### Legacy-Mermaid ratchet
- Baseline: per-file counts (`path → n`) in `contracts/documentation/legacy-mermaid-baseline.v1.yaml`; adding a fence to any listed file, or any new file with a fence, fails; a `--tighten` mode only ever lowers counts / removes entries (shrink-only, mirroring `facet_backlog.rs`).
- Seed: from the committed tree at implementation time (33 fences / 21 files at committed HEAD on 2026-10-16) — NOT the stale 41/26 in ROADMAP SC1 (corrected below). If `main` gains Mermaid fences before this branch merges, CI flags it and the user re-seeds at merge time; growth is never auto-admitted.
- Counting: fence-aware via `vox_diagram::fence::extract`, info string exactly `mermaid`, `docs/src/archive/` excluded; prose mentions and fences nested inside 4-backtick example blocks do not count.
- Runs in doc lint (CI + fast pre-push tier) with a base-ref compare so a PR that grows the count fails even if its own baseline file was edited upward.

### `vox-diagram` fence lint & crate edge
- Diagnostic format `docs/src/<file>.md:<line>:<col>: diagram/<code>: <message>`, fence-body spans mapped back to file lines; Errors fail the lint, Warnings print but don't block.
- Info string exactly `vox-diagram` (after `lint.rs`'s comma split); tests prove the `vox` doctest runner (`doctest.rs` whole-string match) never compiles it and that `vox-diagram,foo` is handled consistently by both paths.
- Crate edge (USER-AUTHORIZED in chat 2026-10-16): add exactly `vox-doc-pipeline -> vox-diagram` with ONE `exceptions` entry in `contracts/ci/crate-edges.allow.v1.json` citing that authorization. No other edge. Never regenerate a baseline to admit anything.
- `pub use vox_diagram as diagram;` re-export from `vox-doc-pipeline` (so the CLI in Phase 88 needs no third edge).
- Remove `orphan_exempt` for `vox-diagram` in `docs/src/architecture/layers.toml`.
- Self-heal bot: the bot (`claims/self_heal_bot.rs`) exists only in local `main`'s unpushed commit `ceb1b5d1a`, not on this branch. User decision 2026-10-17: expose `diagrams::fence_issues(md)` now; wire the bot (~3 lines + test) and re-seed the Mermaid baseline when this branch next merges `main`.
- Keep `pipeline/mod.rs` edits small (already over the 500-line limit) — new logic in new modules.

### Roadmap correction (2026-10-17)
- SC1 seed changes from "41 fences / 26 files" to "the committed-tree count at implementation time (33/21 on 2026-10-16)".

### Claude's Discretion
- Module names (`diagrams.rs`, `mermaid_gate.rs`), YAML schema shape, CLI flag spelling for tighten, fixture layout.

</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-doc-pipeline/src/` has `lint.rs` (comma-split info), `doctest.rs` (whole-string `vox`), `facet_backlog.rs` (shrink-only ratchet pattern to copy), self-heal bot module, `pipeline/mod.rs` (>500 lines).
- `vox_diagram::{fence::extract, check, Diagnostic}` (Phase 83).
- Gates: `vox ci crate-edges`, `vox-arch-check` (orphan rule), `vox ci pre-push` fast tier.

</code_context>

<specifics>
## Specific Ideas

Tests: a fixture docs tree with (a) a new mermaid fence → lint fails, (b) a 4-backtick example containing ```mermaid → not counted, (c) a broken vox-diagram fence → `file:line:col: diagram/<code>`, (d) a valid one → passes.

</specifics>

<deferred>
## Deferred Ideas

- Gating `docs/superpowers/` fences.

</deferred>
