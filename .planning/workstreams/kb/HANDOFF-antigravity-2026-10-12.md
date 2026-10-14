# Handoff: finish the KB Phase 4 close-out (PR #645)

Written 2026-10-12 by Claude (Claude Code desktop tab) for the **Antigravity harness + Gemini Flash**.
Read this whole file once, then work the checklist in section 4 **in order**. Do not skip ahead,
and do not improvise outside it. When a step says STOP, stop and ask the user (Bertrand).

---

## 0. The one-paragraph version

Phases 1-3 of the KB ("Vox Knowledge Base") workstream are done and merged into the working
branch. **Phase 4 (Taxonomy Contract and Facet Lint) is fully implemented, reviewed, fixed and
pushed as draft PR #645, and hosted CI is green.** What is left is bookkeeping and one user
decision: close out the Phase 4 UAT, refresh a stale verification digest, mark the phase complete
in the GSD planning files, push, and (only if the user says so) mark the PR ready. There is also
an optional, ranked list of small follow-ups (section 5). **Nothing here needs new feature work.**

**Definition of done:** `gsd-tools query verification.status` for phase 4 returns `passed`,
`phase.complete 04` has run, the planning commit is pushed to the PR branch, CI on that head is
green, and the user has answered the TAX-07 question (section 6). Do NOT merge the PR and do NOT
mark it ready for review unless the user explicitly asks.

---

## 1. Where everything is

| Thing | Value |
|---|---|
| Repo worktree (work ONLY here) | `/Users/brbrainerd/dev/vox/.claude/worktrees/research-detection-chat-gui-05d826` |
| Branch | `claude/vox-knowledge-base-research-2964d2` (tracks `origin/<same name>`) |
| PR | https://github.com/vox-foundation/vox/pull/645, **draft**, base `main` |
| GSD workstream | `kb` (every GSD command takes `--ws kb`) |
| Phase dir | `.planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/` |
| GSD tool | `node ~/.claude/gsd-core/bin/gsd-tools.cjs query <verb> ... --ws kb` |
| Project rules (READ FIRST) | `AGENTS.md`, `CLAUDE.md` at the worktree root |
| Your own overlay | `GEMINI.md` at the worktree root |

Phase 4 files you will touch: `04-UAT.md`, `04-VERIFICATION.md`, `04-CONTEXT.md` (read-only),
`deferred-items.md`, plus the workstream `ROADMAP.md`, `STATE.md`, `state.json`, `REQUIREMENTS.md`.

Three untracked files exist in `.planning/workstreams/kb/` and **must not be committed or deleted**:
`config.json`, `milestone.lock`, `pr-body-draft.md`.

---

## 2. State snapshot (verified 2026-10-12, re-verify before acting)

- Local HEAD `cb88aae30`. **The remote branch is one commit ahead**: `ebf55524d`, from
  `github-actions[bot]` (the `ssot-autoregen` job). It rewrote
  `contracts/reports/test-inventory.v1.json` back to CI's version. So you must fetch and merge
  before you push (step A below).
- PR #645: `MERGEABLE` / `CLEAN`, draft. **24 checks pass, 1 skipped, 0 failing**: `scan`
  (gitleaks), `docs-quality`, `Linux (fmt, guards, clippy - affected)`, `Tests (affected,
  shard 1/2/3)`, `GUI UI (typecheck, vitest, Playwright)`, `Check, Build, and Test (Rust)`.
- Branch is 0 behind `main` as of the last fetch. `main` moves; re-check.
- Phase 4 status in GSD: all 8 plans done, `verification.status` = **`stale`** (and the file
  says `human_needed`). `04-UAT.md` has 3 items, all still `[pending]`.
- Phases 1-3: complete (Phase 3 was closed out earlier in this same tab).
- The installed `vox` binary is build 6778, current. Its git pre-push hook runs the **fast tier**
  (~30 s) and now passes. If the hook ever says the installed `vox` is stale, see section 7.

### What Phase 4 delivered (so you can describe it, not rebuild it)
New L0 crate `vox-kb-facets` (facet struct, taxonomy, summary block, `effective_training_eligible`);
`contracts/documentation/kb-taxonomy.v1.yaml` (11 domains, 55 topics);
committed JSON Schema `kb-facets.v1.schema.json` plus generated GUI type; a facet lint in
`vox-doc-pipeline` (19 `facet-*` rule ids); a generated shrink-only backlog
`kb-facet-backlog.v1.yaml` (667 docs) with a pre-push gate; MENS builder now uses the shared
eligibility rule and strips summary blocks; altLabel query expansion in `vox_kb_search`.
Locked user decisions are D-01..D-09 in `04-CONTEXT.md`. **Do not relitigate them.**

