# Phase 4: GUI/Dashboard Architecture Consolidation - Research

**Researched:** 2026-09-22
**Domain:** Documentation ratification + audit (ADR status, SSOT command-surface parity, UI boundary doc, desktop-build verification) — no new runtime dependencies
**Confidence:** HIGH (all four requirement areas verified against source/doc content read this session)

## Summary

This phase is a **verification-and-ratification phase**, not a build phase: three of four requirements (GUI-01, GUI-02, GUI-04) resolve to confirming/documenting an already-correct implementation state, and one (GUI-03) resolves to writing a boundary rule that does not yet exist as a single artifact. The single highest-value finding from this research is a **verified blocking defect in CONTEXT.md's own D-01 instruction**: setting ADR-045's frontmatter `status: "accepted"` will fail `vox-doc-pipeline`'s lint gate, because `"accepted"` is not in the enforced `VALID_STATUS` enum (`crates/vox-doc-pipeline/src/pipeline/lint.rs:37-44`: `current|experimental|legacy|research|roadmap|deprecated`). Every other Accepted/locked ADR in this repo (037, 041, 047) keeps frontmatter `status: "current"` and encodes the real decision status only in the body `**Status**: ...` line — the plan must follow that exact precedent, not the frontmatter flip CONTEXT.md describes literally.

GUI-02's audit is close to a formality: `vox-gui`'s only three Rust-side CommandCatalog consumers (`catalog.rs`, `action_manifest.rs`, `discovery.rs`) all call `vox_cli::command_catalog::build_catalog()` directly, and the only TS-side consumer (`Catalog.tsx`) sources `skills` from an `invoke('get_command_catalog')` call — no hardcoded command list exists anywhere in `crates/vox-gui/ui/src`. `navigation.ts`'s `PARENT_CHILD_MAP` is NOT a command-catalog violation — it is IA/grouping metadata for GUI-native views (chat, agents, knowledge, etc.) that have no CLI-command equivalent; `CommandCatalogEntry` carries no section/grouping field, so this could never be catalog-derived. Document this explicitly rather than "fixing" it.

GUI-03's corrected boundary source (`external-frontend-interop-plan-2026.md`, per CONTEXT.md D-02) describes a **language-level** boundary (`.vox` `component` sources vs. hand-authored `.tsx` imported via `import react ... from "<spec>"`) that is already parser-implemented (`crates/vox-compiler/src/parser/descent/decl/head_import.rs`). But `vox-gui` itself — the surface GUI-03 is scoped to per the Phase Boundary — contains **zero** `.vox` component sources (0 of 319 UI source files); it is 100% hand-authored TSX consuming `@tanstack/react-query` and Tauri `invoke()`. The concrete, checkable boundary rule for THIS phase is therefore a **surface-level** one already half-captured in `contracts/frontend/surface-ownership.v1.yaml` (vox-gui = canonical operator surface, hand-authored; marquee-app = external React interop reference) — GUI-03's deliverable is writing this down as an explicit rule plus confirming no `.vox`-compiled component has been smuggled into `vox-gui/ui/src`, not inventing new compiler enforcement.

GUI-04's three ADR-037 desktop claims are **all independently verified true in code**, contradicting the "still hint-only" framing in ADR-037's own 2026-05-11 Context section (which describes the pre-migration state, not the current one): `crates/vox-codegen/src/codegen_rust/emit/mod.rs::generate_tauri_workspace` (invoked via `RustAppShell::TauriApp` for `CompileKind::Desktop`) emits a real `src-tauri/Cargo.toml` + `main.rs` + `build.rs`, explicitly documented as omitting Axum/rust-embed; `crates/vox-cli/src/commands/bundle.rs::build_single_binary` detects that `src-tauri/Cargo.toml` and routes to `build_tauri_app`, which runs real `cargo tauri build --no-bundle`; and `docs/src/architecture/layers.toml` already carries `no-axum-in-generated-app-emit` / `no-rust-embed-in-generated-cargo` / `no-capacitor-in-app-codegen` forbidden patterns with tracked `exempt_files`. GUI-04 is a documentation-only close-out — updating ADR-037's own "Consequences"/status framing to reflect implementation completion, no code changes needed.

