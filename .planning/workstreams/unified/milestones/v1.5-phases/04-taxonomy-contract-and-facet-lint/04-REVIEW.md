---
phase: 04-taxonomy-contract-and-facet-lint
reviewed: 2026-10-11T00:00:00Z
depth: standard
files_reviewed: 38
files_reviewed_list:
  - crates/vox-kb-facets/src/lib.rs
  - crates/vox-kb-facets/src/frontmatter.rs
  - crates/vox-kb-facets/src/summary.rs
  - crates/vox-kb-facets/src/provenance.rs
  - crates/vox-kb-facets/src/taxonomy.rs
  - crates/vox-kb-facets/src/backlog.rs
  - crates/vox-kb-facets/Cargo.toml
  - crates/vox-kb-facets/tests/facet_schema_golden.rs
  - crates/vox-doc-pipeline/src/pipeline/facets.rs
  - crates/vox-doc-pipeline/src/pipeline/facets_tests.rs
  - crates/vox-doc-pipeline/src/pipeline/facet_backlog.rs
  - crates/vox-doc-pipeline/src/pipeline/lint.rs
  - crates/vox-doc-pipeline/src/pipeline/mod.rs
  - crates/vox-doc-pipeline/src/pipeline/types.rs
  - crates/vox-corpus/src/corpus/extract_docs.rs
  - crates/vox-corpus/src/corpus/extract_docs_facet_tests.rs
  - crates/vox-search/src/kb_ids.rs
  - crates/vox-search/src/library/chunk.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/tests/kb_library_facets.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-cli/src/commands/kb/facet_backlog.rs
  - crates/vox-cli/src/commands/ci/pre_push.rs
  - crates/vox-cli/src/commands/ci/run_body.rs
  - crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs
  - crates/vox-cli/src/commands/kb.rs
  - crates/vox-cli-ci/src/kb_facets_ts.rs
  - crates/vox-cli-ci/src/lib.rs
  - crates/vox-cli-ci/src/cmd_enums.rs
  - crates/vox-gui/ui/src/generated/kbFacets.generated.ts
  - crates/vox-gui/ui/src/__tests__/kbFacets.generated.test.ts
  - scripts/ssot-regen.vox
  - contracts/documentation/kb-taxonomy.v1.yaml
  - contracts/documentation/kb-facets.v1.schema.json
  - contracts/ci/crate-edges.allow.v1.json
  - contracts/kb/retrieval-baseline.v1.json
  - contracts/documentation/kb-facet-backlog.v1.yaml
  - contracts/index.yaml
findings:
  critical: 1
  warning: 5
  info: 6
  total: 12
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-10-11
**Depth:** standard
**Files Reviewed:** 38
**Status:** issues_found

## Summary

Reviewed the new L0 `vox-kb-facets` crate and its consumers: the doc lint, MENS doc extractor, Library indexer, MCP search tool, backlog gate, TS generator and contracts. Static review only (no builds).

Checked and found sound:
- The three crate-edge exceptions match D-09 exactly.
- `retrieval-baseline.v1.json` changed only in epoch, note and chunker. Metrics are untouched.
- No hashing is used in the L0 crate.
- `is_iso_date` and the slicing in `contains_token` cannot panic on multibyte input.
- Taxonomy label collisions are caught at load across pref, alt and deprecated labels and across domains and topics. `pref_label` is validated kebab-case, so the phrase appended by `expand_query` cannot inject query syntax.
- The backlog gate fails closed on a git failure, an unresolvable base, a deleted file or an invalid base file. It bootstraps only on a successful `ls-tree` that lists nothing.
- Rule ids come from an enum, not from doc content, so they cannot be spoofed. The two meta ids are not backloggable.
- Unknown backlog rule ids are rejected at load.
- Archive handling is consistent between the lint (`facets.rs:216`) and the MENS walker (`extract_docs.rs:553`).

The main defect is in the shared frontmatter splitter. It fails open for any doc whose frontmatter is not recognised, and the old substring fallback that used to catch `training_eligible: false` in those docs was deleted.

