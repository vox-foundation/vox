---
phase: 84-mermaid-emitter-strict-importer
verified: 2026-10-16T00:00:00Z
status: passed
score: 3/3 must-haves verified
covered_files:
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-03-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-04-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-04-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-05-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-05-SUMMARY.md"
  - "crates/vox-diagram/src/mermaid/escape.rs"
  - "crates/vox-diagram/src/mermaid/export.rs"
  - "crates/vox-diagram/src/mermaid/export_seq.rs"
  - "crates/vox-diagram/src/mermaid/import/build.rs"
  - "crates/vox-diagram/src/mermaid/import/flow.rs"
  - "crates/vox-diagram/src/mermaid/import/ids.rs"
  - "crates/vox-diagram/src/mermaid/import/lines.rs"
  - "crates/vox-diagram/src/mermaid/import/mod.rs"
  - "crates/vox-diagram/src/mermaid/import/seq.rs"
  - "crates/vox-diagram/src/mermaid/import/state.rs"
  - "crates/vox-diagram/src/mermaid/import/text.rs"
  - "crates/vox-diagram/src/mermaid/mod.rs"
  - "crates/vox-diagram/tests/fixtures/corpus-roundtrip/unsupported.json"
  - "crates/vox-diagram/tests/fixtures/corpus/MANIFEST.json"
  - "crates/vox-diagram/tests/mermaid_corpus.rs"
  - "crates/vox-diagram/tests/mermaid_fixpoint.rs"
  - "docs-astro/tests/lib/mermaid-oracle.mjs"
  - "docs-astro/tests/unit/mermaid-oracle.test.mjs"
  - "docs-astro/tests/unit/workflow-paths.test.mjs"
covered_digest: "v3:sha256:8196fa0fb384d6be6bcc88ed73fbdc245b78d25e371c2f579fd6d0fd13e365df"
behavior_unverified: 0
overrides_applied: 0
advisory:
  - finding: "Review-fix iteration 3 (7 warnings, commits da472c0af..3231baf0a) was not followed by a fresh code review; the fixer marks every logic change 'requires human verification'"
    category: other
    reason: "Behaviour is covered by new tests and a 54,000-block differential run per the fix report, but no independent reviewer has read the iteration-3 diff"
    evidence_status: "tests green (340 pass); differential-fuzz claim not re-run by verifier"
  - finding: "Open Info findings IN-01..IN-04 from 84-REVIEW.md (oracle node identity ignores ids; render-only Mermaid semantics fa:/$$/maxTextSize outside the parser oracle; freeze_corpus swallows IO errors; direction predicate is a documented-as-superset)"
    category: other
    reason: "Out of critical_warning fix scope; none contradicts a success criterion"
    evidence_status: "none provided"
---

# Phase 84: Mermaid Emitter & Strict Importer Verification Report

**Phase Goal:** Any diagram can be exported to Mermaid that real Mermaid parses, and a maintainer can import existing Mermaid blocks without anything being silently lost.
**Verified:** 2026-10-16
**Status:** passed
**Re-verification:** No - initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Mermaid exported from any graph or sequence IR (hostile labels/ids such as `end`, `o`, `x`, quotes, `<br>`) is accepted by a pinned real Mermaid parser (sole oracle) | VERIFIED | `docs-astro/package.json` pins `mermaid 11.17.2` and `jsdom 30.0.1` exactly. The oracle (`docs-astro/tests/lib/mermaid-oracle.mjs`) imports only `jsdom` and the real Mermaid package, with no Rust output and no `mermaid.rs`. Node suite passes 82/82. `export-hostile` has 21 fixtures (g00-g16 graph: keyword ids incl. `end`/`End`/`graph`/`subgraph`/`style`/`classDef`/`click`, edge-letter ids, quotes/entities, HTML/`<br>`, injection, direction text; s00-s03 sequence). Each asserts `accepts(exported) === true` and canonical sets equal the IR's. Rust side: `mermaid_export*`, `mermaid_export_props` (totality proptest) green. Mutation proofs are SUMMARY claims, not re-run by the verifier. |
| 2 | Import of flowchart/graph/state/sequence block yields a diagram or fails closed with a by-line `unsupported` list; documented leniencies work, nothing else dropped unlisted | VERIFIED | `src/mermaid/import/{flow,state,seq,lines,text,ids,build}.rs` are substantive. Tests `mermaid_import`, `_state`, `_seq`, `_limits`, `_props` (totality and ImportReport invariant), `_traps`, `_words`, `_review`, `mermaid_unsupported`, `mermaid_catalog` all pass (340 pass / 1 skipped). `unsupported.json` shows gantt reports `header:gantt` at line 1 and the markdown-string block reports `markdown-string` on lines 2, 6, 7. `drop_styling` lists dropped lines. |
| 3 | Over the frozen live corpus, import then export gives node/edge-set equality vs the original, with the original's sets from the pinned parser; gantt and markdown-string block reported unsupported, not mangled | VERIFIED | Live count at HEAD (non-archive `docs/src`): 33 fences / 21 files, matching `corpus/` (33 `.mmd` + `MANIFEST.json`). `corpus-roundtrip/` has 31 x (`.vdg`, `.export.mmd`, `.sets.json`) = 93 files, plus `unsupported.json` (2 entries). The Node test per block runs `threeWay(original, export, sets)`, with C(B) from real Mermaid on the original, C(E) on the export, R from Rust's imported IR. The gantt block is accepted by Mermaid yet refused. The markdown-string block is rejected by Mermaid and refused. Each stem has `sets.json` XOR an `unsupported.json` entry, and listed blocks have no export. All 33 stems pass in the Node run. |

