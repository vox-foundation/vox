---
phase: 83
phase_name: "core-crate-parser-ir-diagnostics"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 66
  lessons: 5
  patterns: 18
  surprises: 7
missing_artifacts:
  - "UAT.md"
---

# Phase 83 Learnings: core-crate-parser-ir-diagnostics

## Decisions

### Declared every phase dependency (serde, pulldown-cmark, strsim, insta, p...
Declared every phase dependency (serde, pulldown-cmark, strsim, insta, proptest, serde_json) in this plan so Cargo.toml/Cargo.lock have one owning plan; pulldown-cmark and strsim are not yet referenced by code

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-01-SUMMARY.md

---

### Tracer lexer treats an unterminated string as Unknown('"') and has no es...
Tracer lexer treats an unterminated string as Unknown('"') and has no escape/comment/kebab-ident handling; plan 02 owns those

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-01-SUMMARY.md

---

### Resolver keeps the first declaration on duplicate ids with no diagnostic...
Resolver keeps the first declaration on duplicate ids with no diagnostic yet (marked with a ponytail: comment); plan 03 adds duplicate-id/shadow reporting and nested scopes

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-01-SUMMARY.md

---

### Diagnostics echo at most 20 chars of user text in messages
Diagnostics echo at most 20 chars of user text in messages

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-01-SUMMARY.md

---