---

## 3. Hard rules (these are not suggestions)

1. **Never force-push. Never rebase.** To integrate remote changes use `git fetch` then
   `git merge origin/<branch>` (a real merge commit).
2. **Never use `git push --no-verify`** unless the user explicitly approves *that specific push*
   in chat. Approval for an earlier push does not carry over.
3. **Do not weaken or edit security guards to get green.** In particular leave alone:
   `crates/vox-db/tests/tiered_writer_invariant.rs`, the sanitizer in
   `crates/vox-db-types/src/kb_text.rs`, `contracts/kb/eval/corpus.jsonl` (frozen eval snapshot,
   canary-protected), and `.gitleaks.toml` (the two allowlisted SHAs are exact public Hugging Face
   revision SHAs; do not widen it).
4. **Crate-edge exceptions are user-only.** `contracts/ci/crate-edges.allow.v1.json` holds exactly
   three authorized `-> vox-kb-facets` exceptions (D-09). Never add another, and never regenerate
   a baseline/ledger to admit an edge you introduced. (`vox ci crate-edges --tighten` that only
   *removes* stale edges is always allowed.)
5. **Do not hand-edit generated files.** Regenerate them: one command regenerates everything the
   drift check verifies: `vox run scripts/ssot-regen.vox` (needs `VOX_SKIP_FRESHNESS_CHECK=1` only
   if the hook says stale). **Exception: do not commit `contracts/reports/test-inventory.v1.json`
   from a local regen.** CI's bot owns it and will overwrite it. If it shows as modified after a
   local regen, `git checkout -- contracts/reports/test-inventory.v1.json`.
6. **No new `.sh` / `.py` glue scripts** (project is VoxScript-first). Throwaway checks go in your
   scratch dir, never committed.
7. **Do not touch `docs/src/archive/`** and do not ingest it.
8. **Docs you author under `docs/src/` need YAML frontmatter** (`title`, `description`,
   `category`), and since Phase 4, also facets: `topic`, `domain`, `authorship`, plus a 40-120 word
   `<!-- kb:summary -->...<!-- /kb:summary -->` block directly under the H1. Check with
   `cargo run -q -p vox-doc-pipeline -- --lint-only --paths <path relative to docs/src>`.
   Planning files under `.planning/` do not need facets.
9. **Commit style:** imperative subject under 72 chars, body explains *why*. One logical change per
   commit. Add your own harness's attribution trailer if it has one.
10. **Local verification is lean on purpose.** Run `cargo check -p <crate>` and the specific tests
    you touched, not the whole workspace. Hosted CI is the real gate. Never run
    `cargo clippy --workspace` or workspace-wide tests locally.
11. **Secrets:** never write a real key/token anywhere. Use `.env.example` for placeholders.
12. If something needs a **product decision**, STOP and ask. Do not pick for the user.

---

## 4. The checklist (do these in order)

Run every command from the worktree root. Cargo is slow: run builds in the background and poll,
never block on a single command for minutes. For any cargo command use a scratch target dir
(another process runs `cargo clean` on `target/`):

```bash
export CARGO_TARGET_DIR=/private/tmp/antigravity-kb-target CARGO_INCREMENTAL=0
```

### A. Sync with the remote branch (the bot pushed a commit)
```bash
git status --short                # expect only the 3 untracked kb files
git fetch origin claude/vox-knowledge-base-research-2964d2 main
git log --oneline HEAD..origin/claude/vox-knowledge-base-research-2964d2   # expect the bot commit
git merge --no-edit origin/claude/vox-knowledge-base-research-2964d2
git log --oneline HEAD..origin/main                                        # expect empty
```
If `main` is ahead, merge it the same way (`git merge origin/main`). If that conflicts in more
than trivial ways, **STOP and report** the conflicting files. (Earlier, a 20-file conflict needed
careful hand resolution; `contracts/ci/crate-edges.allow.v1.json` is an append-only ledger, so
resolve it by UNION and keep every `exceptions` entry from both sides, then validate with
`jq . contracts/ci/crate-edges.allow.v1.json`.)

### B. Confirm CI is green on the current remote head
```bash
gh pr checks 645 --repo vox-foundation/vox
gh pr view 645 --json mergeable,mergeStateStatus,isDraft,headRefOid
```
Expect no failures and `MERGEABLE`. If something is red, go to section 8 (triage playbook), fix,
push, and re-check. Do not proceed to C until green.