**Score:** 3/3 truths verified (0 behavior-unverified)

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-diagram/src/mermaid/{mod,escape,export,export_seq}.rs` | VERIFIED | Present, substantive, used by tests |
| `crates/vox-diagram/src/mermaid/import/*.rs` (8 files) | VERIFIED | Substantive; wired through `import::import` |
| `docs-astro/tests/lib/mermaid-oracle.mjs`, `tests/unit/mermaid-oracle.test.mjs` | VERIFIED | Run green; existence tests fail the suite if a fixture family goes missing |
| `crates/vox-diagram/tests/fixtures/{corpus,corpus-roundtrip,export-hostile,import-cases*}` | VERIFIED | Present at the stated counts |
| `docs/src/architecture/where-things-live.md` Mermaid row | VERIFIED | Line 235 |
| docs-quality.yml path filter for fixtures | VERIFIED | `workflow-paths.test.mjs` "runs the Mermaid oracle when vox-diagram fixtures change" passes |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| Rust `export` goldens | Node oracle (`threeWay`) | WIRED | Committed fixtures are consumed by `node --test`; Rust goldens pin the same files |
| `docs-quality.yml` push/pull_request paths | oracle suite | WIRED | Asserted by `workflow-paths.test.mjs` |

### Behavioral Spot-Checks (run by verifier)

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Full crate suite | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 340 passed, 1 skipped | PASS |
| Lints | `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean | PASS |
| Oracle + workflow paths | `node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs` | 82 pass / 0 fail | PASS |

### Probe Execution

No phase-declared probes. Step skipped.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| REND-06 | 84-01, 84-02, 84-05 | Export accepted by pinned real Mermaid parser | SATISFIED | Truth 1; marked Complete in REQUIREMENTS.md |
| MIG-01 | 84-01, 84-03, 84-04, 84-05 | Import flowchart/graph/state/sequence; fail closed with by-line `unsupported` | SATISFIED | Truth 2 |
| MIG-02 | 84-01, 84-05 | Node/edge-set equality via import -> export round trip over corpus | SATISFIED | Truth 3 |

All three phase requirement IDs appear in plan frontmatter and in REQUIREMENTS.md. No orphans: the traceability table maps only these three to Phase 84. MIG-04 belongs to Phase 90 and is correctly unchecked.

### Anti-Patterns Found

None blocking. Clippy is clean with `-D warnings`. No `Cargo.toml`, `Cargo.lock`, `layers.toml` or `contracts/ci` change inside the phase. The only manifest change is the additive `docs-astro` devDependencies and lockfile in `54ba24560` (plan 84-01), so no new crate edge.

### Not Run / Honest Limits

- Hosted CI (`vox-arch-check`, `vox ci crate-edges`, `ssot-drift`, `crate-api-sync`, rustdoc `-D warnings`, the Windows compile check) has not run. Nothing is pushed. Per the user's 2026-10-16 acceptance of local evidence, these are deferred to the milestone PR.
- Cross-platform (Linux) behaviour was not exercised; no SVG goldens exist in this phase.
- The differential-fuzz counts in 84-REVIEW-FIX.md (about 54,000 blocks and 45,500 ids) and the mutation proofs in the SUMMARYs were not reproduced by the verifier.

### Advisory (not blocking)

| # | Finding | Why Advisory |
|---|---------|--------------|
| 1 | Review-fix iteration 3 has no follow-up review | New importer and exporter logic (position-aware reserved words, `drop_styling` state declarations, `_self`/`_blank`/`_parent`/`_top` ids) is covered by tests, but no second reader has seen it. The fixer flagged these logic changes for human verification. |
| 2 | Open Info findings IN-01..IN-04 | The oracle keys nodes without ids, so two same-label nodes are indistinguishable. `fa:`/`$$`/50k-char limit are render-only and not parser-detectable. `freeze_corpus` swallows IO errors. The direction trap is a documented case-insensitive superset. None contradicts the three success criteria. |

### Human Verification Required

None required for the phase goal.

### Gaps Summary

No gaps. All three success criteria are backed by behavioural evidence: the real pinned Mermaid parser accepts the exports, the importer fails closed with by-line constructs, and the corpus round trip matches the independent oracle sets (31 round-tripped, 2 reported unsupported). I recommend a quick re-review of the iteration-3 diff before the milestone PR.

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
