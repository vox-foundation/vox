---
phase: 84-mermaid-emitter-strict-importer
reviewed: 2026-10-16T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 13
files_reviewed_list:
  - crates/vox-diagram/src/mermaid/escape.rs
  - crates/vox-diagram/src/mermaid/import/lines.rs
  - crates/vox-diagram/src/mermaid/import/mod.rs
  - crates/vox-diagram/src/mermaid/import/seq.rs
  - crates/vox-diagram/src/mermaid/import/state.rs
  - crates/vox-diagram/src/mermaid/import/text.rs
  - crates/vox-diagram/tests/mermaid_import.rs
  - crates/vox-diagram/tests/mermaid_import_review.rs
  - crates/vox-diagram/tests/mermaid_import_state.rs
  - crates/vox-diagram/tests/support/direction.rs
  - crates/vox-diagram/tests/support/hostile.rs
  - crates/vox-diagram/tests/support/mod.rs
  - docs-astro/tests/unit/mermaid-oracle.test.mjs
findings:
  critical: 0
  warning: 7
  info: 5
  total: 12
status: issues_found
---

# Phase 84: Code Review Report (iteration 2)

**Reviewed:** 2026-10-16
**Depth:** standard, plus differential probing against the pinned real Mermaid 11.17.2 parser
**Files Reviewed:** 13 (the iteration-1 fix surface; unchanged files are not re-listed)
**Status:** issues_found

## Summary

Method: scratch copies of the crate at HEAD and at the pre-fix base `2c385ace1`, built outside the
worktree (`scratchpad/iter2/`). Both importers were run side by side through
`docs-astro/tests/lib/mermaid-oracle.mjs` on the iteration-1 repros, on about 6,000 id/word/position
probes, on about 10,000 corpus-style fuzz blocks (flowchart, state, sequence), and on 120,000
garbage blocks (panic and `internal` check). Every outcome was classified as
`old -> new` x `{ok, refused, mismatch, import-but-oracle-rejects}`. The worktree was not modified.

**All three iteration-1 criticals and all five iteration-1 warnings are resolved against the real oracle.**

| Finding | Re-run result |
|---|---|
| CR-01 | The `direction LR` family (24 whitespace, case and adjacency variants) exports, is accepted by Mermaid and canonicalises back to the IR. The one variant per label slot that the DSL itself rejects (U+2028, U+0085, tab, CR) cannot reach the exporter. |
| CR-02 | `item #3`, `100#`, `end #` and `note # here` in sequence text are refused as `hash`. State and flowchart `#` text still imports, and `#35;` still imports. |
| CR-03 | Flowchart and state `direction LR` text is refused as `direction-in-text`. The encoded form `#100;irection LR` still imports, and export then import round-trips. |
| WR-01 | `#a1b;`, `#x41;` and `#_a;` are refused as `entity`, in every diagram kind. |
| WR-02 | `A:::foo --> B`, `A --> B:::foo` and `A:::x --> B:::y: lbl` under `drop_styling` keep their transitions, and the oracle sets match. |
| WR-03 | A 250-character id is refused as `id-too-long` at its own line (flowchart, state and sequence). A 200-character id imports. Label, node-count and depth limits were already named. |
| WR-04 | Largely fixed (all 46 listed sequence, flowchart and state words are now refused), but see WR-02 and WR-03 below: it both over- and under-reaches. |
| WR-05 | `node --test` gives 74/74. The minimums are exactly the on-disk counts (21, 10, 31, 33), so deleting any single fixture fails. |

There are no panics and no `internal` reports across 120,000 garbage blocks, in either mode.
The corpus (33 blocks) and `import-cases` (10) behave as before. Only the intentional `gantt` and
markdown-string refusals remain, and all remaining corpus blocks round-trip against the oracle.

The hunt for regressions found two things. The `drop_styling` path regressed for lines that
declare a state with a class shorthand. The WR-04 fix refuses valid Mermaid in several narrow
positions where the pre-fix importer was correct. It also still accepts several words Mermaid
rejects, and one test now asserts that. Probing also turned up residual silent divergences that were
not in iteration 1: a cross-line form of the `direction` trap, `&` fan-out without spaces, and dash
edge text containing `--`. Last, the exporter emits four flowchart ids that Mermaid rejects.

## Warnings

### WR-01: `drop_styling` regression: a state line that only declares a class is now refused with a misleading construct

