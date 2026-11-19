---
phase: 86
phase_name: "sequence-kind"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 29
  lessons: 8
  patterns: 14
  surprises: 5
missing_artifacts:
  - "UAT.md"
---

# Phase 86 Learnings: sequence-kind

## Decisions

### actor, reply, note and over are contextual, not reserved
actor, reply, note and over are contextual, not reserved: `actor <id>` declares, `reply <id> ->` is a dashed message, `note over` starts a note, and the same words before `->` are ordinary actor ids. Mermaid's parser accepts `participant note` and the importer case 12-seq-words declares Note/loop, so reserving them would refuse blocks Mermaid reads. The graph RESERVED list stays the only reserved list (reserved-word for actor ids is 86-02).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-01-SUMMARY.md

---

### Stats for a sequence
Stats for a sequence: nodes = actors, edges = events (messages plus notes), containers and max_depth 0; documented on the Stats fields.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-01-SUMMARY.md

---

### Sequence geometry runs in i64 and is narrowed to Half once, after a 100 ...
Sequence geometry runs in i64 and is narrowed to Half once, after a 100 000 px canvas check, so a hand-built IR with huge labels returns RenderError::Geometry instead of overflowing.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-01-SUMMARY.md

---

### Lifeline bottoms are part of the canvas bounds, so nothing drawn can lie...
Lifeline bottoms are part of the canvas bounds, so nothing drawn can lie outside the viewBox.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-01-SUMMARY.md

---

### Tracer (Task 1)
Tracer (Task 1): two actors and one message pass through every stage. `check` dispatches on `Document.kind`, `fmt` prints the canonical form, `mermaid::export` equals `export_sequence`, and `svg::render` draws lifelines, actor boxes, an arrow with its marker and a label whose background box lies strictly between the lifelines. 20 renders give one string.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-01-SUMMARY.md

---

### Width-aware lanes
Width-aware lanes: each gap is at least the widest crossing label box plus 8 px each side (a 70-character label from lane 0 to lane 2 widens both gaps).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-01-SUMMARY.md

---

### Every SeqEvent shape renders
Every `SeqEvent` shape renders: solid and dashed messages, self-message loops on the right, notes over one or two actors. A hand-built IR naming an unknown actor is `RenderError::InvalidIr`, never a panic.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-01-SUMMARY.md

---

### Expansion (Task 2)
Expansion (Task 2): `reply`, self-messages and `note over a` / `note over a, b` parse, format canonically and round trip through the canonical fixture `sequence_all_constructs.vdg` (fmt fixed point, export has `-->>` and `Note over p0,p2`, render passes the checker with 2 notes).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-01-SUMMARY.md

---

### diagram/undeclared-actor (30 codes), duplicate-id with a related span; t...
`diagram/undeclared-actor` (30 codes), `duplicate-id` with a related span; the IR is withheld on either.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-01-SUMMARY.md

---

### Insertion point for a declared actor
Insertion point for a declared actor: after the line of the last `actor` statement, so new lanes follow declared ones in first-mention order; `insert_decl` (first statement) when the block has no actor or its `}` shares that line. Same-offset inserts keep diagnostic order, which is source order.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-02-SUMMARY.md

---

### > (headless dashed) rewrites to reply a -> b, like -->>
`-->` (headless dashed) rewrites to `reply a -> b`, like `-->>`: the DSL has no other dashed form.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-02-SUMMARY.md

---

### participant with a tail that is neither as nor the end of the statement ...
`participant` with a tail that is neither `as` nor the end of the statement is an unexpected-token error (actor still declared), not a Mermaid rewrite.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-02-SUMMARY.md

---

### A lexically bad id (BadIdent) gets the mermaid-syntax diagnostic without...
A lexically bad id (`BadIdent`) gets the mermaid-syntax diagnostic without a fix; the lexer already reported it.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-02-SUMMARY.md

---

### The sequence printer reuses the graph Printer (visibility change only) r...
The sequence printer reuses the graph `Printer` (visibility change only) rather than copying 70 lines of die-stream helpers.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-02-SUMMARY.md