## Critical Issues

### CR-01: Unrecognised or unterminated frontmatter is treated as "no facets", so a doc stays MENS-eligible and the `training_eligible: false` opt-out is lost

**File:** `crates/vox-kb-facets/src/frontmatter.rs:23-45`, `crates/vox-kb-facets/src/lib.rs:135-142`, `crates/vox-corpus/src/corpus/extract_docs.rs:122-141`

**Issue:**
`frontmatter_range` and `frontmatter_yaml` return `None` unless line 1 is exactly `---` at byte 0, and the closing `---` is found. `parse_facets` maps `None` to `Facets::default()`, and `effective_training_eligible(default)` is `true`. These inputs therefore all become "grandfathered, eligible, no facets":

- a UTF-8 BOM before `---`;
- a leading blank line or space before `---`;
- an unterminated block (no closing `---`);
- CR-only line endings, because the splitter splits on `\n` only.

`read_utf8_path_capped` does not strip a BOM. `normalize_text` is only called in the indexer. The doc linter's own legacy check (`lint.rs:196`, `content.trim_start().starts_with("---")`) accepts the leading-whitespace case, so the two splitters disagree.

The previous `parse_frontmatter` handled exactly these shapes. When `!content.starts_with("---")` or `splitn(3, "---").len() < 3`, it fell back to `content.contains("training_eligible: false")` and returned ineligible. The new code deleted that fallback.

Consequences:

1. Regression: a doc with a BOM, leading blank or unterminated frontmatter that says `training_eligible: false` now trains. The `real_docs_eligibility_unchanged...` test only proves this for today's checked-in docs. There is no BOM, unterminated or leading-whitespace case in `extract_docs_facet_tests.rs`.
2. Bypass of TAX-06: `origin: web-promoted` or `authorship: llm-generated` without `human_attested_by` in such frontmatter is invisible to `forced_training_ineligible`, so the doc trains.
3. The facet lint cannot catch it for backlogged docs. The invisible facets just reproduce the already-excused `facet-missing-*` set, so an existing backlog doc edited this way stays lint-green (exit 0) and eligible. New docs are blocked only incidentally, by `facet-missing-topic` and similar hard errors.
4. `crates/*/README.md` and `crates/*/docs/**` are also walked by the MENS builder. No facet lint covers them, so there is no second line of defence there.

The code comment at `extract_docs.rs:122` ("Fail closed (TAX-06)") is false for these shapes.

**Fix:**
Make "looks like frontmatter but did not parse" an error rather than "none". For example, in `parse_facets`:

```rust
pub fn parse_facets(doc_text: &str) -> Result<Facets, String> {
    // Intent check: the first non-empty line (BOM and blanks tolerated) is a `---` fence.
    let intent = doc_text
        .trim_start_matches('\u{feff}')
        .lines()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| l.trim_end() == "---");
    match frontmatter_yaml(doc_text) {
        Some(y) if !y.trim().is_empty() => serde_yaml::from_str::<Facets>(y).map_err(|e| e.to_string()),
        Some(_) => Ok(Facets::default()),
        None if intent => Err("frontmatter fence present but block is unterminated, \
                               indented or preceded by a BOM/blank line".into()),
        None => Ok(Facets::default()),
    }
}
```

Also:
- Strip a leading BOM inside `frontmatter_range` and `frontmatter_yaml`, or normalize CRLF, CR and BOM at the top of `parse_facets`.
- In `extract_docs::parse_frontmatter`, keep the legacy `training_eligible: false` substring guard for the `None` branch.
- Add tests for BOM, leading blank line, unterminated block and CR-only input, each with `origin: web-promoted` and with `training_eligible: false`. All must return an empty pair list.

## Warnings

### WR-01: A re-epoch lets any rows be added, including rows for docs that did not exist at the base ref

**File:** `crates/vox-kb-facets/src/backlog.rs:166-177`, `crates/vox-cli/src/commands/kb/facet_backlog.rs:29-52`, `crates/vox-doc-pipeline/src/pipeline/facet_backlog.rs:78-92`

