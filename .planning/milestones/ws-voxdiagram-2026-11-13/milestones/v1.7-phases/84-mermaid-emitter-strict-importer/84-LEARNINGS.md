---
phase: 84
phase_name: "mermaid-emitter-strict-importer"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 34
  lessons: 21
  patterns: 4
  surprises: 10
missing_artifacts:
  - "UAT.md"
---

# Phase 84 Learnings: mermaid-emitter-strict-importer

## Decisions

### Unit tests live inline in each new src file (all four are well under 400...
Unit tests live inline in each new src file (all four are well under 400 lines), not in sibling tests.rs files

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-01-SUMMARY.md

---

### Importer id grammar rejects ids ending in '-' so a--->b fails closed ins...
Importer id grammar rejects ids ending in '-' so `a--->b` fails closed instead of reading as `a-` --> `b`

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-01-SUMMARY.md

---

### Dangling edge endpoints are exported as implicit Mermaid nodes (bare id,...
Dangling edge endpoints are exported as implicit Mermaid nodes (bare id, `n<i>` when unsafe), as the plan specifies; containers are 84-02's slice

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-01-SUMMARY.md

---

### Task 1 (tracer), 54ba24560
**Task 1 (tracer), `54ba24560`.** `vox_diagram::mermaid` with the final public types, a flat-graph `export`, the tracer `import` (header, `<id> --> <id>` lines, fail-closed `unsupported` with block-relative lines, `limit-source` gate), and `escape.rs` with every RESEARCH encoding row and `keeps_id`. `print::push_quoted` extracted from `label()` with no behaviour change. Test support `roundtrip.rs` builds canonical sets from the IR only. Node helper + test drive the three-way assertions against the committed `00-minimal` fixtures. `mermaid`/`jsdom` added as exact devDependencies with pnpm 11.28.6.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Task 2 (oracle breadth + CI), ca7bdb8d9
**Task 2 (oracle breadth + CI), `ca7bdb8d9`.** `canonState` (`[*]` to Start/End; composite/note/other states throw) and `canonSeq` (actors by index, solid/dashed messages, `over` notes; anything else throws). Fixture loops for `import-cases-roundtrip`, `corpus-roundtrip` (accounting against `unsupported.json`, `markdown-string` must be rejected by Mermaid) and `export-hostile`. `docs-quality.yml` path filter on both `push` and `pull_request`, guarded by a `workflow-paths` test.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Task 3 (corpus), 12561ffb6
**Task 3 (corpus), `12561ffb6`.** `freeze_corpus` (`#[ignore]`, the only code reading `docs/`) writes 33 fixtures and `MANIFEST.json` from tracked `docs/src` files via `fence::extract`, skipping `docs/src/archive/`. `corpus_manifest_matches_fixtures` enforces 33 fixtures, 21 sources, no archive, header census 30/1/1/1. Block 10 is the gantt, 19 the sequence, 29 the how-to-rust-crate-imports line 53 block, 32 the populi state diagram (matches the RESEARCH inventory). A second freezer run left `git status` on the corpus directory empty.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Ids are assigned nodes first, then containers, then dangling endpoints, ...
Ids are assigned nodes first, then containers, then dangling endpoints, all through one `Ids` so Mermaid's single namespace holds across subgraphs

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-02-SUMMARY.md

---

### Blank message text prints a bare p0->>p1:; blank actor label and note te...
Blank message text prints a bare `p0->>p1:`; blank actor label and note text print `#32;` (the real parser accepted both, no fallback needed)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-02-SUMMARY.md

---

### A note with empty over exports Note over
A note with empty `over` exports `Note over : text` (degenerate, documented outside the acceptance guarantee)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-02-SUMMARY.md

---

### The hostile case table lives in tests/support/hostile.rs (shared with th...
The hostile case table lives in `tests/support/hostile.rs` (shared with the property generators) to keep every file under 400 non-blank lines

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-02-SUMMARY.md

---