### Tracer verified end to end before anything else was built
Tracer verified end to end before anything else was built: the minimal graph yields `valid: true`, nodes a/b, edge a->b, `schema_version: 1`, and its report JSON is snapshot-locked (`graph_constructs__tracer_minimal_graph_report.snap`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-01-SUMMARY.md

---

### Error paths each produce exactly one diagnostic and withhold the IR
Error paths each produce exactly one diagnostic and withhold the IR: `diagram/unexpected-token` (line 2, col 12, message names `:` and the found string), `diagram/undeclared-node` (line 5, col 10), `diagram/limit-source` (span starts at byte 65,536, pulled back to a char boundary; a 65,536-byte source is accepted).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-01-SUMMARY.md

---

### Independent mistakes on separate lines are each reported (resync at newl...
Independent mistakes on separate lines are each reported (resync at newline/`}`), covered by `independent_errors_are_all_reported`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-01-SUMMARY.md

---

### 18 tests pass under INSTA_UPDATE=no; cargo check -p vox-diagram --all-ta...
18 tests pass under `INSTA_UPDATE=no`; `cargo check -p vox-diagram --all-targets` and `cargo clippy -p vox-diagram --all-targets -- -D warnings` are clean.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-01-SUMMARY.md

---

### Governance: cargo run -p vox-arch-check exits 0 with no vox-diagram find...
Governance: `cargo run -p vox-arch-check` exits 0 with no `vox-diagram` finding; `vox ci crate-edges` OK (700 edges within baseline, only the exempt `workspace-hack` edge added); `vox ci crate-api-sync` OK; crate-edges and fan-in baselines untouched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-01-SUMMARY.md

---

### Resolver scopes
Resolver scopes: Vec<Scope> with BTreeMap names, one namespace per scope; pass B replays pass A's pre-order scope numbering instead of storing a scope id per AST node

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-02-SUMMARY.md

---

### Lexer reports its own problems and still emits a best-effort token (Str{...
Lexer reports its own problems and still emits a best-effort token (Str{terminated:false}, BadIdent), and the parser accepts those tokens as a label/ident, so one mistake yields exactly one diagnostic

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-02-SUMMARY.md

---

### direction: is a statement only directly inside graph { }; inside a conta...
`direction:` is a statement only directly inside `graph { }`; inside a container it parses as an edge and gives unexpected-token (loud, not silently ignored)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-02-SUMMARY.md

---

### container-endpoint help names the container's first declared child (trac...
container-endpoint help names the container's first declared child (tracked in Scope.first_child, since the BTreeMap is alphabetical)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-02-SUMMARY.md

---

### Tracer first
Tracer first: the canonical example (direction, container `backend`, `{ shape: cylinder }` attr, labelled edges, dotted `backend.gateway`) flows source -> AST -> scoped resolution -> `GraphIr`; nodes come out in DFS order `[client, backend.gateway, backend.engine, backend.db]`, edges in source order, and the IR JSON is byte-identical over repeated runs (`ir_json_is_stable`, 3 iterations).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-02-SUMMARY.md

---

### Resolution: innermost-first then outward, forward references valid, a ->...
Resolution: innermost-first then outward, forward references valid, `a -> o.leaf.x` gives `diagram/not-a-container` on `x`, `client -> backend` gives `diagram/container-endpoint` (no fixes, help names `backend.gateway`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-02-SUMMARY.md

---

### Lexer: kebab ids (api-gw, a->b, a-->b leaves the dash run), \" \\ \n esc...
Lexer: kebab ids (`api-gw`, `a->b`, `a-->b` leaves the dash run), `\"` `\\` `\n` escapes, `//` comments, CRLF as one newline, BOM skipped, and four non-cascading diagnostics (`unterminated-string`, `invalid-escape`, `forbidden-char` per char with exact span, `invalid-identifier`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-02-SUMMARY.md

---

### Trivia: comments land in leading / header / own-line / trailing / contai...
Trivia: comments land in leading / header / own-line / trailing / container header+close / document close / after-`}` positions; `blank_before` follows the A3 rule; multi-line attr blocks; explicit `shape: rect` is stored as the default.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-02-SUMMARY.md

---

### all_constructs.vdg (right direction, six shapes, three escapes, kebab id...
`all_constructs.vdg` (right direction, six shapes, three escapes, kebab id, 3-level nesting, inner+outer dotted refs, innermost-first, self-loop, every comment position) is valid and snapshot-locked; its CRLF variant yields the same IR once `span` keys are stripped.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-02-SUMMARY.md

---

### Fence body comes from pulldown-cmark Text events (indentation/blockquote...
Fence body comes from pulldown-cmark Text events (indentation/blockquote prefix already stripped); open_line/body_range/closed are derived from the block's offset range, with closed requiring a distinct last line of only the opener's fence char at >= opener length

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-03-SUMMARY.md

---

### No lowercasing of the info string (deliberate difference from the claims...
No lowercasing of the info string (deliberate difference from the claims extractor)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-03-SUMMARY.md

---

### fence::extract walks Parser::new_ext(md, ENABLE_YAML_STYLE_METADATA_BLOC...
`fence::extract` walks `Parser::new_ext(md, ENABLE_YAML_STYLE_METADATA_BLOCKS).into_offset_iter()`; a fence matches only when `info.trim() == info` exactly, so `vox`, `vox-diagram,x`, `mermaid`, prose/inline-code mentions and examples nested in a 4-backtick `md` block are all excluded.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-03-SUMMARY.md

---

### Tilde fences, list-item indentation, blockquote > prefixes, unclosed-at-...
Tilde fences, list-item indentation, blockquote `>` prefixes, unclosed-at-EOF, shorter-than-opener closers, empty bodies, CRLF input and YAML frontmatter plus an MDX `import` line are all covered; CRLF and LF produce equal bodies after `normalize`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-03-SUMMARY.md

---

### sequence_ir.rs defines SequenceIr, Actor, SeqEvent (#[serde(tag = "type"...
`sequence_ir.rs` defines `SequenceIr`, `Actor`, `SeqEvent` (`#[serde(tag = "type", rename_all = "kebab-case")]`, `Message{dashed}`, `Note{over}`) with zero `pub fn`; `lib.rs` gained `pub mod fence;` and `pub mod sequence_ir;`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-03-SUMMARY.md

---

### Final run: 17 tests pass (fence_scan 11, fence::tests 4, sequence_ir 2);...
Final run: 17 tests pass (`fence_scan` 11, `fence::tests` 4, `sequence_ir` 2); `cargo check -p vox-diagram --all-targets` clean; `cargo fmt -p vox-diagram` applied.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-03-SUMMARY.md

---

### Collector tracks has_error separately from storage, so a flood of warnin...
Collector tracks has_error separately from storage, so a flood of warnings can never push an Error out and make a report look valid

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-04-SUMMARY.md

---

### Items past the node/edge cap stay in the AST (the IR is withheld by the ...
Items past the node/edge cap stay in the AST (the IR is withheld by the Error anyway) so edges to them do not cascade into undeclared-node; the one error is emitted at the end with the total

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-04-SUMMARY.md

---

### skip_to_stmt_end is brace-aware
skip_to_stmt_end is brace-aware: braces opened in the skipped region (a multi-line attr block after a broken node) are skipped with it, except it stops at a line-leading node/container

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-04-SUMMARY.md

---

### Unquoted and single-quoted labels are recovered as the label text, so a ...
Unquoted and single-quoted labels are recovered as the label text, so a mistaken label is exactly one expected-quoted-label

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-04-SUMMARY.md

---

### After a non-graph header word or non-word first token the rest of the fi...
After a non-graph header word or non-word first token the rest of the file is dropped silently, so a wrong diagram kind is one unknown-diagram-kind

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-04-SUMMARY.md

---

### Tracer: node a "A" (missing colon) plus b -> c gives exactly unexpected-...
Tracer: `node a "A"` (missing colon) plus `b -> c` gives exactly `unexpected-token` and `undeclared-node`; the broken `a` is still declared, so no false undeclared-node for it. Container headers get the same treatment (garbled label with a `{` on the line still parses its body; no `{` on the line declares it empty).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-04-SUMMARY.md

---

### Collector (diag.rs)
Collector (`diag.rs`): sort by `(span.start, code, span.end)`, exact-duplicate removal, 99 real diagnostics plus a trailing `limit-diagnostics` Warning; parser halts when it is full or when fuel runs out (sentinel added once).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-04-SUMMARY.md

---

### Catalogue: expected-quoted-label, unclosed-brace (one per open {, span =...
Catalogue: `expected-quoted-label`, `unclosed-brace` (one per open `{`, span = that `{`), `trailing-content` (one error over everything after the closing `}`), `unknown-diagram-kind`, `reserved-word`, `invalid-direction` (help names the closest word via `strsim::levenshtein`), `duplicate-direction`, `direction-in-container`, `invalid-shape`, `unknown-attribute`, `duplicate-id` (related = first declaration), `shadowed-id` (Warning, report stays valid), `empty-label`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-04-SUMMARY.md

---

### Caps: limit-nodes / limit-edges (one each, message carries the total), l...
Caps: `limit-nodes` / `limit-edges` (one each, message carries the total), `limit-label` (decoded chars, multibyte-safe), `limit-depth` (container past 4 skips its body with a brace counter). The lexer also stops building diagnostics at the report cap.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-04-SUMMARY.md

---

### 20 broken fixtures, a snapshot-locked six-mistakes diagnostics JSON, a c...
20 broken fixtures, a snapshot-locked six-mistakes diagnostics JSON, a committed 27-line sorted code golden checked as a subset of `ALL_CODES`; `ALL_CODES.len()` assertion bumped 9 -> 10 -> 23 -> 27.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-04-SUMMARY.md

---

### fmt = check gate + parse + print (two linear passes); check.rs untouched
fmt = check gate + parse + print (two linear passes); check.rs untouched

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-05-SUMMARY.md

---

### Generator ids are unique by a DFS counter suffix (kebab and snake styles...
Generator ids are unique by a DFS counter suffix (kebab and snake styles), so reserved words and Mermaid statement words can never collide and uniqueness needs no pool

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-05-SUMMARY.md

---

### Edges are generated as indices into the document's declaration-ordered n...
Edges are generated as indices into the document's declaration-ordered nodes (mod count), so every edge targets a declared node, forward references included; an edge in a node-less document is dropped

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-05-SUMMARY.md

---

### Noise is a shrinkable die stream where die 0 always means the plainest l...
Noise is a shrinkable die stream where die 0 always means the plainest layout, so counterexamples shrink toward clean text

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-05-SUMMARY.md

---

### S2 printer split into tests/support/noisy.rs (400-line cap); mod.rs keep...
S2 printer split into tests/support/noisy.rs (400-line cap); mod.rs keeps a thin noisy_source wrapper so the interface is unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-05-SUMMARY.md

---

### Tracer: fmt(source) flows check -> parse -> to_source; canonical.vdg is ...
Tracer: `fmt(source)` flows check -> parse -> `to_source`; `canonical.vdg` is its own canonical form. Shadowed-id Warning documents format; any Error returns `None`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-05-SUMMARY.md

---

### Printer rules, all golden-locked
Printer rules, all golden-locked: 4-space indent, one statement per line, order preserved (`direction` never hoisted), ` { shape: x }` only when not rect, multi-line attr blocks collapse, labels re-escaped (`\\`, `\"`, `\n`; tab raw), blank runs collapse to one, none at block edges (A3), `graph { // h`, `node ... // t`, container header comment after `{`, close comment after `}`, comments before `graph` and after the final `}`, `//x` stays `//x`, CRLF in gives LF out, empty container on two lines, comment-only container keeps comments.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-05-SUMMARY.md

---

### 14 fmt_golden tests (4 reviewed insta snapshots) and 3 print::tests; eac...
14 `fmt_golden` tests (4 reviewed insta snapshots) and 3 `print::tests`; each golden asserts `fmt(fmt(x)) == fmt(x)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-05-SUMMARY.md

---

### Generators: containers nested at most 3 deep, six shapes, optional direc...
Generators: containers nested at most 3 deep, six shapes, optional direction in a random root slot, Unicode and escape-heavy labels (A8-forbidden characters and `\r` excluded), noisy layout (tabs/spaces/odd indent, 0-3 blank lines, own-line and trailing comments incl. directly before `}`, explicit `shape: rect`, `{shape:x}`, `, }`, multi-line attr blocks, zero-space `a->b`, trailing spaces, missing final newline, CRLF).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-05-SUMMARY.md

---

### Properties (128 cases each, --retries 0)
Properties (128 cases each, `--retries 0`): P1 zero diagnostics and IR equals the model oracle; P2 idempotent plus LF/one-trailing-newline/no-trailing-whitespace; P3 span-stripped AST (comments and `blank_before` included) and IR unchanged by fmt; P5 IR JSON identical across two checks.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-05-SUMMARY.md

---

### A group is rejected whole if any edit is out of range, off a char bounda...
A group is rejected whole if any edit is out of range, off a char boundary, or overlaps an earlier-accepted edit or a mate; equal-offset inserts are legal and keep diagnostic order (never deduplicated)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-06-SUMMARY.md

---

### Overlap scan is O(k^2) in accepted edits (check caps a report at 100 dia...
Overlap scan is O(k^2) in accepted edits (check caps a report at 100 diagnostics); marked with a ponytail comment naming the interval-map upgrade

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-06-SUMMARY.md

---

### unclosed-brace carries its \n} fix only when the scan is at EOF; an attr...
unclosed-brace carries its `\n}` fix only when the scan is at EOF; an attribute block cut short by the next declaration has no single safe edit, so no fix

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-06-SUMMARY.md

---

### undeclared-node gets a fix only for a single-segment ref (declared in th...
undeclared-node gets a fix only for a single-segment ref (declared in the referencing scope) or the LAST segment of a dotted ref (declared in the resolved container); a scope with no `{` or no `}` yields no fix this round, so an unclosed scope is closed first and the declaration comes next round

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-06-SUMMARY.md

---

### Block records its brace offsets as an unserialised Option<(u32, u32)>; t...
Block records its brace offsets as an unserialised `Option<(u32, u32)>`; two bare offsets (not Spans) keep `Entry` under clippy's large-variant threshold, and `serde(skip)` keeps the AST JSON (and P3) unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-06-SUMMARY.md

---

### Pure fix builders live in fixgen.rs (quote, declare_node); quote drops \...
Pure fix builders live in `fixgen.rs` (`quote`, `declare_node`); `quote` drops `\r` and forbidden chars so a recovered label can never reintroduce a lexer error

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-06-SUMMARY.md

---

### Tracer: a graph with two unclosed scopes produces two identical \n} EOF ...
Tracer: a graph with two unclosed scopes produces two identical `\n}` EOF inserts; both apply (no dedup), the result is valid, and `repair` converges in one round. `repair` on valid input is 0 rounds and unchanged; `max_rounds` 0 returns the input.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-06-SUMMARY.md

---

### apply_fixes unit tests cover
`apply_fixes` unit tests cover: out-of-range group skipped whole, inverted span, multibyte-splitting edit, overlap (later group loses, insert strictly inside a replaced range), intra-group overlap, group atomicity, offset ordering, same-offset insert order, fix-less diagnostics ignored.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-06-SUMMARY.md

---

### Fix shapes (each asserted as exact (original text, replacement))
Fix shapes (each asserted as exact `(original text, replacement)`): unterminated-string closes after the trimmed text; invalid-escape doubles just the backslash; forbidden-char deletes the char; expected-quoted-label wraps with `\` and `"` escaped (single quotes stripped); empty-label becomes the owner id, an empty edge label is deleted back to the target; invalid-shape fixes only an exact case-fold match (`hexagon` has none); undeclared-node follows Rule R (end of the `{` line, indent + 4, single-line scope inserts after `{` with a trailing newline, first diagnostic per (scope, id) owns it).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-06-SUMMARY.md