**File:** `crates/vox-diagram/src/mermaid/import/state.rs:122-146` (`scan`), `:75-85` (`transition`)
**Issue:** The iteration-1 fix for WR-02 stopped treating `:::` as "drop the whole line" under
`drop_styling` and routes every `:::` line to `transition()`. A line with no `-->` falls into the
description branch and is refused. Under `drop_styling: true` (the mode whose purpose is to import
styled diagrams):

```
stateDiagram-v2
    classDef bad fill:#f00
    Idle --> Run
    Run:::bad          <- old: dropped, imports OK; new: refused [[4, "state:description"]]
```

`Run:::bad : desc` is also refused as `state:description`. `state Run:::bad` is refused as
`statement` (the old importer imported it). The default (non-drop) mode is unchanged. The fix report
only claims the transition forms. The construct name is also wrong: nothing here is a description.
**Fix:** In `transition()`'s no-arrow branch, when `drop_styling` is set and the line is
`<id>:::<class>` (use `shorthand()`), record `styling:shorthand` in `dropped` and return the
declaration as a bare state (or, minimally, `Ok(None)` as before, but only after confirming the id
passes `state_id`). Add `Run:::bad` after a transition to `mermaid_import_review.rs`.

### WR-02: The WR-04 word lists refuse valid Mermaid in several positions (regression against pre-fix behaviour)

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:44-77` (`FLOW_WORDS`, `STATE_WORDS`, `flow_id`, `state_id`); `crates/vox-diagram/src/mermaid/import/seq.rs:67-81`; `crates/vox-diagram/src/mermaid/import/state.rs:86-110`
**Issue:** The fix applies one blanket check to every id position, but the pinned lexer rejects
these words only in some positions. Each case below was accepted by the oracle with equal sets and
imported fine before the fix. All are now refused.

- Flowchart `subgraph` ids: `flow_id` is used for subgraph ids (`lines.rs:247`), but Mermaid accepts `subgraph style`, `subgraph class`, `subgraph classDef`, `subgraph linkStyle`, `subgraph subgraph`, `subgraph flowchart` and `subgraph flowchart-elk`. This is 10 cases, all now `reserved-id`.
- Sequence `Title->>B: hi`, `TITLE->>B`, `title-1->>B` and `participant title-a`: the lexer only reads `title` followed by whitespace. The case-insensitive compare (and the new `-` head split) refuse a participant named `Title`. `Note`, `Over` and `links` are also accepted by Mermaid in declaration-only use (`participant Note`, alone).
- State `A --> Note: hi`, `A --> Style: hi`, `A --> Class: hi`, `A --> State: hi` and `A --> Scale: hi` (any case): Mermaid accepts a target followed by `: label`. It rejects the same word with no label, which is the case that was probed. 16 old-ok cases refused in a 120-case probe, and about 100 in the fuzz runs.
- State `A --> B: ` (colon, then only a space) is valid in Mermaid, but lines are trimmed before scanning, so the new `empty-label` check refuses it. The fix report acknowledges this one. The old test that imported it was inverted rather than split.

This is fail-closed, so it is not unsafe, but the fix report's claim that it "encoded exactly what it
rejects" is not accurate, and `sequence_ids_that_mermaid_reads_as_keywords_are_refused_in_any_case`
(which lists `Title`) and `flowchart_ids_...` (which tests `subgraph` ids through `a --> w` only)
pin the over-refusal.
**Fix:** Use a position-aware check. For subgraph ids, call `valid_id` plus only the words that
`subgraph <id>` really rejects (`end`, `click`, `href`, `call`, `default`; probe to confirm). Drop
`title` from the sequence word list unless followed by whitespace. For state, apply `STATE_WORDS`
only when the id is not followed by `:`. For the trailing-space label, compare against the untrimmed
line, or document and test it as an accepted limitation.

### WR-03: WR-04 is incomplete: `click --> b` still fabricates content, and `href`, `call`, `_self`, `_blank`, `_parent`, `_top`, `accTitle`, `accDescr` are still accepted

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:36-47` (comment and `FLOW_WORDS`); `crates/vox-diagram/tests/mermaid_import.rs:244-252`; `crates/vox-diagram/src/mermaid/import/seq.rs:19-34`
**Issue:** The `FLOW_WORDS` comment says `click`, `href` and `call` are rejected "only as an edge
target". The oracle disagrees:

- `click --> b` is accepted by Mermaid but reads as an empty graph (`nodes: []`, `edges: []`), so the importer invents two nodes and an edge Mermaid never saw. This is the same defect class as CR-03. Bare `click` is rejected by Mermaid.
- `href --> b`, `call --> b`, `a --> click`, `a --> href`, `a --> call`, `subgraph click`, `subgraph href`, `subgraph call` and `subgraph default` are rejected by Mermaid and accepted by the importer.
- `_self`, `_blank`, `_parent`, `_top` (flowchart ids) are rejected by Mermaid and accepted by the importer.
- Sequence `a->>accTitle: hi`, `participant accTitle` and the `accDescr` equivalents (any case) are accepted by the importer. Mermaid rejects the receiver form and the oracle's own `canon` throws on the others.

The exporter's `KEYWORDS` already lists `click`, `href`, `call` and `default` as "rejected or
mishandled wherever they appear", so importer and exporter disagree. The inverted test
`import_styling_head_words_can_still_be_ids` now asserts that `click --> b` and bare `click` import
(`source.is_some()`), and the oracle contradicts both.
**Fix:** Add `click`, `href`, `call` (and `default` for subgraph ids) to the flowchart check,
exact case and head-before-`-`. Add `_self`, `_blank`, `_parent`, `_top` to flowchart ids, and
`acctitle` and `accdescr` to the sequence list. Remove `click --> b` and `click` from the "can still
be ids" test and assert them refused. If a valid-source worry remains, probe `click[x] --> b` (the
oracle accepts it) and refuse only the bare-id forms.

### WR-04: CR-03 is still open across lines in state diagrams: a trailing `direction` plus a following `lr`/`tb`/`bt`/`rl`/`td` line

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:158-170` (`decode_in`); `crates/vox-diagram/src/mermaid/import/state.rs` (`scan`)
**Issue:** Mermaid's state lexer reads `direction\s+(lr|rl|tb|td|bt)` case-insensitively, and `\s+`
spans a newline and blank lines. The importer only checks within one text, so these are all
accepted and imported:

```
stateDiagram-v2
    A --> Direction        <- id named Direction (also DIRECTION)
    lr --> B               <- oracle: empty graph, direction right
```
```
stateDiagram-v2
    A --> B: go direction
    btn --> C              <- also tbd, rl_x, lr, td..; blank lines between do not help