**Primary recommendation:** Treat this phase as three documentation edits (ADR-045 body Status line + classification JSON sync; ADR-037 desktop-clause completion note; a new/extended boundary-rule doc section) plus one written audit report (GUI-02, confirming — not fixing — the existing clean state), with zero new package installs and zero required source-code changes to `crates/vox-gui` or `crates/vox-cli`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| ADR/doc ratification (GUI-01) | Docs / Governance | `.planning/intel/` metadata | Doc frontmatter + body edit; classification JSON is a GSD-ingest artifact, not a runtime file |
| CommandCatalog SSOT (GUI-02) | Backend (`vox-cli` reflects clap; `vox-gui` Tauri commands) | Frontend (React consumes via `invoke()`) | `build_catalog()` lives in `vox-cli`; GUI is a pure consumer over Tauri IPC — no business logic in the webview per ADR-045 |
| Vox-native vs. React-interop UI boundary (GUI-03) | Compiler/codegen (`.vox` `component` → TSX) | Docs (`contracts/frontend/*`, interop plan) | The distinction is a language/codegen concept; `vox-gui` itself sits entirely on the "hand-authored React" side, so enforcement for THIS phase is documentation + a one-time audit, not a new compiler check |
| Desktop app shell (GUI-04) | Codegen (`vox-codegen::codegen_rust::emit`) → Build orchestration (`vox-cli::commands::bundle`) | — | `vox compile --target desktop` (codegen) and `vox bundle` (build orchestration, invokes `cargo tauri build`) are two cooperating tiers, not one; do not assume `compile` alone produces a binary |

## Standard Stack

No new libraries are introduced by this phase. Existing stack (unchanged, versions confirmed from `crates/vox-gui/ui/package.json` — already installed, not newly added):

| Library | Version (installed) | Purpose | Source |
|---------|---------|---------|--------|
| `@tauri-apps/api` | (workspace-pinned, Tauri 2) | IPC `invoke()`/`listen()` — the only allowed transport per ADR-045 rule 2 | `[VERIFIED: crates/vox-gui/ui/src/transport.ts:1-2]` `import { invoke } from '@tauri-apps/api/core'` |
| `react` | ^19.2.8 | UI runtime | `[VERIFIED: crates/vox-gui/ui/package.json]` |
| `@tanstack/react-query` | ^5.102.8 | Data-fetching/caching for Tauri IPC results | `[VERIFIED: crates/vox-gui/ui/package.json]` |
| `@tanstack/react-virtual` | ^3.14.10 | List virtualization | `[VERIFIED: crates/vox-gui/ui/package.json]` |
| `tauri` (Rust crate) | 2.x | Desktop app shell, `cargo tauri build` | `[VERIFIED: crates/vox-codegen/src/codegen_rust/emit/mod.rs:641 doc comment + build.rs invocation at crates/vox-cli/src/commands/bundle.rs:313]` |