---

### 13 repairable fixtures (one per fix-bearing code, plus mixed, multi-ghos...
13 repairable fixtures (one per fix-bearing code, plus mixed, multi-ghost, single-line scope, unclosed-then-ghost); all reach zero errors in one round except `13` which needs two (close first, declare second), each stable under a second repair. All 13 repaired texts were reviewed by eye before committing.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-06-SUMMARY.md

---

### fixes_only_on_errors (no Warning has fixes, all fix spans in range and o...
`fixes_only_on_errors` (no Warning has fixes, all fix spans in range and on boundaries, at least 10 fix-bearing diagnostics) and `non_mechanical_codes_have_no_fixes` (duplicate-id, container-endpoint, not-a-container, reserved-word, invalid-direction, unknown-attribute, trailing-content, limit-nodes/depth/label all exercised and fix-free).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-06-SUMMARY.md

---

### Free-text Mermaid forms are single lexer tokens (links with their |text|...
Free-text Mermaid forms are single lexer tokens (links with their |text| / -- text --> label, id[Label] bracket forms, whole styling lines, %% comments, :::name), so label text never reaches the identifier rules and cannot raise lexical noise such as invalid-identifier on `2px`

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-07-SUMMARY.md

---

### M3 (id[Label]) is reported by the resolver, not the parser
M3 (id[Label]) is reported by the resolver, not the parser: the in-edge fix declares the node via Rule R, which needs the scope's braces. The parser only annotates the AST (`Ref.bracket`, `NodeDecl.bracket`, `#[serde(skip)]` and boxed so Entry stays small)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-07-SUMMARY.md