```

The oracle returns no nodes and direction `right`. The importer returns four nodes and direction
`down`. The same shape fails in the flowchart lexer only when the trap is bare (no closing bracket
or quote), so the practical exposure is state text and state ids named `Direction`. Probe: 5 of 15
cross-line variants mismatch.
**Fix:** In `state::state`, run `has_direction_trap` over each line joined to the next non-blank line
(`format!("{line}\n{next}")`), or refuse a state line whose trimmed text ends with `direction`
(case-insensitive) unless it is the whole `direction XX` statement. Add `Direction` and
`go direction` followed by `lr` lines to the review tests.

### WR-05: Flowchart `&` fan-out is accepted without surrounding spaces; Mermaid reads `b&c` as one id

**File:** `crates/vox-diagram/src/mermaid/import/lines.rs:201-211` (`group`)
**Issue:** `group()` trims whitespace on both sides of `&` but does not require it. Probe:

| Input | Oracle | Importer |
|---|---|---|
| `a-->b&c` | nodes `a`, `b&c`; one edge | nodes `a`, `b`, `c`; two edges |
| `a-->b&_` | one node `b&_` | `b` and `_` |
| `a-->b &c`, `a-->b& c` | rejected | accepted |

`a --> b & c` matches. This silently changes the graph (not caused by iteration 1; missed by the
iteration-1 `&` DoS sweep, which only looked at cost).
**Fix:** In `group`, require whitespace before `&` and after it when the left reference is a bare id.
For example, check `cur.starts_with(char::is_whitespace)` before the trim and refuse `&` otherwise
as `statement`. Add `a-->b&c` to `mermaid_unsupported`.

### WR-06: Dash edge text containing `--` or `---` is accepted and mis-read

**File:** `crates/vox-diagram/src/mermaid/import/lines.rs:179-193` (`arrow`), `text.rs:15` (`DASH_SPECIAL`)
**Issue:** `A -- text -->` allows only a quote as a special character. Mermaid ends the edge text
at the first `--`, so:

| Input | Oracle | Importer |
|---|---|---|
| `A -- a--b --> B` | rejected | edge label `a--b` |
| `A -- a --- b --> B` | `A -a-> b`, `b --> B` | `A -"a --- b"-> B` |
| `A -- a--->b --> B` | `A -a-> b`, `b --> B` | edge label `a-` |

Same silent-divergence class as the iteration-1 criticals (neither was in the iteration-1
findings).
**Fix:** In the unquoted dash branch, refuse (`edge-label`) any text that contains `--`, or that
starts or ends with `-`, before `-->`. Quoted dash text is not affected.

### WR-07: The exporter emits flowchart ids that Mermaid rejects (`_self`, `_blank`, `_parent`, `_top`)

**File:** `crates/vox-diagram/src/mermaid/escape.rs:7-22` (`KEYWORDS`), reached from `export.rs`
**Issue:** The DSL accepts these ids (leading underscore). The exporter keeps them as ids, because
only the 14 `KEYWORDS` are renamed to `nN`. Real Mermaid rejects them:
`graph { node _top: "X" node b: "Y" _top -> b: "L" }` exports `_top["X"]` and
`_top -->|"L"| b`, and `accepts()` is false. This is the same class as the keyword sweep that the
module doc claims is pinned against the real parser; the iteration-1 id sweep did not include
Mermaid's link-target names. Verified by sweeping about 75 id words through export and oracle:
these four are the only failures.
**Fix:** Add `"_self"`, `"_blank"`, `"_parent"`, `"_top"` to `KEYWORDS` (and grow the array
length). Add a `g01-keyword-ids` entry so the oracle fixtures pin them.

## Info

### IN-01: Oracle identity and trim semantics are weaker than they look (carried from iteration 1)

**File:** `docs-astro/tests/lib/mermaid-oracle.mjs:55-73`; `crates/vox-diagram/tests/support/roundtrip.rs:46-80`
**Issue:** Unchanged. Nodes are keyed by `[label, shape, containerPath]` with no id, so two
same-label nodes are indistinguishable. JS `trim()` and Rust `trim()` differ on U+FEFF. `parentOf`
is last-writer-wins.
**Fix:** Add a duplicate-label case to `import-cases` and normalise trimming on both sides.

### IN-02: Render-only Mermaid semantics are outside the parser oracle (carried from iteration 1)

**File:** `crates/vox-diagram/src/mermaid/import/lines.rs`; `crates/vox-diagram/src/mermaid/mod.rs` docs
**Issue:** Unchanged. `fa:` icons, `$$` KaTeX and Mermaid's 50,000-character `maxTextSize` apply at
render time only. The module doc mentions only the 500-edge limit.
**Fix:** Refuse `fa:` and `$$`, or document them; note the text-size limit next to the edge limit.

### IN-03: `freeze_corpus` swallows IO errors and walks the working tree (carried from iteration 1)

**File:** `crates/vox-diagram/tests/freeze_corpus.rs:12-28`, `:46-52`
**Issue:** Unchanged (the file was not touched by iteration 1). `read_dir(...).into_iter().flatten().flatten()`
and `let _ = fs::remove_file` hide errors, and the "tracked docs" walk includes untracked files.
**Fix:** Propagate errors with `panic!`, and take the file list from `git ls-files` or an env var.

### IN-04: The `direction` predicate is a case-insensitive superset, and its test copy is not independent

**File:** `crates/vox-diagram/src/mermaid/escape.rs:24-45`; `crates/vox-diagram/tests/support/direction.rs:34-46`
**Issue:** The flowchart lexer is case-sensitive on both the keyword and the token (`Direction LR`,
`direction tb`, `direction tbd` and `direction btw` stay plain text), but the state lexer is
case-insensitive. The shared predicate follows the state lexer, so the importer now refuses
`A["direction tbd"]` and `A["Direction lr"]` in a flowchart, which the oracle accepts. The comment
says "deliberate superset", so this is an accepted trade, but the importer docs do not list it.
`direction::trap` in the tests is a near line-for-line copy of `direction_traps` (using
`trim_start()`, which does not treat U+FEFF as whitespace). The comment calls it "independent of
`escape.rs`", but the exporter handles U+FEFF and the test predicate does not.
**Fix:** Document the superset in `mermaid/mod.rs`. Add U+FEFF to the test predicate, or compare
against the oracle result instead of a second copy.

### IN-05: Small leftovers

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:198-203`, `:28`; `state.rs` (`root_start`, `root_end`)
**Issue:** The `free` doc comment merges three clauses into one 150-character line. `valid_id`'s
`id-too-long` check also applies to sequence ids, which never reach the DSL (a 250-character
participant is refused for no reason). State ids `root_start` and `root_end` import as ordinary
states, but Mermaid treats them as the `[*]` start and end (probe: oracle sets differ).
**Fix:** Re-wrap the comment; apply the length check only on the DSL paths; add the two ids to
`STATE_WORDS`.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (iteration 2)_
