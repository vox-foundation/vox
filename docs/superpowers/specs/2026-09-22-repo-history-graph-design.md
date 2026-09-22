---
title: "Repo History Graph (vox graph history)"
description: "File-level, first-parent history layer on graphify: lineage through splits/merges, per-file mechanical flags, lazy catch-up, and focus/forgotten/log/search queries for humans and LLMs."
category: "architecture"
status: "proposed"
date: 2026-09-22
---

# Repo History Graph (`vox graph history`)

## 1. Why

5,801 commits (≈4,300 agent-authored) are only readable as raw `git log` text. We want
one navigable, searchable history layer that serves:

1. **LLM retrieval**: "how has X changed?" or "look at git for Y", answered from a structured index instead of raw diffs.
2. **Honesty checks**: grounding README, docs, and CHANGELOG claims in what actually changed and when.
3. **Focus**: a sense of where effort went over time.
4. **Forgotten areas**: surfacing important code that hasn't had a meaningful change in a long time.
5. **Token spend** (future): attributing spend to change. Deferred, see §9.

## 2. Decisions (from design interview, 2026-09-21/22)

| Decision | Choice | Reason |
|---|---|---|
| Display unit | **File** nodes, change-over-time edges | TOESTUB keeps 92% of files ≤500 lines, so a file is already a coherent unit |
| Symbols | Used **only for lineage detection** | Avoids making history symbol-granular |
| Lineage | **Kept** through renames, splits, and merges | Defactoring is continuous: files grow past the cap, then get trimmed |
| Lineage detection | **Symbol migration first, line matching as fallback** | git `-C -C` finds nothing for splits (see `05e42775a`) |
| Refs | **`main` only, first-parent** (1,981 commits) | History is immutable and append-only |
| Freshness | **Lazy catch-up on every query**, no hook or CI | Hooks and CI jobs are what silently died (claim ledger, effort audit) |
| Mechanical changes | **Flagged per file, never dropped** | A misclassification can be fixed by re-flagging; mixed commits are common |
| Lint-driven body edits | **Real, stays real** (measured 2026-09-22) | Per-hint counts over 2,025 commits: `fmt` 73 (17.8% already mechanical), `lint` 51 (2.0% already mechanical), `regen` 47 (34.0% already mechanical), `hakari` 2 (0%). Sampled 20 of the 50 non-mechanical `lint`-hinted commits and read each with `git show --stat`: 11/20 were pure lint fixes with no behaviour change (clippy rewrites, doc frontmatter, dead comment/attribute removal); 9/20 were not — several add a *new* lint rule to the compiler (`lint.effect.unresolvable_deps`, `lint.handler.uncancellable_async`, wiring the MENS-decorator lint into the CLI pipeline), change the doc-lint tool's own behaviour (widening it to `.mdx`, changing its category vocabulary), drop an existing gate (`config-hygiene`), or fix a real classification bug alongside the lint cleanup. 11/20 is well short of the 18/20 bar, so the `lint` subject hint does **not** get promoted to `mechanical` in v2 — it stays a hint only, no `SCHEMA_VERSION` bump. |

**Built pending owner confirmation** (implemented as the default; owner can override later):
- Merge commits keep the subjects of the commits inside the merged branch as searchable text
  (`inner_subjects`).
- Storage is append-only JSONL, not `vox-db` (§4).

Recommended but **not yet confirmed by the owner**:
- SessionStart one-line brief (§7) — gated separately on explicit owner approval before the
  `.claude/settings.json` hook is added.

## 3. Precondition: graphify symbol identity (done on a branch)

The v4 extractor fix is `ff50ba1e7` on `worktree-agent-a90351cff459543c0`:
- Adds method, enum, trait, and type nodes.
- IDs are container-qualified: `<path>::<container>*::<name>[#n]`.
- Zero duplicate IDs; graph goes from 33k to 46k nodes.