---

### A chain or & fan-out gets ONE diagnostic whose fix replaces the whole st...
A chain or `&` fan-out gets ONE diagnostic whose fix replaces the whole statement by the expanded edge lines (per-arrow diagnostics are suppressed then, so no overlapping groups); with no exact expansion it gets a fix-less diagnostic plus the per-arrow fixes

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-07-SUMMARY.md

---

### The link label is inserted after the target's bracket text (C(Process)
The link label is inserted after the target's bracket text (`C(Process): "yes"`, not between id and bracket), so a pending C(Process) rewrite stays whole and the two groups touch instead of overlap

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-07-SUMMARY.md

---

### The lexer caches the end of the current line (rest_of_line)
The lexer caches the end of the current line (`rest_of_line`): a line full of links or brackets is scanned once, not once per token

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-07-SUMMARY.md

---

### M5 closes the block with }\n when the source already ends in a newline (...
M5 closes the block with `}\n` when the source already ends in a newline (`\n}\n` otherwise), so repair leaves no blank line before the brace

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 83-07-SUMMARY.md

---

### Tracer: a --> b flows lexer (one MArrow token) -> lenient Edge in the AS...
Tracer: `a --> b` flows lexer (one `MArrow` token) -> lenient Edge in the AST -> one Error on the `-->` span with fix `-->` => `->` -> `repair`, with the IR withheld until repaired (A2). `A --> B` with both ends undeclared converges in 1 round (the arrow and both declarations are independent fixes), better than the planned 2. `==>`, `-.->`, `---`, `--x`, `--o` report without a fix.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-07-SUMMARY.md

