---
phase: 84-mermaid-emitter-strict-importer
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 37
files_reviewed_list:
  - .github/workflows/docs-quality.yml
  - crates/vox-diagram/src/lib.rs
  - crates/vox-diagram/src/mermaid/escape.rs
  - crates/vox-diagram/src/mermaid/export.rs
  - crates/vox-diagram/src/mermaid/export_seq.rs
  - crates/vox-diagram/src/mermaid/import/build.rs
  - crates/vox-diagram/src/mermaid/import/flow.rs
  - crates/vox-diagram/src/mermaid/import/lines.rs
  - crates/vox-diagram/src/mermaid/import/mod.rs
  - crates/vox-diagram/src/mermaid/import/seq.rs
  - crates/vox-diagram/src/mermaid/import/state.rs
  - crates/vox-diagram/src/mermaid/import/text.rs
  - crates/vox-diagram/src/mermaid/mod.rs
  - crates/vox-diagram/src/print.rs
  - crates/vox-diagram/tests/freeze_corpus.rs
  - crates/vox-diagram/tests/mermaid_corpus.rs
  - crates/vox-diagram/tests/mermaid_export.rs
  - crates/vox-diagram/tests/mermaid_export_props.rs
  - crates/vox-diagram/tests/mermaid_export_seq.rs
  - crates/vox-diagram/tests/mermaid_fixpoint.rs
  - crates/vox-diagram/tests/mermaid_import.rs
  - crates/vox-diagram/tests/mermaid_import_limits.rs
  - crates/vox-diagram/tests/mermaid_import_props.rs
  - crates/vox-diagram/tests/mermaid_import_seq.rs
  - crates/vox-diagram/tests/mermaid_import_state.rs
  - crates/vox-diagram/tests/mermaid_roundtrip.rs
  - crates/vox-diagram/tests/mermaid_unsupported.rs
  - crates/vox-diagram/tests/support/hostile.rs
  - crates/vox-diagram/tests/support/mod.rs
  - crates/vox-diagram/tests/support/roundtrip.rs
  - docs-astro/package.json
  - docs-astro/pnpm-lock.yaml
  - docs-astro/tests/lib/mermaid-oracle.mjs
  - docs-astro/tests/unit/mermaid-oracle.test.mjs
  - docs-astro/tests/unit/workflow-paths.test.mjs
  - docs/src/api/vox-diagram.generated.md
  - docs/src/architecture/where-things-live.md
findings:
  critical: 3
  warning: 5
  info: 3
  total: 11
status: issues_found
---

# Phase 84: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard (plus differential probing against the pinned real Mermaid 11.17.2 parser)
**Files Reviewed:** 37 (fixture goldens excluded)
**Status:** issues_found

## Summary

I built scratch copies of the crate (outside the worktree) and ran the real pinned Mermaid parser
through the repo's own `docs-astro/tests/lib/mermaid-oracle.mjs` against about 700 hostile exporter
inputs and about 90 importer inputs. The worktree was not modified.