**Issue:**
When `head.epoch > base.epoch`, `tighten_only_violations` accepts anything as long as the note differs from the base note. Nothing checks who or what re-seeded.

An agent can run `--seed-facet-backlog --epoch-note "x"` and list its own brand-new, facet-less doc as excused. That is exactly the case D-03 says must never happen: "A new doc is never listed". The gate and the lint both pass. The only control is the prose rule "USER-AUTHORIZED-ONLY", which is the same discipline the repo says must be verified by mutation, not by convention.

**Fix:**
Make the gate enforce that a re-epoch still cannot admit new paths. In `run_facet_backlog_gate`, take the set of paths that exist at the base ref (`git ls-tree -r --name-only <base> -- docs/src`). On a re-epoch, flag any head row whose path is not in that set, or whose rule is new for a path already present in the base backlog. Admitting new paths at a re-epoch would then need a separate explicit marker that is not producible by the seed command.

### WR-02: `human_attested_by` is satisfied by any non-blank scalar, including `false`, `none`, `n/a`, `todo`

**File:** `crates/vox-kb-facets/src/lib.rs:147-158`

**Issue:**
`serde_yaml` 0.9 deserializes any plain scalar into `Option<String>`, so `human_attested_by: false`, `no`, `none`, `n/a`, `TBD`, `TODO`, `unknown` or `claude` all count as human attestation. Only `""` and whitespace are rejected. An LLM-authored page can pass the TAX-06 override with a placeholder it happens to emit.

**Fix:**
Treat a small set of sentinels as absent, case-insensitively, in both `forced_training_ineligible` and the lint. Suggested set: `true`, `false`, `yes`, `no`, `none`, `null`, `n/a`, `na`, `tbd`, `todo`, `unknown`. Add a test that `human_attested_by: false` on an LLM page stays ineligible.

### WR-03: Misspelled facet keys are silently ignored, so a typo defaults a page to eligible and unfaceted

**File:** `crates/vox-kb-facets/src/lib.rs:95-98`, `crates/vox-doc-pipeline/src/pipeline/facets.rs:381-391`

**Issue:**
`Facets` ignores unknown keys by design. Keys such as `training_eligble: false`, `human-attested-by`, `Origin:`, `authorship_model_` and `origin :` therefore have no effect, and neither the lint nor the MENS builder reports them. A near-miss of the opt-out key (`training_eligble: false`) leaves the page eligible, with no signal. The value typo (`origin: web-promotd`) is covered by a test, but the key typo is the symmetrical failure and is not.

**Fix:**
In `facet_issues`, parse the frontmatter as a mapping. For every key that is not a known facet but is within edit distance 1-2 of one, or equals a known facet case-insensitively or with `-` for `_`, emit a hard `FacetIssue::UnknownFacetKey { key, did_you_mean }`. Reuse the existing `suggest` helper. Add the rule to the backloggable ids.

### WR-04: `expand_query` odd-quote guard counts escaped quotes

**File:** `crates/vox-kb-facets/src/taxonomy.rs:208-210`

**Issue:**
`query.matches('"').count() % 2 == 1` counts `\"` as a quote. A query like `\"foo "` has two `"` but one real (unescaped) phrase opener. The guard passes, the expansion appends ` "topic x"` and closes or garbles the user's phrase. The lenient parser swallows the result, so it degrades recall rather than failing, and there is no injection because the appended text is validated. It is still an incorrect guard.

**Fix:**
Count only unescaped quotes, for example:

```rust
let mut real = 0; let mut esc = false;
for c in query.chars() {
    match (esc, c) { (true, _) => esc = false, (false, '\\') => esc = true, (false, '"') => real += 1, _ => {} }
}
if real % 2 == 1 { return query.to_string(); }
```

### WR-05: Fence tracking ignores fence type and length

**File:** `crates/vox-kb-facets/src/summary.rs:31-33,46-51`, `crates/vox-kb-facets/src/provenance.rs:62-64`