**Package Legitimacy Audit:** N/A — this phase adds no new dependencies. Skip the legitimacy-check protocol.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| A second GUI-command lint/CI gate | A new bespoke "no manual nav" static-analysis tool | Follow the SSOT-codegen idiom already used by `vox ci gui-catalog-parity` (`crates/vox-cli/src/commands/ci/gui_catalog_parity.rs`, compares `contracts/operations/catalog.v1.yaml` against `crates/vox-gui/ui/src/types/catalog.ts`) if a permanent check is added (Claude's discretion per D-03) | Repo already has a working pattern for catalog-vs-TS-type drift detection; don't invent a second mechanism |
| Vox-native vs. interop boundary enforcement | A new forbidden_pattern rule in `layers.toml` guessing at file paths | Document the rule in prose first (GUI-03's actual deliverable); `layers.toml`/`vox-arch-check` currently has **zero** rules mentioning "island" or "vox-native" (`[VERIFIED: grep over docs/src/architecture/layers.toml and crates/vox-arch-check/src/forbidden_patterns.rs returned no matches]`) — inventing a mechanical check is out of this phase's evidenced scope unless CONTEXT.md discretion is stretched to cover it |

**Key insight:** Every "don't-hand-roll" risk in this phase is about *scope creep into new tooling*, not about missing libraries — the phase is fundamentally a documentation/audit exercise.

## Architecture Patterns

### System Architecture Diagram

```
.vox source (examples/golden/dashboard_ui.vox)      crates/vox-gui/ui/src/**/*.tsx (hand-authored, 319 files, 0 .vox)
        │  component/routes keyword                          │  React + @tanstack/react-query
        ▼                                                     ▼
  vox-codegen (TS/React emit)                    Tauri webview (crates/vox-gui/ui)
        │  "Vox-native" track (GUI-03)                        │  invoke()/listen() ONLY — no fetch()/WS (ADR-045 rule 2)
        ▼                                                     ▼
  emitted .tsx (importable by any React app,         crates/vox-gui/src/commands/*.rs (Tauri command handlers)
  or itself `import react X from "…"`)                        │
        │                                                     ├─ catalog.rs ─────┐
        └──────── bidirectional interop (Phase 5) ────────────┤                  ▼
                                                                ├─ action_manifest.rs   vox_cli::command_catalog::build_catalog()
                                                                └─ discovery.rs             (reflects clap CommandCatalog — SSOT)
                                                                                    │
  vox compile --target desktop                                                    ▼
        │  RustAppShell::TauriApp                              crates/vox-cli/src/command_catalog.rs
        ▼
  vox-codegen::emit::generate_tauri_workspace
  (src-tauri/Cargo.toml, main.rs, build.rs — no axum/rust-embed)
        │
        ▼
  vox bundle  →  build_single_binary()
        │  finds src-tauri/Cargo.toml present
        ▼
  build_tauri_app()  →  `cargo tauri build --no-bundle`  →  real desktop installer
```

### Recommended Project Structure

No new files/folders needed. Doc edits land in:
```
docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md   # GUI-01: body Status line + (see Pitfall 1) frontmatter left as-is
docs/src/adr/037-tauri-convergence.md                    # GUI-04: desktop-clause completion note
docs/src/architecture/external-frontend-interop-plan-2026.md  # GUI-03: candidate location for an explicit boundary-rule subsection, OR
contracts/frontend/surface-ownership.v1.yaml              # GUI-03: candidate location to extend with an explicit "authoring-track" field per surface
.planning/intel/decisions.md                              # GUI-01: sync the ADR-045 entry's status/note
.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json  # GUI-01: flip locked:false→true, confidence:medium→high
```

### Pattern 1: ADR ratification via body Status line (not frontmatter)
**What:** Every Accepted/locked ADR in this repo signals its status via an explicit body line `**Status**: <value> (<date>)`, immediately under the H1 title. Frontmatter `status:` stays `"current"` regardless of decision-locked state.
**When to use:** GUI-01 (ADR-045 ratification).
**Verified example (the literal template to mirror):**
```markdown
# ADR 037 — Tauri Convergence

**Status**: Partially superseded (2026-06-12) by [adr-NNN](../architecture/adr-NNN-scope-tauri-desktop-only.md) — the mobile-target clause only (Tauri no longer covers `--target mobile-android`/`--target mobile-ios`; React Native + Expo does). The desktop-convergence decision below is unaffected and remains Accepted.
**Date**: 2026-05-11
```
`[VERIFIED: docs/src/adr/037-tauri-convergence.md:11-13]`

For ADR-045 (currently has NO body Status line at all — `[VERIFIED: docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md:1-8]`, frontmatter is `status: "current"`, body starts directly at `## Context`), the exact target text should be:
```markdown
# ADR 045 — Tauri GUI Replaces Axum Dashboard

**Status**: Accepted (2026-09-22)

## Context
```

### Pattern 2: CommandCatalog is a flat clap reflection with NO grouping metadata
**What:** `CommandCatalogEntry` (`crates/vox-cli/src/command_catalog.rs:53-69`) has fields `path, command, about, aliases, has_subcommands, compiled_in, source_group, feature_gate, tier, capability_id, arguments` — no `section`/`parent`/`group_label` field for GUI IA placement.
**When to use:** When auditing whether `navigation.ts`'s `PARENT_CHILD_MAP` (sidebar section grouping: Review/Agents/Knowledge/Workspace/Commands/Compute/Settings) violates ADR-045 rule 1.
**Verified conclusion:** It does not violate the rule. `PARENT_CHILD_MAP` groups GUI-native *view keys* (many of which — chat, dashboard, memory, browser — have no 1:1 CLI command at all) into IA sections; it cannot be derived from `CommandCatalog` because the catalog carries no such data. The rule's actual enforcement scope is: no `.tsx` file should independently declare a *command's existence* (name/args/about) outside of what `get_command_catalog()` returns. Only `crates/vox-gui/ui/src/components/surfaces/Catalog/Catalog.tsx` renders catalog entries, and it sources them via `skills` prop ← `invoke('get_command_catalog')` (typed via `crates/vox-gui/ui/src/transport.ts` + `crates/vox-gui/ui/src/types/tauri.ts`'s `CommandCatalog` type), with no fallback/hardcoded array: `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Catalog/Catalog.tsx:6-10]` — `const catalog = { generated_from: 'tauri:get_command_catalog', entries: skills ?? [] };`

### Anti-Patterns to Avoid
- **Conflating the two catalogs:** `contracts/operations/catalog.v1.yaml` (checked by the existing `vox ci gui-catalog-parity` against `crates/vox-gui/ui/src/types/catalog.ts`) is a *separate* SSOT from `vox_cli::command_catalog::build_catalog()` (the ADR-045 CommandCatalog). They cover different surfaces (Operations Catalog = capability/MCP/CLI cross-mapping; CommandCatalog = clap-tree reflection for the GUI sidebar). GUI-02's audit is about the second one; do not present the first as satisfying it. `[VERIFIED: crates/vox-cli/src/commands/ci/gui_catalog_parity.rs:25-27,246]`
- **Setting ADR frontmatter `status: "accepted"`:** see Pitfall 1 below — this is a hard lint failure, not a style choice.

## Common Pitfalls

### Pitfall 1: `status: "accepted"` in ADR frontmatter fails the doc-pipeline lint
**What goes wrong:** CONTEXT.md's D-01 literally instructs updating ADR-045's frontmatter `status: "current"` → `status: "accepted"`. This value is rejected.
**Why it happens:** `vox-doc-pipeline` enforces a closed enum for the `status:` frontmatter key: `[VERIFIED: crates/vox-doc-pipeline/src/pipeline/lint.rs:37-44]` `pub(crate) const VALID_STATUS: &[&str] = &["current", "experimental", "legacy", "research", "roadmap", "deprecated"];`. Any other value trips `LintKind::UnknownStatus` at the `status:` line (`[VERIFIED: crates/vox-doc-pipeline/src/pipeline/lint.rs:405-416]`), and there is a dedicated test (`suggest_maps_typo_to_nearest_status_by_edit_distance`, line 853) proving a bogus status value (e.g. `"bogus"`) produces a lint error — "accepted" would trip the identical path. Cross-checked against every other ADR: `grep -hE '^status:' docs/src/adr/*.md` across 24 ADRs with a frontmatter `status:` key returns only `"current"` (17), `"deprecated"` (3), `"research"` (4) — **zero** use `"accepted"`, including locked ADRs 041 and 047 (`[VERIFIED: docs/src/adr/041-durable-functions-completion-2026.md:1-7, docs/src/adr/047-iroh-transport.md:1-7]` both keep `status: "current"`). `docs/src/contributors/documentation-governance.md:190` documents this exact enum as the canonical vocabulary.
**How to avoid:** Follow Pattern 1 exactly — leave ADR-045's frontmatter `status:` at `"current"`, add the body `**Status**: Accepted (2026-09-22)` line only. This still satisfies GUI-01's underlying requirement (an explicit, unambiguous Accepted marker) and matches the repo's own established precedent for every other locked ADR.
**Warning signs:** `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` failing with `UnknownStatus`.

### Pitfall 2: `.planning/intel/` classification metadata drifts if not updated alongside the doc edit
**What goes wrong:** GUI-01's acceptance criterion ("locked-equivalent classification") is not automatically satisfied by editing the ADR file — the GSD ingest pipeline wrote a separate machine-readable snapshot that a future `/gsd-plan-phase` or `/gsd-progress` run may re-read as still-unratified.
**Why it happens:** `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` currently reads `[VERIFIED: .planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json]` `{"confidence": "medium", "locked": false, ..., "notes": "No explicit 'Status: Accepted' line in document body. Frontmatter has status='current' which is non-standard for ADRs."}`, and `.planning/intel/decisions.md:266-272` carries a matching prose note ("Not marked locked/Accepted... medium confidence. ... Treat as authoritative pending a formal Accepted status line."). Neither is read by any Vox-repo runtime tool — they are GSD planning-session artifacts only — but they are the literal "classification" GUI-01's acceptance criterion refers to.
**How to avoid:** After editing the ADR, also flip the JSON's `"confidence": "medium"` → `"high"` and `"locked": false` → `true` (update or clear the stale `"notes"` field), and update the corresponding paragraph in `decisions.md`. This is a two-file, mechanical sync — not new tooling.
**Warning signs:** A later `/gsd-progress` or audit pass re-flagging ADR-045 as still-unratified despite the doc edit landing.

### Pitfall 3: `vox compile --target desktop` alone does not produce a binary — `vox bundle` does
**What goes wrong:** Verifying GUI-04 claim (a) by only reading `compile.rs`'s `CompileKind::Desktop` arm leads to the wrong conclusion that Tauri build tooling is never invoked, because `compile.rs` itself only calls `bundle::run(..., BundleMode::App, ...)` (which does the codegen AND, deeper in its call chain, the `cargo tauri build` invocation) plus a secondary `emit_tauri_and_assets` step that re-writes `tauri.conf.json` with project-specific bundle identifier/display-name/capabilities on top of the codegen-emitted generic one.
**Why it happens:** The pipeline is codegen (`vox-codegen::emit::generate_tauri_workspace`, real `src-tauri/Cargo.toml`+`main.rs`+`build.rs`) → build orchestration (`vox-cli::commands::bundle::build_single_binary`, which checks `src-tauri/Cargo.toml.is_file()` and branches to `build_tauri_app` → `cargo tauri build --no-bundle`). `[VERIFIED: crates/vox-cli/src/commands/bundle.rs:240-243,309-346; crates/vox-codegen/src/codegen_rust/emit/mod.rs:295-384,641-642]`. Reading only one half of this (e.g. only `vox-tauri-codegen`'s doc comments, which are stale — they still say "src-tauri Rust crate wiring is completed by `cargo tauri init` ... on [the consumer's] side", describing the PRE-migration state) gives a false-negative.
**How to avoid:** Trace the full `compile --target desktop` → `bundle::run` → `build_single_binary` → `build_tauri_app` chain before concluding claim (a) is unmet. It is met.
**Warning signs:** Citing `crates/vox-tauri-codegen/src/lib.rs`'s module doc comments as current-state evidence — they describe the crate's own narrow scope (config/hints), not the full pipeline that also runs through `vox-codegen`.

## Code Examples

### GUI-01: ADR-045 target edit (verified current state → target)
```markdown
<!-- Source: docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md, verified current content lines 1-8 -->
<!-- CURRENT frontmatter (leave unchanged — see Pitfall 1): -->
---
title: "ADR 045 — Tauri GUI Replaces Axum Dashboard"
date: "2026-05-11"
status: "current"
category: "Architecture Decisions (ADRs)"
---

<!-- CURRENT body opening (line 8): -->
# ADR 045 — Tauri GUI Replaces Axum Dashboard

## Context
...

<!-- TARGET body opening — insert Status line, mirroring ADR-037's exact convention verified at docs/src/adr/037-tauri-convergence.md:11 -->
# ADR 045 — Tauri GUI Replaces Axum Dashboard

**Status**: Accepted (2026-09-22)

## Context
...
```

### GUI-02: The three verified CommandCatalog consumer call sites
```rust
// Source: crates/vox-gui/src/commands/catalog.rs (full file, 5 lines)
#[tauri::command]
pub fn get_command_catalog() -> Result<serde_json::Value, String> {
    let catalog = vox_cli::command_catalog::build_catalog();
    serde_json::to_value(&catalog).map_err(|e| e.to_string())
}
```
```rust
// Source: crates/vox-gui/src/commands/action_manifest.rs:191 (verified via Bash grep in this session)
let catalog = vox_cli::command_catalog::build_catalog();
```
```rust
// Source: crates/vox-gui/src/commands/discovery.rs:6,62,89 (verified via Bash grep in this session)
use vox_cli::command_catalog::{CommandCatalogEntry, build_catalog};
// ...
let catalog = build_catalog();
```

### GUI-04: The forbidden-pattern rules already enforcing "no axum/rust-embed in generated desktop apps"
```toml
# Source: docs/src/architecture/layers.toml:466-484 (verified, current content)
[[forbidden_pattern]]
name             = "no-axum-in-generated-app-emit"
pattern          = 'axum::serve|axum::Router::new|axum::routing'
file_glob        = "crates/vox-codegen/src/codegen_rust/emit/**/*.rs"
exempt_files     = [
    "crates/vox-codegen/src/codegen_rust/emit/http.rs",
]
allow_annotation = "// vox-arch-check: allow axum-app-emit"
reason           = "Generated desktop/mobile applications must use Tauri; Axum emit is reserved for native-binary, dashboard, server, and daemon/API lanes."

[[forbidden_pattern]]
name             = "no-rust-embed-in-generated-cargo"
pattern          = 'rust-embed'
file_glob        = "crates/vox-codegen/src/codegen_rust/emit/**/*.rs"
exempt_files     = [
    "crates/vox-codegen/src/codegen_rust/emit/mod.rs",
]
allow_annotation = "// vox-arch-check: allow rust-embed-app-emit"
reason           = "Generated desktop/mobile applications bundle assets through Tauri `dist/`; rust-embed remains only for non-Tauri targets during migration."
```
GUI-04's item (c) ("layers.toml lists the forbidden pattern... with current violations tracked in exempt_files") is **already satisfied** — both rules exist with populated `exempt_files`. No edit needed here; cite this in the ratification note as evidence, don't re-author it.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | GUI-03's deliverable should be written into `external-frontend-interop-plan-2026.md` and/or `contracts/frontend/surface-ownership.v1.yaml` rather than a brand-new doc file | Recommended Project Structure | Low — either location satisfies "documented"; the plan should pick one and note it, no functional difference. Flagged `[ASSUMED]` because CONTEXT.md does not pin the exact target file, only the target *doc set* to research against. |
| A2 | The `.planning/intel/` classification-JSON sync (Pitfall 2) is in-scope for the planner to task, not out-of-band GSD housekeeping | Common Pitfalls | Low-medium — if descoped, GUI-01's acceptance criterion ("locked-equivalent classification") is not fully met by a pure doc edit; worth a one-line planner decision either way. |

**All other claims in this research are `[VERIFIED]`** against files read directly in this session (paths and line ranges cited inline) — no other assumptions were required. No package-legitimacy or version-verification claims apply (no new dependencies).

## Open Questions

1. **Where exactly should the GUI-03 boundary rule live?**
   - What we know: `contracts/frontend/surface-ownership.v1.yaml` already has the right *shape* (per-surface `status`/`role`/`notes`) and could gain an `authoring_track: vox-native | react-hand-authored | hybrid` field per surface entry; `external-frontend-interop-plan-2026.md` already has the right *prose* content (Phase 5) but is framed as a forward-looking plan, not a locked rule doc.
   - What's unclear: Whether the planner should extend the YAML (machine-checkable, minimal), add a prose subsection to the interop plan (matches "documented rule" wording most literally), or both.
   - Recommendation: Do both in one task — a short YAML field addition (cheap, future-checkable) plus a one-paragraph prose rule (satisfies "documented... enforced" wording) referencing the verified fact that `vox-gui/ui/src` currently has 0 `.vox`-derived files.

2. **Does GUI-04's "confirmed complete" require touching ADR-037's own status line, or is a separate confirmation note enough?**
   - What we know: ADR-037's body Status line already says the desktop clause "remains Accepted" (unaffected by the mobile supersession) — this text is already correct and true.
   - What's unclear: Whether GUI-04's acceptance criterion is satisfied by the status quo (cite ADR-037 as-is plus the verified code evidence in a phase-completion note) or requires an ADR-037 edit adding an explicit "(verified 2026-09-22: desktop clause implementation complete)" annotation.
   - Recommendation: Add the annotation — cheap, and it closes the loop for future readers who would otherwise have to re-derive the same three code-verification facts this research just did.

## Sources

### Primary (HIGH confidence — direct file reads/greps this session)
- `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` — full content read
- `docs/src/adr/037-tauri-convergence.md` — full content read
- `docs/src/architecture/adr-NNN-scope-tauri-desktop-only.md` — full content read
- `docs/src/architecture/external-frontend-interop-plan-2026.md` — full content read
- `docs/src/architecture/external-frontend-interop-phase5-component-interop-subspec-2026.md` — partial (grep-targeted)
- `docs/src/architecture/layers.toml` — lines 440-609 read
- `docs/src/architecture/tauri-convergence-migration-plan-2026.md` — grep-targeted (task list, phase headers)
- `docs/src/contributors/documentation-governance.md` — status vocabulary section read
- `contracts/frontend/{gui-compatibility,surface-ownership,dependency-policy}.v1.yaml` — full content read
- `crates/vox-cli/src/command_catalog.rs` — lines 1-90 read
- `crates/vox-gui/src/commands/{catalog.rs,action_manifest.rs,discovery.rs}` — read/grepped
- `crates/vox-gui/ui/src/lib/navigation.ts` — read in full
- `crates/vox-gui/ui/src/components/surfaces/Catalog/Catalog.tsx` — read in full
- `crates/vox-gui/ui/src/transport.ts` — partial read
- `crates/vox-cli/src/commands/compile.rs` — lines 70-330 read
- `crates/vox-cli/src/commands/bundle.rs` — lines 1-350 read
- `crates/vox-codegen/src/codegen_rust/emit/mod.rs` — lines 280-680 read
- `crates/vox-codegen/src/codegen_rust/mod.rs` — grep-targeted (test references to `RustAppShell::TauriApp`)
- `crates/vox-doc-pipeline/src/pipeline/lint.rs` — lines 37-44, 380-470, 853-891 read
- `crates/vox-cli/src/commands/ci/gui_catalog_parity.rs` — lines 1-60, 225-246 read
- `.planning/intel/decisions.md` — lines 149-280 read
- `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` — full content read
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `04-CONTEXT.md` — full content read

### Secondary (MEDIUM confidence)
- None — no WebSearch/Context7 lookups were needed; this is an entirely in-repo verification phase.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- ADR-045 ratification mechanics (GUI-01): HIGH — frontmatter enum, body precedent, and classification-JSON location all directly read
- CommandCatalog SSOT audit (GUI-02): HIGH — all consumer call sites enumerated and read; no hardcoded fallback found
- UI boundary (GUI-03): HIGH on "what exists today" (0 .vox files in vox-gui, no compiler-level boundary lint); MEDIUM on "where the new rule doc should live" (A1, genuinely open)
- Desktop convergence (GUI-04): HIGH — full codegen→build call chain traced and verified

**Research date:** 2026-09-22
**Valid until:** 30 days (stable, doc/audit-only phase; re-verify if `vox-doc-pipeline` lint rules or the Tauri codegen pipeline change materially)
