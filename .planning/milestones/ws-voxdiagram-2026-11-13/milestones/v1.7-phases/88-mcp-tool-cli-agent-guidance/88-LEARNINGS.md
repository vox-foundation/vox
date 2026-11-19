---
phase: 88
phase_name: "mcp-tool-cli-agent-guidance"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 15
  lessons: 6
  patterns: 6
  surprises: 4
missing_artifacts:
  - "UAT.md"
---

# Phase 88 Learnings: mcp-tool-cli-agent-guidance

## Decisions

### Envelope field order and serde attributes exactly as the plan's interfac...
Envelope field order and serde attributes exactly as the plan's interfaces block; normalized_source is always serialized (null while errors remain)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-01-SUMMARY.md

---

### Mermaid input path is the DSL path over the converted DSL, so diagnostic...
Mermaid input path is the DSL path over the converted DSL, so diagnostics spans point into normalized_source

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-01-SUMMARY.md

---

### Args are spelled-out fields (no flatten) so unknown-field refusal works;...
Args are spelled-out fields (no flatten) so unknown-field refusal works; a missing source is reported by serde as invalid arguments naming `source`

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-01-SUMMARY.md

---

### Task 1 (tracer)
Task 1 (tracer): a broken graph goes `handle_tool_call` -> dispatch arm -> `diagram_tools` -> `vox_diagram::validate` and returns `success: true`, `valid: false`, two diagnostics (`diagram/undeclared-node`, `diagram/expected-quoted-label`) each with `fixes[]`, `normalized_source: null`, `stats`, `schema_version: 1`; the data equals `serde_json::to_value(validate(..))`. Catalog row, governance, schema, fixture, projections and the edge + exception landed with it.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-01-SUMMARY.md

---

### Task 2: from_mermaid (conversion as normalized_source + unsupported: [],...
Task 2: `from_mermaid` (conversion as `normalized_source` + `unsupported: []`, or `valid: false` + by-line `unsupported`), `emit` svg/mermaid, `emit_error` for render-cap failures, `dropped`, strict arguments, `spawn_blocking`, determinism; four insta goldens pin the JSON shape.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-01-SUMMARY.md

---

### Task 3: advertised_description appends GRAMMAR_CARD (measured under the ...
Task 3: `advertised_description` appends `GRAMMAR_CARD` (measured under the 600 o200k-token budget by `grammar_card_fits_600_tokens`), `lang` profile entry, full SSOT regeneration, gates, and the ledger mutation proof.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-01-SUMMARY.md

---

### Help/doc lines contain no '
Help/doc lines contain no ': ' so they stay plain YAML scalars in the catalog and equal the generated surface text

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-02-SUMMARY.md

---

### fmt without flags prints to stdout; warnings use the same file:line:col
fmt without flags prints to stdout; warnings use the same file:line:col: code: message shape (exit code carries severity)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-02-SUMMARY.md

---

### Catalog side_effect_class set (diagram, fmt, render, export, import
Catalog side_effect_class set (diagram, fmt, render, export, import: writes_files; check: read_only) because the capability-index side-effect ratchet rejects unclassified cli rows

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-02-SUMMARY.md

---

### Task 2: fmt (stdout, --check, --write), render, export, import with -o F...
Task 2: `fmt` (stdout, `--check`, `--write`), `render`, `export`, `import` with `-o FILE`; `import` prints `<file>:<line>: unsupported: <construct>` (and `dropped:`) lines; a render over the caps exits 1 with the `emit_error` text. `--json` refuses `-o`/`--write`/`--check` before reading input (exit 2). `--write` refuses `-`, writes only a valid, differing canonical form, via a sibling temp file, permission copy and `rename`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-02-SUMMARY.md

---

### Registration
Registration: six catalog rows, regenerated command registry / capability registry / model manifest / capability index / surface registry (one `cli_group: diagram`, tier `none`, numstat `8 0`) / coverage / test inventory / TS, a `### vox diagram` section, index and reachability rows in `cli.md`, and five new paths in the baseline. `crates/vox-cli/Cargo.toml` is byte-unchanged; `vox ci crate-edges` reports no new edge.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-02-SUMMARY.md

---

### Page example fences are the card's examples with the inline // comments ...
Page example fences are the card's examples with the inline // comments dropped (the card keeps them as teaching text); both check clean

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-03-SUMMARY.md

---

### Gantt statement kept as 'VoxDiagram has no Gantt kind, so Gantt charts s...
Gantt statement kept as 'VoxDiagram has no Gantt kind, so Gantt charts stay Mermaid' per plan; no mermaid fence added

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 88-03-SUMMARY.md

---