---

### Catalogue (one named test per row, 24 tests in mermaid_catalog, each ass...
Catalogue (one named test per row, 24 tests in `mermaid_catalog`, each asserting exact codes, fix spans and replacement text): M2 pipe and dash labels (quotes stripped, `<br>` -> `\n`, `\` and `"` escaped, label inserted after the target); M3 statement and in-edge forms for rect/rounded/diamond/cylinder, quoted and punctuated labels (`A[API: gw (beta) / x.y]` is never split), unfixable shapes (`((x))`, `([x])`, `[[x]]`, `{{x}}`, `>x]`, unclosed), repeats (same label: bare id only; different: `duplicate-id`, no silent text loss); M4 three subgraph forms with one diagnostic and two edits, title-only no fix; M5 all five direction tokens, none, trailing `;`, with and without a trailing newline; M6 `direction LR` / `direction: LR`; M7 `%%` and `%%{` directive; M8 trailing and mid-line `;`; M9 eight styling heads plus `:::name`, and `class -> style` stays an edge; M10 `&` fan-out and chains, with Mermaid arrows, labels and trailing labels.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-07-SUMMARY.md

---

### 6 corpus-shaped fixtures (header, subgraphs, A[Label] --> B, three label...
6 corpus-shaped fixtures (header, subgraphs, `A[Label] --> B`, three label spellings, `%%`, `;`, `&`, chain, nested subgraphs) are invalid, repair to valid within 5 rounds, are stable under a second repair, and match reviewed snapshots.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-07-SUMMARY.md

---

### Grammar card
Grammar card: 1,747 bytes (budget 2,600), no triple-backtick lines. Four dogfood tests: the example checks with zero diagnostics and is a `fmt` fixed point; each of the 10 "Do NOT write" examples yields `diagram/mermaid-syntax`; the six reserved words on the card are each rejected as ids; size budget.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-07-SUMMARY.md

---

### S4 mutate_and_repair (128 cases; a temporary 5,000-case run also passed)
S4 `mutate_and_repair` (128 cases; a temporary 5,000-case run also passed): arrow `->` => `-->`, unquoted edge label ending its line, deleted final `}`, dropped closing `"`, `node A: "L"` => `A[L]`. Sites come from the AST; a companion test proves every kind has a real site and repairs back, so the property cannot pass by always falling back to the same corruption.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 83-07-SUMMARY.md

## Lessons

### None. IN-01, IN-02 and IN-03 are Info findings, outside fix_scope: criti...
None. IN-01, IN-02 and IN-03 are Info findings, outside `fix_scope: critical_warning`.

**Context:** Surfaced in structured phase code review.
**Source:** 83-REVIEW-FIX.md

---

### _Fixed: 2026-10-16_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_
_Fixed: 2026-10-16_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_

**Context:** Surfaced in structured phase code review.
**Source:** 83-REVIEW-FIX.md

---

### # Iteration 2
# Iteration 2

**Context:** Surfaced in structured phase code review.
**Source:** 83-REVIEW-FIX.md

---

### Hosted CI run Test
**Hosted CI run** **Test:** Push the branch via draft PR; read `linux`, `tests`, `ssot-drift`, `vox ci crate-edges`, `vox-arch-check`, `crate-api-sync`, hakari results. **Expected:** Green, with no new consumer edge to `vox-diagram`. **Why human:** Nothing has been pushed, and the local `vox` binary is stale so those `vox ci` gates were not re-run by this verifier.

**Context:** Verified in phase acceptance checks.
**Source:** 83-VERIFICATION.md

---

### container_endpoint_is_rejected does not assert the exact span (the code ...
`container_endpoint_is_rejected` does not assert the exact span (the code reports the endpoint ref span, so SC4 holds, but a span regression would not be caught by that test).

**Context:** Verified in phase acceptance checks.
**Source:** 83-VERIFICATION.md

## Patterns

### Internals are pub(crate) so tdd-guard only gates the thin public API; ev...
Internals are pub(crate) so tdd-guard only gates the thin public API; every module with value-asserting inline tests

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-01-SUMMARY.md

---

### Messages use 'expected X, found Y' with backticked tokens, e.g
Messages use 'expected X, found Y' with backticked tokens, e.g. expected `:`, found string "A"

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-01-SUMMARY.md

---

### Panic-free slicing via span::slice (str::get) plus crate-level deny(unwr...
Panic-free slicing via span::slice (str::get) plus crate-level deny(unwrap/expect/panic)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-01-SUMMARY.md

---

### Size gate before lexing; LineIndex built once per call
Size gate before lexing; LineIndex built once per call

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-01-SUMMARY.md

---

### Hoisted two-pass resolution with BTreeMap scopes (no hash collections)
Hoisted two-pass resolution with BTreeMap scopes (no hash collections)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-01-SUMMARY.md

---

### Statement-boundary resync (newline or }) so independent errors are all r...
Statement-boundary resync (newline or }) so independent errors are all reported

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-01-SUMMARY.md