**Issue:**
`is_fence` toggles on any line starting with ``` or `~~~`. A `~~~` line inside a ``` block, or a 3-backtick fence inside a 4-backtick fence, flips the state. This is separate from the accepted "indented fence" limitation in `deferred-items.md`.

Effects:
- `inspect_summary` can report `Missing`, `Unterminated` or `Misplaced` for a valid doc, or count a marker inside a real fence.
- `quote_word_counts` can misclassify a blockquote as in-fence and drop it from the quote cap.

The MENS strip is fence-agnostic, so training is unaffected. Only the lint and the cap are.

**Fix:**
Track the opening fence character and its length, and close only on a fence of the same character with length greater than or equal to the opener (CommonMark). Share one helper between `summary.rs` and `provenance.rs`.

## Info

### IN-01: Leftover duplicate of the shared splitter in `vox-search`

**File:** `crates/vox-search/src/kb_ids.rs:65,79-81`

`line_spans` is still privately copied in `kb_ids.rs`, and `frontmatter_range` is now a pass-through wrapper. The "defactored-from vox-search" marker in the L0 crate is stale now that `vox-search` depends on it. Re-export `vox_kb_facets::frontmatter::line_spans` (make it `pub`) and delete the copy.

### IN-02: Seed tooling edge cases

**File:** `crates/vox-doc-pipeline/src/pipeline/mod.rs:257-259`, `crates/vox-doc-pipeline/src/pipeline/facet_backlog.rs:29`

- `--epoch-note=<text>` is not parsed. The note comes back empty, so the error message is misleading.
- `--epoch-note --paths=x` accepts the next flag as the note.
- `live_violations` uses `collect_md_files`, which, unlike `gather_md_files`, does not skip a locally generated, gitignored `docs/src/SUMMARY.md`. Seeding on a machine that has built the Astro site can add a row CI never sees. Skip `SUMMARY.md` and reject notes that start with `--`.

### IN-03: Smaller generator and CLI issues

**File:** `crates/vox-cli-ci/src/kb_facets_ts.rs:46-52`, `crates/vox-cli/src/commands/ci/run_body.rs:106-115`

- A `$ref` name is emitted without an `ident()` check or a check that it exists in `definitions`. A bad ref yields an undefined or garbage TS type despite the "closed over known shapes" claim.
- `KB_FACET_KEYS` order depends on `serde_json` `preserve_order` being unified into `vox-cli-ci`. Today the workspace hack guarantees it. A comment or an assertion would make the dependency explicit.
- `--kb-facets` silently shadows `--fields`.

### IN-04: MCP search warning leaks an absolute path and reads the file synchronously

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:121-123,147`

- `taxonomy_warning` embeds `{e:#}`, which includes the full repo path from `read <abs path>`. This is returned to MCP clients, so report only the relative `TAXONOMY_REL`.
- `load_taxonomy` does a blocking capped file read on the async path on every search. Cache it, or use `spawn_blocking`.

### IN-05: Indexer drops valid `topic` and `domain` card lines when any other facet fails to parse

**File:** `crates/vox-search/src/library/chunk.rs:192`

`parse_facets(&text).unwrap_or_default()` discards a valid `topic` and `domain` if, for example, `authorship: robot` is invalid. altLabel recall for that doc silently disappears. Parse only `topic` and `domain` leniently for the card, for example via a small `serde_yaml::Value` read.

### IN-06: "Promoted" and "risky" status is purely self-declared

**File:** `crates/vox-kb-facets/src/lib.rs:147-157`, `crates/vox-doc-pipeline/src/pipeline/facets.rs:323-377`

`forced_training_ineligible` and the sources and quote rules key only on `origin: web-promoted` and `authorship`. A page with `authorship: human`, populated `sources[]` with web URLs and no `origin` is eligible and escapes the quote cap. This matches TAX-06 as literally written, but consider a lint rule: a non-empty `sources[]` or any `https://` source URL requires `origin` to be set.

---

_Reviewed: 2026-10-11_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