**It can't merge yet.** It leaves `crates/vox-graph-reader/src/ast.rs` at 584 non-blank lines and
`rebuild.rs` at 738, over the TOESTUB 500-line error limit. Split both first.
Lineage detection relies on the `<container>::<name>` suffix being stable when a symbol
moves between files; the fix provides exactly that.

## 4. Storage

Corpus id `repo-history`, stored at `<git-common-dir>/vox-cache/repo-history/s<SCHEMA>-e<EXTRACTOR>/`,
shared by every worktree of the repo (not per-worktree). Each `(schema_version, extractor_version)`
pair gets its own version directory, so a version bump never wipes a still-valid store; sibling
version directories unused for 30 days are pruned. The store is append-only, and records are keyed
by commit SHA.

```text
state.json      { schema_version, extractor_version, last_sha }
commits.jsonl   one row per first-parent commit
changes.jsonl   one row per (commit, file)
lineage.jsonl   one row per (commit, from_path, to_path)
```

**Tip and validity.** The tip is `origin/main`, falling back to local `main` when that ref doesn't
exist. `load` establishes validity by walking the first-parent `parent` chain back from
`state.last_sha`: rows are valid only if they lie on the chain that ends at the current tip. If the
tip has been rewritten (force-pushed, rebased) so `last_sha` is no longer an ancestor, the store
rolls back to the newest ingested commit that is still an ancestor of the new tip, discarding rows
past it, rather than wiping everything. A full reset happens only when no ingested commit is an
ancestor of the tip, or the store itself is corrupt.

- **Size.** About 21k first-parent code-file changes plus the other file types comes to a few MB. Load everything into memory per query.
- **Why not `vox-db`.** Every query is a scan or group-by over at most a few hundred thousand rows. A DB
  would add a schema, migrations, and a crate edge for no measurable gain.
  `ponytail:` in-memory scan; move to vox-db tables if a query exceeds ~200 ms.
- **Git access.** `vox-graph-reader` is L0 and `vox-git` is L1, so graph-reader can't depend on vox-git.
  v1 runs `git` as an argv subprocess (no shell). Moving to `vox-git`/gix later
  needs a user-approved crate-edge entry, so we don't do it now.

### 4.1 Records

```text
Commit  { sha, parent, ts, author, agent (from Co-Authored-By), subject, body,
          is_merge, inner_subjects: [str] (merge^1..merge^2), mechanical: bool
          (= every file mechanical), subject_hint: fmt|lint|regen|hakari|null }

Change  { sha, path, status: A|M|D|R, added, deleted,
          symbols: { added: [id], removed: [id] } | null (unparseable),
          mechanical: bool, reasons: [generated|whitespace|symbol_neutral] }

Lineage { sha, from, to, kind: rename|split|merge|move,
          method: git_rename|symbol|line, weight: 0..1, evidence: u32 }
```

## 5. Ingest

`catch_up()` runs first in every history query. It first acquires `ingest.lock` in the version
directory — a file holding a random token from `std::hash::RandomState`, deleted only by the token's
owner — so parallel agent sessions can't double-append; if the lock is already held, `catch_up`
returns `CatchUp::Busy` and the query answers from whatever is already on disk.

1. Read `state.json` for this `(schema_version, extractor_version)` version directory; if it doesn't
   exist, this is a fresh build for that version. Otherwise apply the tip/validity rollback rule
   from §4: roll back to the newest ingested commit still an ancestor of the tip, or fully reset only
   when no ingested commit is an ancestor or the store is corrupt.
2. `git rev-list --first-parent --reverse <last_sha>..main`.
3. For each commit:
   - Get numstat and name-status (with `-M` for git renames) against the first parent.
   - For merges, also collect `inner_subjects` from `merge^1..merge^2`.
4. For each changed file:
   - Classify mechanical (§5.1).
   - For parseable files (`rs ts tsx js jsx py`), run the v4 extractor on the before and after
     blobs, touched files only, and diff the symbol sets and per-symbol body hashes.
5. Detect lineage (§5.2).
6. Append the rows, then atomically rewrite `state.json` last.
   A crash mid-append means rows past `last_sha` get truncated on the next start.