### C. Close the Phase 4 UAT
`04-UAT.md` has three tests. Edit the file (frontmatter `status`, `updated`, each `result:`,
the `## Summary` counts, and replace the `## Current Test` body with `[testing complete]`):

1. **"Hosted CI is green on the phase PR"** -> `result: pass` **only if** step B shows green. Add a
   `reason:` line with the CI run URL.
2. **"GUI vitest and typecheck"** -> `result: pass` if the `GUI UI (typecheck, vitest,
   Playwright)` check on the PR is `pass` (it was). Add the check URL as `reason:`.
3. **"Decide TAX-07 `sources[]` scope"** -> **this is the user's decision. STOP and ask** with the
   exact question in section 6. Record their answer verbatim as `result: pass` plus
   `reason: "<their answer, date>"`. If they say the rule should become unscoped (also fail
   non-promoted pages), do NOT implement it in this PR; add a one-line item to
   `deferred-items.md` and note it in the UAT reason.

When all three have results, set `status: complete`, keep counts consistent (`total: 3`), and
commit:
```bash
node ~/.claude/gsd-core/bin/gsd-tools.cjs query commit "test(04): complete UAT" \
  --files .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-UAT.md --ws kb
```

### D. Refresh the stale verification digest
`verification.status` is `stale` because tracked "covered files" changed after the verifier ran.
The changes since verification were CI-fix only (list them in the note below). Re-fingerprint:
```bash
P=.planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint
G=~/.claude/gsd-core/bin/gsd-tools.cjs
# 1) list the covered files from the verification frontmatter
awk '/^covered_files:/{f=1;next} f&&/^  - /{sub(/^  - /,"");print;next} f&&!/^$/{exit}' $P/04-VERIFICATION.md > /tmp/covered.txt
wc -l < /tmp/covered.txt            # expect ~59
# 2) recompute the digest over that same list
node $G query verification.fingerprint $P $(tr '\n' ' ' < /tmp/covered.txt) --ws kb
# 3) write the printed covered_digest into the frontmatter, and set the status
node $G query frontmatter.set $P/04-VERIFICATION.md --field covered_digest --value "<the new v1:sha256:... value>" --ws kb
node $G query frontmatter.set $P/04-VERIFICATION.md --field status --value passed --ws kb
node $G query verification.status $P --ws kb            # must print "passed"
node $G phase uat-passed 04 --require-verification --ws kb   # must show "passed": true
```
Then append this section to the **body** of `04-VERIFICATION.md` (honesty note, required because
you are re-stamping rather than re-running the verifier):

> ### Post-verification changes (2026-10-11/12)
> Code-review fixes were verified in the original run. After it, only CI-driven fixes landed:
> clippy `derivable_impls` in `vox-corpus` `extract_docs.rs`; the `vox-db` legacy-export guard test
> moved out of the deprecated `codex_legacy` module into `local_tests.rs`; three SQL comments in
> `vox-db` `schema/domains/quarantine.rs` reworded; a `kb` parent entry added to
> `contracts/operations/catalog.v1.yaml` (+ regenerated registries, baseline fixture, reports);
> a gitleaks exact-SHA allowlist; facets added to `docs/src/architecture/terminal-session-host-design-2026.md`;
> a merge of `origin/main` (including a re-scoped answer-overlap scorer in `vox-cli-research`).
> No behavior of the Phase 4 contract changed. Hosted CI is green on the final head.

If `verification.fingerprint` complains a file in the list no longer exists, remove that line from
`covered_files` and add any new Phase-4 implementation file that is missing, then re-run.

### E. Mark the phase complete and commit the tracking files
```bash
node $G query phase.complete 04 --ws kb          # prints a JSON result; "warnings" about SUMMARY paths are harmless noise
git status --short
W=.planning/workstreams/kb
node $G query commit "docs(phase-04): complete phase execution" \
  --files $W/ROADMAP.md $W/STATE.md $W/state.json $W/REQUIREMENTS.md $P/04-VERIFICATION.md --ws kb
```
(Use the real `$P` and `$W` values from above. If `REQUIREMENTS.md` has no diff, drop it from
`--files`.) TAX-01..TAX-07 are already `[x] Complete`.

### F. Push (through the hook, no bypass)
```bash
git fetch origin claude/vox-knowledge-base-research-2964d2
git log --oneline HEAD..origin/claude/vox-knowledge-base-research-2964d2   # empty? if not, merge it (step A)
git push origin HEAD:claude/vox-knowledge-base-research-2964d2
```
If the hook fails, read the failing step in its output. Typical causes and fixes are in section 8.
After the push, **re-check CI** (step B). The `ssot-autoregen` bot may push one more commit;
if so, merge it and do not push again unless you have real changes.