---

### Task 1 (tracer)
Task 1 (tracer): `diagram/undeclared-actor` carries a declare fix (`actor <id>: "<id>"`), once per id, after the last actor line; `repair` fixes `a -> b / b -> c / c -> b` in one round with lanes `a, b, c`. Actor ids in `RESERVED` give `diagram/reserved-word` (no fix). Cap messages say "actors" / "events" for sequences. `fixgen::insert_after_line` added (LF, CRLF, trailing comment kept, out of range `None`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-02-SUMMARY.md

---

### Task 2: sequenceDiagram header (rewrite to sequence { plus the EOF }), p...
Task 2: `sequenceDiagram` header (rewrite to `sequence {` plus the EOF `}`), `participant X [as Y]` / `actor X [as Y]`, `->>`, `-->>`, `-->`, `Note`/`NOTE OVER`, unquoted text; `--x`, `==>`, `-.->`, `-->|t|` and `Note right of` are reported without a fix and the statement still parses. `repair` of the corpus block `19-scientia-...-l112.mmd` is valid, stable and equals the importer's IR.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-02-SUMMARY.md

---

### Task 3: sequence_props (128 cases each): S1 model oracle, S2 fmt idempot...
Task 3: `sequence_props` (128 cases each): S1 model oracle, S2 fmt idempotence/canonical shape, S3 AST and IR preserved by fmt, S4 five Mermaid corruptions (plus brace deletion fallback) repair back to the original IR; `every_corruption_kind_applies_and_repairs` proves each kind applies.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-02-SUMMARY.md

---

### Note text soft-wraps at the same 60 cells as message text (Claude's disc...
Note text soft-wraps at the same 60 cells as message text (Claude's discretion; the context only fixed the message rule). Actor labels keep the 28-cell graph node wrap.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-03-SUMMARY.md

---

### The oracle does not forbid an intermediate lifeline passing behind a lab...
The oracle does not forbid an intermediate lifeline passing behind a label box that crosses it (the label background is filled); it requires clearance only from the message's own two lifelines, as the plan specifies.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-03-SUMMARY.md

---

### REND-05 is not ticked
REND-05 is not ticked: SC3 (the corpus block renders, golden plus oracle) and the by-eye review of the sequence SVGs are still open in 86-04.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-03-SUMMARY.md

---

### Task 2: self-message gap rule: gap i is also at least SELF_W + SELF_LABE...
Task 2: self-message gap rule: gap `i` is also at least `SELF_W + SELF_LABEL_GAP + label width + LANE_CLEAR` for a labelled self-message on lane `i`; on the last lane the canvas grows instead. `generated_sequences_never_overlap_or_clip` (128 cases, 1-6 actors, 0-14 events, labels to 130 characters with CJK, emoji and newlines) passes, asserting all 128 drew. Goldens `31-seq-self-and-notes`, `32-seq-replies-and-wide-actors`, `33-seq-unicode`; `sequence_goldens_pass_the_oracle` also runs the oracle over every sequence case in the directory.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-03-SUMMARY.md

---

### Task 3: limits::MAX_RENDER_ACTORS = 100 / MAX_RENDER_EVENTS = 200; layou...
Task 3: `limits::MAX_RENDER_ACTORS = 100` / `MAX_RENDER_EVENTS = 200`; `layout::seq::validate` runs before any placement in the plan's order (caps, duplicate actor id, label length through `check_label` (now `pub(crate)`), note arity, then 86-01's unknown actor in `steps`). `RenderError::TooLarge` docs and the `svg::render` doc cover sequences. Hostile text in all four slots (actor label, message, note, title) equals the sanitized, whitespace-squeezed fragment; a 64-case property over arbitrary text is accepted by the checker; no user text in any attribute; 20 fresh renders are identical for `sequence_all_constructs.vdg` and the four goldens; 64 generated sequences render to equal bytes twice; `sequence_geometry_is_integer_only` guards `layout/seq.rs`, `layout/seq/canvas.rs`, `layout/seq/tests.rs` and `svg/seq.rs` (float needles built at run time).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-03-SUMMARY.md

---

### ImportReport.sequence is kept (planned decision)
ImportReport.sequence is kept (planned decision). It carries spans located in the Mermaid block; the IR of `source` has spans into the DSL text. Invariant now: source.is_some() == unsupported.is_empty() for every kind; sequence.is_some() exactly for an imported sequenceDiagram block. Documented on the struct.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-04-SUMMARY.md

---

### An empty Note text (Note over A:) is now refused as empty-label
An empty Note text (`Note over A:`) is now refused as `empty-label`: the DSL rejects `note over a: ""`, so it cannot be represented. Reuses the existing construct name, no catalogue change.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-04-SUMMARY.md

---

### REND-05 ticked
REND-05 ticked: width-aware lanes were delivered and verified in 86-03 (oracle, goldens 30-33); this plan closes SC3 by rendering the corpus block (golden plus geometry oracle). The by-eye browser review is evidence only and is pending (see below).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 86-04-SUMMARY.md

---

### print::sequence_source prints actors, one blank line, then messages (rep...
`print::sequence_source` prints actors, one blank line, then messages (`reply ` for dashed, `: "text"` only when non-empty) and `note over a[, b]`, all labels via `push_quoted`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-04-SUMMARY.md

---

### import/mod.rs::formatted(dsl) extracted from finish (graph behaviour unc...
`import/mod.rs::formatted(dsl)` extracted from `finish` (graph behaviour unchanged); the sequence path uses it, so an in-limit Mermaid block whose DSL outgrows `MAX_SOURCE_BYTES` is refused with exactly one `limit-source` at line 1 (unit test with 312 messages of 200 chars), never a partial report.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-04-SUMMARY.md

---

### Tracer tracer_corpus_sequence_round_trips
Tracer `tracer_corpus_sequence_round_trips`: source is a `fmt` fixed point, re-checks to the same IR (spans stripped; 8 actors, 8 messages + 1 note), renders checker-clean, passes `assert_sequence_geometry`, equals the new golden, and `export` is byte-equal to the committed `corpus-roundtrip/19-....export.mmd`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-04-SUMMARY.md

---

### Every import test, the import property (unsupported.is_empty() == source...
Every import test, the import property (`unsupported.is_empty() == source.is_some()`), the corpus accounting, `mermaid_roundtrip` (31 corpus `.vdg`, two import-case `.vdg`) and the F2 fixpoint now check the DSL round trip. Exactly three new `.vdg` files appeared; no `.export.mmd` / `.sets.json` golden changed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 86-04-SUMMARY.md

## Lessons

### 1. [Rule 1 - Bug] Empty Note over A: imported but could not print as DSL
**1. [Rule 1 - Bug] Empty `Note over A:` imported but could not print as DSL**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 86-04-SUMMARY.md

---

### Found during
**Found during:** Task 1 (`seq_text_rules` failed with `internal`, the fail-closed fallback, once `source` was filled).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 86-04-SUMMARY.md

---

### Issue: the DSL checker reports diagram/empty-label for note over a: "", ...
**Issue:** the DSL checker reports `diagram/empty-label` for `note over a: ""`, so an accepted block with an empty note produced DSL that `fmt` refused.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 86-04-SUMMARY.md

---

### Fix: import/seq.rs::note refuses an empty note text with the existing co...
**Fix:** `import/seq.rs::note` refuses an empty note text with the existing construct `empty-label`; the assertion in `seq_text_rules` changed from "imports with empty text" to "refused as empty-label". Fail-closed behaviour stays as the second line of defence.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 86-04-SUMMARY.md

---

### None. IN-01..IN-04 are out of scope (fix_scope: critical_warning).
None. IN-01..IN-04 are out of scope (`fix_scope: critical_warning`).

**Context:** Surfaced in structured phase code review.
**Source:** 86-REVIEW-FIX.md

---

### _Fixed: 2026-10-17_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_
_Fixed: 2026-10-17_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_

**Context:** Surfaced in structured phase code review.
**Source:** 86-REVIEW-FIX.md

---

### By-eye render review of the sequence contact sheet
**By-eye render review of the sequence contact sheet**

**Context:** Verified in phase acceptance checks.
**Source:** 86-VERIFICATION.md

---

### Test: Open 86-CONTACT-SHEET.html in a real browser, light and dark panes...
**Test:** Open `86-CONTACT-SHEET.html` in a real browser, light and dark panes.

**Context:** Verified in phase acceptance checks.
**Source:** 86-VERIFICATION.md

## Patterns

### Parser.seq selects sequence_statement() in statement(); the graph arm mo...
Parser.seq selects sequence_statement() in statement(); the graph arm moved into graph_statement().

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-01-SUMMARY.md

---

### Owner::Note: an empty note text is reported with no fix.
Owner::Note: an empty note text is reported with no fix.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-01-SUMMARY.md

---

### Mutation proof on the gap rule
Mutation proof on the gap rule: deleting the label term fails three tests.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-01-SUMMARY.md

---

### contextual keywords
contextual keywords: actor/reply/note/over are plain identifiers unless the next token says otherwise

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-01-SUMMARY.md

---

### sequence geometry in i64 half-pixels, narrowed once in a canvas stage th...
sequence geometry in i64 half-pixels, narrowed once in a canvas stage that fails closed past 100 000 px

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-01-SUMMARY.md

---

### one SVG preamble (open_document) shared by graph and sequence
one SVG preamble (open_document) shared by graph and sequence

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-01-SUMMARY.md

---

### Mutation proof
Mutation proof: copy, mutate with a grep presence check, named test fails, restore with diff -q.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-02-SUMMARY.md

---

### sequence declare fix mirrors graph Rule R
sequence declare fix mirrors graph Rule R: insert_after_line for a multi-line scope with actors, insert_decl otherwise

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-02-SUMMARY.md

---

### Mermaid recognizers are Parser methods in one file, called from sequence...
Mermaid recognizers are Parser methods in one file, called from sequence_statement; every scan spends fuel and consumes what it scans

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-02-SUMMARY.md

---

### sequence oracle reads only svgcheck's read-back plus IR order (actors to...
sequence oracle reads only svgcheck's read-back plus IR order (actors to nodes/lifelines, messages to edge_paths, non-blank messages to edge_labels, notes to notes)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-03-SUMMARY.md

---

### validate() returns the id-to-lane map, so place() resolves lanes once an...
validate() returns the id-to-lane map, so place() resolves lanes once and validation order is one function

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-03-SUMMARY.md

---

### Fail closed on printed DSL
Fail closed on printed DSL: the IR is reported only together with DSL that fmt accepts.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-04-SUMMARY.md

---

### one shared formatted(dsl) refusal path (limit-source / internal) for gra...
one shared formatted(dsl) refusal path (limit-source / internal) for graph and sequence imports

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-04-SUMMARY.md

---

### every import test strips spans from both the imported IR and the re-chec...
every import test strips spans from both the imported IR and the re-checked DSL IR before comparing

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 86-04-SUMMARY.md

## Surprises

### Estimate variance on 86-01
Estimate variance on 86-01: planned 130000 tokens vs 22300 actual (0.17x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 86-01-PLAN.md, 86-01-SUMMARY.md

---

### Estimate variance on 86-02
Estimate variance on 86-02: planned 110000 tokens vs 15400 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 86-02-PLAN.md, 86-02-SUMMARY.md

---

### Estimate variance on 86-03
Estimate variance on 86-03: planned 115000 tokens vs 14100 actual (0.12x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 86-03-PLAN.md, 86-03-SUMMARY.md

---

### Estimate variance on 86-04
Estimate variance on 86-04: planned 90000 tokens vs 10000 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 86-04-PLAN.md, 86-04-SUMMARY.md

---

### 1. [Rule 1 - Bug] Empty Note over A: imported but could not print as DSL
**1. [Rule 1 - Bug] Empty `Note over A:` imported but could not print as DSL**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 86-04-SUMMARY.md