### 5.1 Mechanical classification (per file)

A file change is `mechanical` if **any** of these reasons applies:

- `generated`: `git check-attr --source=<sha> linguist-generated` evaluated against the commit's own
  tree (not the working tree) reports it as set. This covers `*.generated.md`, `Cargo.lock`,
  and `contracts/reports/*.v1.json`. `.gitattributes` is already the SSOT for these.
- `whitespace`: `git diff -w --numstat` for the file reports `0 0`. Applies only to the allowlisted
  extensions `rs ts tsx js jsx json toml css html sql`.
- `symbol_neutral`: **Rust only** (TS and Python would need tree-sitter token normalization, deferred).
  The file is parseable and its whole-file normalized token text is unchanged: `use` items and
  `#[doc]`/`#[derive]` attributes are stripped, then the remaining token text is compared as a whole.
  There is no per-symbol hash. `use` items are not symbols, so import churn is neutral.

`subject_hint` records what the commit message claims. It is **never** used to set `mechanical`
in v1. After backfill, compare hint against classification to decide whether lint edits should become mechanical.

### 5.2 Lineage detection (per commit)

- **Sources**: files deleted, or that lost at least 40 lines.
- **Targets**: files added, or that gained at least 40 lines.
- **Eligibility**: generated files, and files whose change is flagged mechanical `M`, are excluded
  from lineage entirely. The line fallback (step 3) only pairs files sharing the same extension.
  A line edge also needs `matched/new(T) ≥ 0.3`, in addition to the deleted-side weight. Lines that
  appear in 3 or more eligible files are dropped before matching, so common boilerplate (a closing
  brace, an empty match arm) can't manufacture a false lineage edge.

1. **Git renames** (`R` status): emit `rename`, method `git_rename`, weight = git's similarity score.
2. **Symbol migration** (parseable sources): take each symbol removed from source S whose
   `<container>::<name>` suffix appears as added in target T. Weight = moved / symbols in S before the commit.
   Emit when moved ≥ 2 or weight ≥ 0.2.
3. **Line fallback** (unparseable files, or source lines left unexplained after steps 1–2):
   - Normalize lines: trim, then drop blank lines, lone braces, `use`/`import`, `#[derive`, and license headers.
   - Match S's deleted lines against T's added lines.
   - Weight = matched / deleted(S). Emit when weight ≥ 0.3 and at least 15 lines match.
   - `ponytail:` fixed thresholds; tune on the known split set (§8) if precision is poor.
4. **Kind**: one source to two or more targets is `split`; two or more sources to one target is `merge`; one-to-one is `move`.

A file's **history** is its own change rows plus, recursively, those of its lineage ancestors. Each
ancestor's rows are weighted by its lineage edge weight.

## 6. Queries

Queries are available as CLI `vox graph history <sub>` and, over MCP, as the **single** tool
`vox_search_history`, dispatched by a `query` discriminator (not one MCP tool per subcommand). The
CLI prints text; `--json` or MCP returns JSON. Every MCP answer carries `{complete, behind, rows}`
so a caller can tell a fully caught-up answer from a partial one. **Every view excludes mechanical
changes by default**; `--include-mechanical` opts back in.

| Query | Answers | Output |
|---|---|---|
| `log <path>` | How has this file (and what it was split or renamed from) changed? | Commits with subject, agent, lines, lineage hops. Merge rows list up to 5 inner subjects that touched the path. |
| `focus [--since 30d] [--by crate\|dir]` | Where did effort go? | Non-mechanical churn per area, ranked. Fan-in counts only crate pairs declared in `contracts/ci/crate-edges.allow.v1.json`, cached. |
| `timeline [--since] [--bucket week]` | Sense of change over time | Top areas per bucket, plus merge and feat subjects |
| `forgotten [--min-age 60d]` | What important code have I not touched? | Areas ranked by age since last meaningful change × current fan-in (from `repo-code-graph`) |
| `search <text>` | Where is the commit that mentioned X? | Term counting over subject, inner merge subjects, body, touched paths, and symbol IDs. v1 reuses the graphify lexical scorer; semantic search goes through `vox-search` later (AGENTS §Agent Skills search rule) |