### Task 1 (tracer), 52f67a214
**Task 1 (tracer), `52f67a214`.** `export.rs` builds a `Tree` (members and children per container position, top-level lists) and emits with an explicit `Step::{Open, Close}` stack, 4 spaces per depth, edges last. A node naming no container is top level; a container nests only under an EARLIER container (cycle-free by construction); a repeated container id resolves to its first occurrence; edges to container ids and dangling endpoints are handled. `g00-nested` (4-deep, empty container, self-loop, sibling containers sharing a local id, depth-1 to depth-4 edges) is accepted by Mermaid with equal sets, empty `subgraph` included.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-02-SUMMARY.md

---

### Task 2, 621a74ee2
**Task 2, `621a74ee2`.** 15 hostile graph cases `g01..g15` (keyword ids, edge-letter ids, `n<k>` collisions, quotes and entities, HTML, backticks, colon and percent guards, punctuation, whitespace, Unicode placeholders, long labels, statement injection, all six shapes, edge labels incl. empty and blank). `mermaid_export_props.rs` (256 cases each): line count `1 + nodes + 2 x containers + edges`, per-line classification with payload safety (no raw `"`, `<`, `>`, `&`, backtick or control character apart from the `<br>` newline encoding, every `#` opens an entity), unique safe ids in one namespace, and the same over `model_graph()` + `noisy_source()` + `check()` with a byte-identical second export. Unit tests cover degenerate IRs and a 500-deep container chain.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-02-SUMMARY.md

---

### Task 3, a74b9b0d4
**Task 3, `a74b9b0d4`.** `export_sequence` (opaque `p<i>` ids, dangling ids appended in first-appearance order labelled with the id, notes capped at two ids), `escape::seq_text` on a shared `encode(s, seq)`, `canon_sequence` in the test support, goldens `s00..s03`, and a sequence property.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-02-SUMMARY.md

---