---

### Trivia attach rules
Trivia attach rules: closing-line comment of a container goes to ContainerDecl.close_trailing (Item.trailing stays None); first entry of a block / first comment of leading|trailing is never blank_before

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-02-SUMMARY.md

---

### Parser unit tests live in parser/tests.rs (declared #[cfg(test)] mod tes...
Parser unit tests live in parser/tests.rs (declared #[cfg(test)] mod tests from parser/mod.rs) to stay under the 400-line file budget

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-02-SUMMARY.md

---

### Types-only modules (sequence_ir) expose no pub fn so tdd-guard has nothi...
Types-only modules (sequence_ir) expose no pub fn so tdd-guard has nothing to flag

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-03-SUMMARY.md

---

### Fixture harness
Fixture harness: first line `// expect: code, code`, sorted multiset compare, spans checked in range and on char boundaries

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-04-SUMMARY.md

---

### Echoed user text goes through diag::clip (40 chars plus an ellipsis)
Echoed user text goes through diag::clip (40 chars plus an ellipsis)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-04-SUMMARY.md

---

### Golden harness
Golden harness: once(src) helper + explicit fmt(&a) == a idempotence assert per case

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-05-SUMMARY.md

---

### Independent oracle
Independent oracle: expected IR built from the model with no parser/resolver call

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-05-SUMMARY.md

---

### Fix tests assert (original text at the fix span, replacement) pairs thro...
Fix tests assert (original text at the fix span, replacement) pairs through the public API

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-06-SUMMARY.md

---

### Repairable-set harness
Repairable-set harness: every fixture must be invalid, repair to valid in <= 5 rounds, be stable under a second repair, and match a snapshot

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-06-SUMMARY.md

---

### Mermaid catalogue tests assert (original text at fix span, replacement) ...
Mermaid catalogue tests assert (original text at fix span, replacement) pairs through the public API

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-07-SUMMARY.md

---

### Corpus-shaped fixtures
Corpus-shaped fixtures: invalid at start, valid within 5 rounds, stable under a second repair, snapshot of the repaired text

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-07-SUMMARY.md

---

### S4 sites are located with the AST (never by text search), so a -> inside...
S4 sites are located with the AST (never by text search), so a `->` inside a label or comment is never corrupted

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 83-07-SUMMARY.md

## Surprises

### Estimate variance on 83-01
Estimate variance on 83-01: planned 80000 tokens vs 12500 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-01-PLAN.md, 83-01-SUMMARY.md

---

### Estimate variance on 83-02
Estimate variance on 83-02: planned 90000 tokens vs 14200 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-02-PLAN.md, 83-02-SUMMARY.md

---

### Estimate variance on 83-03
Estimate variance on 83-03: planned 45000 tokens vs 3100 actual (0.07x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-03-PLAN.md, 83-03-SUMMARY.md

---

### Estimate variance on 83-04
Estimate variance on 83-04: planned 95000 tokens vs 20000 actual (0.21x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-04-PLAN.md, 83-04-SUMMARY.md

---

### Estimate variance on 83-05
Estimate variance on 83-05: planned 85000 tokens vs 9500 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-05-PLAN.md, 83-05-SUMMARY.md

---

### Estimate variance on 83-06
Estimate variance on 83-06: planned 85000 tokens vs 13000 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-06-PLAN.md, 83-06-SUMMARY.md

---

### Estimate variance on 83-07
Estimate variance on 83-07: planned 95000 tokens vs 19500 actual (0.21x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 83-07-PLAN.md, 83-07-SUMMARY.md