### G. Report to the user
Send a short message: what is done, the PR URL, CI state, and the TAX-07 answer you recorded.
Ask whether they want the PR marked **ready for review** (`gh pr ready 645`). Do not do it
unprompted, and never merge.

---

## 5. Optional follow-ups (only if the user asks; ranked)

All logged in `04-REVIEW.md` / `deferred-items.md` / the Phase 3 review. None blocks Phase 4.

1. **Redact-before-sanitize ordering** (Phase 3 review's old WR-02; security-relevant, small).
   `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs` lines ~91-92 (`clean()`) and ~143-147:
   a secret split by a zero-width or tag character is not matched by the 20+ char redact regexes,
   then the sanitizer strips the invisible character and stores the secret in plaintext. Fix: call
   `redact_owned(&sanitize(raw))` (sanitize first, redact second) at both sites, test-first, with a
   test that a zero-width-split token is redacted.
2. **Indented-fence limitation** in `crates/vox-kb-facets/src/summary.rs` (`is_fence` uses
   `starts_with`; CommonMark allows up to 3 leading spaces). Test-first fix.
3. **Review INFO items IN-01..IN-06** in `04-REVIEW.md` (duplicate `line_spans` in `vox-search`,
   seed-tooling arg parsing, TS generator `$ref` validation, MCP `taxonomy_warning` embeds an
   absolute path and reads the taxonomy synchronously, indexer drops valid topic/domain card lines
   when another facet fails to parse, promoted status is self-declared).
4. **Review the answer-overlap scoring change.** In the main merge, `vox-search/src/evaluation.rs`
   helpers moved to `vox-cli-research/src/answer_overlap.rs::answer_keyword_hit_rate`, taking main's
   hardened rules (an empty/vacuous answer now scores **0.0**, not 1.0). That changes Phase 1 eval
   numbers; confirm with the user that is intended and, if so, re-baseline per the Phase 1 rules.
5. **Stale crate-edge baseline warning**: `vox ci crate-edges` warns `vox-gui -> vox-db-types`
   is gone; `vox ci crate-edges --tighten` is allowed (removal only).
6. **Pre-existing, not ours**: `vox-arch-check` fails on `no-hardcoded-abs-path` at
   `crates/vox-orchestrator-mcp/src/browser_tools.rs:965`; `cargo hakari verify` fails on
   main's `vox-orchestrator-models`. Report, do not fix, unless asked.
7. **Phase 3 accepted residuals (user chose "accept as pre-release")**: legacy `fact`/`concept`
   re-type and Library trust backfill only run below baseline 96 in `vox-db/src/store/open.rs`.
   Leave unless the user reverses that.
8. **Durable memory** (project rule in `AGENTS.md`/`~/.claude/CLAUDE.md`): consider adding short
   notes to `~/dev/memory/` for: the stale-installed-`vox` pre-push trap; the SSOT bot owns
   `test-inventory.v1.json`; `scripts/ssot-regen.vox` regenerates all drift at once. Follow the
   front-matter format in `~/dev/AGENTS.md`; check for an existing file first.

**Out of scope for this handoff:** Phases 5-12 of the milestone. Phase 5 needs a research step first
(`Research flag: YES`). Only start them if the user asks; the GSD entry point is
`/gsd-discuss-phase 5 --ws kb` (Claude Code skill) or reading `ROADMAP.md` directly.

---

## 6. The one question for the user (needed for UAT item 3)

> **TAX-07 `sources[]` completeness: promoted-only, or unscoped?**
> ROADMAP success criterion 1 says a doc fails the lint for a "`sources[]` entry without
> url/date/hash" (unscoped). REQUIREMENTS TAX-07 and your Phase 4 CONTEXT scope the required
> url/date/hash check to `origin: web-promoted` pages, and the code follows that (malformed
> date/hash fail on any page). Should I **keep it promoted-only** (and record that the ROADMAP
> wording is looser than the requirement), or **also hard-fail any page with an incomplete
> `sources[]` entry** (a follow-up, not part of this PR)?

Recommended default to offer: keep promoted-only. Do not assume; wait for their answer.

---

## 7. Environment gotchas (learned the hard way in this tab)

- **Stale installed `vox` + pre-push hook.** The hook links `vox-doc-pipeline` inside the installed
  `vox` binary. When that binary is older than the tree it can reject things the tree supports
  (once: category "Library"). It is currently build 6778 and fine. If it goes stale again:
  `cargo install --locked --path crates/vox-cli --force` (long release build; replaces the user's
  installed `vox`, so ask first), then push normally. Do not bypass with `--no-verify` without
  explicit approval. `VOX_SKIP_FRESHNESS_CHECK=1` only silences the freshness warning.
- **`target/` is unreliable** (another session runs `cargo clean`). Always use a scratch
  `CARGO_TARGET_DIR`. A build broker queues cargo builds; "queued (cap N)" is normal.
- **macOS BSD `sed`/`perl -0pi`**: avoid in-place regex edits. Use your editor tool or an exact-match
  replacement, and always verify with a `grep -c` before/after.
- **Generated-file drift cascades.** Adding one operation/command drifts several generated
  artifacts (registries, docs, reports, model manifest). Run `vox run scripts/ssot-regen.vox` once
  instead of fixing one hook error at a time. Remember rule 5 about `test-inventory.v1.json`.
- **Command-paths baseline fixture** is regenerated with
  `UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline`
  (the test intentionally panics after writing; that is expected). The file shows as binary in
  `git diff`; use `git diff --text`.
- **Doc facet lint is strict for new docs.** A doc added by `main` after the backlog was seeded
  fails `docs-quality` until it has facets (this happened once). Fix the doc; do not grow the
  backlog (that needs user authorization, D-04). For an LLM-assisted doc with no
  `human_attested_by`, `training_eligible` must be `false` (the lint rejects `true`).
- **nextest filters** match test *names*, not binaries. For an integration test use
  `cargo nextest run -p <crate> --test <binary-name>`.
- **A deprecated module's `#[test]` fns** trip `-D warnings` via libtest's generated const and an
  `allow` inside the module cannot cover it; put such tests in a non-deprecated file
  (`vox-db/src/local_tests.rs` has `#![allow(deprecated)]`).

---

## 8. CI triage playbook (if checks go red)

1. `gh pr checks 645 --repo vox-foundation/vox` -> note the failing names and URLs.
2. `gh run view <run-id> --job <job-id> --log-failed` (strip ANSI codes with
   `sed -E 's/\x1b\[[0-9;]*m//g'`).
3. Common failures and the fix:
   | Failing check | Usual cause | Fix |
   |---|---|---|
   | `scan` (gitleaks) | a 40-hex string tripped `sourcegraph-access-token` | confirm it is not a secret; allowlist the **exact value** in `.gitleaks.toml` with a comment (never a path-wide allow) |
   | `docs-quality` | a doc lacks facets or has a bad category | add facets (rule 8), or fix frontmatter |
   | `Linux (fmt, guards, clippy)` | fmt, a guard, or `clippy -D warnings` | `rustfmt --edition 2024 --config-path . <files>`; `cargo clippy -p <crate> --all-targets --locked -- -D warnings` |
   | `Tests (affected, shard N)` | a real test failure or generated-file drift | run that test locally by name; if it is a drift test, regenerate (section 7) |
   | `Check, Build, and Test (Rust)` | just mirrors the others | fix the others |
4. Fix, commit, run the push through the hook, then re-check. Never push a fix without having run
   the failing test or command locally first.

---

## 9. GSD cheat sheet (the planning system the repo uses)

`G=~/.claude/gsd-core/bin/gsd-tools.cjs`; all verbs take `--ws kb`.

| Need | Command |
|---|---|
| Is a phase verified? | `node $G query verification.status <phaseDir> --ws kb` |
| Did UAT + verification pass? | `node $G phase uat-passed 04 --require-verification --ws kb` |
| Mark phase done | `node $G query phase.complete 04 --ws kb` |
| Commit planning files | `node $G query commit "<msg>" --files <paths> --ws kb` |
| Set a frontmatter field | `node $G query frontmatter.set <file> --field <k> --value <v> --ws kb` |
| Roadmap overview | `node $G query roadmap.analyze --ws kb` |

If your harness cannot run `node`, the same effects are plain file edits: set `status:` in the
frontmatter of `04-VERIFICATION.md` / `04-UAT.md`, tick the Phase 4 box and set its progress row to
Complete in `ROADMAP.md`, and update `STATE.md`'s "Current Position". Then verify with `git diff`.

---

## 10. How to work (for a small, fast model)

- One step at a time. After each step, run its verification command and quote the output.
- Prefer reading the file over guessing. Use `git log`, `git diff --stat`, `gh pr checks` as ground
  truth, not this document, which can go stale (re-verify section 2 before acting).
- If a command's output contradicts this document, trust the output and tell the user.
- Keep diffs minimal and scoped to the step. Do not refactor, rename, or "tidy" anything.
- When unsure whether an action is allowed, check section 3, then ask.
- Final message to the user must state: what you did, what you verified (with the command), what
  is still pending on them, and the PR URL.