`forgotten` multiplies by fan-in so that a dormant leaf utility doesn't outrank a dormant
module that 40 others depend on.

## 7. Visibility

The claim ledger and the effort audit both died because nobody ever saw their output. So:
- `brief` never ingests — it never runs `catch_up()`, only reads what's already on disk, and reports
  "N commits behind" instead of catching up itself. This keeps it fast and side-effect-free enough to
  run at session start.
- Add one SessionStart line to `.claude/settings.json`: `vox graph history brief` prints the top 3 focus areas
  from the last 7 days and the single highest-ranked forgotten area.
  - Budget: 1 s.
  - If the store is behind, print `history: N commits behind (vox graph history brief)` and exit 0. It never blocks session start.
  - **Gated on owner approval.** Editing `.claude/settings.json` to add this hook requires explicit
    owner approval in chat before an agent makes the change (AGENTS §Global Constraints).
- The MCP tool (`vox_search_history`) makes history reachable from any agent session without a human
  remembering the command.

## 8. Tests (write first)

- **Mechanical**: a fixture repo (tmp git) with
  - an fmt-only commit (whitespace),
  - a lockfile-only commit (generated),
  - a `use`-reorder commit (symbol_neutral),
  - a mixed commit where one real edit plus fmt drift in 2 files yields 1 real row and 2 mechanical rows, and commit `mechanical=false`.
- **Lineage (symbols)**: split a 3-symbol file into 2 files and assert a `split` with weights summing to about 1.
- **Lineage (lines)**: the same split for a `.toml` or `.vox` file, via line fallback.
- **Lineage (merge)**: two files folded into one yields `merge`.
- **Real-history regression**: `05e42775a` (engine.rs → host/input/resolve) must yield `split` edges.
  `a7aaf48d4` (candle cuda+metal → core) must yield `merge` and `rename` edges.
- **Catch-up**:
  - A second query after one new commit ingests exactly one commit.
  - An `extractor_version` bump triggers a full rebuild.
  - A truncated `changes.jsonl` past `last_sha` is repaired.
- **First-parent**: a merge commit carries `inner_subjects`, and inner commits get no rows of their own.
- **Default filter**: `focus` omits mechanical rows; `--include-mechanical` restores them.

## 9. Deferred / out of scope

- **Token spend (use 5).** Needs an exact join key. Today's time-window matching in
  `vox-effort-audit/src/hybrid/transcripts.rs` returns `Ambiguous` under parallel sessions. Future work: a
  `Session-Id:` commit trailer stamped by the agent harness, joined in `Commit.agent`.
- **`.vox` symbol extraction.** graphify parses only `rs ts tsx js jsx py`. `.vox` files get line-fallback
  lineage only, until a `.vox` extractor exists.
- **Effort-audit visibility.** A separate problem: nothing runs `vox audit effort`. Track it on its own.
- **Non-`main` branches and worktrees.** Out of scope by decision.
- **Symbol-granular history views.** Out of scope; symbols are lineage evidence only.
- **Embeddings or semantic commit search.** Later, via `vox-search`.

## 10. Rollout

1. Split `ast.rs`/`rebuild.rs` under 500 lines, then land `ff50ba1e7`.
2. Store, ingest, and mechanical classification in `vox-graph-reader` (new `history/` module, each file ≤500 lines), with the §8 tests.
3. Lineage detection, then run the real-history regression tests.
4. The `log`, `focus`, `forgotten`, `search`, and `timeline` queries: CLI plus MCP.
5. The SessionStart brief.
6. Backfill once, then spot-check `focus --since 30d` against the known crate churn from 2026-09-21
   (vox-gui 654, vox-cli 227, orchestrator-mcp 163, …; note those numbers counted all reachable commits, not first-parent).