### Task 2: page sections (vox_validate_diagram input/output tables and repa...
Task 2: page sections (`vox_validate_diagram` input/output tables and repair loop, CLI table and exit codes, Mermaid import, legacy-Mermaid policy), `AGENTS.md` pure insertion (7 added, 0 removed), one cli.md sentence linking the page, two where-things-live rows.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-03-SUMMARY.md

---

### Task 3: regenerated via the ssot-regen.vox generator list plus crate-api...
Task 3: regenerated via the `ssot-regen.vox` generator list plus `crate-api-sync --write`; only `contracts/reports/test-inventory.v1.json` (new tests) and `docs/src/api/vox-diagram.generated.md` (card doc comment) changed, both committed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 88-03-SUMMARY.md

## Lessons

### 1. [Rule 3 - Blocking] Generator invocation without PATH override. The s...
**1. [Rule 3 - Blocking] Generator invocation without PATH override.** The shell policy blocks `PATH=... vox run scripts/ssot-regen.vox` (exit 126, as in plan 88-01). Ran the identical generator list from `scripts/ssot-regen.vox` via `target/debug/vox ci ...` (script in the session scratchpad), after rebuilding `vox-cli` because the freshness guard keys on commit count. No freshness bypass.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 88-03-SUMMARY.md

---

### 2. [Scope] GUI typecheck deferred. crates/vox-gui/ui/node_modules is abs...
**2. [Scope] GUI typecheck deferred.** `crates/vox-gui/ui/node_modules` is absent, so `pnpm typecheck` was not run; no UI source changed in this plan (generated data unchanged this run) and the hosted `ui` job covers it.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 88-03-SUMMARY.md

---

### 3. profile-parity not green (pre-existing, see above); every other named...
**3. profile-parity not green** (pre-existing, see above); every other named gate passes.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 88-03-SUMMARY.md

---

### Page example fences drop the card's inline // comments; otherwise the pl...
Page example fences drop the card's inline `//` comments; otherwise the plan was executed as written.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 88-03-SUMMARY.md

---

### No gaps. All four ROADMAP success criteria hold on the committed tree, v...
No gaps. All four ROADMAP success criteria hold on the committed tree, verified against code and by running the built `vox` binary and the targeted test suites. The two red items above predate the phase and are recorded in `deferred-items.md`.

**Context:** Verified in phase acceptance checks.
**Source:** 88-VERIFICATION.md

---

### _Verified: 2026-10-17_ _Verifier: Claude (gsd-verifier)_
_Verified: 2026-10-17_ _Verifier: Claude (gsd-verifier)_

**Context:** Verified in phase acceptance checks.
**Source:** 88-VERIFICATION.md

## Patterns

### Render failure is surfaced as emit_error, never dropped
Render failure is surfaced as emit_error, never dropped

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 88-01-SUMMARY.md

---

### Pure envelope in the domain crate, thin MCP handler
Pure envelope in the domain crate, thin MCP handler: serde-parsed Args with deny_unknown_fields, CPU work in spawn_blocking

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 88-01-SUMMARY.md

---

### Advertised description derived at runtime from one constant (advertised_...
Advertised description derived at runtime from one constant (advertised_description) so the card is never copied

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 88-01-SUMMARY.md

---

### Oversized input is read with take(MAX_SOURCE_BYTES + 1) and decoded loss...
Oversized input is read with take(MAX_SOURCE_BYTES + 1) and decoded lossily so the core's limit-source diagnostic is reported, not a UTF-8 error

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 88-02-SUMMARY.md

---

### One execute(cmd, json, stdin, out, err) -> i32 over injected streams; er...
One execute(cmd, json, stdin, out, err) -> i32 over injected streams; errors map to exit 2 in one place

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 88-02-SUMMARY.md

---

### In-place rewrite through sibling temp file + rename; invalid source neve...
In-place rewrite through sibling temp file + rename; invalid source never written

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 88-02-SUMMARY.md

## Surprises

### Estimate variance on 88-01
Estimate variance on 88-01: planned 95000 tokens vs 6400 actual (0.07x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 88-01-PLAN.md, 88-01-SUMMARY.md

---

### Estimate variance on 88-02
Estimate variance on 88-02: planned 85000 tokens vs 7000 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 88-02-PLAN.md, 88-02-SUMMARY.md

---

### Estimate variance on 88-03
Estimate variance on 88-03: planned 70000 tokens vs 3600 actual (0.05x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 88-03-PLAN.md, 88-03-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Generator invocation without PATH override. The s...
**1. [Rule 3 - Blocking] Generator invocation without PATH override.** The shell policy blocks `PATH=... vox run scripts/ssot-regen.vox` (exit 126, as in plan 88-01). Ran the identical generator list from `scripts/ssot-regen.vox` via `target/debug/vox ci ...` (script in the session scratchpad), after rebuilding `vox-cli` because the freshness guard keys on commit count. No freshness bypass.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 88-03-SUMMARY.md