### Semantic pass lives in its own import/build.rs (not in the plan's file l...
Semantic pass lives in its own `import/build.rs` (not in the plan's file list) so every file stays under 400 non-blank lines

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-03-SUMMARY.md

---

### A refused subgraph header still opens a frame (Stmt::BadOpen), so its en...
A refused `subgraph` header still opens a frame (`Stmt::BadOpen`), so its `end` does not become a spurious `stray-end`; an unclosed refused header also reports `unclosed-subgraph`

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-03-SUMMARY.md

---

### Unquoted edge text has its own special-character set per form
Unquoted edge text has its own special-character set per form: `-->|t|` rejects `( ) [ ] { } | " @`, `-- t -->` rejects only `"` (measured with the pinned Mermaid; corpus block 11 has `-- Yes (Iter < Max) -->`)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-03-SUMMARY.md

---

### #<digits>; that is not a valid scalar value (surrogate, > U+10FFFF) is e...
`#<digits>;` that is not a valid scalar value (surrogate, > U+10FFFF) is `entity`, not literal text: a literal would render differently from Mermaid's own decoding

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-03-SUMMARY.md

---

### Limit constructs (limit-nodes, limit-edges, limit-depth) are reported on...
Limit constructs (`limit-nodes`, `limit-edges`, `limit-depth`) are reported once, on the crossing line, not on every later line

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-03-SUMMARY.md

---

### MIG-01 is NOT ticked
MIG-01 is NOT ticked: it covers flowchart, state and sequence; only flowchart/graph is delivered here

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-03-SUMMARY.md

---

### Task 1 (tracer), cebc0a6ea
**Task 1 (tracer), `cebc0a6ea`.** Four-module split. `lines.rs` scans one trimmed line into a statement or a construct name; `text.rs` owns the DSL id rule and label decoding; `build.rs` does the semantic pass (first-closed membership via a `Vec<Frame>` stack, never recursion); `flow.rs` prints scoped DSL (`to_dsl`), placing each edge in its deepest common scope with endpoints relative to it. `import_cross_scope_edge_uses_dotted_path` matches the plan's exact DSL and the IR edge is `client` to `backend.gateway`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-03-SUMMARY.md

---

### Task 2, 50619bd10
**Task 2, `50619bd10`.** All six shapes, `&` fan-out, chains, the four edge-text forms (`-->|t|`, `-->|"t"|`, `-- t -->`, `-- "t" -->`), entity and `<br>` decoding, quoted-text-is-text, subgraph forms, `direction` handling. 15 `import_*` tests. `corpus_blocks_import_or_fail_closed_as_expected`: 29 blocks import (fmt fixed point, `check().valid`); exactly 4 fail closed (gantt `header:gantt`, how-to-rust-crate-imports line 53 only `markdown-string`, populi `header:stateDiagram-v2`, scientia line 112 `header:sequenceDiagram`); block 06 (deep-research line 238) reports two `subgraph-direction` in `dropped`; no other block has any `dropped`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-03-SUMMARY.md

---

### Task 3, 0219f6439
**Task 3, `0219f6439`.** Full catalogue (31 named tests in `mermaid_unsupported.rs`), `drop_styling`, caps, 6 adversarial blocks, 2 totality properties. Fan-out is counted before it is materialised (running product, early exit at 100).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-03-SUMMARY.md

---

### A raw ; check that is entity-aware (#59; is text, any other ; is semicol...
A raw `;` check that is entity-aware (`#59;` is text, any other `;` is `semicolon`) lives in text::semicolon and is shared by state and sequence scanners

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-04-SUMMARY.md

---

### Because every HTML entity carries a raw ;, R&amp;D in state/sequence tex...
Because every HTML entity carries a raw `;`, `R&amp;D` in state/sequence text reports `semicolon` (flowchart labels still report `html-entity`); both fail closed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-04-SUMMARY.md

---

### Participant and node maps use BTreeMap
Participant and node maps use BTreeMap: determinism_guard forbids hash collections in src/

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-04-SUMMARY.md

---

### Sequence ids that are Mermaid keywords (loop, end, actor, title, ...) ar...
Sequence ids that are Mermaid keywords (loop, end, actor, title, ...) are `reserved-id`, since Mermaid cannot parse them as participants

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-04-SUMMARY.md

---

### Empty state transition text (A --> B:) imports as no label (an empty lab...
Empty state transition text (`A --> B:`) imports as no label (an empty label equals no label)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-04-SUMMARY.md

---

### Task 1 (tracer), 53e4c71c3
**Task 1 (tracer), `53e4c71c3`.** `import/state.rs` scans one line to a transition or a construct name, creates nodes at first mention (terminals included, so `end` is declared where `Leased --> [*]` first appears), builds a `flow::Graph` and prints through `flow::to_dsl` + `fmt`. `import/mod.rs` was refactored so `flowchart()` and `state()` share `finish()` for report assembly. `text::free` added. The populi.md line 211 block imports, is a `fmt` fixed point and `check()`s valid.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-04-SUMMARY.md

---

### Task 2, 3b674a5b2
**Task 2, `3b674a5b2`.** `import/seq.rs`: `participant`, `participant X as Label`, `->>`, `-->>`, `Note over X[,Y]` (any case of `note`), implicit participants in first-mention order, `participant-after-use`, `redeclared-participant`, caps, spans as `Location` over the trimmed statement byte range. Full catalogues for both kinds, `drop_styling` in both, property extension (80 fragments, `stateDiagram-v2` and `sequenceDiagram` headers), adversarial bounds. The scientia line 112 block imports with 8 actors, 8 messages and 1 note (the plan's "9 messages" counted the note; asserted exactly as `(8, 8, 1)`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-04-SUMMARY.md

---

### roundtrip_dir is shared by import_cases_roundtrip and corpus_roundtrip; ...
roundtrip_dir is shared by import_cases_roundtrip and corpus_roundtrip; corpus_roundtrip asserts 31 round-tripped, 30 .vdg, and the two refusals keyed by MANIFEST source + open_line

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-05-SUMMARY.md

---

### F2 text is drawn from DSL-legal characters (generator domain), because t...
F2 text is drawn from DSL-legal characters (generator domain), because the importer refuses forbidden characters even as numeric entities by design

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 84-05-SUMMARY.md

---

### Task 1 (tracer)
**Task 1 (tracer):** `roundtrip_dir` refactor, `corpus_roundtrip`, 31 corpus goldens plus `unsupported.json`. Node: 33 `corpus-roundtrip/` tests pass (31 three-way plus the two listed-block expectations). The markdown-string block lists three lines (2, 6, 7), all `markdown-string`; the Rust test asserts one construct name per refused block.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-05-SUMMARY.md

---

### Task 2: nine import cases (shapes, three-level nested subgraphs with a p...
**Task 2:** nine import cases (shapes, three-level nested subgraphs with a plain-titled subgraph, quoted entities, fan-out chain, membership edge cases, `flowchart BT`, multi-terminal state, sequence with implicit participants / dashed / single-actor note / empty message / alias punctuation, renamed ids). Node: 10 `import-cases-roundtrip/` tests pass. The oracle confirmed Mermaid's own "first closed scope wins" membership for a node listed in two sibling subgraphs. F1/F2 pass at 128 cases and at `PROPTEST_CASES=3000`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-05-SUMMARY.md

---

### Task 3: where-things-live row after the "Diagram DSL (VoxDiagram)" row; ...
**Task 3:** where-things-live row after the "Diagram DSL (VoxDiagram)" row; API page regenerated by `vox ci crate-api-sync --write`, then verified.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-05-SUMMARY.md

## Lessons

### [Judgement] Hostile table in tests/support/hostile.rs
**[Judgement] Hostile table in `tests/support/hostile.rs`.** The plan lists `mermaid_export.rs` for the case table; one file would exceed the 400-line rule and the property generators reuse the fragments, so the table and a shared `payload_problem` checker live in the support module (registered in `support/mod.rs`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-02-SUMMARY.md

---

### [Judgement] 500-deep chain, not deeper
**[Judgement] 500-deep chain, not deeper.** Output size grows with the square of depth (indentation); 500 proves no recursion at about 1M characters.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-02-SUMMARY.md

---

### Test-side fix, no source change
**Test-side fix, no source change.** A first draft of `export_ids_are_global` used a same-scope node and container named `a`, which the DSL itself rejects as a duplicate; rewritten with the container nested under another container.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-02-SUMMARY.md

---

### Otherwise: plan executed as written.
Otherwise: plan executed as written.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-02-SUMMARY.md

---

### [Judgement] Extra module import/build.rs
**[Judgement] Extra module `import/build.rs`.** The plan names `mod/text/lines/flow`; the semantic pass (node bookkeeping, membership, caps, clashes) would have pushed `flow.rs` past 400 lines, and the plan states the model/printer reuse boundary for 84-04 as `Graph` + `to_dsl`, which `flow.rs` still owns.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-03-SUMMARY.md

---

### [Judgement] New test file mermaid_import_limits.rs for the six adversari...
**[Judgement] New test file `mermaid_import_limits.rs`** for the six adversarial blocks, because `mermaid_unsupported.rs` already sits at 399 non-blank lines. The cap tests (`limit_*`, `expansion_cap_*`) stay in `mermaid_unsupported.rs` so the catalogue greps hold.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-03-SUMMARY.md

---

### [Rule 1 - plan text vs corpus] Unquoted -- text --> edge text allows ( )...
**[Rule 1 - plan text vs corpus] Unquoted `-- text -->` edge text allows `( ) [ ] { } | @`.** The plan applies one unquoted-text rule (`( ) [ ] { } | " @`) to node labels, titles and edge text, but corpus block 11 contains `-- Yes (Iter < Max) -->` (and the plan requires every non-markdown corpus block to import). Probed with the pinned Mermaid: the dash form rejects only `"`, the pipe form rejects the full set. Implemented per form (`NODE_SPECIAL`, `DASH_SPECIAL`); test `import_edge_text_forms` covers the parenthesised case.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-03-SUMMARY.md

---

### [Judgement] Invalid scalar numeric entities (#55296;) are entity, where ...
**[Judgement] Invalid scalar numeric entities (`#55296;`) are `entity`**, where the plan only says other `#` text stays literal.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-03-SUMMARY.md

---

### [Judgement] R&amp;D in state/sequence text reports semicolon, not html-e...
**[Judgement] `R&amp;D` in state/sequence text reports `semicolon`, not `html-entity`.** An HTML entity always contains a raw `;`, which ends a Mermaid statement, so `semicolon` is reported first. Both are fail-closed; flowchart labels (where `;` is legal inside brackets) still report `html-entity`. The `html-entity` branch of `decode` is simply unreachable from free text.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-04-SUMMARY.md

---

### [Rule 1 - Bug] Entity-aware semicolon check
**[Rule 1 - Bug] Entity-aware semicolon check.** The plan's "a raw `;` is `semicolon`" would have rejected the exporter's own `#59;` encoding. `text::semicolon` skips `;` that terminate `#name;` / `#digits;` entities, so the exporter's encoded forms import back as text (test `seq_text_rules`: `a #59; b #119;rap: #96;x#96;` decodes to ``a ; b wrap: `x` ``).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-04-SUMMARY.md

---

### [Judgement] Keyword ids are reserved-id in sequences (loop, end, actor, ...
**[Judgement] Keyword ids are `reserved-id` in sequences** (`loop`, `end`, `actor`, `title`, ...), an addition beyond the plan's list, because Mermaid cannot parse them as participants and the exporter never emits them.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-04-SUMMARY.md

---

### [Judgement] Unspaced -x is not an arrow
**[Judgement] Unspaced `-x` is not an arrow.** `A-xB: hi` is lexed as the hyphenated id `A-xB` (the DSL id rule allows inner hyphens), then `statement`. Spaced `A -x B` reports `arrow:-x`. Fail-closed either way.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-04-SUMMARY.md

---

### 1. [Rule 1 - Bug] rustdoc intra-doc links in mermaid/mod.rs did not reso...
**1. [Rule 1 - Bug] rustdoc intra-doc links in `mermaid/mod.rs` did not resolve**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-05-SUMMARY.md

---

### Found during
Found during: Task 3 closing gate.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-05-SUMMARY.md

---

### Issue: [export] / [import] in the module docs failed -D warnings ("no it...
Issue: `[`export`]` / `[`import`]` in the module docs failed `-D warnings` ("no item named export in scope") because the module and the fn share a name and the short description is re-rendered from the parent scope.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-05-SUMMARY.md

---

### Fix: [export()](crate::mermaid::export()) and the same for import.
Fix: `[`export()`](crate::mermaid::export())` and the same for `import`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 84-05-SUMMARY.md

---

### None. The Info findings (IN-01, IN-02, IN-03) were out of scope for fix_...
None. The Info findings (IN-01, IN-02, IN-03) were out of scope for `fix_scope: critical_warning`.

**Context:** Surfaced in structured phase code review.
**Source:** 84-REVIEW-FIX.md

---

### _Fixed: 2026-10-16_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_
_Fixed: 2026-10-16_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_

**Context:** Surfaced in structured phase code review.
**Source:** 84-REVIEW-FIX.md

---

### # Iteration 3
# Iteration 3

**Context:** Surfaced in structured phase code review.
**Source:** 84-REVIEW-FIX.md

---

### None required for the phase goal.
None required for the phase goal.

**Context:** Verified in phase acceptance checks.
**Source:** 84-VERIFICATION.md

---

### No gaps. All three success criteria are backed by behavioural evidence: ...
No gaps. All three success criteria are backed by behavioural evidence: the real pinned Mermaid parser accepts the exports, the importer fails closed with by-line constructs, and the corpus round trip matches the independent oracle sets (31 round-tripped, 2 reported unsupported). I recommend a quick re-review of the iteration-3 diff before the milestone PR.

**Context:** Verified in phase acceptance checks.
**Source:** 84-VERIFICATION.md

## Patterns

### family(dir, suffix, body) in the oracle test
family(dir, suffix, body) in the oracle test: a fixture directory is skipped only when absent, and fails when present but empty

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 84-01-SUMMARY.md

---

### Goldens fail on a missing file and on any byte difference; UPDATE_GOLDEN...
Goldens fail on a missing file and on any byte difference; UPDATE_GOLDEN=1 rewrites

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 84-01-SUMMARY.md

---

### Rust writes golden fixtures, Node reads them (UPDATE_GOLDEN=1 blesses)
Rust writes golden fixtures, Node reads them (UPDATE_GOLDEN=1 blesses)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 84-01-SUMMARY.md

---

### Oracle independence
Oracle independence: Rust canonical sets come from the IR alone; Node sets come from the real parser

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 84-01-SUMMARY.md

## Surprises

### Estimate variance on 84-01
Estimate variance on 84-01: planned 95000 tokens vs 13400 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 84-01-PLAN.md, 84-01-SUMMARY.md

---

### Estimate variance on 84-02
Estimate variance on 84-02: planned 90000 tokens vs 12100 actual (0.13x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 84-02-PLAN.md, 84-02-SUMMARY.md

---

### [Judgement] Hostile table in tests/support/hostile.rs
**[Judgement] Hostile table in `tests/support/hostile.rs`.** The plan lists `mermaid_export.rs` for the case table; one file would exceed the 400-line rule and the property generators reuse the fragments, so the table and a shared `payload_problem` checker live in the support module (registered in `support/mod.rs`).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 84-02-SUMMARY.md

---

### Estimate variance on 84-03
Estimate variance on 84-03: planned 105000 tokens vs 17350 actual (0.17x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 84-03-PLAN.md, 84-03-SUMMARY.md

---

### [Rule 1 - plan text vs corpus] Unquoted -- text --> edge text allows ( )...
**[Rule 1 - plan text vs corpus] Unquoted `-- text -->` edge text allows `( ) [ ] { } | @`.** The plan applies one unquoted-text rule (`( ) [ ] { } | " @`) to node labels, titles and edge text, but corpus block 11 contains `-- Yes (Iter < Max) -->` (and the plan requires every non-markdown corpus block to import). Probed with the pinned Mermaid: the dash form rejects only `"`, the pipe form rejects the full set. Implemented per form (`NODE_SPECIAL`, `DASH_SPECIAL`); test `import_edge_text_forms` covers the parenthesised case.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 84-03-SUMMARY.md

---

### Estimate variance on 84-04
Estimate variance on 84-04: planned 70000 tokens vs 11871 actual (0.17x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 84-04-PLAN.md, 84-04-SUMMARY.md

---

### [Rule 1 - Bug] Entity-aware semicolon check
**[Rule 1 - Bug] Entity-aware semicolon check.** The plan's "a raw `;` is `semicolon`" would have rejected the exporter's own `#59;` encoding. `text::semicolon` skips `;` that terminate `#name;` / `#digits;` entities, so the exporter's encoded forms import back as text (test `seq_text_rules`: `a #59; b #119;rap: #96;x#96;` decodes to ``a ; b wrap: `x` ``).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 84-04-SUMMARY.md

---

### [Judgement] Unspaced -x is not an arrow
**[Judgement] Unspaced `-x` is not an arrow.** `A-xB: hi` is lexed as the hyphenated id `A-xB` (the DSL id rule allows inner hyphens), then `statement`. Spaced `A -x B` reports `arrow:-x`. Fail-closed either way.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 84-04-SUMMARY.md

---

### Estimate variance on 84-05
Estimate variance on 84-05: planned 95000 tokens vs 4300 actual (0.05x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 84-05-PLAN.md, 84-05-SUMMARY.md

---

### 1. [Rule 1 - Bug] rustdoc intra-doc links in mermaid/mod.rs did not reso...
**1. [Rule 1 - Bug] rustdoc intra-doc links in `mermaid/mod.rs` did not resolve**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 84-05-SUMMARY.md