The encoder is sound for everything except one Mermaid lexer trap. The sweep covered `%%`, `style`,
`classDef`, `:`, `#`, `&`, `<`, `` ` ``, `;`, `|`, `@{`, `$$`, `fa:`, keyword ids, `o`/`x` ids,
control characters, bidi characters and U+0085. Every one round-tripped except labels containing
`direction <TB|TD|LR|RL|BT>`.

The importer is not fully fail-closed. There are three classes where it accepts text and produces a
graph or sequence that differs from what real Mermaid reads from the same block:

- a raw `#` in sequence text,
- `direction XX` inside any flowchart or state text,
- `#word;` forms that are not purely alphabetic or purely numeric.

The importer also fails closed with a misleading `internal` construct on legitimate in-limit input.
The oracle suite can skip whole fixture families silently.

Verified clean:

- No panics. All string slicing is on ASCII boundaries.
- No recursion. Output is deterministic (BTree collections only).
- DoS inputs of about 64 KiB (22 shapes, including nesting, `&` fan-out, `#` and `<` runs, bracket soup) finish in milliseconds in a debug build.
- The cross-scope dotted-path rewrite, shadowing and first-closed membership matched the oracle on every nested and cross-scope case I tried.
- The `docs-quality.yml` path filters are correct for both `push` and `pull_request`, and the concurrency block is intact.

## Critical Issues

### CR-01: Exporter emits labels containing `direction <TB|TD|LR|RL|BT>`; Mermaid silently drops the node or edge, or rejects the whole diagram

**File:** `crates/vox-diagram/src/mermaid/escape.rs:26-56` (`encode`), reached from `export.rs` node, edge and subgraph lines
**Issue:** Mermaid's flowchart lexer matches `.*direction\s+(TB|TD|LR|RL|BT)[^\n]*` anywhere on a line. This applies even inside a quoted label. The whole line, including the node declaration or edge it sits on, is swallowed as a direction statement. Probing the real parser with exported text:

- Node label `p direction LR q` gives `a["p direction LR q"]`. The parse is accepted but node `a` is gone from `getVertices()`. This is silent data loss.
- Edge label `go direction TB now` gives `a -->|"go direction TB now"| b`. The parse is accepted but the edge is gone.
- Container title `my direction RL box` makes the whole diagram fail to parse.

It triggers for `xdirection LR` and `direction LRX` as well. The `direction` keyword guard in `KEYWORDS` covers ids only, and `FRAGMENTS` in `tests/support/hostile.rs` has no `direction` entry, so neither the proptests nor the oracle fixtures can see it. This breaks the module's headline promise ("verified against a pinned real Mermaid parser", "a label can never smuggle a second statement").
**Fix:** In `encode` (flowchart mode), numerically encode the leading `d` or `D` of any `direction` that is followed by whitespace and one of the five tokens. Match case-insensitively so it is a safe superset.
```rust
fn direction_hits(s: &str) -> Vec<usize> {
    let lower = s.to_ascii_lowercase();
    lower
        .match_indices("direction")
        .filter(|(i, _)| {
            let tail = &lower[i + 9..];
            let rest = tail.trim_start();
            rest.len() < tail.len()
                && ["tb", "td", "lr", "rl", "bt"].iter().any(|d| rest.starts_with(d))
        })
        .map(|(i, _)| i)
        .collect()
}
// in encode(): if !seq { let hits = direction_hits(s); ... }
// inside the char loop: if hits.contains(&at) { write!(out, "#{};", u32::from(c)); continue; }
```
Add `"x direction LR y"` and `"direction TB"` to `FRAGMENTS` and to the `export-hostile` fixtures so the oracle pins it. Sequence text (`fz3`) was unaffected, so apply this only when `seq` is false.

### CR-02: Importer accepts a raw `#` in sequence text; Mermaid truncates the text at the `#`

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:147-175` (`free`, `semicolon`), used by `import/seq.rs` for aliases, messages and notes
**Issue:** After entity placeholder substitution, any remaining raw `#` ends the text in the Mermaid sequence lexer (the text rule is `[^#\n;]*`). `A->>B: item #3` parses in Mermaid as message text `item`. The importer keeps `item #3`. Probed:

| Block | Mermaid reads | Importer yields |
|---|---|---|
| `participant B as 100#` | `100` | `100#` |
| `A->>B: end #` | `end` | `end #` |
| `Note over A: note # here` | `note` | `note # here` |

This is silent content alteration, and the importer's `R(B)` does not equal the oracle's `C(B)`. `free()` already treats `;` this way (`semicolon()` refuses it). `#` needs the same rule, but only for the sequence callers (`wrap == true`). State text tolerates `#` (verified `OK` against the oracle).
**Fix:** In `free`, when `wrap` is true, refuse a `#` that does not begin a complete entity.
```rust
// text.rs, new helper next to semicolon()
fn stray_hash(raw: &str) -> Result<bool, &'static str> {
    let mut i = 0;
    while let Some(c) = raw[i..].chars().next() {
        i += c.len_utf8();
        if c == '#' {
            match entity(&raw[i..])? {
                Some((_, n)) => i += n,
                None => return Ok(true),
            }
        }
    }
    Ok(false)
}
// in free(): if wrap && stray_hash(raw)? { return Err("hash"); }
```
Also run it on the whole line in `seq::scan` for symmetry with `semicolon`. Add `A->>B: item #3` and `participant A as x # y` to `mermaid_unsupported` and `import-cases`.

### CR-03: Importer accepts `direction XX` inside flowchart and state text; Mermaid swallows the line or flips the diagram direction

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:93` (`decode`), `import/state.rs` (transition labels), `import/lines.rs` (node and edge text)
**Issue:** This is the importer-side mirror of CR-01.

- `A["go direction LR"] --> B` and `A -->|"direction LR"| B`: Mermaid swallows the whole line, so its graph is empty. The importer returns two nodes and an edge, inventing content Mermaid never saw.
- State: `A --> B: go direction LR`. Mermaid treats the label's `direction LR` as a diagram direction directive. It drops state `A` and sets the direction to `right`, while the importer returns direction `down` and keeps `A`.

Both were confirmed with the oracle (`canon(original)` versus the round-tripped import). A strict, fail-closed importer must refuse text that Mermaid itself does not read as text.
**Fix:** In `decode` (shared by all callers), return `Err("direction-in-text")` when the same pattern as in CR-01 appears (case-insensitive `direction`, then whitespace, then `TB|TD|LR|RL|BT` prefix). Reuse one shared predicate with `escape.rs`, for example `pub(crate) fn has_direction_trap(&str) -> bool` in `mermaid/mod.rs`. That removes the exporter and importer drift.

## Warnings

### WR-01: `entity()` recognises only all-letter or all-digit `#…;`; Mermaid's rule is `#\w+;`

**File:** `crates/vox-diagram/src/mermaid/import/text.rs:61-90`
**Issue:** Mermaid rewrites every `#\w+;` (letters, digits and `_`) into an HTML entity before lexing. The importer decodes only `#letters;` and `#digits;`, and passes anything else through literally. Probed:

- `A["x#a1b;c"]`: Mermaid label `x&a1b;c`, importer label `x#a1b;c`.
- `#x41;`: Mermaid `&x41;`, importer literal.
- `#_a;`: Mermaid `&_a;`, importer literal.

`#x41;` is a natural hex spelling, so this is plausible in real blocks. The semicolon scan for sequence text is also defeated, because it flags `;` as a statement end instead of refusing the entity.
**Fix:** After `#`, take the maximal `[A-Za-z0-9_]+` run. If it is followed by `;`, treat it as an entity. For all digits, parse it. For all letters, look it up in the named table. For anything else, return `Err("entity")`. Fail closed rather than guess.

### WR-02: State import with `drop_styling` deletes whole transitions (and their states) that contain `:::`

**File:** `crates/vox-diagram/src/mermaid/import/state.rs:93`
**Issue:** `line.contains(":::")` turns the entire line into `Ok(None)` under `drop_styling`. `A:::foo --> B` is a real transition decorated with a class. Probe: `stateDiagram-v2\n A:::foo --> B\n B --> C` with `drop_styling: true` returned only `B -> C`. Edge `A->B` and node `A` are gone, with only `styling:shorthand` in `dropped`. The flowchart importer handles the same construct correctly (it drops the class and keeps the edge, `lines.rs` `reference`). This also fires on a label such as `A --> B: see x:::y`.
**Fix:** Strip the `:::name` suffix from the endpoint instead of dropping the line. Split on `-->` first, then strip `:::ident` from each endpoint in `endpoint()`, recording the drop. Keep refusing in the default mode.

### WR-03: Valid in-limit blocks are refused as `internal` at line 1 (ids over 200 chars, or output over 64 KiB)

**File:** `crates/vox-diagram/src/mermaid/import/mod.rs:86-91` (`finish`), `import/text.rs:16` (`valid_id` has no length cap)
**Issue:** `finish` maps any `fmt()` failure to `refused(1, "internal")`. `fmt` fails when the generated DSL exceeds `MAX_SOURCE_BYTES` or when an id is longer than the DSL's 200-character cap. Two reproducible cases:

- A 250-character node id.
- 500 nodes with 23-character ids plus about 1000 edges in chain lines. The Mermaid block is 32 KB, but the DSL, which repeats `node id: "id"` and one edge line per edge, exceeds 64 KiB.

It does fail closed, but the line, the construct name and the documented contract are wrong. `mermaid_import_props.rs:3` asserts "never reports `internal`", and that invariant is false. The soup generator never produces long ids or large graphs.
**Fix:** Pre-check id length in `valid_id` (`> MAX_LABEL_CHARS` gives `Err("id-too-long")`). In `finish`, when `fmt` returns `None`, report `limit-source` if `dsl.len() > MAX_SOURCE_BYTES`, else keep `internal`. Add both cases to `mermaid_import_limits.rs`.

### WR-04: Importer accepts text that real Mermaid rejects (reserved-word list narrower than Mermaid's)

**File:** `crates/vox-diagram/src/mermaid/import/seq.rs:62-66`; `import/text.rs:16-33`; `import/state.rs:20-30`
**Issue:** Verified against the oracle (`accepts()` is false), yet the importer returns a diagram:

- Sequence: ids `End`, `Loop`, `Alt`, `Opt`, `Par`, `Rect`, `Break`, `Critical`, `Option`, `And`, `Else`, `Title`, `Box`, `Actor`, `Participant`, `Activate`, `Autonumber`, `Links`, `Note`, `Over`, `over`, `Create`, `Destroy`, `Deactivate`. Mermaid's sequence lexer is case-insensitive, while `BLOCKS.contains(&s)` is a case-sensitive compare.
- Flowchart: `a --> end` (`end` is not in `RESERVED`).
- State: `a-b --> c`, `A --> B:` (trailing colon, empty label), `click --> A`.

A "strict" importer turns invalid Mermaid into a diagram without a signal, so the user believes their source was valid. This is not data corruption, but it contradicts the fail-closed stance.
**Fix:** Compare `s.to_ascii_lowercase()` against `BLOCKS` and `KEYWORDS` in `seq::id`. Add `end` (and the `KEYWORDS` list in `escape.rs`) to a shared reserved-id check for flowchart ids. Refuse a state transition with an empty label after `:`. Decide explicitly whether `-` is legal in state ids (Mermaid rejects it).

### WR-05: The Node oracle suite silently skips whole fixture families when a directory is missing

**File:** `docs-astro/tests/unit/mermaid-oracle.test.mjs:80-81`, `:105`
**Issue:** `family()` does `if (!existsSync(dir)) return;`, and the corpus block is wrapped in `if (existsSync('corpus-roundtrip'))`. If `import-cases-roundtrip`, `corpus-roundtrip` or `export-hostile` is deleted, renamed or fails to land, the unit run still goes green with only the five helper tests. The comment "a directory is skipped only when absent" shows this was a deliberate wave-ordering convenience from the plan, but it is now a vacuity hole in the only independent check of the exporter.
**Fix:** Now that all families exist, make absence a failure: replace the early `return` with `test(\`${dir}: exists\`, () => assert.ok(existsSync(...)))`. Do the same for the corpus block, and assert minimum counts (`>= 31` round-tripped corpus stems).

## Info

### IN-01: Oracle identity and trim semantics are weaker than they look

**File:** `docs-astro/tests/lib/mermaid-oracle.mjs:55-73`; `crates/vox-diagram/tests/support/roundtrip.rs:46-80`
**Issue:**

- Nodes are identified by `[label, shape, containerTitlePath]`, with no id. Two same-label nodes in the same container, or two containers with the same title, are indistinguishable. A misrouted edge between them passes.
- The oracle trims with JS `trim()` and Rust trims with `str::trim()`. They differ on U+FEFF, which JS trims and Rust does not. My probe showed this as a spurious three-way inequality for a BOM-only label.
- The oracle's `parentOf` is last-writer-wins over `getSubGraphs()`. It matches Mermaid's behaviour in every nested case tried, but that is an assumption, not an assertion.

**Fix:** Add a duplicate-label case to `import-cases`, and normalise trimming identically on both sides (for example, trim only ASCII and common Unicode spaces in both).

### IN-02: Render-only Mermaid semantics are outside what the parser oracle can see

**File:** `crates/vox-diagram/src/mermaid/import/lines.rs`; `mod.rs` module docs
**Issue:** `mermaid.parse` and `getDiagramFromText` stop before rendering. The importer happily accepts labels such as `fa:fa-car` (icon in render) and `$$x^2$$` (KaTeX in render). Mermaid's default `maxTextSize` (50,000 characters, enforced by `render`) also applies. I confirmed `parse` accepts a 65 KB block but did not test `render`. A maximal 64 KiB DSL, or an `#NN;`-inflated export, can exceed it. The module doc mentions only the 500-edge limit.
**Fix:** Either refuse `fa:` and `$$` in the importer (fail closed), or document them as render-time reinterpretations. Add the 50,000-character note next to the 500-edge note in `mermaid/mod.rs`.

### IN-03: `freeze_corpus` swallows IO errors and walks the working tree

**File:** `crates/vox-diagram/tests/freeze_corpus.rs:12-28`, `:46-52`
**Issue:** `read_dir(...).into_iter().flatten().flatten()` and the `let _ = fs::remove_file` calls hide errors. The doc comment says "tracked docs" and "clean checkout of HEAD", but the walk includes untracked and uncommitted docs. A dirty tree could freeze fences from files that are not in HEAD. It is `#[ignore]`d and one-shot, so the risk is low. A silently partial refreeze would shift the hard-coded counts (33, 21, 31, 30) with a confusing failure elsewhere.
**Fix:** Propagate errors with `panic!`, as `write` already does. Rename `tracked_docs` or filter through `git ls-files` outside the arch-guarded test (for example by taking a file list from an env var).

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
