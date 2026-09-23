# Repo History Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `vox graph history` (CLI plus the MCP tool `vox_search_history`). It gives a file-level, first-parent history of `origin/main` with lineage through renames, splits, and merges; per-file mechanical-change flags; and `log`/`focus`/`forgotten`/`search`/`timeline`/`brief` views. Every query except `brief` catches up on new commits before answering.

**Architecture:** A new `history/` module in `vox-graph-reader` (L0). It shells out to `git` using argv only, reuses the v4 symbol extractor to detect lineage, and stores append-only JSONL in **one store shared by every worktree**, namespaced by version: `<git-common-dir>/vox-cache/repo-history/s<SCHEMA>-e<EXTRACTOR>/`. Rows count as valid only if they are on the first-parent chain that ends at `state.last_sha`. The CLI (`vox-cli`) and MCP (`vox-orchestrator-mcp`) both call the single facade `history::api::answer`, and neither adds a new crate edge.

**Tech Stack:** Rust, `syn` + `proc-macro2` + `quote` (Rust normalization), tree-sitter (existing extractor), `serde_json`, the `git` CLI (**≥2.40**: `check-attr --source`, `cat-file --batch`, `rev-parse --path-format`, `trailers` format), clap.

**Spec:** `docs/superpowers/specs/2026-09-22-repo-history-graph-design.md`
**Grill record:** `.superpowers/review/2026-09-22-repo-history-graph-grill.md` (16 exchanges, consensus). Amendments from it are marked `<!-- AMENDED: G<n> — … -->`, where `n` is the exchange number.

## Global Constraints

- Every source file stays **≤500 non-blank lines, test modules included** (TOESTUB `arch/god_object`, Error at 500).
- **No new workspace crate edges.** `vox-graph-reader` must not depend on `vox-git`, `vox-config`, `vox-crypto`, or `vox-db`. New crates.io deps are allowed only if already in root `[workspace.dependencies]` (`proc-macro2 = "1"` is). No RNG crate: lock tokens come from `std::hash::RandomState`.
- **Test-first:** every new `pub fn` needs a `#[test]` in the **same file** (the pre-commit `tdd-guard` blocks the commit otherwise). Integration tests go in `crates/vox-graph-reader/tests/`.
- Run `git` with argv only, never a shell, and clear `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` so running inside a git hook can't retarget it. git **≥2.40**. <!-- AMENDED: G4 — check-attr --source -->
- The tip is `origin/main` when that ref exists, otherwise local `main`. Only its first-parent line is ingested. Catch-up never fetches. <!-- AMENDED: G2 — local main is rewritten -->
- **Mechanical rows are flagged, never dropped.** Every view excludes them unless `include_mechanical` is set.
- **Owner-gated:** do not edit `.claude/settings.json` (the SessionStart hook) without explicit owner approval in chat. <!-- AMENDED: G11 -->
- Build broker: run plain `cargo` from PATH. Never `cargo fmt --all`; use `cargo fmt -p <crate>`.
- Do not push or open PRs. Commit on the task branch.
- Commit messages: imperative subject under 72 chars, ending with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

## Spec deltas (this plan is authoritative where they differ; Task 0 updates the spec)

1. **Symbol-neutral classification** compares whole-file normalized token text: `use` items and `#[doc]`/`#[derive]` attributes are removed, then the text is compared. There is no per-symbol BLAKE3 hash.
2. `ChangeRec` drops `hunks` and `symbols.modified` because nothing reads them.
3. `search` counts terms over subject, inner merge subjects, body, and touched paths plus symbol IDs.
4. An `ingest.lock` file stops parallel agent sessions from double-appending. It holds a token, and only the token's owner deletes it. `CatchUp::Busy` means another process holds the lock, so the query answers from disk.
5. MCP exposes **one** tool, `vox_search_history`, with a `query` discriminator.
6. `symbol_neutral` applies to Rust only. TS and Python would need tree-sitter token normalization, which is deferred. <!-- AMENDED: G11 -->
7. **Store location:** `<git-common-dir>/vox-cache/repo-history/s<SCHEMA>-e<EXTRACTOR>/`, shared by all worktrees. Each version gets its own directory. Sibling version directories unused for 30 days are pruned. <!-- AMENDED: G1, G15 -->
8. **Tip and validity:**
   - The tip is `origin/main`, falling back to `main`.
   - `load` walks the first-parent `parent` chain back from `last_sha`.
   - A rewritten tip rolls the store back to the newest ingested commit that is still an ancestor. A full reset happens only when no ingested commit is an ancestor, or when the store is corrupt.
   <!-- AMENDED: G2, G3, G13 -->
9. **Classification rules:**
   - `generated` is evaluated against the commit's own tree (`check-attr --source=<sha>`).
   - `whitespace` applies only to `rs ts tsx js jsx json toml css html sql`.
   <!-- AMENDED: G4, G8 -->
10. **Lineage eligibility:**
    - Generated files, and mechanical `M` files, are excluded from lineage.
    - The line fallback only pairs files with the same extension.
    - A line edge also needs `matched/new(T) ≥ 0.3`.
    - Lines present in 3 or more eligible files are dropped before matching.
    <!-- AMENDED: G9 -->
11. **Query behaviour:**
    - `brief` never ingests and reports "N commits behind".
    - MCP runs with a 60 s budget, and every answer carries `{complete, behind, rows}`.
    - Fan-in counts only crate pairs declared in `contracts/ci/crate-edges.allow.v1.json`, and it is cached.
    - `log` merge rows list up to 5 inner subjects that touched the path.
    - The `subject_hint` verdict is decided in Task 10.
    <!-- AMENDED: G6, G7, G10, G14, G16 -->
12. **Built pending owner confirmation:** JSONL storage and merge `inner_subjects`. **Gated on owner approval:** the SessionStart hook. <!-- AMENDED: G11 -->

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/vox-graph-reader/src/ast_ts.rs` (new) | TS/JS/Python tree-sitter helpers moved out of `ast.rs` (TOESTUB) |
| `crates/vox-graph-reader/src/rebuild_resolve.rs` (new) | `resolve_edges` + its tests moved out of `rebuild.rs` (TOESTUB) |
| `crates/vox-graph-reader/src/history/mod.rs` | module list |
| `.../history/model.rs` | row types, `State`, `HistoryData`, `SCHEMA_VERSION`, `store_dir_name` |
| `.../history/store.rs` | JSONL append (torn-line safe), chain-walk load, atomic state, token lock, best-effort version pruning |
| `.../history/git.rs` | git subprocess wrappers, `-z` parsers, `BlobReader` |
| `.../history/classify.rs` | mechanical reasons, Rust normalization, subject hint |
| `.../history/lineage.rs` | symbol suffixes, normalized lines, split/merge/rename detection |
| `.../history/ingest.rs` | `ingest_commit`, `catch_up` (with rollback) |
| `.../history/query.rs` | `area_of`, `focus_between`, `log`, `forgotten`, `fan_in_by_area`, `allowed_crate_edges`, `search`, `timeline`, `brief` |
| `.../history/api.rs` | `HistoryQuery`, `answer` facade, store-dir resolution, fan-in cache, text rendering |
| `crates/vox-graph-reader/tests/common/mod.rs` | temp git repo fixture |
| `crates/vox-graph-reader/tests/history_git_tests.rs` | git plumbing against a fixture repo |
| `crates/vox-graph-reader/tests/history_ingest_tests.rs` | ingest, catch-up, rollback, real-history regressions (ignored) |
| `crates/vox-graph-reader/tests/history_api_tests.rs` | facade: complete/partial/brief/busy answers on a fixture repo |
| `crates/vox-graph-reader/tests/fixtures/history_carrier_pairs.txt` | hand-seeded golden crate pairs for the carrier merge; additions need owner review |
| `crates/vox-cli/src/commands/graphify/history.rs` | `vox graph history` clap + run |
| `crates/vox-orchestrator-mcp/src/history_tools.rs` | `vox_search_history` handler |

---

### Task 0: Workspace and docs

**Files:**
- Modify: `docs/superpowers/specs/2026-09-22-repo-history-graph-design.md`
- Add: this plan

**Interfaces:** Consumes: `origin/main` plus commit `ff50ba1e7` (graphify symbol-identity fix, currently on `worktree-agent-a90351cff459543c0`). Produces: branch `repo-history-graph` = `origin/main` + `ff50ba1e7` + both docs.

<!-- AMENDED: #1 — the fix branch's base 877406d84 is 17 commits behind origin/main (92d738d73), missing e.g. 86888c521 "clear 1.98 clippy wave", without which Task 8/9 clippy gates fail; branch from origin/main and cherry-pick the fix instead -->
- [ ] **Step 1: Create the worktree from `origin/main` and bring the graphify fix onto it**

```bash
git -C /Users/brbrainerd/dev/vox worktree add .worktrees/repo-history-graph -b repo-history-graph origin/main
cd /Users/brbrainerd/dev/vox/.worktrees/repo-history-graph
git cherry-pick ff50ba1e7
git log --oneline origin/main..HEAD            # exactly one commit: ff50ba1e7's cherry-pick
cargo test -p vox-graph-reader 2>&1 | tail -3  # all pass on the new base
```

If the cherry-pick conflicts, resolve it following AGENTS.md "Move + reformat = a duplicate definition". Check `grep -c 'fn <name>'` for each item and re-run the tests. From here on, "the base" means `origin/main`.

- [ ] **Step 2: Bring in the spec and plan (they are uncommitted in the main checkout)**

```bash
mkdir -p docs/superpowers/specs docs/superpowers/plans
cp /Users/brbrainerd/dev/vox/docs/superpowers/specs/2026-09-22-repo-history-graph-design.md docs/superpowers/specs/
cp /Users/brbrainerd/dev/vox/docs/superpowers/plans/2026-09-22-repo-history-graph.md docs/superpowers/plans/
```

- [ ] **Step 3: Apply "Spec deltas" 1–12 to the spec.**
  - In §4.1, delete `hunks` and `modified`.
  - In §4, replace the store path with delta 7, and state validity via chain walk (delta 8).
  - In §5, replace step 1 with the rollback rule (delta 8). Add the `ingest.lock` token sentence (delta 4).
  - In §5.1, replace the body-hash bullet with the normalized-token-text rule. Restrict `symbol_neutral` to Rust (delta 6), and add the commit-tree and allowlist rules (delta 9).
  - In §5.2, add the delta 10 eligibility rules.
  - In §6, change `search` to "term counting over subject/inner/body/paths/symbols". State that MCP is the single tool `vox_search_history`, returning `{complete, behind, rows}`. Add the fan-in filter and the `log` inner subjects.
  - In §7, `brief` never ingests. The SessionStart line is gated on owner approval.
  - In §2, move "inner subjects" and "JSONL" from "not yet confirmed" to "built pending confirmation".

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers
git commit -m "docs: add repo history graph spec and implementation plan"
```

---

### Task 1: Split `ast.rs` and `rebuild.rs` under the TOESTUB cap

`ff50ba1e7` left `ast.rs` at 584 non-blank lines and `rebuild.rs` at 738. This is a pure move with no behavior change.

**Files:**
- Create: `crates/vox-graph-reader/src/ast_ts.rs`, `crates/vox-graph-reader/src/rebuild_resolve.rs`
- Modify: `crates/vox-graph-reader/src/ast.rs`, `crates/vox-graph-reader/src/rebuild.rs`, `crates/vox-graph-reader/src/lib.rs`

**Interfaces:** Produces: no API change. `crate::ast::{extract_ast, extract_ast_in_module, extract_ast_in_module_with_wrappers, ExtractedNode, ExtractedEdge, ExtractedGraph, EXTRACTOR_VERSION}` are unchanged.

- [ ] **Step 1: Record the baseline**

```bash
for f in ast rebuild; do grep -cv '^\s*$' crates/vox-graph-reader/src/$f.rs; done   # expect 584, 738
cargo test -p vox-graph-reader 2>&1 | tail -3                                      # all pass
```

- [ ] **Step 2: Move the tree-sitter helpers.** Cut the functions `push_children`, `ts_fn_like`, `ts_class_like`, `ts_scope`, `ts_enclosing_fn`, `string_literal_value`, `arg_string_literal`, and `arg_object_string_field` (currently `ast.rs` ~lines 462–590, each `#[cfg(feature = "tree-sitter-grammars")]` where it is today) into `ast_ts.rs`. Make each one `pub(crate)`, and put this header at the top of the new file:

```rust
//! Tree-sitter (TS/JS/Python) helpers for `ast.rs`. Split out to stay under the
//! TOESTUB 500-line cap; no behavior of their own.
#![cfg(feature = "tree-sitter-grammars")]

use std::collections::HashMap;
```

Add `mod ast_ts;` to `lib.rs` next to `pub mod ast;`, and `use crate::ast_ts::*;` inside the `#[cfg(feature = "tree-sitter-grammars")]` block in `ast.rs` that calls them. If a moved helper uses `ast.rs`-private items (for example `IdAlloc`), make those items `pub(crate)` instead of moving them.

- [ ] **Step 3: Move edge resolution.** Cut `fn resolve_edges` (from its doc comment at ~line 83 through its closing brace at ~206; leave `RebuildMeta`'s doc comment at ~208 in place) <!-- AMENDED: #21 — 80–209 would orphan RebuildMeta's doc --> and the whole `#[cfg(test)] mod resolve_tests { … }` into `rebuild_resolve.rs`. Make the function `pub(crate) fn resolve_edges`, add `mod rebuild_resolve;` to `lib.rs`, and in `rebuild.rs` add `use crate::rebuild_resolve::resolve_edges;`. In the moved test module, change `use super::*;` so it still resolves: `super` is now `rebuild_resolve`, which is what the tests need.

- [ ] **Step 4: Verify the sizes and that behavior is unchanged**

```bash
for f in ast ast_ts rebuild rebuild_resolve; do echo "$f $(grep -cv '^\s*$' crates/vox-graph-reader/src/$f.rs)"; done   # every value ≤ 500
cargo test -p vox-graph-reader 2>&1 | tail -3        # same pass count as Step 1
cargo clippy -p vox-graph-reader --all-targets -- -D warnings
cargo run -q -p vox-code-audit --bin toestub -- crates/vox-graph-reader/ 2>&1 | grep -i 'too large' || echo "no oversized file"
```

<!-- AMENDED: #7 — the god_object detector also counts every `fn `/`impl ` line per file (>12); ast.rs has 48 today, so "no god_object" is unreachable. That count is non-blocking (pre-push enforce-warn fails only Critical); gate on file size only -->
Expected: all four files ≤500, tests still pass, clippy clean, and `no oversized file`. The detector's per-file `fn`/`impl` line count still reports on `ast.rs`. That report was there before this change and doesn't block anything.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader
git commit -m "refactor(graph): split ast/rebuild under the TOESTUB 500-line cap"
```

---

### Task 2: History model and store

<!-- AMENDED: G1, G3, G13, G15 — chain-walk validity, token-owned lock with touch, version-namespaced dirs + pruning -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/mod.rs`, `.../history/model.rs`, `.../history/store.rs`
- Modify: `crates/vox-graph-reader/src/lib.rs` (add `pub mod history;`)

**Interfaces:**
- Produces:
  - `history::model::{SCHEMA_VERSION: u32, CommitRec, ChangeRec, SymbolDelta, Reason, LineageRec, LineageKind, LineageMethod, State, HistoryData}`
  - `State::fresh() -> State`, `State::is_current(&self) -> bool`
  - `model::store_dir_name() -> String` (`"s<SCHEMA>-e<EXTRACTOR>"`)
  - `history::store::HistoryStore` with:
    - `open(&Path) -> io::Result<Self>`
    - `state() -> io::Result<State>`
    - `reset() -> io::Result<()>`
    - `append(&CommitRec, &[ChangeRec], &[LineageRec]) -> io::Result<()>`
    - `commit_state(&str) -> io::Result<()>`
    - `load() -> io::Result<HistoryData>`
    - `load_checked() -> io::Result<(HistoryData, bool)>` (`false` = broken chain)
    - `try_lock() -> io::Result<Option<IngestLock>>`
  - `IngestLock::touch(&self) -> io::Result<()>`
  - `store::prune_old_versions(root: &Path, keep: &Path, max_age: Duration) -> usize` (best effort, never fails)
  <!-- AMENDED: #19 — `repair` removed: the chain walk already ignores orphans, and a rewrite under a lock takeover could clobber a live writer's rows -->

- [ ] **Step 1: Write `history/mod.rs`**

```rust
//! First-parent repo history (spec: docs/superpowers/specs/2026-09-22-repo-history-graph-design.md).
pub mod model;
pub mod store;
```

Add `pub mod history;` to `crates/vox-graph-reader/src/lib.rs` now. <!-- AMENDED: #9 — declare modules before the red step so it fails for the right reason -->
(Each later task adds its own `pub mod …;` line **in its Step 1**, before writing tests.)

- [ ] **Step 2: Write `model.rs` with its failing tests first**

```rust
//! Row types for the repo-history store (spec §4.1).
use serde::{Deserialize, Serialize};

/// Bump when a row shape changes. Each version gets its own store directory.
pub const SCHEMA_VERSION: u32 = 1;

/// Version-namespaced store directory name. A binary only reads and writes rows its own
/// code produced, so builds from different branches never wipe each other's store.
pub fn store_dir_name() -> String {
    format!("s{SCHEMA_VERSION}-e{}", crate::ast::EXTRACTOR_VERSION)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommitRec {
    pub sha: String,
    /// First parent; `load` walks this chain to decide which rows are valid.
    pub parent: Option<String>,
    /// Committer time, unix seconds.
    pub ts: i64,
    pub author: String,
    /// First `Co-Authored-By` name, e.g. `Claude Opus 5`.
    pub agent: Option<String>,
    pub subject: String,
    pub body: String,
    pub is_merge: bool,
    /// Subjects of `merge^1..merge^2` (empty for non-merges). Built pending owner confirmation.
    pub inner_subjects: Vec<String>,
    /// True when every changed file is mechanical.
    pub mechanical: bool,
    /// What the subject claims (`fmt`, `lint`, `regen`, `hakari`); never sets `mechanical`.
    pub subject_hint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Generated,
    Whitespace,
    SymbolNeutral,
}

/// Symbol ids with the file path stripped (`Type::method`, `free_fn`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct SymbolDelta {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChangeRec {
    pub sha: String,
    pub path: String,
    pub old_path: Option<String>,
    /// git name-status letter: `A`, `M`, `D`, `R`, `C`, `T`.
    pub status: String,
    pub added: u32,
    pub deleted: u32,
    /// `None` when the file type is not parsed by the extractor.
    pub symbols: Option<SymbolDelta>,
    pub mechanical: bool,
    pub reasons: Vec<Reason>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageKind {
    Rename,
    Split,
    Merge,
    Move,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageMethod {
    GitRename,
    Symbol,
    Line,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LineageRec {
    pub sha: String,
    pub from: String,
    pub to: String,
    pub kind: LineageKind,
    pub method: LineageMethod,
    /// Share of `from` that went to `to` (0..=1).
    pub weight: f32,
    /// Symbols moved or lines matched.
    pub evidence: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub schema_version: u32,
    pub extractor_version: String,
    pub last_sha: Option<String>,
}

impl State {
    pub fn fresh() -> Self {
        State {
            schema_version: SCHEMA_VERSION,
            extractor_version: crate::ast::EXTRACTOR_VERSION.to_string(),
            last_sha: None,
        }
    }

    /// True when rows on disk were produced by the current code. In a version-namespaced
    /// directory a mismatch only means corruption.
    pub fn is_current(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
            && self.extractor_version == crate::ast::EXTRACTOR_VERSION
    }
}

#[derive(Clone, Debug, Default)]
pub struct HistoryData {
    pub commits: Vec<CommitRec>,
    pub changes: Vec<ChangeRec>,
    pub lineage: Vec<LineageRec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_is_current_and_stale_extractor_is_not() {
        assert!(State::fresh().is_current());
        let stale = State { extractor_version: "3".into(), ..State::fresh() };
        assert!(!stale.is_current());
    }

    #[test]
    fn enums_serialize_snake_case_and_dir_name_carries_versions() {
        assert_eq!(serde_json::to_string(&Reason::SymbolNeutral).unwrap(), "\"symbol_neutral\"");
        assert_eq!(serde_json::to_string(&LineageMethod::GitRename).unwrap(), "\"git_rename\"");
        assert_eq!(store_dir_name(), format!("s{SCHEMA_VERSION}-e{}", crate::ast::EXTRACTOR_VERSION));
    }
}
```

- [ ] **Step 3: Write `store.rs` tests first.** Start the file with only this test module so it fails to compile.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::model::*;
    use std::time::{Duration, SystemTime};

    fn commit(sha: &str, parent: Option<&str>) -> CommitRec {
        CommitRec { sha: sha.into(), parent: parent.map(Into::into), ts: 1, author: "a".into(), agent: None,
            subject: "s".into(), body: String::new(), is_merge: false, inner_subjects: vec![],
            mechanical: false, subject_hint: None }
    }
    fn change(sha: &str, path: &str) -> ChangeRec {
        ChangeRec { sha: sha.into(), path: path.into(), old_path: None, status: "M".into(),
            added: 1, deleted: 0, symbols: None, mechanical: false, reasons: vec![] }
    }

    #[test]
    fn load_follows_the_parent_chain_not_file_order() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        s.append(&commit("a", None), &[change("a", "x.rs")], &[]).unwrap();
        s.commit_state("a").unwrap();
        // Local cherry-picks p1, p2 get ingested, then the tip is rewritten.
        s.append(&commit("p1", Some("a")), &[change("p1", "p.rs")], &[]).unwrap();
        s.append(&commit("p2", Some("p1")), &[change("p2", "p.rs")], &[]).unwrap();
        s.commit_state("p2").unwrap();
        // Rollback to `a` without compaction, then the upstream commit u1 lands on `a`.
        s.commit_state("a").unwrap();
        s.append(&commit("u1", Some("a")), &[change("u1", "u.rs")], &[]).unwrap();
        s.commit_state("u1").unwrap();
        let d = s.load().unwrap();
        assert_eq!(d.commits.iter().map(|c| c.sha.as_str()).collect::<Vec<_>>(), ["a", "u1"]);
        assert_eq!(d.changes.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), ["x.rs", "u.rs"]);
    }

    #[test]
    fn overlapping_writer_duplicates_collapse_and_orphans_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        for _ in 0..2 {
            s.append(&commit("a", None), &[change("a", "x.rs")], &[]).unwrap();
        }
        s.commit_state("a").unwrap();
        s.append(&commit("b", Some("a")), &[change("b", "y.rs")], &[]).unwrap(); // state never advanced
        let d = s.load().unwrap();
        assert_eq!((d.commits.len(), d.changes.len()), (1, 1));
    }

    // AMENDED: #6 — the next append must not be glued onto a torn fragment.
    #[test]
    fn a_torn_line_does_not_swallow_the_next_append() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        s.append(&commit("a", None), &[change("a", "x.rs")], &[]).unwrap();
        s.commit_state("a").unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(dir.path().join("changes.jsonl"))
            .unwrap().write_all(b"{\"sha\":\"a\",\"pa").unwrap();
        s.append(&commit("b", Some("a")), &[change("b", "y.rs")], &[]).unwrap();
        s.commit_state("b").unwrap();
        assert_eq!(s.load().unwrap().changes.len(), 2, "b's row survives the torn line before it");
    }

    // AMENDED: #20 — the stale-lock takeover branch is otherwise untested.
    #[test]
    fn a_stale_lock_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        std::mem::forget(s.try_lock().unwrap().expect("lock")); // a crashed owner never releases
        let old = SystemTime::now() - Duration::from_secs(700);
        std::fs::File::options().write(true).open(dir.path().join("ingest.lock")).unwrap().set_modified(old).unwrap();
        assert!(s.try_lock().unwrap().is_some());
    }

    #[test]
    fn broken_chain_is_reported_and_torn_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        s.append(&commit("b", Some("missing")), &[], &[]).unwrap();
        s.commit_state("b").unwrap();
        assert!(!s.load_checked().unwrap().1, "chain does not reach a root commit");
        s.reset().unwrap();
        s.append(&commit("a", None), &[change("a", "x.rs")], &[]).unwrap();
        s.commit_state("a").unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(dir.path().join("changes.jsonl"))
            .unwrap().write_all(b"{\"sha\":\"a\",\"pa").unwrap();
        let (d, intact) = s.load_checked().unwrap();
        assert!(intact);
        assert_eq!(d.changes.len(), 1);
    }

    #[test]
    fn missing_state_is_fresh_and_reset_clears_everything() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        assert_eq!(s.state().unwrap(), State::fresh());
        s.append(&commit("a", None), &[], &[]).unwrap();
        s.commit_state("a").unwrap();
        s.reset().unwrap();
        assert_eq!(s.state().unwrap().last_sha, None);
        assert!(s.load().unwrap().commits.is_empty());
    }

    #[test]
    fn lock_is_exclusive_and_only_its_owner_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        let lock_file = dir.path().join("ingest.lock");
        let a = s.try_lock().unwrap().expect("first lock");
        assert!(s.try_lock().unwrap().is_none());
        // Simulate a takeover after a stall: another process now owns the file.
        std::fs::write(&lock_file, "other-process-token").unwrap();
        drop(a);
        assert!(lock_file.exists(), "a stalled owner must not delete the new owner's lock");
        std::fs::remove_file(&lock_file).unwrap();
        let b = s.try_lock().unwrap().expect("lock after release");
        b.touch().unwrap();
        drop(b);
        assert!(!lock_file.exists());
    }

    #[test]
    fn prune_removes_only_old_sibling_version_dirs() {
        let root = tempfile::tempdir().unwrap();
        let dir = |n: &str| {
            let d = root.path().join(n);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("state.json"), "{}").unwrap();
            d
        };
        let (keep, old, fresh) = (dir("s1-e4"), dir("s1-e3"), dir("s1-e2"));
        let (notes, lookalike) = (dir("notes"), dir("server-env")); // AMENDED: #15 — never prune non-version dirs
        let forty_days_ago = SystemTime::now() - Duration::from_secs(40 * 86_400);
        for d in [&old, &notes, &lookalike] {
            std::fs::File::options().write(true).open(d.join("state.json")).unwrap().set_modified(forty_days_ago).unwrap();
        }
        assert_eq!(prune_old_versions(root.path(), &keep, Duration::from_secs(30 * 86_400)), 1);
        assert!(!old.exists() && keep.exists() && fresh.exists() && notes.exists() && lookalike.exists());
    }
}
```

- [ ] **Step 4: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --lib history::store`
Expected: FAIL (compile errors: `HistoryStore` not found).

- [ ] **Step 5: Implement `store.rs` above the test module**

```rust
//! Append-only JSONL store for repo history (spec §4). Validity comes from the first-parent
//! chain: `load` walks `parent` back from `state.last_sha`, so rows from interrupted appends,
//! rolled-back commits, or overlapping writers are ignored regardless of file order.
use super::model::{ChangeRec, CommitRec, HistoryData, LineageRec, State};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, RandomState};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const COMMITS: &str = "commits.jsonl";
const CHANGES: &str = "changes.jsonl";
const LINEAGE: &str = "lineage.jsonl";
const STATE: &str = "state.json";
const LOCK: &str = "ingest.lock";
/// A lock untouched for this long is from a crashed ingest; `touch` runs after every commit.
const STALE_LOCK: Duration = Duration::from_secs(600);

pub struct HistoryStore {
    dir: PathBuf,
}

/// Held for one catch-up. The file stores a token, and `drop` removes it only when the token
/// is still ours, so a stalled owner never undoes a later takeover.
pub struct IngestLock {
    path: PathBuf,
    token: String,
}

impl IngestLock {
    /// Refresh the lock's mtime so a long backfill is not mistaken for a crashed one.
    pub fn touch(&self) -> io::Result<()> {
        OpenOptions::new().write(true).open(&self.path)?.set_modified(SystemTime::now())
    }
}

impl Drop for IngestLock {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path).is_ok_and(|t| t == self.token) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn new_token() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    // RandomState is seeded per process from OS randomness: no RNG dependency needed.
    format!("{}-{:016x}", std::process::id(), RandomState::new().hash_one(nanos))
}

impl HistoryStore {
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        Ok(Self { dir: dir.to_path_buf() })
    }

    pub fn state(&self) -> io::Result<State> {
        match fs::read_to_string(self.dir.join(STATE)) {
            Ok(s) => serde_json::from_str(&s).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(State::fresh()),
            Err(e) => Err(e),
        }
    }

    /// Delete all rows and the state (corrupt store, or no ingested commit survives a rewrite).
    pub fn reset(&self) -> io::Result<()> {
        for f in [COMMITS, CHANGES, LINEAGE, STATE] {
            match fs::remove_file(self.dir.join(f)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }

    /// Append one commit's rows. Follow with `commit_state`.
    pub fn append(&self, commit: &CommitRec, changes: &[ChangeRec], lineage: &[LineageRec]) -> io::Result<()> {
        append_rows(&self.dir.join(CHANGES), changes)?;
        append_rows(&self.dir.join(LINEAGE), lineage)?;
        append_rows(&self.dir.join(COMMITS), std::slice::from_ref(commit))
    }

    /// Atomically record `sha` as the last fully ingested commit.
    pub fn commit_state(&self, sha: &str) -> io::Result<()> {
        let st = State { last_sha: Some(sha.to_string()), ..State::fresh() };
        let tmp = self.dir.join("state.json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&st).map_err(io::Error::other)?)?;
        fs::rename(tmp, self.dir.join(STATE))
    }

    pub fn load(&self) -> io::Result<HistoryData> {
        Ok(self.load_checked()?.0)
    }

    /// Rows on the first-parent chain ending at `state.last_sha`, oldest first, deduplicated.
    /// The flag is `false` when `last_sha` is set but the chain never reaches a root commit.
    pub fn load_checked(&self) -> io::Result<(HistoryData, bool)> {
        let Some(last) = self.state()?.last_sha else {
            return Ok((HistoryData::default(), true));
        };
        let rows: Vec<CommitRec> = read_rows(&self.dir.join(COMMITS))?;
        let by_sha: HashMap<&str, &CommitRec> = rows.iter().map(|c| (c.sha.as_str(), c)).collect();
        let mut commits: Vec<CommitRec> = Vec::new();
        let mut cur = Some(last.as_str());
        let mut intact = false;
        while let Some(sha) = cur {
            let Some(c) = by_sha.get(sha) else { break };
            if commits.len() > rows.len() {
                break; // a parent cycle can only come from a corrupt file
            }
            commits.push((*c).clone());
            cur = c.parent.as_deref();
            intact = cur.is_none();
        }
        commits.reverse();
        let valid: HashSet<String> = commits.iter().map(|c| c.sha.clone()).collect();
        let mut seen = HashSet::new();
        let changes: Vec<ChangeRec> = read_rows::<ChangeRec>(&self.dir.join(CHANGES))?.into_iter()
            .filter(|r| valid.contains(&r.sha) && seen.insert((r.sha.clone(), r.path.clone())))
            .collect();
        let mut seen = HashSet::new();
        let lineage: Vec<LineageRec> = read_rows::<LineageRec>(&self.dir.join(LINEAGE))?.into_iter()
            .filter(|r| valid.contains(&r.sha) && seen.insert((r.sha.clone(), r.from.clone(), r.to.clone())))
            .collect();
        Ok((HistoryData { commits, changes, lineage }, intact))
    }

    /// `None` when another live process is ingesting.
    pub fn try_lock(&self) -> io::Result<Option<IngestLock>> {
        let path = self.dir.join(LOCK);
        for _ in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    let token = new_token();
                    f.write_all(token.as_bytes())?;
                    return Ok(Some(IngestLock { path, token }));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    match fs::metadata(&path).and_then(|m| m.modified()) {
                        Ok(t) if SystemTime::now().duration_since(t).unwrap_or_default() <= STALE_LOCK => return Ok(None),
                        Ok(_) => {
                            let _ = fs::remove_file(&path);
                        }
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e),
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }
}

/// `s<digits>-e<alnum/.>`: the only directory names `prune_old_versions` may delete.
fn is_version_dir(name: &str) -> bool {
    let Some((schema, ext)) = name.strip_prefix('s').and_then(|r| r.split_once("-e")) else { return false };
    !schema.is_empty() && schema.bytes().all(|b| b.is_ascii_digit())
        && !ext.is_empty() && ext.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
}

/// Best-effort delete of sibling version directories under `root`, other than `keep`,
/// whose `state.json` has not been written for `max_age`. Returns how many were removed.
/// Errors are skipped: housekeeping must never fail a query.
/// ponytail: age-based cleanup; move under `vox graph gc` if it ever misfires.
pub fn prune_old_versions(root: &Path, keep: &Path, max_age: Duration) -> usize {
    let Ok(entries) = fs::read_dir(root) else { return 0 };
    let mut removed = 0;
    for e in entries.flatten() {
        let p = e.path();
        if p == keep || !p.is_dir() || !is_version_dir(&e.file_name().to_string_lossy()) {
            continue;
        }
        let Ok(stamp) = fs::metadata(p.join(STATE)).or_else(|_| fs::metadata(&p)).and_then(|m| m.modified()) else { continue };
        if SystemTime::now().duration_since(stamp).unwrap_or_default() > max_age && fs::remove_dir_all(&p).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn append_rows<T: Serialize>(path: &Path, rows: &[T]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut f = OpenOptions::new().create(true).read(true).append(true).open(path)?;
    let mut buf = Vec::new();
    // A torn final line (crash mid-write) must not swallow the first new row.
    if ends_without_newline(&mut f)? {
        buf.push(b'\n');
    }
    for r in rows {
        serde_json::to_writer(&mut buf, r).map_err(io::Error::other)?;
        buf.push(b'\n');
    }
    f.write_all(&buf)
}

fn ends_without_newline(f: &mut File) -> io::Result<bool> {
    let len = f.metadata()?.len();
    if len == 0 {
        return Ok(false);
    }
    f.seek(SeekFrom::Start(len - 1))?;
    let mut last = [0u8; 1];
    f.read_exact(&mut last)?;
    Ok(last[0] != b'\n')
}

/// JSONL rows; unparseable lines (a torn final write) are skipped.
fn read_rows<T: DeserializeOwned>(path: &Path) -> io::Result<Vec<T>> {
    let f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        if let Ok(row) = serde_json::from_str(&line?) {
            out.push(row);
        }
    }
    Ok(out)
}
```

- [ ] **Step 6: Run the tests and confirm they pass**

Run: `cargo test -p vox-graph-reader --lib history:: && grep -cv '^\s*$' crates/vox-graph-reader/src/history/store.rs`
Expected: PASS (10 tests), and the line count ≤500. `RandomState::hash_one` and `File::set_modified` need Rust ≥1.76 and ≥1.75 respectively; the pinned toolchain is newer.

- [ ] **Step 7: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader/src/history crates/vox-graph-reader/src/lib.rs
git commit -m "feat(graph): add repo-history row model and chain-validated JSONL store"
```

---

### Task 3: Git plumbing

<!-- AMENDED: G1, G2, G4, G6, G14 — common dir, origin/main tip, check-attr --source, rev-list --count, ls-tree, per-path inner subjects -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/git.rs`, `crates/vox-graph-reader/tests/common/mod.rs`, `crates/vox-graph-reader/tests/history_git_tests.rs`
- Modify: `crates/vox-graph-reader/src/history/mod.rs` (add `pub mod git;`)

**Interfaces:**
- Produces:
  - `git::EMPTY_TREE: &str`
  - `git::Git::new(&Path)` with:
    - `rev_parse(&str) -> io::Result<String>`
    - `is_ancestor(&str, &str) -> bool`
    - `common_dir() -> io::Result<PathBuf>`
    - `default_tip() -> String`
    - `first_parent_range(Option<&str>, &str) -> io::Result<Vec<String>>` (callers use `.len()` for "commits behind")
    - `commit_meta(&str) -> io::Result<CommitMeta>`
    - `inner_subjects(merge: &str, path: Option<&str>) -> io::Result<Vec<String>>` (oldest first; `Some(path)` = only commits touching it)
    <!-- AMENDED: #7 — `impl Git` had 15 methods (TOESTUB cap 12): `count_range` dropped, `path_inner_subjects` folded into `inner_subjects`, `range` made a free fn -->
    - `file_deltas(Option<&str>, &str) -> io::Result<Vec<FileStat>>`
    - `generated(rev: &str, paths: &[String]) -> io::Result<HashSet<String>>`
    - `ls_tree(rev: &str) -> io::Result<HashSet<String>>`
  - `git::CommitMeta { sha, parents: Vec<String>, ts: i64, author, subject, body, co_authors: Vec<String> }`
  - `git::FileStat { path, old_path: Option<String>, status: String, score: Option<u8>, added: u32, deleted: u32, binary: bool, whitespace_only: bool }`
  - `git::BlobReader::spawn(&Path) -> io::Result<Self>`, `read(&mut self, rev: &str, path: &str, max_bytes: usize) -> io::Result<Option<String>>`
  - Test fixture: `common::Repo::{new, path, git, write, commit}`

Verified git 2.55 behavior these parsers rely on:
- `--numstat -z -M` writes `a\td\tpath\0` for a normal entry and `a\td\t\0old\0new\0` for a rename.
- `--name-status -z -M` writes `M\0path\0` or `R080\0old\0new\0`.
- `diff -w --numstat` **omits** files whose only change is whitespace.
- `cat-file --batch` answers `<oid> blob <size>\n<content>\n`, or `<name> missing\n`.
- `check-attr --source=<rev>` reads attributes from that tree (`git check-attr --source=HEAD linguist-generated -- Cargo.lock` → `true`).

- [ ] **Step 1: Write the fixture helper `tests/common/mod.rs`**

```rust
//! Temp git repo fixture for history tests.
#![allow(dead_code)]
use std::path::Path;
use std::process::Command;

pub struct Repo {
    pub dir: tempfile::TempDir,
}

impl Repo {
    pub fn new() -> Self {
        let r = Repo { dir: tempfile::tempdir().unwrap() };
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["config", "user.email", "t@example.com"]);
        r.git(&["config", "user.name", "Tester"]);
        r.git(&["config", "commit.gpgsign", "false"]);
        r.git(&["config", "core.hooksPath", "/dev/null"]);
        r
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) -> String {
        let o = Command::new("git")
            .arg("-C").arg(self.path()).args(args)
            .env_remove("GIT_DIR").env_remove("GIT_INDEX_FILE").env_remove("GIT_WORK_TREE")
            .output().unwrap();
        assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8(o.stdout).unwrap()
    }

    pub fn write(&self, rel: &str, content: &str) {
        let p = self.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    /// Stage everything (including deletions) and commit; returns the new HEAD sha.
    pub fn commit(&self, msg: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--allow-empty", "-m", msg]);
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }
}
```

- [ ] **Step 2: Write the failing integration tests `tests/history_git_tests.rs`**

```rust
mod common;
use common::Repo;
use std::collections::HashSet;
use vox_graph_reader::history::git::{BlobReader, Git};

#[test]
fn first_parent_range_skips_branch_commits_and_reads_inner_subjects() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c1 = r.commit("feat: one");
    r.git(&["checkout", "-q", "-b", "side"]);
    r.write("b.txt", "2\n");
    r.commit("wip: inner");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--no-ff", "side", "-m", "merge side"]);
    let m = r.git(&["rev-parse", "HEAD"]).trim().to_string();
    let g = Git::new(r.path());
    assert_eq!(g.first_parent_range(None, "main").unwrap(), vec![c1.clone(), m.clone()]);
    assert_eq!(g.first_parent_range(Some(&c1), "main").unwrap(), vec![m.clone()]);
    assert_eq!(g.inner_subjects(&m, None).unwrap(), vec!["wip: inner".to_string()]);
    assert_eq!(g.inner_subjects(&m, Some("b.txt")).unwrap(), vec!["wip: inner".to_string()]);
    assert!(g.inner_subjects(&m, Some("a.txt")).unwrap().is_empty());
    assert!(g.is_ancestor(&c1, &m));
}

#[test]
fn tip_count_tree_and_common_dir() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c1 = r.commit("one");
    r.write("b.txt", "2\n");
    r.commit("two");
    let g = Git::new(r.path());
    assert_eq!(g.default_tip(), "main", "no origin remote -> local main");
    r.git(&["update-ref", "refs/remotes/origin/main", &c1]);
    assert_eq!(g.default_tip(), "origin/main");
    assert_eq!(g.first_parent_range(None, "main").unwrap().len(), 2);
    assert_eq!(g.first_parent_range(Some(&c1), "main").unwrap().len(), 1);
    assert_eq!(g.ls_tree(&c1).unwrap(), HashSet::from(["a.txt".to_string()]));
    let other = tempfile::tempdir().unwrap();
    let wt = other.path().join("wt");
    r.git(&["worktree", "add", "-q", "-b", "side", wt.to_str().unwrap()]);
    let canon = |p: std::path::PathBuf| p.canonicalize().unwrap();
    assert_eq!(canon(Git::new(&wt).common_dir().unwrap()), canon(g.common_dir().unwrap()), "one store for every worktree");
}

#[test]
fn commit_meta_reads_co_author_trailer() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c = r.commit("feat: x\n\nbody line\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>");
    let m = Git::new(r.path()).commit_meta(&c).unwrap();
    assert_eq!(m.subject, "feat: x");
    assert!(m.parents.is_empty());
    assert_eq!(m.co_authors, vec!["Claude Opus 5 <noreply@anthropic.com>".to_string()]);
}

#[test]
fn file_deltas_report_rename_score_counts_and_whitespace_only() {
    let r = Repo::new();
    r.write("ws.rs", "fn a() {\n    1\n}\n");
    r.write("old.txt", "a\nb\nc\nd\ne\n");
    r.write("real.rs", "fn b() {}\n");
    let c1 = r.commit("init");
    r.write("ws.rs", "fn a() {\n  1\n}\n");
    r.git(&["mv", "old.txt", "new.txt"]);
    r.write("new.txt", "a\nb\nc\nd\ne\nf\n");
    r.write("real.rs", "fn b() { 2 }\n");
    let c2 = r.commit("two");
    let d = Git::new(r.path()).file_deltas(Some(&c1), &c2).unwrap();
    let by = |p: &str| d.iter().find(|f| f.path == p).unwrap().clone();
    let ren = by("new.txt");
    assert_eq!((ren.status.as_str(), ren.old_path.as_deref()), ("R", Some("old.txt")));
    assert!(ren.score.unwrap() >= 50);
    assert_eq!((ren.added, ren.deleted), (1, 0));
    assert!(by("ws.rs").whitespace_only);
    assert!(!by("real.rs").whitespace_only);
}

#[test]
fn root_commit_diffs_against_empty_tree() {
    let r = Repo::new();
    r.write("a.txt", "1\n2\n");
    let c = r.commit("init");
    let d = Git::new(r.path()).file_deltas(None, &c).unwrap();
    assert_eq!((d[0].status.as_str(), d[0].added), ("A", 2));
}

#[test]
fn generated_reads_the_commits_own_gitattributes() {
    let r = Repo::new();
    r.write(".gitattributes", "*.gen.md linguist-generated=true\nCargo.lock linguist-generated\n");
    let c = r.commit("attrs");
    // An uncommitted worktree edit must not change how commit `c` is classified.
    r.write(".gitattributes", "src/*.rs linguist-generated\n");
    let paths = ["x.gen.md".to_string(), "Cargo.lock".to_string(), "src/a.rs".to_string()];
    let g = Git::new(r.path()).generated(&c, &paths).unwrap();
    assert!(g.contains("x.gen.md") && g.contains("Cargo.lock") && !g.contains("src/a.rs"), "{g:?}");
}

#[test]
fn blob_reader_reads_text_and_reports_missing() {
    let r = Repo::new();
    r.write("a.txt", "hello\n");
    let c = r.commit("init");
    let mut b = BlobReader::spawn(r.path()).unwrap();
    assert_eq!(b.read(&c, "a.txt", 1 << 20).unwrap().as_deref(), Some("hello\n"));
    assert_eq!(b.read(&c, "nope.txt", 1 << 20).unwrap(), None);
    assert_eq!(b.read(&c, "a.txt", 2).unwrap(), None, "over max_bytes");
    assert_eq!(b.read(&c, "a.txt", 1 << 20).unwrap().as_deref(), Some("hello\n"), "stream still in sync");
}
```

- [ ] **Step 3: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --test history_git_tests`
Expected: FAIL (`history::git` not found).

- [ ] **Step 4: Implement `git.rs`, including its in-file parser tests**

```rust
//! `git` subprocess plumbing for repo history. argv only, never a shell.
//! `vox-graph-reader` is L0 and cannot depend on `vox-git` (L1); see spec §4.
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Git's well-known empty tree: the diff base of a root commit.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

pub struct Git {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitMeta {
    pub sha: String,
    pub parents: Vec<String>,
    pub ts: i64,
    pub author: String,
    pub subject: String,
    pub body: String,
    pub co_authors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileStat {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub score: Option<u8>,
    pub added: u32,
    pub deleted: u32,
    pub binary: bool,
    pub whitespace_only: bool,
}

/// Config pinned on every call so porcelain output doesn't depend on the host.
/// AMENDED: #10 — `log.showSignature` corrupts `git show` parsing, and `diff.algorithm`, renames and the
/// rename limit change numstat/status, so ingest would not be deterministic per SHA.
const PINNED: &[&str] = &[
    "-c", "log.showSignature=false", "-c", "diff.algorithm=myers", "-c", "diff.renames=true",
    "-c", "diff.renameLimit=32767", "-c", "core.quotePath=false",
];

/// `git -C root`, isolated from the caller: every inherited `GIT_*` variable (hooks set
/// `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_CONFIG_PARAMETERS`, …) is removed, and `PINNED` config applies.
fn git_cmd(root: &Path) -> Command {
    let mut c = Command::new("git");
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            c.env_remove(&k);
        }
    }
    c.arg("-C").arg(root).args(PINNED);
    c
}

fn lines(s: String) -> Vec<String> {
    s.lines().map(str::to_string).collect()
}

fn range(since: Option<&str>, tip: &str) -> String {
    since.map_or(tip.to_string(), |s| format!("{s}..{tip}"))
}

impl Git {
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf() }
    }

    fn run(&self, args: &[&str]) -> io::Result<String> {
        let out = git_cmd(&self.root).args(args).output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        String::from_utf8(out.stdout).map_err(io::Error::other)
    }

    pub fn rev_parse(&self, rev: &str) -> io::Result<String> {
        Ok(self.run(&["rev-parse", "--verify", &format!("{rev}^{{commit}}")])?.trim().to_string())
    }

    pub fn is_ancestor(&self, a: &str, b: &str) -> bool {
        git_cmd(&self.root).args(["merge-base", "--is-ancestor", a, b])
            .status().map(|s| s.success()).unwrap_or(false)
    }

    /// Absolute git common dir, shared by every worktree of the repository.
    pub fn common_dir(&self) -> io::Result<PathBuf> {
        Ok(PathBuf::from(self.run(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?.trim()))
    }

    /// `origin/main` when it exists (upstream history only grows), else local `main`.
    pub fn default_tip(&self) -> String {
        if self.rev_parse("refs/remotes/origin/main").is_ok() { "origin/main".into() } else { "main".into() }
    }

    /// First-parent commits after `since` (exclusive) through `tip`, oldest first.
    pub fn first_parent_range(&self, since: Option<&str>, tip: &str) -> io::Result<Vec<String>> {
        Ok(lines(self.run(&["rev-list", "--first-parent", "--reverse", &range(since, tip)])?))
    }

    pub fn commit_meta(&self, sha: &str) -> io::Result<CommitMeta> {
        let fmt = "--format=%H%x00%P%x00%ct%x00%an%x00%s%x00%b%x00%(trailers:key=Co-Authored-By,valueonly,separator=%x1f)";
        parse_commit_meta(&self.run(&["show", "-s", fmt, sha])?)
    }

    /// Subjects a merge brought in (`merge^1..merge^2`), oldest first; with `path`, only
    /// the commits that touched it.
    pub fn inner_subjects(&self, merge: &str, path: Option<&str>) -> io::Result<Vec<String>> {
        let span = format!("{merge}^1..{merge}^2");
        let mut args = vec!["log", "--reverse", "--format=%s", span.as_str()];
        if let Some(p) = path {
            args.extend(["--", p]);
        }
        Ok(lines(self.run(&args)?))
    }

    /// Per-file stats of `sha` against `parent` (empty tree for a root commit).
    pub fn file_deltas(&self, parent: Option<&str>, sha: &str) -> io::Result<Vec<FileStat>> {
        let base = parent.unwrap_or(EMPTY_TREE);
        let diff = |extra: &[&str]| -> io::Result<String> {
            let mut a = vec!["diff", "--no-ext-diff", "--no-textconv", "-M", "-z"];
            a.extend_from_slice(extra);
            a.extend_from_slice(&[base, sha]);
            self.run(&a)
        };
        let status = parse_name_status_z(&diff(&["--name-status"])?);
        let counts = parse_numstat_z(&diff(&["--numstat"])?);
        let ws = parse_numstat_z(&diff(&["--numstat", "-w"])?);
        Ok(merge_stats(status, &counts, &ws))
    }

    /// Paths whose `linguist-generated` attribute is set or `true` in `rev`'s own tree,
    /// so classification is deterministic per commit whichever worktree ingests it.
    pub fn generated(&self, rev: &str, paths: &[String]) -> io::Result<HashSet<String>> {
        if paths.is_empty() {
            return Ok(HashSet::new());
        }
        let mut child = git_cmd(&self.root)
            .args(["check-attr", &format!("--source={rev}"), "-z", "--stdin", "linguist-generated"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let input: Vec<u8> = paths.iter().flat_map(|p| p.bytes().chain([0])).collect();
        // Write on a thread: a large path list can fill stdout's pipe before stdin drains.
        let writer = std::thread::spawn(move || stdin.write_all(&input));
        let out = child.wait_with_output()?;
        writer.join().map_err(|_| io::Error::other("check-attr writer panicked"))??;
        if !out.status.success() {
            // e.g. git < 2.40 has no `--source`: fail loudly instead of reporting nothing generated.
            return Err(io::Error::other(format!("git check-attr: {}", String::from_utf8_lossy(&out.stderr).trim())));
        }
        Ok(parse_check_attr_z(&String::from_utf8_lossy(&out.stdout)))
    }

    /// Paths present at `rev` (the history tip, not the current worktree).
    pub fn ls_tree(&self, rev: &str) -> io::Result<HashSet<String>> {
        Ok(self.run(&["ls-tree", "-r", "-z", "--name-only", rev])?
            .split('\0').filter(|p| !p.is_empty()).map(str::to_string).collect())
    }
}

pub(crate) fn parse_commit_meta(out: &str) -> io::Result<CommitMeta> {
    let f: Vec<&str> = out.trim_end_matches('\n').splitn(7, '\0').collect();
    if f.len() < 7 {
        return Err(io::Error::other(format!("unexpected `git show` output: {out:?}")));
    }
    Ok(CommitMeta {
        sha: f[0].to_string(),
        parents: f[1].split_whitespace().map(str::to_string).collect(),
        ts: f[2].trim().parse().map_err(io::Error::other)?,
        author: f[3].to_string(),
        subject: f[4].to_string(),
        body: f[5].trim().to_string(),
        co_authors: f[6].split('\u{1f}').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
    })
}

pub(crate) fn parse_name_status_z(out: &str) -> Vec<FileStat> {
    let mut v = Vec::new();
    let mut it = out.split('\0').filter(|t| !t.is_empty());
    while let Some(st) = it.next() {
        let (letter, score) = st.split_at(1);
        let old_path = matches!(letter, "R" | "C").then(|| it.next().unwrap_or("").to_string());
        let path = it.next().unwrap_or("").to_string();
        v.push(FileStat { path, old_path, status: letter.to_string(), score: score.parse().ok(), ..Default::default() });
    }
    v
}

/// path (new path for renames) -> (added, deleted, binary).
pub(crate) fn parse_numstat_z(out: &str) -> HashMap<String, (u32, u32, bool)> {
    let mut m = HashMap::new();
    let mut it = out.split('\0').filter(|t| !t.is_empty());
    while let Some(tok) = it.next() {
        let mut p = tok.splitn(3, '\t');
        let (a, d, path) = (p.next().unwrap_or(""), p.next().unwrap_or(""), p.next().unwrap_or(""));
        let path = if path.is_empty() {
            let _old = it.next();
            it.next().unwrap_or("").to_string()
        } else {
            path.to_string()
        };
        m.insert(path, (a.parse().unwrap_or(0), d.parse().unwrap_or(0), a == "-"));
    }
    m
}

pub(crate) fn merge_stats(
    mut status: Vec<FileStat>,
    counts: &HashMap<String, (u32, u32, bool)>,
    ws: &HashMap<String, (u32, u32, bool)>,
) -> Vec<FileStat> {
    for f in &mut status {
        if let Some(&(a, d, b)) = counts.get(&f.path) {
            (f.added, f.deleted, f.binary) = (a, d, b);
        }
        // `diff -w --numstat` omits (or zeroes) whitespace-only files.
        f.whitespace_only = f.status == "M" && !f.binary
            && ws.get(&f.path).is_none_or(|&(a, d, _)| a == 0 && d == 0);
    }
    status
}

pub(crate) fn parse_check_attr_z(out: &str) -> HashSet<String> {
    let t: Vec<&str> = out.split('\0').collect();
    t.chunks_exact(3).filter(|c| matches!(c[2], "set" | "true")).map(|c| c[0].to_string()).collect()
}

/// One long-lived `git cat-file --batch` for all blob reads in an ingest.
pub struct BlobReader {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl BlobReader {
    pub fn spawn(root: &Path) -> io::Result<Self> {
        let mut child = git_cmd(root).args(["cat-file", "--batch"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self { child, stdin, stdout })
    }

    /// `rev:path` as UTF-8; `None` when missing, not a blob, non-UTF-8, or over `max_bytes`.
    pub fn read(&mut self, rev: &str, path: &str, max_bytes: usize) -> io::Result<Option<String>> {
        writeln!(self.stdin, "{rev}:{path}")?;
        self.stdin.flush()?;
        let mut header = String::new();
        self.stdout.read_line(&mut header)?;
        let mut parts = header.split_whitespace();
        let (_name, kind, size) = (parts.next(), parts.next(), parts.next());
        let Some(size) = size.and_then(|s| s.parse::<usize>().ok()) else {
            return Ok(None); // `missing` / `ambiguous`: header only, no body
        };
        let mut buf = vec![0u8; size + 1]; // body + trailing LF
        self.stdout.read_exact(&mut buf)?;
        buf.pop();
        if kind != Some("blob") || size > max_bytes {
            return Ok(None);
        }
        Ok(String::from_utf8(buf).ok())
    }
}

impl Drop for BlobReader {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_z_handles_rename_and_binary() {
        let m = parse_numstat_z("1\t1\ta.rs\x001\t0\t\x00b.txt\x00c.txt\x00-\t-\timg.png\x00");
        assert_eq!(m["a.rs"], (1, 1, false));
        assert_eq!(m["c.txt"], (1, 0, false));
        assert_eq!(m["img.png"], (0, 0, true));
        assert!(!m.contains_key("b.txt"));
    }

    #[test]
    fn name_status_z_reads_score_and_old_path() {
        let v = parse_name_status_z("M\x00a.rs\x00R080\x00b.txt\x00c.txt\x00");
        assert_eq!(v[0].status, "M");
        assert_eq!((v[1].status.as_str(), v[1].score, v[1].old_path.as_deref(), v[1].path.as_str()),
            ("R", Some(80), Some("b.txt"), "c.txt"));
    }

    #[test]
    fn merge_stats_flags_whitespace_only_modifications() {
        let status = parse_name_status_z("M\x00ws.rs\x00M\x00real.rs\x00A\x00new.rs\x00");
        let counts = parse_numstat_z("1\t1\tws.rs\x001\t1\treal.rs\x003\t0\tnew.rs\x00");
        let ws = parse_numstat_z("1\t1\treal.rs\x003\t0\tnew.rs\x00");
        let v = merge_stats(status, &counts, &ws);
        assert!(v[0].whitespace_only && !v[1].whitespace_only && !v[2].whitespace_only);
    }

    #[test]
    fn check_attr_z_accepts_set_and_true() {
        let s = parse_check_attr_z("a\x00linguist-generated\x00set\x00b\x00linguist-generated\x00unspecified\x00c\x00linguist-generated\x00true\x00");
        assert!(s.contains("a") && s.contains("c") && !s.contains("b"));
    }

    #[test]
    fn every_call_pins_output_affecting_config() {
        let args: Vec<String> = git_cmd(Path::new("/r")).get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(args.contains(&"log.showSignature=false".to_string()) && args.contains(&"core.quotePath=false".to_string()));
    }

    #[test]
    fn commit_meta_parses_fields() {
        let m = parse_commit_meta("abc\x00p1 p2\x0042\x00Ann\x00feat: x\x00body\n\x00A <a@x>\x1fB <b@x>\n").unwrap();
        assert_eq!(m.parents, vec!["p1", "p2"]);
        assert_eq!(m.ts, 42);
        assert_eq!(m.co_authors, vec!["A <a@x>", "B <b@x>"]);
    }
}
```

Add `pub mod git;` to `history/mod.rs`.

- [ ] **Step 5: Run the tests and confirm they pass**

Run: `cargo test -p vox-graph-reader --lib history::git && cargo test -p vox-graph-reader --test history_git_tests`
Expected: PASS (6 unit + 7 integration). `Option::is_none_or` needs Rust ≥1.82; the pinned toolchain is newer.

- [ ] **Step 6: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader
git commit -m "feat(graph): add git plumbing for repo history ingest"
```

---

### Task 4: Mechanical classification

<!-- AMENDED: G8 — whitespace reason gated to a whitespace-insignificant extension allowlist -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/classify.rs`
- Modify: `crates/vox-graph-reader/src/history/mod.rs` (`pub mod classify;`), `crates/vox-graph-reader/Cargo.toml` (`proc-macro2 = { workspace = true }` under `[dependencies]`)

**Interfaces:**
- Consumes: `model::Reason`
- Produces:
  - `classify::classify(path: &str, status: &str, generated: bool, whitespace_only: bool, before: Option<&str>, after: Option<&str>) -> Vec<Reason>` (mechanical ⇔ non-empty; `whitespace`/`symbol_neutral` only for `status == "M"`)
  <!-- AMENDED: #4 — a pure `git mv` of a .rs file was `symbol_neutral` → mechanical, hiding the rename and making `forgotten` date a freshly moved crate from history start -->
  - `classify::rust_normalized(&str) -> Option<String>`
  - `classify::subject_hint(&str) -> Option<&'static str>`

- [ ] **Step 1: Add `pub mod classify;` to `history/mod.rs`, then write the failing tests**, as the file's only content plus `use super::model::Reason;` <!-- AMENDED: #9 -->

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn neutral(b: &str, a: &str) -> bool {
        classify("x.rs", "M", false, false, Some(b), Some(a)).contains(&Reason::SymbolNeutral)
    }

    #[test]
    fn formatting_imports_derives_and_docs_are_symbol_neutral() {
        assert!(neutral("fn a() -> u8 { 1 }", "fn a()->u8{\n    1\n}\n"));
        assert!(neutral("use a::b;\nuse c::d;\nfn f() {}", "use c::d;\nuse a::b;\nfn f() {}"));
        assert!(neutral("#[derive(Debug)]\nstruct S;", "#[derive(Debug, Clone)]\nstruct S;"));
        assert!(neutral("/// old\nfn f() {}", "/// new doc\nfn f() {}"));
        assert!(neutral("mod m { use x::y; fn g() {} }", "mod m { fn g() {} }"));
    }

    #[test]
    fn body_changes_and_unparseable_files_are_not_neutral() {
        assert!(!neutral("fn a() -> u8 { 1 }", "fn a() -> u8 { 2 }"));
        assert!(!neutral("fn a() {", "fn a() {}"));
        assert!(!neutral("#[cfg(test)]\nfn a() {}", "fn a() {}"), "non-doc attrs matter");
    }

    #[test]
    fn generated_and_whitespace_flags_and_non_rust() {
        assert_eq!(classify("Cargo.lock", "M", true, false, None, None), vec![Reason::Generated]);
        assert_eq!(classify("a.ts", "M", false, true, Some("x"), Some("x ")), vec![Reason::Whitespace]);
        // AMENDED: #20 — valid Rust text in a .ts file, so deleting the `.rs` check fails this test.
        assert!(classify("a.ts", "M", false, false, Some("fn f() {}"), Some("fn  f(){}")).is_empty(), "symbol check is Rust-only");
    }

    #[test]
    fn renames_and_adds_are_never_mechanical_unless_generated() {
        let src = "fn f() {}\n";
        assert!(classify("b.rs", "R", false, false, Some(src), Some(src)).is_empty());
        assert!(classify("b.rs", "A", false, true, None, Some(src)).is_empty());
        assert_eq!(classify("b.lock", "R", true, false, None, None), vec![Reason::Generated]);
    }

    #[test]
    fn indentation_sensitive_files_never_get_whitespace() {
        assert!(classify("a.py", "M", false, true, Some("if x:\n    y()\n"), Some("if x:\ny()\n")).is_empty());
        assert!(classify("c.yaml", "M", false, true, Some("a:\n  b: 1\n"), Some("a:\nb: 1\n")).is_empty());
        assert!(classify("Makefile", "M", false, true, None, None).is_empty(), "no extension = not allowlisted");
    }

    #[test]
    fn subject_hint_matches_words_not_substrings() {
        assert_eq!(subject_hint("chore(hakari): resync"), Some("hakari"));
        assert_eq!(subject_hint("style: rustfmt pass"), Some("fmt"));
        assert_eq!(subject_hint("fix clippy lints"), Some("lint"));
        assert_eq!(subject_hint("chore(ssot): regenerate registry"), Some("regen"));
        assert_eq!(subject_hint("feat: show more information"), None);
    }
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --lib history::classify`
Expected: FAIL (compile error: cannot find function `classify`).

- [ ] **Step 3: Implement**

```rust
//! Per-file mechanical classification (spec §5.1). A change is mechanical when any
//! reason applies; the commit subject is recorded as a hint but never decides.
use super::model::Reason;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;

/// Extensions where whitespace never changes meaning. Indentation-sensitive or unknown
/// types (py, yaml, md, vox, Makefile, …) never get `Whitespace`: hiding a real change
/// costs more than showing a formatting one.
const WS_INSIGNIFICANT: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "json", "toml", "css", "html", "sql"];

fn ws_insignificant(path: &str) -> bool {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| WS_INSIGNIFICANT.contains(&e))
}

pub fn classify(path: &str, status: &str, generated: bool, whitespace_only: bool, before: Option<&str>, after: Option<&str>) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if generated {
        reasons.push(Reason::Generated);
    }
    // Only a modification can be "no meaningful change"; a rename or add is always real.
    if status != "M" {
        return reasons;
    }
    if whitespace_only && ws_insignificant(path) {
        reasons.push(Reason::Whitespace);
    }
    if path.ends_with(".rs") {
        if let (Some(b), Some(a)) = (before.and_then(rust_normalized), after.and_then(rust_normalized)) {
            if b == a {
                reasons.push(Reason::SymbolNeutral);
            }
        }
    }
    reasons
}

/// Token text of a Rust file with `use` items and `#[doc]` / `#[derive]` attributes
/// removed, so formatting, import churn, derive churn, and doc edits normalize away.
/// `None` when the file does not parse.
pub fn rust_normalized(src: &str) -> Option<String> {
    let mut file = syn::parse_file(src).ok()?;
    strip_uses(&mut file.items);
    Some(strip_attrs(file.to_token_stream()).to_string())
}

fn strip_uses(items: &mut Vec<syn::Item>) {
    items.retain(|i| !matches!(i, syn::Item::Use(_)));
    for item in items.iter_mut() {
        if let syn::Item::Mod(m) = item {
            if let Some((_, inner)) = &mut m.content {
                strip_uses(inner);
            }
        }
    }
}

fn strip_attrs(ts: TokenStream) -> TokenStream {
    let mut out: Vec<TokenTree> = Vec::new();
    let mut toks = ts.into_iter().peekable();
    while let Some(t) = toks.next() {
        if matches!(&t, TokenTree::Punct(p) if p.as_char() == '#') {
            let mut look = toks.clone();
            let bang = matches!(look.peek(), Some(TokenTree::Punct(b)) if b.as_char() == '!');
            if bang {
                look.next();
            }
            if let Some(TokenTree::Group(g)) = look.peek() {
                if g.delimiter() == Delimiter::Bracket && is_doc_or_derive(g.stream()) {
                    if bang {
                        toks.next();
                    }
                    toks.next();
                    continue;
                }
            }
        }
        out.push(match t {
            TokenTree::Group(g) => TokenTree::Group(Group::new(g.delimiter(), strip_attrs(g.stream()))),
            other => other,
        });
    }
    out.into_iter().collect()
}

fn is_doc_or_derive(attr: TokenStream) -> bool {
    matches!(attr.into_iter().next(), Some(TokenTree::Ident(i)) if i == "doc" || i == "derive")
}

/// What a commit subject claims. Recorded only; v1 never uses it to set `mechanical`.
pub fn subject_hint(subject: &str) -> Option<&'static str> {
    let lower = subject.to_ascii_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let has = |f: &dyn Fn(&str) -> bool| words.iter().any(|w| f(w));
    if has(&|w| w == "hakari") {
        Some("hakari")
    } else if has(&|w| w == "fmt" || w == "rustfmt" || w.starts_with("format")) {
        Some("fmt")
    } else if has(&|w| w == "clippy" || w == "lint" || w == "lints") {
        Some("lint")
    } else if has(&|w| w == "regen" || w == "regenerate" || w == "resync") {
        Some("regen")
    } else {
        None
    }
}
```

Add `proc-macro2 = { workspace = true }` to `crates/vox-graph-reader/Cargo.toml` `[dependencies]`.

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p vox-graph-reader --lib history::classify`
Expected: PASS (6 tests). If `cargo hakari verify` is part of the local gates and it complains, run `cargo hakari generate && cargo hakari manage-deps` and include the `workspace-hack` diff in this commit.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader Cargo.lock crates/workspace-hack
git commit -m "feat(graph): classify mechanical file changes for repo history"
```

---

### Task 5: Lineage detection

<!-- AMENDED: G9 — eligibility (skip), same-extension line fallback, target-share floor, boilerplate-line filter; symbols precomputed once per file -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/lineage.rs`
- Modify: `crates/vox-graph-reader/src/history/mod.rs` (`pub mod lineage;`)

**Interfaces:**
- Consumes: `crate::ast::extract_ast_in_module`, `model::{LineageRec, LineageKind, LineageMethod}`
- Produces:
  - `lineage::MIN_CHURN: u32 = 40`
  - `lineage::FileDelta<'a> { path: &'a str, old_path: Option<&'a str>, status: &'a str, score: Option<u8>, added: u32, deleted: u32, before: Option<&'a str>, after: Option<&'a str>, skip: bool, syms: Option<(HashSet<String>, HashSet<String>)> }`
  - `lineage::parseable(&str) -> bool`
  - `lineage::symbol_suffixes(path: &str, content: &str) -> HashSet<String>`
  - `lineage::file_symbols(path: &str, before: Option<&str>, after: Option<&str>) -> Option<(HashSet<String>, HashSet<String>)>`
  - `lineage::normalized_lines(&str) -> Vec<String>`
  - `lineage::detect(sha: &str, files: &[FileDelta]) -> Vec<LineageRec>`

Rules:
- **Sources** are non-`skip` `D` files, and non-`skip` `M` files with `deleted ≥ 40`. Their removed content is all of `before` for `D`, and `before − after` for `M`.
- **Targets** are non-`skip` `A`/`R` files, and non-`skip` `M` files with `added ≥ 40`. Their new content is all of `after` for `A`/`R`, and `after − before` for `M`.
- The caller sets `skip` for generated files and for mechanical `M` files.
- Each git rename is a `rename` edge, and it also counts toward its target's incoming total when deciding `merge`.
- **Symbol edge:** moved ≥ 2, or moved ≥ 1 with weight ≥ 0.2.
- **Line edge**, only between files with the same extension:
  - matched ≥ 15,
  - matched/removed(S) ≥ 0.3,
  - matched/new(T) ≥ 0.3,
  - after dropping normalized lines present in 3 or more eligible files.

- [ ] **Step 1: Add `pub mod lineage;` to `history/mod.rs`, then write the failing tests** <!-- AMENDED: #9 -->

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fd<'a>(path: &'a str, status: &'a str, before: Option<&'a str>, after: Option<&'a str>) -> FileDelta<'a> {
        let n = |s: Option<&str>| s.map_or(0, |c| c.lines().count() as u32);
        FileDelta { path, old_path: None, status, score: None,
            added: if status == "D" { 0 } else { n(after) }, deleted: if status == "A" { 0 } else { n(before) },
            before, after, skip: false, syms: file_symbols(path, before, after) }
    }

    const ABC: &str = "fn alpha() { 1 }\nfn beta() { 2 }\nfn gamma() { 3 }\n";

    #[test]
    fn symbol_split_into_two_files() {
        let files = [
            fd("a.rs", "D", Some(ABC), None),
            fd("b.rs", "A", None, Some("fn alpha() { 1 }\nfn beta() { 2 }\n")),
            fd("c.rs", "A", None, Some("fn gamma() { 3 }\n")),
        ];
        let out = detect("s", &files);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.kind == LineageKind::Split && e.method == LineageMethod::Symbol && e.from == "a.rs"));
        let total: f32 = out.iter().map(|e| e.weight).sum();
        assert!((total - 1.0).abs() < 1e-4, "weights sum to 1, got {total}");
    }

    #[test]
    fn methods_keep_identity_across_files() {
        let s = symbol_suffixes("x/a.rs", "struct E;\nimpl E { fn run(&self) {} }\n");
        assert!(s.contains("E::run") && s.contains("E"), "{s:?}");
    }

    #[test]
    fn line_fallback_splits_a_toml_file() {
        let block = |p: &str| (0..20).map(|i| format!("{p}_{i} = {i}\n")).collect::<String>();
        let (x, y) = (block("x"), block("y"));
        let whole = format!("{x}{y}");
        let files = [fd("a.toml", "D", Some(&whole), None), fd("x.toml", "A", None, Some(&x)), fd("y.toml", "A", None, Some(&y))];
        let out = detect("s", &files);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.method == LineageMethod::Line && e.kind == LineageKind::Split));
    }

    #[test]
    fn rename_plus_folded_duplicate_is_a_merge() {
        let body = "fn load() { 1 }\nfn save() { 2 }\n";
        let mut ren = fd("core/io.rs", "R", Some(body), Some(body));
        ren.old_path = Some("cuda/io.rs");
        ren.score = Some(100);
        let files = [ren, fd("metal/io.rs", "D", Some(body), None)];
        let out = detect("s", &files);
        let kinds: Vec<_> = out.iter().map(|e| (e.from.as_str(), e.kind)).collect();
        assert!(kinds.contains(&("cuda/io.rs", LineageKind::Rename)), "{kinds:?}");
        assert!(kinds.contains(&("metal/io.rs", LineageKind::Merge)), "{kinds:?}");
    }

    // AMENDED: #5 — the same symbols copied into two targets must not give away 200% of the source.
    #[test]
    fn a_source_never_gives_away_more_than_all_of_itself() {
        let files = [fd("a.rs", "D", Some(ABC), None), fd("b.rs", "A", None, Some(ABC)), fd("c.rs", "A", None, Some(ABC))];
        let out = detect("s", &files);
        assert_eq!(out.len(), 2);
        let total: f32 = out.iter().map(|e| e.weight).sum();
        assert!(total <= 1.0 + 1e-4, "outgoing weight {total} > 1");
    }

    #[test]
    fn skipped_files_never_take_part() {
        let mut src = fd("a.rs", "D", Some(ABC), None);
        src.skip = true;
        assert!(detect("s", &[src, fd("b.rs", "A", None, Some(ABC))]).is_empty());
    }

    #[test]
    fn line_fallback_needs_same_extension_and_target_share() {
        let block = |p: &str, n: usize| (0..n).map(|i| format!("{p}_{i} = {i}\n")).collect::<String>();
        let moved = block("m", 20);
        assert!(detect("s", &[fd("a.toml", "D", Some(&moved), None), fd("b.md", "A", None, Some(&moved))]).is_empty(),
            "different extensions never pair");
        let big = format!("{moved}{}", block("other", 180));
        assert!(detect("s", &[fd("a.toml", "D", Some(&moved), None), fd("b.toml", "A", None, Some(&big))]).is_empty(),
            "moved lines are only 10% of the new target");
    }

    #[test]
    fn lines_shared_by_three_files_are_boilerplate() {
        let common = (0..20).map(|i| format!("pin_{i} = \"1.0\"\n")).collect::<String>();
        let src = format!("{common}unique_a = 1\n");
        let files = [fd("a.toml", "D", Some(&src), None), fd("x.toml", "A", None, Some(&common)), fd("z.toml", "A", None, Some(&common))];
        assert!(detect("s", &files).is_empty());
    }

    #[test]
    fn small_edits_and_unrelated_files_produce_no_lineage() {
        let files = [fd("a.rs", "M", Some(ABC), Some("fn alpha() { 9 }\n")), fd("z.rs", "A", None, Some("fn zeta() {}\n"))];
        assert!(detect("s", &files).is_empty());
        assert_eq!(normalized_lines("use a::b;\n\n  }\n  let x = 1;\n"), vec!["let x = 1;"]);
        assert!(parseable("a.tsx") && !parseable("a.toml"));
    }
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --lib history::lineage`
Expected: FAIL (compile error: cannot find function `detect`).

- [ ] **Step 3: Implement**

```rust
//! Rename / split / merge lineage between files within one commit (spec §5.2):
//! symbol migration where the extractor parses the file, normalized-line overlap otherwise.
use super::model::{LineageKind, LineageMethod, LineageRec};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Lines an `M` file must lose (source) or gain (target) to be considered.
pub const MIN_CHURN: u32 = 40;
const SYMBOL_MIN_MOVED: usize = 2;
const SYMBOL_MIN_WEIGHT: f32 = 0.2;
// ponytail: fixed thresholds; tune against the real-history regressions if precision is poor.
const LINE_MIN_MATCHED: usize = 15;
const LINE_MIN_WEIGHT: f32 = 0.3;
/// A normalized line present in this many eligible files is boilerplate.
const COMMON_LINE_FILES: usize = 3;

pub struct FileDelta<'a> {
    pub path: &'a str,
    pub old_path: Option<&'a str>,
    pub status: &'a str,
    pub score: Option<u8>,
    pub added: u32,
    pub deleted: u32,
    pub before: Option<&'a str>,
    pub after: Option<&'a str>,
    /// Generated, or a mechanical modification: never a lineage source or target.
    pub skip: bool,
    /// `(before, after)` symbol suffixes for parseable files, computed once by the caller.
    pub syms: Option<(HashSet<String>, HashSet<String>)>,
}

pub fn parseable(path: &str) -> bool {
    matches!(ext(path), "rs" | "ts" | "tsx" | "js" | "jsx" | "py")
}

fn ext(path: &str) -> &str {
    Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("")
}

/// Symbol ids minus the file path and any `#n` suffix (`Type::method`, `free_fn`):
/// the part that survives a move between files.
pub fn symbol_suffixes(path: &str, content: &str) -> HashSet<String> {
    let g = crate::ast::extract_ast_in_module(Path::new(path), content, path);
    g.nodes.iter()
        .filter_map(|n| n.id.strip_prefix(path)?.strip_prefix("::").map(strip_dup_suffix))
        .collect()
}

/// Before/after symbol sets for a parseable file; `None` otherwise.
pub fn file_symbols(path: &str, before: Option<&str>, after: Option<&str>) -> Option<(HashSet<String>, HashSet<String>)> {
    parseable(path).then(|| {
        let s = |c: Option<&str>| c.map(|c| symbol_suffixes(path, c)).unwrap_or_default();
        (s(before), s(after))
    })
}

fn strip_dup_suffix(s: &str) -> String {
    match s.rsplit_once('#') {
        Some((head, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => head.to_string(),
        _ => s.to_string(),
    }
}

/// Trimmed lines without blanks, bracket-only lines, imports, derives, and `//` comments.
pub fn normalized_lines(content: &str) -> Vec<String> {
    content.lines().map(str::trim)
        .filter(|l| {
            !(l.is_empty()
                || l.chars().all(|c| "{}()[];,".contains(c))
                || l.starts_with("use ") || l.starts_with("pub use ")
                || l.starts_with("import ") || l.starts_with("from ")
                || l.starts_with("#[derive") || l.starts_with("//"))
        })
        .map(str::to_string).collect()
}

type Bag = HashMap<String, usize>;

fn bag(lines: Vec<String>) -> Bag {
    let mut m = Bag::new();
    for l in lines {
        *m.entry(l).or_insert(0) += 1;
    }
    m
}

fn bag_minus(a: &Bag, b: &Bag) -> Bag {
    a.iter().filter_map(|(k, &n)| {
        let r = n.saturating_sub(b.get(k).copied().unwrap_or(0));
        (r > 0).then(|| (k.clone(), r))
    }).collect()
}

fn bag_overlap(a: &Bag, b: &Bag) -> usize {
    a.iter().map(|(k, &n)| n.min(b.get(k).copied().unwrap_or(0))).sum()
}

/// Content that left (`source`) or arrived (`!source`): (total symbols, moved set) and lines.
fn delta_side(f: &FileDelta, source: bool) -> (Option<(usize, HashSet<String>)>, Bag) {
    let whole = (source && f.status == "D") || (!source && matches!(f.status, "A" | "R"));
    let empty = HashSet::new();
    let syms = f.syms.as_ref().map(|(b, a)| {
        let (from, other) = if source { (b, a) } else { (a, b) };
        let other = if whole { &empty } else { other };
        (from.len(), from.difference(other).cloned().collect())
    });
    let (from, other) = if source { (f.before, f.after) } else { (f.after, f.before) };
    let other = if whole { "" } else { other.unwrap_or("") };
    (syms, bag_minus(&bag(normalized_lines(from.unwrap_or(""))), &bag(normalized_lines(other))))
}

pub fn detect(sha: &str, files: &[FileDelta]) -> Vec<LineageRec> {
    let rec = |from: &str, to: &str, kind, method, weight, evidence| LineageRec {
        sha: sha.to_string(), from: from.to_string(), to: to.to_string(), kind, method, weight, evidence,
    };
    let mut out: Vec<LineageRec> = files.iter()
        .filter(|f| f.status == "R")
        .filter_map(|f| Some(rec(f.old_path?, f.path, LineageKind::Rename, LineageMethod::GitRename,
            f32::from(f.score.unwrap_or(100)) / 100.0, 0)))
        .collect();

    let sources: Vec<&FileDelta> = files.iter()
        .filter(|f| !f.skip && f.before.is_some() && (f.status == "D" || (f.status == "M" && f.deleted >= MIN_CHURN)))
        .collect();
    let targets: Vec<&FileDelta> = files.iter()
        .filter(|f| !f.skip && f.after.is_some() && (matches!(f.status, "A" | "R") || (f.status == "M" && f.added >= MIN_CHURN)))
        .collect();
    let mut src_sides: Vec<_> = sources.iter().map(|s| delta_side(s, true)).collect();
    let mut tgt_sides: Vec<_> = targets.iter().map(|t| delta_side(t, false)).collect();

    // Lines held by 3+ eligible files (version pins, table rows) say nothing about lineage.
    let common: HashSet<String> = {
        let mut holders: HashMap<&str, HashSet<&str>> = HashMap::new();
        for (f, (_, b)) in sources.iter().zip(&src_sides).chain(targets.iter().zip(&tgt_sides)) {
            for l in b.keys() {
                holders.entry(l.as_str()).or_default().insert(f.path);
            }
        }
        holders.into_iter().filter(|(_, h)| h.len() >= COMMON_LINE_FILES).map(|(l, _)| l.to_string()).collect()
    };
    for (_, b) in src_sides.iter_mut().chain(tgt_sides.iter_mut()) {
        b.retain(|l, _| !common.contains(l));
    }

    let mut edges: Vec<(usize, usize, LineageMethod, f32, u32)> = Vec::new();
    for (si, s) in sources.iter().enumerate() {
        let (s_syms, s_lines) = &src_sides[si];
        let s_total: usize = s_lines.values().sum();
        for (ti, t) in targets.iter().enumerate() {
            if t.path == s.path {
                continue;
            }
            let (t_syms, t_lines) = &tgt_sides[ti];
            if let (Some((total, removed)), Some((_, added))) = (s_syms, t_syms) {
                let moved = removed.intersection(added).count();
                let weight = if *total == 0 { 0.0 } else { moved as f32 / *total as f32 };
                if moved >= SYMBOL_MIN_MOVED || (moved >= 1 && weight >= SYMBOL_MIN_WEIGHT) {
                    edges.push((si, ti, LineageMethod::Symbol, weight, moved as u32));
                    continue;
                }
            }
            if s_total == 0 || ext(s.path) != ext(t.path) {
                continue;
            }
            let t_total: usize = t_lines.values().sum();
            let matched = bag_overlap(s_lines, t_lines);
            let weight = matched as f32 / s_total as f32;
            let t_share = if t_total == 0 { 0.0 } else { matched as f32 / t_total as f32 };
            if matched >= LINE_MIN_MATCHED && weight >= LINE_MIN_WEIGHT && t_share >= LINE_MIN_WEIGHT {
                edges.push((si, ti, LineageMethod::Line, weight, matched as u32));
            }
        }
    }

    // A symbol or line copied into several targets must not count twice: scale each source's
    // outgoing weights so they sum to at most 1.
    let mut sums: HashMap<usize, f32> = HashMap::new();
    for &(s, _, _, w, _) in &edges {
        *sums.entry(s).or_insert(0.0) += w;
    }
    for e in &mut edges {
        let total = sums[&e.0];
        if total > 1.0 {
            e.3 /= total;
        }
    }

    let mut per_source: HashMap<usize, usize> = HashMap::new();
    let mut per_target: HashMap<&str, usize> = HashMap::new();
    for r in &out {
        *per_target.entry(r.to.as_str()).or_insert(0) += 1;
    }
    for &(s, t, ..) in &edges {
        *per_source.entry(s).or_insert(0) += 1;
        *per_target.entry(targets[t].path).or_insert(0) += 1;
    }
    let mut detected = Vec::with_capacity(edges.len());
    for (s, t, method, weight, evidence) in edges {
        let kind = if per_source[&s] >= 2 {
            LineageKind::Split
        } else if per_target[targets[t].path] >= 2 {
            LineageKind::Merge
        } else {
            LineageKind::Move
        };
        detected.push(rec(sources[s].path, targets[t].path, kind, method, weight, evidence));
    }
    out.extend(detected);
    out
}
```

(`per_target` borrows `out`'s strings, which is why detected edges are collected into `detected` and appended at the end.)

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p vox-graph-reader --lib history::lineage && grep -cv '^\s*$' crates/vox-graph-reader/src/history/lineage.rs`
Expected: PASS (9 tests), and a line count ≤500. If `methods_keep_identity_across_files` fails, print the ids with `dbg!(&s)` and check the v4 id format in `ast.rs`.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader/src/history
git commit -m "feat(graph): detect rename/split/merge lineage between files"
```

---

### Task 6: Ingest and catch-up

<!-- AMENDED: G2, G4, G5, G9, G12, G13 — rollback on rewrite, check-attr per commit, first-parent carrier regressions (#[ignore], panic on missing SHA, golden pairs), lock touch, parse once -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/ingest.rs`, `crates/vox-graph-reader/tests/history_ingest_tests.rs`, `crates/vox-graph-reader/tests/fixtures/history_carrier_pairs.txt` (seeded by hand in Step 5)
- Modify: `crates/vox-graph-reader/src/history/mod.rs` (`pub mod ingest;`)

**Interfaces:**
- Consumes: Tasks 2–5 (`IngestLock::touch`, `HistoryStore::load_checked`, `Git::generated(rev, …)`, `lineage::file_symbols`)
- Produces:
  - `ingest::CatchUp { Done { ingested: usize, rebuilt: bool }, OverBudget { ingested: usize, behind: usize }, Busy, Skipped }`
  - `ingest::ingest_commit(&Git, &mut BlobReader, sha: &str) -> io::Result<(CommitRec, Vec<ChangeRec>, Vec<LineageRec>)>`
  - `ingest::catch_up(repo_root: &Path, store: &HistoryStore, lock: &IngestLock, tip: &str, budget: Option<Duration>, progress: &mut dyn FnMut(usize, usize)) -> io::Result<CatchUp>`
  - `ingest::co_author_name(&str) -> String`

- [ ] **Step 1: Add `pub mod ingest;` to `history/mod.rs`, then write the failing integration tests** <!-- AMENDED: #9 -->

```rust
mod common;
use common::Repo;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;
use vox_graph_reader::history::git::{BlobReader, Git};
use vox_graph_reader::history::ingest::{CatchUp, catch_up, ingest_commit};
use vox_graph_reader::history::model::{LineageKind, LineageMethod, Reason};
use vox_graph_reader::history::store::HistoryStore;

fn store() -> (tempfile::TempDir, HistoryStore) {
    let d = tempfile::tempdir().unwrap();
    let s = HistoryStore::open(d.path()).unwrap();
    (d, s)
}

fn run(r: &Repo, s: &HistoryStore, budget: Option<Duration>) -> CatchUp {
    let lock = s.try_lock().unwrap().expect("lock");
    catch_up(r.path(), s, &lock, "main", budget, &mut |_, _| {}).unwrap()
}

#[test]
fn mixed_commit_flags_only_the_fmt_drift_files() {
    let r = Repo::new();
    r.write("real.rs", "fn a() -> u8 { 1 }\n");
    r.write("drift1.rs", "fn b() {\n    1;\n}\n");
    r.write("drift2.rs", "use x::y;\nuse z::w;\nfn c() {}\n");
    r.commit("init");
    r.write("real.rs", "fn a() -> u8 { 2 }\n");
    r.write("drift1.rs", "fn b() {\n  1;\n}\n");
    r.write("drift2.rs", "use z::w;\nuse x::y;\nfn c() {}\n");
    let c = r.commit("feat: real change\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>");
    let g = Git::new(r.path());
    let (commit, changes, _) = ingest_commit(&g, &mut BlobReader::spawn(r.path()).unwrap(), &c).unwrap();
    let flag = |p: &str| changes.iter().find(|x| x.path == p).unwrap();
    assert!(!flag("real.rs").mechanical);
    assert!(flag("drift1.rs").reasons.contains(&Reason::Whitespace));
    assert!(flag("drift2.rs").reasons.contains(&Reason::SymbolNeutral));
    assert!(!commit.mechanical);
    assert_eq!(commit.agent.as_deref(), Some("Claude Opus 5"));
}

#[test]
fn catch_up_is_incremental_and_merges_carry_inner_subjects() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    r.commit("feat: one");
    let (_d, s) = store();
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 1, rebuilt: false });
    r.git(&["checkout", "-q", "-b", "side"]);
    r.write("b.txt", "2\n");
    r.commit("wip: inner");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--no-ff", "side", "-m", "merge side"]);
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 1, rebuilt: false });
    let d = s.load().unwrap();
    assert_eq!(d.commits.len(), 2, "inner branch commit gets no row of its own");
    assert!(d.commits[1].is_merge);
    assert_eq!(d.commits[1].inner_subjects, vec!["wip: inner".to_string()]);
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 0, rebuilt: false });
}

#[test]
fn rewritten_tip_rolls_back_instead_of_rebuilding() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c1 = r.commit("one");
    r.write("a.txt", "2\n");
    r.commit("two (dropped by the rewrite)");
    let (_d, s) = store();
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 2, rebuilt: false });
    r.git(&["reset", "-q", "--hard", &c1]);
    r.write("b.txt", "3\n");
    let c3 = r.commit("three");
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 1, rebuilt: false });
    let shas: Vec<String> = s.load().unwrap().commits.into_iter().map(|c| c.sha).collect();
    assert_eq!(shas, vec![c1, c3]);
}

#[test]
fn corrupt_state_triggers_full_rebuild() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c = r.commit("one");
    let (d, s) = store();
    run(&r, &s, None);
    let st = format!("{{\"schema_version\":1,\"extractor_version\":\"0\",\"last_sha\":\"{c}\"}}");
    std::fs::write(d.path().join("state.json"), st).unwrap();
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 1, rebuilt: true });
    // AMENDED: #16 — an unparseable state.json used to fail every query forever.
    std::fs::write(d.path().join("state.json"), "not json").unwrap();
    assert_eq!(run(&r, &s, None), CatchUp::Done { ingested: 1, rebuilt: true });
    assert_eq!(s.load().unwrap().commits.len(), 1);
}

// AMENDED: #20 — without `skip`, the generated source would become a lineage parent of src/b.rs.
#[test]
fn generated_files_never_take_part_in_lineage() {
    let r = Repo::new();
    let fns = |names: &[&str]| names.iter().map(|n| format!("fn {n}() {{\n    let v = \"{n}\";\n    drop(v);\n}}\n")).collect::<String>();
    r.write(".gitattributes", "gen/** linguist-generated\n");
    r.write("gen/a.rs", &fns(&["g0", "g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8", "g9"]));
    r.commit("init");
    std::fs::remove_file(r.path().join("gen/a.rs")).unwrap();
    // Two of the generated fns plus plenty of new code: too dissimilar for a git rename.
    r.write("src/b.rs", &fns(&["g0", "g1", "n0", "n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9", "n10", "n11"]));
    let c = r.commit("hand-copy two generated fns");
    let (_, _, lineage) = ingest_commit(&Git::new(r.path()), &mut BlobReader::spawn(r.path()).unwrap(), &c).unwrap();
    assert!(lineage.iter().all(|e| e.from != "gen/a.rs"), "{lineage:?}");
}

#[test]
fn zero_budget_stops_before_ingesting_and_reports_behind() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    r.commit("one");
    r.write("a.txt", "2\n");
    r.commit("two");
    let (_d, s) = store();
    assert_eq!(run(&r, &s, Some(Duration::ZERO)), CatchUp::OverBudget { ingested: 0, behind: 2 });
}

#[test]
fn split_commit_records_lineage() {
    let r = Repo::new();
    let fns = |names: &[&str]| names.iter().map(|n| format!("fn {n}() {{\n    let v = \"{n}\";\n    drop(v);\n}}\n")).collect::<String>();
    // 14 fns x 4 lines; keeping 2 deletes 48 lines, safely over MIN_CHURN (40).
    r.write("big.rs", &fns(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n"]));
    r.commit("init");
    r.write("big.rs", &fns(&["a", "b"]));
    r.write("part1.rs", &fns(&["c", "d", "e", "f", "g", "h"]));
    r.write("part2.rs", &fns(&["i", "j", "k", "l", "m", "n"]));
    let c = r.commit("refactor: split big.rs");
    let (_, _, lineage) = ingest_commit(&Git::new(r.path()), &mut BlobReader::spawn(r.path()).unwrap(), &c).unwrap();
    let to: Vec<_> = lineage.iter().filter(|e| e.from == "big.rs" && e.kind == LineageKind::Split).map(|e| e.to.as_str()).collect();
    assert!(to.contains(&"part1.rs") && to.contains(&"part2.rs"), "{lineage:?}");
}

/// The oldest first-parent commit of `tip` that contains `sha`: what `catch_up` really ingests.
fn carrier(g: &Git, tip: &str, sha: &str) -> String {
    let full = g.rev_parse(sha).unwrap_or_else(|_| panic!("{sha} not in this clone: run in a full-history checkout"));
    g.first_parent_range(None, tip).unwrap().into_iter()
        .find(|x| g.is_ancestor(&full, x)).expect("carrier on the first-parent line")
}

fn crate_of(p: &str) -> String {
    p.split('/').take(2).collect::<Vec<_>>().join("/")
}

#[test]
#[ignore = "needs full history; run: cargo test -p vox-graph-reader --test history_ingest_tests -- --ignored real_history"]
fn real_history_split_and_merge_are_detected_on_the_carrier() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let g = Git::new(&root);
    let tip = g.default_tip();
    let c = carrier(&g, &tip, "05e42775a");
    assert_eq!(c, carrier(&g, &tip, "a7aaf48d4"), "both changes arrive via one merge");
    let (_, _, lineage) = ingest_commit(&g, &mut BlobReader::spawn(&root).unwrap(), &c).unwrap();

    let engine: Vec<_> = lineage.iter()
        .filter(|e| e.from.ends_with("vox-plugin-browser/src/engine.rs") && e.kind == LineageKind::Split).collect();
    assert!(engine.len() >= 2, "engine.rs split into host/input/resolve: {engine:?}");
    assert!(engine.iter().all(|e| e.to.starts_with("crates/vox-plugin-browser/")), "{engine:?}");
    assert!(lineage.iter().any(|e| e.kind == LineageKind::Rename), "candle fold has git renames");
    assert!(lineage.iter().any(|e| e.kind == LineageKind::Merge && e.to.contains("candle-core")),
        "cuda+metal duplicates fold into core");

    // Precision: every crate pair of a non-rename edge was reviewed by a human once.
    let pairs: BTreeSet<String> = lineage.iter().filter(|e| e.kind != LineageKind::Rename)
        .map(|e| format!("{} -> {}", crate_of(&e.from), crate_of(&e.to))).collect();
    let mut per_target: HashMap<&str, usize> = HashMap::new();
    for e in lineage.iter().filter(|e| e.method == LineageMethod::Line) {
        *per_target.entry(e.to.as_str()).or_default() += 1;
    }
    assert!(per_target.values().all(|&n| n <= 3), "too many line edges into one target: {per_target:?}");
    // AMENDED: #12 — the golden is seeded by hand; the code under test never writes it.
    // VOX_HISTORY_BLESS only writes a `.new` candidate for the owner to diff and review.
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/history_carrier_pairs.txt");
    if std::env::var_os("VOX_HISTORY_BLESS").is_some() {
        let candidate = golden.with_extension("txt.new");
        std::fs::write(&candidate, pairs.iter().map(|p| format!("{p}\n")).collect::<String>()).unwrap();
        panic!("wrote {candidate:?}: diff it against the golden and get owner review before replacing");
    }
    let allowed: BTreeSet<String> = std::fs::read_to_string(&golden).expect("hand-seeded golden missing")
        .lines().map(str::to_string).collect();
    let unreviewed: Vec<_> = pairs.difference(&allowed).collect();
    assert!(unreviewed.is_empty(), "unreviewed crate pairs: {unreviewed:?}");
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --test history_ingest_tests`
Expected: FAIL (`history::ingest` not found).

- [ ] **Step 3: Implement `ingest.rs`**

```rust
//! Walk the tip's first-parent line into the store (spec §5).
use super::classify::{classify, subject_hint};
use super::git::{BlobReader, Git};
use super::lineage::{self, FileDelta};
use super::model::{ChangeRec, CommitRec, LineageRec, SymbolDelta};
use super::store::{HistoryStore, IngestLock};
// AMENDED: #4 — `classify` takes the status so renames/adds are never whitespace/symbol-neutral.
use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Blobs above this are never read (vendored bindings, generated schemas).
const MAX_BLOB_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq)]
pub enum CatchUp {
    Done { ingested: usize, rebuilt: bool },
    OverBudget { ingested: usize, behind: usize },
    /// Another process holds `ingest.lock`; answer from what is on disk.
    Busy,
    /// `brief` never ingests.
    Skipped,
}

/// `Claude Opus 5 <noreply@anthropic.com>` -> `Claude Opus 5`.
pub fn co_author_name(trailer: &str) -> String {
    trailer.split('<').next().unwrap_or(trailer).trim().to_string()
}

fn symbol_delta(before: &HashSet<String>, after: &HashSet<String>) -> SymbolDelta {
    let mut added: Vec<String> = after.difference(before).cloned().collect();
    let mut removed: Vec<String> = before.difference(after).cloned().collect();
    added.sort();
    removed.sort();
    SymbolDelta { added, removed }
}

pub fn ingest_commit(git: &Git, blobs: &mut BlobReader, sha: &str) -> io::Result<(CommitRec, Vec<ChangeRec>, Vec<LineageRec>)> {
    let meta = git.commit_meta(sha)?;
    let parent = meta.parents.first().cloned();
    let stats = git.file_deltas(parent.as_deref(), sha)?;
    let generated = git.generated(sha, &stats.iter().map(|f| f.path.clone()).collect::<Vec<_>>())?;

    let mut contents = Vec::with_capacity(stats.len());
    for f in &stats {
        let before = match (&parent, f.binary || f.status == "A") {
            (Some(p), false) => blobs.read(p, f.old_path.as_deref().unwrap_or(&f.path), MAX_BLOB_BYTES)?,
            _ => None,
        };
        let after = if f.binary || f.status == "D" { None } else { blobs.read(sha, &f.path, MAX_BLOB_BYTES)? };
        contents.push((before, after));
    }

    // Parse each file once; the symbol sets feed both ChangeRec and lineage.
    let mut changes = Vec::with_capacity(stats.len());
    let mut deltas = Vec::with_capacity(stats.len());
    for (f, (before, after)) in stats.iter().zip(&contents) {
        let syms = lineage::file_symbols(&f.path, before.as_deref(), after.as_deref());
        let is_generated = generated.contains(&f.path);
        let reasons = classify(&f.path, &f.status, is_generated, f.whitespace_only, before.as_deref(), after.as_deref());
        let mechanical = !reasons.is_empty();
        changes.push(ChangeRec {
            sha: sha.to_string(), path: f.path.clone(), old_path: f.old_path.clone(), status: f.status.clone(),
            added: f.added, deleted: f.deleted, symbols: syms.as_ref().map(|(b, a)| symbol_delta(b, a)),
            mechanical, reasons,
        });
        deltas.push(FileDelta {
            path: &f.path, old_path: f.old_path.as_deref(), status: &f.status, score: f.score,
            added: f.added, deleted: f.deleted, before: before.as_deref(), after: after.as_deref(),
            skip: is_generated || (f.status == "M" && mechanical), syms,
        });
    }
    let lineage = lineage::detect(sha, &deltas);

    let is_merge = meta.parents.len() > 1;
    let commit = CommitRec {
        sha: sha.to_string(), parent, ts: meta.ts, author: meta.author.clone(),
        agent: meta.co_authors.first().map(|c| co_author_name(c)),
        subject: meta.subject.clone(), body: meta.body.clone(), is_merge,
        inner_subjects: if is_merge { git.inner_subjects(sha, None)? } else { Vec::new() },
        mechanical: !changes.is_empty() && changes.iter().all(|c| c.mechanical),
        subject_hint: subject_hint(&meta.subject).map(str::to_string),
    };
    Ok((commit, changes, lineage))
}

/// Bring the store up to `tip`. If `tip` no longer contains `last_sha`, roll back to the
/// newest ingested commit that is still an ancestor; reset only when none is, the store is
/// corrupt (unparseable state, version mismatch, broken chain), or the next commit does not
/// sit on `last_sha`. With `budget`, stops at the deadline and keeps everything appended so far.
pub fn catch_up(repo_root: &Path, store: &HistoryStore, lock: &IngestLock, tip: &str,
    budget: Option<Duration>, progress: &mut dyn FnMut(usize, usize)) -> io::Result<CatchUp> {
    let start = Instant::now();
    let git = Git::new(repo_root);
    let tip_sha = git.rev_parse(tip)?;
    // AMENDED: #16 — an unparseable state.json is corruption: reset, don't error forever.
    let state = store.state().ok();
    let healthy = match &state {
        Some(s) if s.is_current() => store.load_checked()?.1,
        _ => false,
    };
    let mut rebuilt = false;
    if !healthy {
        store.reset()?;
        rebuilt = true;
    } else if let Some(last) = state.as_ref().and_then(|s| s.last_sha.as_deref()) {
        if !git.is_ancestor(last, &tip_sha) {
            let data = store.load()?;
            match data.commits.iter().rev().find(|c| git.is_ancestor(&c.sha, &tip_sha)) {
                // Rows past it stay on disk but are off the chain, so `load` ignores them.
                Some(c) => store.commit_state(&c.sha)?, // AMENDED: #19 — no rewrite (`repair`) under a possible takeover
                None => {
                    store.reset()?;
                    rebuilt = true;
                }
            }
        }
    }
    let since = store.state()?.last_sha;
    let mut todo = git.first_parent_range(since.as_deref(), &tip_sha)?;
    // AMENDED: #8 — `last..tip` can reach `last` through a non-first parent (foxtrot merge); the
    // first new commit must sit directly on `last` or the chain would break silently.
    if let (Some(s), Some(first)) = (since.as_deref(), todo.first()) {
        if git.commit_meta(first)?.parents.first().map(String::as_str) != Some(s) {
            store.reset()?;
            rebuilt = true;
            todo = git.first_parent_range(None, &tip_sha)?;
        }
    }
    let mut blobs = BlobReader::spawn(repo_root)?;
    for (i, sha) in todo.iter().enumerate() {
        if budget.is_some_and(|b| start.elapsed() >= b) {
            return Ok(CatchUp::OverBudget { ingested: i, behind: todo.len() - i });
        }
        let (c, ch, li) = ingest_commit(&git, &mut blobs, sha)?;
        store.append(&c, &ch, &li)?;
        store.commit_state(sha)?;
        lock.touch()?;
        progress(i + 1, todo.len());
    }
    Ok(CatchUp::Done { ingested: todo.len(), rebuilt })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn co_author_name_strips_email() {
        assert_eq!(co_author_name("Claude Opus 5 <noreply@anthropic.com>"), "Claude Opus 5");
        assert_eq!(co_author_name("Cursor"), "Cursor");
    }
}
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p vox-graph-reader --test history_ingest_tests && cargo test -p vox-graph-reader --lib history::ingest`
Expected: PASS. The real-history test shows as `ignored`.

- [ ] **Step 5: Seed the golden file by hand, then run the real-history regression** (full clone required; this checkout has full history) <!-- AMENDED: #12 -->

```bash
mkdir -p crates/vox-graph-reader/tests/fixtures
cat > crates/vox-graph-reader/tests/fixtures/history_carrier_pairs.txt <<'EOF'
crates/vox-plugin-browser -> crates/vox-plugin-browser
crates/vox-plugin-mens-candle-cuda -> crates/vox-plugin-mens-candle-core
crates/vox-plugin-mens-candle-metal -> crates/vox-plugin-mens-candle-core
EOF
cargo test -p vox-graph-reader --test history_ingest_tests -- --ignored real_history
```

Expected: PASS. If it fails only on `unreviewed crate pairs`:
- Run with `VOX_HISTORY_BLESS=1` to write `history_carrier_pairs.txt.new`.
- Look up each extra pair in `git show --stat a7cdfdb8e`.
- For any pair that isn't a real move, tighten the thresholds in `lineage.rs` until it disappears.
- **Stop and ask the owner** before adding a pair to the golden file. The agent that wrote the detector must not approve its own output.

If the engine or candle assertions fail, **don't loosen them**. Print the lineage and diagnose which threshold is wrong.

- [ ] **Step 6: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader
git commit -m "feat(graph): ingest first-parent history with rollback-safe catch-up"
```

---

### Task 7: Query views

<!-- AMENDED: G10, G14 — fan-in filtered by the crate-edges allowlist; LogEntry.inner for merge rows -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/query.rs`
- Modify: `crates/vox-graph-reader/src/history/mod.rs` (`pub mod query;`)

**Interfaces:**
- Consumes: `model::*`
- Produces:
  - `query::AreaBy { Crate, Dir }` (serde snake_case)
  - `area_of(&str, AreaBy) -> String`
  - `AreaScore { area, commits: u32, lines: u32 }`
  - `focus_between(&HistoryData, from_ts: i64, to_ts: i64, AreaBy, include_mechanical: bool, limit: usize) -> Vec<AreaScore>`
  - `LogEntry { sha, ts, subject, agent, path, added, deleted, weight: f32, mechanical, inner: Vec<String> }`
  - `log(&HistoryData, path: &str, include_mechanical: bool, limit: usize) -> Vec<LogEntry>` (`inner` left empty; the facade fills it)
  - `Forgotten { area, days_since: i64, last_subject, fan_in: u32, score: f64 }`
  - `forgotten(&HistoryData, now_ts: i64, min_age_days: i64, current_paths: &HashSet<String>, fan_in: &HashMap<String, u32>, AreaBy, limit: usize) -> Vec<Forgotten>`
  - `fan_in_by_area(&serde_json::Value, AreaBy, allowed: &HashSet<(String, String)>) -> HashMap<String, u32>`
  - `allowed_crate_edges(&serde_json::Value) -> HashSet<(String, String)>`
  - `SearchHit { sha, ts, subject, score: u32 }`, `search(&HistoryData, &str, limit: usize) -> Vec<SearchHit>`
  - `Bucket { start_ts, top: Vec<AreaScore>, highlights: Vec<String> }`, `timeline(&HistoryData, since_ts, now_ts, bucket_days: i64, AreaBy, per_bucket: usize) -> Vec<Bucket>`
  - `brief(&HistoryData, now_ts, &HashSet<String>, &HashMap<String, u32>) -> String`
  - `pub(crate) const DAY: i64`

- [ ] **Step 1: Add `pub mod query;` to `history/mod.rs`, then write the failing tests** on synthetic data <!-- AMENDED: #9 -->

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::model::*;
    use serde_json::json;

    const NOW: i64 = 200 * DAY;

    fn c(sha: &str, day: i64, subject: &str) -> CommitRec {
        CommitRec { sha: sha.into(), parent: None, ts: day * DAY, author: "a".into(), agent: None,
            subject: subject.into(), body: String::new(), is_merge: false, inner_subjects: vec![],
            mechanical: false, subject_hint: None }
    }
    fn ch(sha: &str, path: &str, mech: bool) -> ChangeRec {
        ChangeRec { sha: sha.into(), path: path.into(), old_path: None, status: "M".into(), added: 5,
            deleted: 1, symbols: Some(SymbolDelta { added: vec!["Engine::run".into()], removed: vec![] }),
            mechanical: mech, reasons: vec![] }
    }
    fn data() -> HistoryData {
        HistoryData {
            commits: vec![c("s1", 10, "feat: old core work"), c("s2", 195, "feat: gui panel"), c("s3", 198, "style: fmt"), c("s4", 190, "refactor: split engine")],
            changes: vec![ch("s1", "crates/core/src/lib.rs", false), ch("s2", "crates/gui/src/a.rs", false),
                ch("s3", "crates/core/src/lib.rs", true), ch("s4", "crates/gui/src/host.rs", false)],
            lineage: vec![LineageRec { sha: "s4".into(), from: "crates/gui/src/engine.rs".into(), to: "crates/gui/src/host.rs".into(),
                kind: LineageKind::Split, method: LineageMethod::Symbol, weight: 0.5, evidence: 3 }],
        }
    }

    #[test]
    fn area_of_groups_by_crate_or_dir() {
        assert_eq!(area_of("crates/vox-cli/src/x.rs", AreaBy::Crate), "crates/vox-cli");
        assert_eq!(area_of("crates/vox-cli/src/x.rs", AreaBy::Dir), "crates/vox-cli/src");
        assert_eq!(area_of("scripts/fmt.vox", AreaBy::Crate), "scripts");
        assert_eq!(area_of("README.md", AreaBy::Crate), ".");
    }

    #[test]
    fn focus_excludes_mechanical_by_default() {
        let f = focus_between(&data(), 180 * DAY, i64::MAX, AreaBy::Crate, false, 10);
        assert_eq!(f[0].area, "crates/gui");
        assert!(f.iter().all(|a| a.area != "crates/core"), "core only had an fmt change recently");
        let all = focus_between(&data(), 180 * DAY, i64::MAX, AreaBy::Crate, true, 10);
        assert!(all.iter().any(|a| a.area == "crates/core"));
    }

    #[test]
    fn log_follows_lineage_with_weight() {
        let mut d = data();
        d.commits.push(c("s0", 5, "feat: engine"));
        d.changes.push(ch("s0", "crates/gui/src/engine.rs", false));
        let l = log(&d, "crates/gui/src/host.rs", false, 10);
        assert_eq!(l.iter().map(|e| e.sha.as_str()).collect::<Vec<_>>(), ["s4", "s0"]);
        assert_eq!(l[1].weight, 0.5);
        assert!(l.iter().all(|e| e.inner.is_empty()), "the facade fills inner subjects");
    }

    #[test]
    fn forgotten_ranks_old_high_fan_in_live_areas() {
        let live: HashSet<String> = ["crates/core/src/lib.rs", "crates/gui/src/a.rs"].map(String::from).into();
        let fan_in = HashMap::from([("crates/core".to_string(), 40)]);
        let f = forgotten(&data(), NOW, 30, &live, &fan_in, AreaBy::Crate, 5);
        assert_eq!(f[0].area, "crates/core");
        assert_eq!(f[0].days_since, 190, "the fmt commit on day 198 does not count");
        assert!(f.iter().all(|x| x.area != "crates/gui"), "gui was touched 5 days ago");
    }

    #[test]
    fn fan_in_counts_only_declared_crate_dependencies() {
        let g = json!({"links": [
            {"source": "crates/a/src/x.rs::f", "target": "crates/b/src/y.rs::g"},
            {"source": "crates/c/src/x.rs::f", "target": "crates/b/src/y.rs::temp_dir"},
            {"source": "crates/b/src/z.rs::h", "target": "crates/b/src/y.rs::g"},
            {"source": "cmd:x", "target": "crates/b/src/y.rs::g"}]});
        let allowed = allowed_crate_edges(&json!({"schema_version": 1, "edges": [["a", "b"]]}));
        assert_eq!(fan_in_by_area(&g, AreaBy::Crate, &allowed), HashMap::from([("crates/b".to_string(), 1)]),
            "c->b is a stdlib homonym (c does not depend on b); b->b is the same area");
        let g2 = json!({"links": [{"source": "crates/b/src/z.rs::h", "target": "crates/b/tests/y.rs::g"}]});
        assert_eq!(fan_in_by_area(&g2, AreaBy::Dir, &HashSet::new()), HashMap::from([("crates/b/tests".to_string(), 1)]),
            "same crate, different dirs needs no allowlist entry");
    }

    #[test]
    fn search_matches_subject_paths_and_symbols() {
        assert_eq!(search(&data(), "panel", 5)[0].sha, "s2");
        assert!(search(&data(), "Engine::run", 5).len() >= 3, "symbol ids are searchable");
        assert!(search(&data(), "", 5).is_empty());
    }

    #[test]
    fn timeline_buckets_and_brief_line() {
        let t = timeline(&data(), 186 * DAY, NOW, 7, AreaBy::Crate, 3);
        assert_eq!(t.len(), 2);
        assert!(t.iter().flat_map(|b| &b.highlights).any(|h| h == "feat: gui panel"));
        let live: HashSet<String> = ["crates/core/src/lib.rs"].map(String::from).into();
        let b = brief(&data(), NOW, &live, &HashMap::new());
        assert!(b.starts_with("history: focus 7d: crates/gui (1)") && b.contains("forgotten: crates/core"), "{b}");
    }
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --lib history::query`
Expected: FAIL (compile error: cannot find `focus_between`, `log`, …).

- [ ] **Step 3: Implement**

```rust
//! Read-side views over loaded history (spec §6). Mechanical rows are excluded unless
//! `include_mechanical`. Everything is an in-memory scan.
//! ponytail: O(rows) per query; move to vox-db tables if a query passes ~200 ms.
use super::model::{CommitRec, HistoryData};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) const DAY: i64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaBy {
    Crate,
    Dir,
}

/// `crates/vox-cli/src/x.rs` -> `crates/vox-cli` (Crate) / `crates/vox-cli/src` (Dir).
pub fn area_of(path: &str, by: AreaBy) -> String {
    match by {
        AreaBy::Dir => path.rsplit_once('/').map_or(".", |(d, _)| d).to_string(),
        AreaBy::Crate => {
            let mut it = path.split('/');
            match (it.next(), it.next(), it.next()) {
                (Some(top @ ("crates" | "apps" | "clients" | "docs" | "contracts")), Some(second), Some(_)) => format!("{top}/{second}"),
                (Some(top), Some(_), _) => top.to_string(),
                _ => ".".to_string(),
            }
        }
    }
}

fn index(d: &HistoryData) -> HashMap<&str, &CommitRec> {
    d.commits.iter().map(|c| (c.sha.as_str(), c)).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AreaScore {
    pub area: String,
    pub commits: u32,
    pub lines: u32,
}

pub fn focus_between(d: &HistoryData, from_ts: i64, to_ts: i64, by: AreaBy, include_mechanical: bool, limit: usize) -> Vec<AreaScore> {
    let idx = index(d);
    let mut acc: HashMap<String, (HashSet<&str>, u32)> = HashMap::new();
    for ch in d.changes.iter().filter(|c| include_mechanical || !c.mechanical) {
        let Some(c) = idx.get(ch.sha.as_str()) else { continue };
        if c.ts < from_ts || c.ts >= to_ts {
            continue;
        }
        let e = acc.entry(area_of(&ch.path, by)).or_default();
        e.0.insert(ch.sha.as_str());
        e.1 += ch.added + ch.deleted;
    }
    let mut v: Vec<AreaScore> = acc.into_iter()
        .map(|(area, (shas, lines))| AreaScore { area, commits: shas.len() as u32, lines }).collect();
    v.sort_by(|a, b| b.commits.cmp(&a.commits).then(b.lines.cmp(&a.lines)).then(a.area.cmp(&b.area)));
    v.truncate(limit);
    v
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LogEntry {
    pub sha: String,
    pub ts: i64,
    pub subject: String,
    pub agent: Option<String>,
    /// The path at the time (an ancestor path when reached through lineage).
    pub path: String,
    pub added: u32,
    pub deleted: u32,
    /// Product of lineage weights from `path` back to this row's file (1.0 = same file).
    pub weight: f32,
    pub mechanical: bool,
    /// For merge rows: inner subjects that touched `path` (filled by `api::answer`).
    pub inner: Vec<String>,
}

/// Changes to `path` and, through lineage, to what it was renamed/split/merged from.
/// An ancestor's rows count only up to the commit that created the edge.
pub fn log(d: &HistoryData, path: &str, include_mechanical: bool, limit: usize) -> Vec<LogEntry> {
    let idx = index(d);
    let ts_of = |sha: &str| idx.get(sha).map_or(i64::MIN, |c| c.ts);
    let mut queue = VecDeque::from([(path.to_string(), 1.0f32, i64::MAX)]);
    let mut seen = HashSet::new();
    let mut scope = Vec::new();
    while let Some((p, w, until)) = queue.pop_front() {
        if scope.len() >= 256 || !seen.insert(p.clone()) {
            continue;
        }
        for e in d.lineage.iter().filter(|e| e.to == p) {
            let t = ts_of(&e.sha);
            if t <= until {
                queue.push_back((e.from.clone(), w * e.weight, t));
            }
        }
        scope.push((p, w, until));
    }
    let mut out = Vec::new();
    for (p, w, until) in &scope {
        for ch in d.changes.iter().filter(|c| &c.path == p && (include_mechanical || !c.mechanical)) {
            let Some(c) = idx.get(ch.sha.as_str()) else { continue };
            if c.ts <= *until {
                out.push(LogEntry { sha: c.sha.clone(), ts: c.ts, subject: c.subject.clone(), agent: c.agent.clone(),
                    path: p.clone(), added: ch.added, deleted: ch.deleted, weight: *w, mechanical: ch.mechanical,
                    inner: Vec::new() });
            }
        }
    }
    out.sort_by(|a, b| b.ts.cmp(&a.ts).then(a.path.cmp(&b.path)));
    out.truncate(limit);
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Forgotten {
    pub area: String,
    pub days_since: i64,
    pub last_subject: String,
    pub fan_in: u32,
    pub score: f64,
}

/// Live areas ranked by days since their last meaningful change × ln(2 + fan-in), so a
/// dormant module others depend on outranks a dormant leaf. Areas with only mechanical
/// changes count from the start of recorded history.
pub fn forgotten(d: &HistoryData, now_ts: i64, min_age_days: i64, current_paths: &HashSet<String>,
    fan_in: &HashMap<String, u32>, by: AreaBy, limit: usize) -> Vec<Forgotten> {
    let live: HashSet<String> = current_paths.iter().map(|p| area_of(p, by)).collect();
    let idx = index(d);
    let mut last: HashMap<String, (i64, String)> = HashMap::new();
    let mut seen_areas = HashSet::new();
    for ch in &d.changes {
        let area = area_of(&ch.path, by);
        if !live.contains(&area) {
            continue;
        }
        seen_areas.insert(area.clone());
        let Some(c) = idx.get(ch.sha.as_str()) else { continue };
        if ch.mechanical {
            continue;
        }
        let slot = last.entry(area).or_insert((i64::MIN, String::new()));
        if c.ts > slot.0 {
            *slot = (c.ts, c.subject.clone());
        }
    }
    let history_start = d.commits.iter().map(|c| c.ts).min().unwrap_or(now_ts);
    for area in seen_areas {
        last.entry(area).or_insert((history_start, "(no meaningful change in recorded history)".into()));
    }
    let mut v: Vec<Forgotten> = last.into_iter().filter_map(|(area, (ts, subject))| {
        let days = (now_ts - ts) / DAY;
        let fi = fan_in.get(&area).copied().unwrap_or(0);
        (days >= min_age_days).then(|| Forgotten { score: days as f64 * (2.0 + f64::from(fi)).ln(), fan_in: fi, days_since: days, last_subject: subject, area })
    }).collect();
    v.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.area.cmp(&b.area)));
    v.truncate(limit);
    v
}

/// `crates/<name>/…` -> `<name>` (directory name == package name for every crate).
fn crate_of(path: &str) -> Option<&str> {
    let mut it = path.split('/');
    (it.next() == Some("crates")).then(|| it.next()).flatten()
}

/// `{"edges": [[from, to], …]}` (`contracts/ci/crate-edges.allow.v1.json`) -> set of pairs.
pub fn allowed_crate_edges(json: &serde_json::Value) -> HashSet<(String, String)> {
    json.get("edges").and_then(|v| v.as_array()).into_iter().flatten()
        .filter_map(|p| Some((p.get(0)?.as_str()?.to_string(), p.get(1)?.as_str()?.to_string())))
        .collect()
}

/// Incoming cross-area call edges per area from a graphify `graph.json`. A cross-crate edge
/// counts only when `allowed` declares that dependency, which drops stdlib homonyms that
/// name resolution bound to repo fns (e.g. `std::env::temp_dir` -> a test helper). Edges
/// touching files outside `crates/` are not counted.
pub fn fan_in_by_area(graph: &serde_json::Value, by: AreaBy, allowed: &HashSet<(String, String)>) -> HashMap<String, u32> {
    let edges = graph.get("links").or_else(|| graph.get("edges")).and_then(|v| v.as_array());
    let mut m = HashMap::new();
    for e in edges.into_iter().flatten() {
        let file = |k: &str| e.get(k).and_then(|v| v.as_str()).and_then(|id| id.split_once("::")).map(|(p, _)| p);
        let (Some(sp), Some(tp)) = (file("source"), file("target")) else { continue };
        let (Some(sc), Some(tc)) = (crate_of(sp), crate_of(tp)) else { continue };
        let (sa, ta) = (area_of(sp, by), area_of(tp, by));
        if sa != ta && (sc == tc || allowed.contains(&(sc.to_string(), tc.to_string()))) {
            *m.entry(ta).or_insert(0) += 1;
        }
    }
    m
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchHit {
    pub sha: String,
    pub ts: i64,
    pub subject: String,
    pub score: u32,
}

/// Term counting: subject ×3, merged-branch subjects ×2, touched paths/symbols ×2, body ×1.
/// ponytail: plain term matching; move to vox-search (tantivy + semantic) when it misses.
pub fn search(d: &HistoryData, query: &str, limit: usize) -> Vec<SearchHit> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let mut touched: HashMap<&str, String> = HashMap::new();
    for ch in &d.changes {
        let s = touched.entry(ch.sha.as_str()).or_default();
        s.push_str(&ch.path.to_lowercase());
        s.push('\n');
        for sym in ch.symbols.iter().flat_map(|x| x.added.iter().chain(&x.removed)) {
            s.push_str(&sym.to_lowercase());
            s.push('\n');
        }
    }
    let mut v: Vec<SearchHit> = d.commits.iter().filter_map(|c| {
        let (subj, body, inner) = (c.subject.to_lowercase(), c.body.to_lowercase(), c.inner_subjects.join("\n").to_lowercase());
        let files = touched.get(c.sha.as_str()).map_or("", String::as_str);
        let score: u32 = terms.iter().map(|t| {
            let t = t.as_str();
            3 * u32::from(subj.contains(t)) + 2 * u32::from(inner.contains(t)) + 2 * u32::from(files.contains(t)) + u32::from(body.contains(t))
        }).sum();
        (score > 0).then(|| SearchHit { sha: c.sha.clone(), ts: c.ts, subject: c.subject.clone(), score })
    }).collect();
    v.sort_by(|a, b| b.score.cmp(&a.score).then(b.ts.cmp(&a.ts)));
    v.truncate(limit);
    v
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bucket {
    pub start_ts: i64,
    pub top: Vec<AreaScore>,
    pub highlights: Vec<String>,
}

/// Fixed-width buckets from `since_ts`: top areas plus up to 5 feat/merge subjects each.
pub fn timeline(d: &HistoryData, since_ts: i64, now_ts: i64, bucket_days: i64, by: AreaBy, per_bucket: usize) -> Vec<Bucket> {
    let width = bucket_days.max(1) * DAY;
    let mut out = Vec::new();
    let mut start = since_ts;
    while start < now_ts {
        let end = start + width;
        let highlights = d.commits.iter()
            .filter(|c| c.ts >= start && c.ts < end && !c.mechanical && (c.is_merge || c.subject.starts_with("feat")))
            .take(5).map(|c| c.subject.clone()).collect();
        out.push(Bucket { start_ts: start, top: focus_between(d, start, end, by, false, per_bucket), highlights });
        start = end;
    }
    out
}

/// One line for SessionStart: top-3 focus areas over 7 days and the top forgotten area.
pub fn brief(d: &HistoryData, now_ts: i64, current_paths: &HashSet<String>, fan_in: &HashMap<String, u32>) -> String {
    let focus = focus_between(d, now_ts - 7 * DAY, i64::MAX, AreaBy::Crate, false, 3);
    let f = if focus.is_empty() {
        "none".to_string()
    } else {
        focus.iter().map(|a| format!("{} ({})", a.area, a.commits)).collect::<Vec<_>>().join(", ")
    };
    let g = forgotten(d, now_ts, 30, current_paths, fan_in, AreaBy::Crate, 1).into_iter().next()
        .map_or("none".to_string(), |x| format!("{} ({}d, fan-in {})", x.area, x.days_since, x.fan_in));
    format!("history: focus 7d: {f} · forgotten: {g}")
}
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p vox-graph-reader --lib history::query && grep -cv '^\s*$' crates/vox-graph-reader/src/history/query.rs`
Expected: PASS (7 tests), and a line count ≤500. If it goes over, move `search` and `SearchHit` into `history/search.rs`, together with their test.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-graph-reader
git add crates/vox-graph-reader/src/history
git commit -m "feat(graph): add focus/log/forgotten/search/timeline history views"
```

---

### Task 8: Facade + `vox graph history` CLI

<!-- AMENDED: G1, G2, G6, G7, G10, G14, G15 — common-dir versioned store, origin/main tip, brief never ingests, {complete,behind,rows}, fan-in cache, merge inner subjects, version pruning -->

**Files:**
- Create: `crates/vox-graph-reader/src/history/api.rs`, `crates/vox-graph-reader/tests/history_api_tests.rs`, `crates/vox-cli/src/commands/graphify/history.rs`, `crates/vox-cli/tests/graph_history_cli.rs`
- Modify:
  - `crates/vox-graph-reader/src/history/mod.rs` (`pub mod api;`)
  - `crates/vox-cli/src/commands/graphify/mod.rs` (one enum variant, one match arm, `mod history;`)
  - `contracts/operations/catalog.v1.yaml`, `docs/src/reference/cli.md`

**Interfaces:**
- Consumes: Tasks 2–7
- Produces:
  - `history::api::CORPUS_ID: &str = "repo-history"`
  - `history::api::HistoryQuery`: a serde enum tagged by `"query"` with variants `log|focus|forgotten|search|timeline|brief`, used as MCP params too
  - `history::api::Paths<'a> { repo_root: &'a Path, code_graph: Option<&'a Path> }` (the store always resolves via `store_dir`; tests use a tempdir repo, whose common dir is the tempdir's `.git`)
  <!-- AMENDED: #15 — the `store_dir` override let pruning run on an arbitrary parent directory -->
  - `history::api::Answer { value: serde_json::Value /* {complete, behind, rows} */, text: String, catch_up: CatchUp, complete: bool, behind: usize }`
  - `history::api::store_dir(&Git) -> io::Result<PathBuf>`
  - `history::api::answer(&Paths, &HistoryQuery, now_ts: i64, budget: Option<Duration>, progress: &mut dyn FnMut(usize, usize)) -> io::Result<Answer>`

<!-- AMENDED: #11 — the facade tests move to an integration file on the shared `common::Repo` fixture; the old inline fixture skipped `commit.gpgsign=false` and fails on signing hosts -->
- [ ] **Step 1: Add `pub mod api;` to `history/mod.rs`, then write the failing tests.** Put the in-file test at the bottom of `api.rs` (it covers the private `ymd`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_json_defaults_and_ymd() {
        let q: HistoryQuery = serde_json::from_str(r#"{"query":"focus"}"#).unwrap();
        assert_eq!(q, HistoryQuery::Focus { since_days: 30, by: AreaBy::Crate, include_mechanical: false, limit: 20 });
        let q: HistoryQuery = serde_json::from_str(r#"{"query":"log","path":"a.rs"}"#).unwrap();
        assert!(matches!(q, HistoryQuery::Log { ref path, limit: 20, .. } if path == "a.rs"));
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(1_758_499_200), "2025-09-22");
    }
}
```

Then write `crates/vox-graph-reader/tests/history_api_tests.rs`:

```rust
mod common;
use common::Repo;
use std::time::Duration;
use vox_graph_reader::history::api::{HistoryQuery, Paths, answer, store_dir};
use vox_graph_reader::history::git::Git;
use vox_graph_reader::history::ingest::CatchUp;
use vox_graph_reader::history::query::AreaBy;
use vox_graph_reader::history::store::HistoryStore;

const FOCUS: HistoryQuery = HistoryQuery::Focus { since_days: 30, by: AreaBy::Crate, include_mechanical: false, limit: 5 };

fn repo() -> Repo {
    let r = Repo::new();
    r.write("crates/k/src/lib.rs", "fn a() {}\n");
    r.commit("feat: k");
    r
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

fn store_of(r: &Repo) -> HistoryStore {
    HistoryStore::open(&store_dir(&Git::new(r.path())).unwrap()).unwrap()
}

#[test]
fn answer_catches_up_then_reports_complete_rows() {
    let r = repo();
    let p = Paths { repo_root: r.path(), code_graph: None };
    let a = answer(&p, &FOCUS, now(), None, &mut |_, _| {}).unwrap();
    assert_eq!(a.catch_up, CatchUp::Done { ingested: 1, rebuilt: false });
    assert!(a.complete && a.value["complete"] == true && a.value["behind"] == 0);
    assert_eq!(a.value["rows"][0]["area"], "crates/k");
    assert!(store_dir(&Git::new(r.path())).unwrap().starts_with(r.path().canonicalize().unwrap().join(".git")),
        "store lives under the repo's git common dir");
}

#[test]
fn brief_never_ingests_and_says_how_far_behind() {
    let r = repo();
    let p = Paths { repo_root: r.path(), code_graph: None };
    let a = answer(&p, &HistoryQuery::Brief, now(), None, &mut |_, _| {}).unwrap();
    assert_eq!(a.catch_up, CatchUp::Skipped);
    assert!(!a.complete && a.text.contains("1 commits behind"), "{}", a.text);
    assert!(store_of(&r).load().unwrap().commits.is_empty());
}

#[test]
fn over_budget_answer_is_marked_partial() {
    let r = repo();
    let p = Paths { repo_root: r.path(), code_graph: None };
    let a = answer(&p, &FOCUS, now(), Some(Duration::ZERO), &mut |_, _| {}).unwrap();
    assert!(!a.complete && a.value["complete"] == false);
    assert!(a.text.starts_with("partial: 1 commits not yet ingested; call again."), "{}", a.text);
}

#[test]
fn busy_store_answers_from_disk_and_is_incomplete() {
    let r = repo();
    let s = store_of(&r);
    let _held = s.try_lock().unwrap().expect("lock");
    let a = answer(&Paths { repo_root: r.path(), code_graph: None }, &FOCUS, now(), None, &mut |_, _| {}).unwrap();
    assert_eq!(a.catch_up, CatchUp::Busy);
    assert!(!a.complete);
}
```

If `starts_with(... .git)` fails on macOS because of the `/private` prefix, canonicalize both sides.

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p vox-graph-reader --lib history::api; cargo test -p vox-graph-reader --test history_api_tests`
Expected: FAIL (compile errors: `answer`, `store_dir`, `ymd` not found).

- [ ] **Step 3: Implement `api.rs` above the tests**

```rust
//! Single entry point for CLI and MCP: resolve the shared store, catch up under the ingest
//! lock (except `brief`), answer, and say whether the answer is complete.
use super::git::Git;
use super::ingest::{CatchUp, catch_up};
use super::model::{HistoryData, store_dir_name};
use super::query::{self, AreaBy, DAY, LogEntry};
use super::store::{HistoryStore, prune_old_versions};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CORPUS_ID: &str = "repo-history";
/// Sibling version stores unused this long are deleted.
const VERSION_MAX_AGE: Duration = Duration::from_secs(30 * 86_400);
const ALLOWLIST: &str = "contracts/ci/crate-edges.allow.v1.json";

fn d7() -> i64 { 7 }
fn d20() -> usize { 20 }
fn d30() -> i64 { 30 }
fn d60() -> i64 { 60 }
fn d90() -> i64 { 90 }
fn by_crate() -> AreaBy { AreaBy::Crate }

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case")]
pub enum HistoryQuery {
    Log { path: String, #[serde(default)] include_mechanical: bool, #[serde(default = "d20")] limit: usize },
    Focus { #[serde(default = "d30")] since_days: i64, #[serde(default = "by_crate")] by: AreaBy, #[serde(default)] include_mechanical: bool, #[serde(default = "d20")] limit: usize },
    Forgotten { #[serde(default = "d60")] min_age_days: i64, #[serde(default = "by_crate")] by: AreaBy, #[serde(default = "d20")] limit: usize },
    Search { text: String, #[serde(default = "d20")] limit: usize },
    Timeline { #[serde(default = "d90")] since_days: i64, #[serde(default = "d7")] bucket_days: i64, #[serde(default = "by_crate")] by: AreaBy },
    Brief,
}

pub struct Paths<'a> {
    pub repo_root: &'a Path,
    /// `repo-code-graph` graph.json, for `forgotten` fan-in. Missing -> fan-in 0.
    pub code_graph: Option<&'a Path>,
}

#[derive(Debug)]
pub struct Answer {
    /// `{"complete": bool, "behind": n, "rows": …}`
    pub value: serde_json::Value,
    pub text: String,
    pub catch_up: CatchUp,
    pub complete: bool,
    pub behind: usize,
}

/// The store shared by every worktree, for this binary's schema/extractor version.
pub fn store_dir(git: &Git) -> io::Result<PathBuf> {
    Ok(git.common_dir()?.join("vox-cache").join(CORPUS_ID).join(store_dir_name()))
}

pub fn answer(p: &Paths, q: &HistoryQuery, now_ts: i64, budget: Option<Duration>, progress: &mut dyn FnMut(usize, usize)) -> io::Result<Answer> {
    let git = Git::new(p.repo_root);
    let tip = git.default_tip();
    let dir = store_dir(&git)?;
    let store = HistoryStore::open(&dir)?;
    let brief = *q == HistoryQuery::Brief;
    let cu = if brief {
        CatchUp::Skipped
    } else {
        match store.try_lock()? {
            Some(lock) => {
                if let Some(root) = dir.parent() {
                    prune_old_versions(root, &dir, VERSION_MAX_AGE); // best effort, never fails the query
                }
                catch_up(p.repo_root, &store, &lock, &tip, budget, progress)?
            }
            None => CatchUp::Busy,
        }
    };
    // AMENDED: #8 — a broken chain is never reported as complete.
    let (d, intact) = store.load_checked()?;
    let behind = git.first_parent_range(d.commits.last().map(|c| c.sha.as_str()), &tip)?.len();
    let complete = intact && behind == 0;
    let (current, fan_in) = match q {
        HistoryQuery::Forgotten { by, .. } => live_and_fan_in(p, &git, &tip, &dir, *by)?,
        HistoryQuery::Brief => live_and_fan_in(p, &git, &tip, &dir, AreaBy::Crate)?,
        _ => Default::default(),
    };
    let (rows, text) = match q {
        HistoryQuery::Log { path, include_mechanical, limit } => {
            let mut rows = query::log(&d, path, *include_mechanical, *limit);
            fill_inner(&git, &d, &mut rows)?;
            render(rows, |e| {
                let via = if e.path == *path { String::new() } else { format!(" via {} (w={:.2})", e.path, e.weight) };
                let inner: String = e.inner.iter().map(|s| format!("\n    · {s}")).collect();
                format!("{} {} +{}/-{} {}{via}{inner}", short(&e.sha), ymd(e.ts), e.added, e.deleted, e.subject)
            })
        }
        HistoryQuery::Focus { since_days, by, include_mechanical, limit } => render(
            query::focus_between(&d, now_ts - since_days * DAY, i64::MAX, *by, *include_mechanical, *limit),
            |a| format!("{:>5} commits {:>7} lines  {}", a.commits, a.lines, a.area)),
        HistoryQuery::Forgotten { min_age_days, by, limit } => render(
            query::forgotten(&d, now_ts, *min_age_days, &current, &fan_in, *by, *limit),
            |f| format!("{:>5}d  fan-in {:>4}  {}  — {}", f.days_since, f.fan_in, f.area, f.last_subject)),
        HistoryQuery::Search { text, limit } => render(query::search(&d, text, *limit),
            |h| format!("{} {} {}", short(&h.sha), ymd(h.ts), h.subject)),
        HistoryQuery::Timeline { since_days, bucket_days, by } => render(
            query::timeline(&d, now_ts - since_days * DAY, now_ts, *bucket_days, *by, 3),
            |b| {
                let areas = b.top.iter().map(|a| format!("{} ({})", a.area, a.commits)).collect::<Vec<_>>().join(", ");
                format!("{}  {areas}{}", ymd(b.start_ts), b.highlights.iter().map(|h| format!("\n    · {h}")).collect::<String>())
            }),
        HistoryQuery::Brief => {
            let s = query::brief(&d, now_ts, &current, &fan_in);
            (serde_json::Value::String(s.clone()), s)
        }
    };
    let text = match (complete, brief) {
        (true, _) => text,
        (false, true) => format!("{text} · {behind} commits behind (run `vox graph history focus` to catch up)"),
        (false, false) => format!("partial: {behind} commits not yet ingested; call again.\n{text}"),
    };
    let value = serde_json::json!({ "complete": complete, "behind": behind, "rows": rows });
    Ok(Answer { value, text, catch_up: cu, complete, behind })
}

/// Merge rows: up to 5 inner subjects that touched the row's path.
fn fill_inner(git: &Git, d: &HistoryData, rows: &mut [LogEntry]) -> io::Result<()> {
    let merges: HashSet<&str> = d.commits.iter().filter(|c| c.is_merge).map(|c| c.sha.as_str()).collect();
    for e in rows.iter_mut().filter(|e| merges.contains(e.sha.as_str())) {
        let subjects = git.inner_subjects(&e.sha, Some(&e.path))?;
        let extra = subjects.len().saturating_sub(5);
        e.inner = subjects.into_iter().take(5).collect();
        if extra > 0 {
            e.inner.push(format!("+{extra} more"));
        }
    }
    Ok(())
}

fn live_and_fan_in(p: &Paths, git: &Git, tip: &str, dir: &Path, by: AreaBy) -> io::Result<(HashSet<String>, HashMap<String, u32>)> {
    let current = git.ls_tree(tip)?;
    let fan_in = match p.code_graph {
        Some(g) => cached_fan_in(dir, g, &p.repo_root.join(ALLOWLIST), by),
        None => HashMap::new(),
    };
    Ok((current, fan_in))
}

/// Fan-in per area, cached in `<store>/fan_in.json`. The cache is keyed by the size and mtime
/// of graph.json and the allowlist, plus the grouping, so a warm `brief` never parses the 14 MB graph.
fn cached_fan_in(dir: &Path, graph: &Path, allow: &Path, by: AreaBy) -> HashMap<String, u32> {
    let stamp = |p: &Path| std::fs::metadata(p).ok().map(|m| {
        let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
        (m.len(), t)
    });
    let key = format!("{by:?}:{:?}:{:?}", stamp(graph), stamp(allow));
    let cache = dir.join("fan_in.json");
    let cached: Option<(String, HashMap<String, u32>)> = std::fs::read(&cache).ok().and_then(|b| serde_json::from_slice(&b).ok());
    if let Some((k, m)) = cached {
        if k == key {
            return m;
        }
    }
    let read = |p: &Path| std::fs::read(p).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    let allowed = read(allow).map(|v| query::allowed_crate_edges(&v)).unwrap_or_default();
    let m = read(graph).map(|v| query::fan_in_by_area(&v, by, &allowed)).unwrap_or_default();
    let _ = std::fs::write(&cache, serde_json::to_vec(&(key, &m)).unwrap_or_default());
    m
}

fn render<T: serde::Serialize>(rows: Vec<T>, line: impl Fn(&T) -> String) -> (serde_json::Value, String) {
    let text = rows.iter().map(&line).collect::<Vec<_>>().join("\n");
    (serde_json::to_value(&rows).unwrap_or_default(), text)
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(9)]
}

/// Unix seconds -> `YYYY-MM-DD` (UTC), Hinnant's days-to-civil.
fn ymd(ts: i64) -> String {
    let z = ts.div_euclid(DAY) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
```

Run `cargo test -p vox-graph-reader --lib history::api && cargo test -p vox-graph-reader --test history_api_tests && grep -cv '^\s*$' crates/vox-graph-reader/src/history/api.rs` and expect PASS (1 + 4 tests) with a line count ≤500.

- [ ] **Step 4: Write the failing CLI parse test `crates/vox-cli/tests/graph_history_cli.rs`**

```rust
use clap::Parser;
use vox_cli::VoxCliRoot;

#[test]
fn graph_history_subcommands_parse() {
    for argv in [
        vec!["vox", "graph", "history", "brief"],
        vec!["vox", "graph", "history", "log", "crates/vox-cli/src/main.rs", "--limit", "5"],
        vec!["vox", "graph", "history", "focus", "--since-days", "7", "--by", "dir", "--json"],
        vec!["vox", "graph", "history", "forgotten", "--min-age-days", "90"],
        vec!["vox", "graph", "history", "search", "hakari"],
        vec!["vox", "graph", "history", "timeline", "--bucket-days", "14"],
    ] {
        VoxCliRoot::try_parse_from(&argv).unwrap_or_else(|e| panic!("{argv:?}: {e}"));
    }
}
```

Run: `cargo test -p vox-cli --test graph_history_cli`
Expected: FAIL (`history` is an unrecognized subcommand).

- [ ] **Step 5: Write `crates/vox-cli/src/commands/graphify/history.rs`**

```rust
//! `vox graph history` — first-parent repo history
//! (docs/superpowers/specs/2026-09-22-repo-history-graph-design.md).
use clap::{Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use vox_graph_reader::history::api::{self, HistoryQuery, Paths};
use vox_graph_reader::history::ingest::CatchUp;
use vox_graph_reader::history::query::AreaBy;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum By {
    Crate,
    Dir,
}

impl From<By> for AreaBy {
    fn from(b: By) -> Self {
        match b {
            By::Crate => AreaBy::Crate,
            By::Dir => AreaBy::Dir,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum HistoryCmd {
    /// Changes to a file, following renames/splits/merges back through lineage.
    Log {
        path: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        include_mechanical: bool,
        #[arg(long)]
        json: bool,
    },
    /// Where effort went: non-mechanical churn per area.
    Focus {
        #[arg(long, default_value_t = 30)]
        since_days: i64,
        #[arg(long, value_enum, default_value_t = By::Crate)]
        by: By,
        #[arg(long)]
        include_mechanical: bool,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Live areas ranked by time since last meaningful change × fan-in.
    Forgotten {
        #[arg(long, default_value_t = 60)]
        min_age_days: i64,
        #[arg(long, value_enum, default_value_t = By::Crate)]
        by: By,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Search commit subjects, bodies, merged-branch subjects, paths, and symbols.
    Search {
        text: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Top areas and feat/merge highlights per time bucket.
    Timeline {
        #[arg(long, default_value_t = 90)]
        since_days: i64,
        #[arg(long, default_value_t = 7)]
        bucket_days: i64,
        #[arg(long, value_enum, default_value_t = By::Crate)]
        by: By,
        #[arg(long)]
        json: bool,
    },
    /// One SessionStart line from data on disk. Never ingests, never fails.
    Brief,
}

fn to_query(cmd: HistoryCmd) -> (HistoryQuery, bool) {
    match cmd {
        HistoryCmd::Log { path, limit, include_mechanical, json } => (HistoryQuery::Log { path, include_mechanical, limit }, json),
        HistoryCmd::Focus { since_days, by, include_mechanical, limit, json } => (HistoryQuery::Focus { since_days, by: by.into(), include_mechanical, limit }, json),
        HistoryCmd::Forgotten { min_age_days, by, limit, json } => (HistoryQuery::Forgotten { min_age_days, by: by.into(), limit }, json),
        HistoryCmd::Search { text, limit, json } => (HistoryQuery::Search { text, limit }, json),
        HistoryCmd::Timeline { since_days, bucket_days, by, json } => (HistoryQuery::Timeline { since_days, bucket_days, by: by.into() }, json),
        HistoryCmd::Brief => (HistoryQuery::Brief, false),
    }
}

pub fn run(cmd: HistoryCmd, repo_root: &Path, code_graph: Option<PathBuf>) -> anyhow::Result<()> {
    let (q, json) = to_query(cmd);
    let brief = q == HistoryQuery::Brief;
    let paths = Paths { repo_root, code_graph: code_graph.as_deref() };
    let mut progress = |done: usize, total: usize| {
        if done == total || done % 100 == 0 {
            eprintln!("history: ingested {done}/{total}");
        }
    };
    match api::answer(&paths, &q, chrono::Utc::now().timestamp(), None, &mut progress) {
        Ok(a) => {
            if a.catch_up == CatchUp::Busy {
                eprintln!("history: another ingest is running; showing data on disk");
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&a.value)?);
            } else {
                println!("{}", a.text);
            }
            Ok(())
        }
        Err(e) if brief => {
            println!("history: unavailable ({e})");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

// AMENDED: #3 — tdd-guard needs an in-file test for this file's `pub fn`; `tests/` doesn't count.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_query_maps_flags() {
        let (q, json) = to_query(HistoryCmd::Focus { since_days: 7, by: By::Dir, include_mechanical: true, limit: 3, json: true });
        assert!(json);
        assert_eq!(q, HistoryQuery::Focus { since_days: 7, by: AreaBy::Dir, include_mechanical: true, limit: 3 });
        assert_eq!(to_query(HistoryCmd::Brief), (HistoryQuery::Brief, false));
    }
}
```

- [ ] **Step 6: Wire it into `graphify/mod.rs`**
  - Add `mod history;` below the `use` block.
  - Add this variant to `GraphifyCmd`:

```rust
    /// First-parent repo history: log, focus, forgotten, search, timeline, brief.
    History {
        #[command(subcommand)]
        cmd: history::HistoryCmd,
    },
```

  - Add this arm to `run`'s `match cmd`:

```rust
        GraphifyCmd::History { cmd } => {
            let code_graph = load_graphify_corpora(repo_root).ok()
                .and_then(|r| r.corpora.into_iter().find(|c| c.id == "repo-code-graph"))
                .map(|c| repo_root.join(c.graph_path));
            history::run(cmd, repo_root, code_graph)
        }
```

- [ ] **Step 7: Register the command**
  - In `contracts/operations/catalog.v1.yaml`, add this entry directly after the `graph.ingest` entry:

```yaml
- id: graph.history
  title: Vox Graph History
  description: First-parent origin/main history with lineage and mechanical-change flags (log, focus, forgotten, search, timeline, brief); lazily ingests new commits into a store under the git common dir.
  description_human: null
  product_lane: platform
  intent_tags:
  - retrieval
  - graph
  side_effect_class: writes_files
  scope_kind: repository
  reversible: true
  requires_repo: true
  preferred_for_models: true
  human_takeover_friendly: true
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp: null
  cli:
    path:
    - graph
    - history
    status: active
    latin_ns: pm
    handler_rust: commands::graphify
    feature_gate: null
    catalog_group: null
    ref_cli_required: true
    reachability_required: null
```

  - In `docs/src/reference/cli.md`, add this row under the `vox graph …` table, after `vox graph rebuild`:

```markdown
| `vox graph history <log\|focus\|forgotten\|search\|timeline\|brief>` | First-parent history of `origin/main` (else `main`) with rename/split/merge lineage. Mechanical (fmt/generated/import-only) changes are flagged and hidden unless `--include-mechanical`. Every subcommand except `brief` catches up on new commits first. One store is shared by all worktrees, at `<git-common-dir>/vox-cache/repo-history/`. `brief` prints one line from disk and says how many commits it is behind. |
```

- [ ] **Step 8: Regenerate, then verify**

<!-- AMENDED: #2 — plain `command-sync` only verifies; `--write` regenerates the md, and the paths baseline is refreshed only by the test's env var -->
```bash
cargo run -q -p vox-cli -- ci operations-sync --target cli --write
cargo run -q -p vox-cli -- ci operations-verify
cargo run -q -p vox-cli -- ci command-sync --write
UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline
cargo test -p vox-cli --test command_catalog_paths_baseline
cargo run -q -p vox-cli -- ci command-compliance
cargo test -p vox-cli --test graph_history_cli
cargo test -p vox-cli --lib graphify::history
cargo test -p vox-graph-reader
cargo clippy -p vox-graph-reader -p vox-cli -- -D warnings
```

Expected: all green. The regenerated `cli-command-surface.generated.md`, `command_catalog_paths_baseline.txt` and `contracts/cli/command-registry.yaml` must each show only the new `graph history` rows.

- [ ] **Step 9: Smoke test against the real repo**

```bash
time cargo run -q -p vox-cli -- graph history focus --since-days 30
cargo run -q -p vox-cli -- graph history brief
```

Expected: the first run prints `history: ingested N/…` progress lines and then a ranked area list. Record the wall time in the commit body. If the backfill takes more than 5 minutes, profile before continuing. `brief` then prints a single `history: focus 7d: …` line with no "behind" suffix.

- [ ] **Step 10: Commit**

<!-- AMENDED: #18 — stage explicit paths so unrelated regenerated drift never rides along -->
```bash
cargo fmt -p vox-graph-reader -p vox-cli
git add crates/vox-graph-reader crates/vox-cli/src/commands/graphify crates/vox-cli/tests/graph_history_cli.rs \
  crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt contracts/operations/catalog.v1.yaml \
  contracts/cli/command-registry.yaml docs/src/reference/cli.md docs/src/reference/cli-command-surface.generated.md
git status --short   # anything else modified is unrelated drift: leave it unstaged
git commit -m "feat(cli): add vox graph history (log/focus/forgotten/search/timeline/brief)"
```

---

### Task 9: MCP tool `vox_search_history`

<!-- AMENDED: G7 — 60 s self-imposed budget; completeness in data -->

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/history_tools.rs`
- Modify:
  - `crates/vox-orchestrator-mcp/src/lib.rs` (`pub mod history_tools;` next to `pub mod graph_tools;`)
  - `crates/vox-orchestrator-mcp/src/dispatch.rs`, `crates/vox-orchestrator-mcp/src/input_schemas.rs`
  - `contracts/operations/catalog.v1.yaml`

**Interfaces:**
- Consumes: `vox_graph_reader::history::api::{answer, Answer, HistoryQuery, Paths}`, `vox_graph_reader::history::ingest::CatchUp`, `crate::server_state::ServerState` (`state.repository.root`), `crate::params::ToolResult`
- Produces: `history_tools::history(state: &ServerState, q: HistoryQuery) -> String`, `history_tools::envelope(&Answer) -> String` (crate-visible), and the MCP tool name `vox_search_history`

- [ ] **Step 1: Add `pub mod history_tools;` to `crates/vox-orchestrator-mcp/src/lib.rs`, then write the failing test in `history_tools.rs`** <!-- AMENDED: #9 -->

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use vox_graph_reader::history::ingest::CatchUp;

    #[test]
    fn envelope_carries_completeness_in_data() {
        let a = Answer {
            value: serde_json::json!({"complete": false, "behind": 3, "rows": []}),
            text: "partial: 3 commits not yet ingested; call again.".into(),
            catch_up: CatchUp::OverBudget { ingested: 1, behind: 3 },
            complete: false,
            behind: 3,
        };
        let v: serde_json::Value = serde_json::from_str(&envelope(&a)).unwrap();
        assert_eq!(v["data"]["complete"], false);
        assert_eq!(v["data"]["behind"], 3);
        assert!(v["meta"]["text"].as_str().unwrap().starts_with("partial:"));
    }
}
```

Run: `cargo test -p vox-orchestrator-mcp --lib history_tools`. Expected: FAIL (compile error: cannot find `envelope`, `Answer`).

- [ ] **Step 2: Implement `history_tools.rs`**

```rust
//! `vox_search_history`: first-parent repo history for agents (same facade as `vox graph history`).
use crate::params::ToolResult;
use crate::server_state::ServerState;
use std::time::Duration;
use vox_graph_reader::history::api::{self, Answer, HistoryQuery, Paths};

/// Self-imposed budget, well under the 120 s dispatch timeout. An unfinished backfill stops,
/// releases the lock, keeps its rows, and reports `complete: false` so the agent calls again.
const BUDGET: Duration = Duration::from_secs(60);

/// Rows plus completeness in `data`; catch-up outcome and rendered text in `meta`.
pub(crate) fn envelope(a: &Answer) -> String {
    ToolResult::ok_with_meta(a.value.clone(), serde_json::json!({ "catch_up": format!("{:?}", a.catch_up), "text": a.text })).to_json()
}

pub async fn history(state: &ServerState, q: HistoryQuery) -> String {
    let root = state.repository.root.clone();
    let res = tokio::task::spawn_blocking(move || {
        let code_graph = vox_config::graphify::load_graphify_corpora(&root).ok()
            .and_then(|r| r.corpora.into_iter().find(|c| c.id == "repo-code-graph"))
            .map(|c| root.join(c.graph_path));
        let paths = Paths { repo_root: &root, code_graph: code_graph.as_deref() };
        api::answer(&paths, &q, chrono::Utc::now().timestamp(), Some(BUDGET), &mut |_, _| {})
    }).await;
    match res {
        Ok(Ok(a)) => envelope(&a),
        Ok(Err(e)) => ToolResult::<serde_json::Value>::err_with_remediation(
            format!("history: {e}"), "Run from a git checkout with `origin/main` or `main`; see `vox graph history focus`.").to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err_with_remediation(format!("history task panicked: {e}"), "Report this with the tool arguments.").to_json(),
    }
}
```

- [ ] **Step 3: Wire dispatch and schema**
  - In `dispatch.rs`, next to `"vox_search_compare"`:

```rust
        "vox_search_history" => {
            Ok(crate::history_tools::history(state, serde_json::from_value(args)?).await)
        }
```

  - In `input_schemas.rs`, next to `"vox_search_compare"`:

```rust
        "vox_search_history" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string","enum":["log","focus","forgotten","search","timeline","brief"],"description":"View: log (a file's changes through rename/split/merge lineage), focus (churn per area), forgotten (dormant high-fan-in areas), search (commit text, paths, symbols), timeline (top areas per bucket), brief (one line)"},"path":{"type":"string","description":"log: repo-relative file path"},"text":{"type":"string","description":"search: terms"},"since_days":{"type":"integer","minimum":1,"description":"focus (default 30) / timeline (default 90)"},"min_age_days":{"type":"integer","minimum":0,"description":"forgotten: default 60"},"bucket_days":{"type":"integer","minimum":1,"description":"timeline: default 7"},"by":{"type":"string","enum":["crate","dir"],"description":"Area grouping (default crate)"},"include_mechanical":{"type":"boolean","description":"Include fmt/generated/import-only changes (default false)"},"limit":{"type":"integer","minimum":1,"description":"Max rows (default 20)"}},"required":["query"],"additionalProperties":false}"#,
        ),
```

  - In `catalog.v1.yaml`, add after the `graph.compare` entry:

```yaml
- id: graph.history_mcp
  title: Vox Search History
  description: First-parent repo history for agents (log through lineage, focus, forgotten, search, timeline, brief). Ingests at most 60 s per call into a cache under the git common dir; results carry complete/behind, and an incomplete result means call again.
  description_human: null
  product_lane: platform
  intent_tags:
  - retrieval
  - graph
  side_effect_class: writes_files
  scope_kind: repository
  reversible: true
  requires_repo: true
  preferred_for_models: true
  human_takeover_friendly: true
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp:
    name: vox_search_history
    http_read_role_eligible: false
    tier: core
  cli: null
```

- [ ] **Step 4: Regenerate, then verify**

```bash
cargo run -q -p vox-cli -- ci operations-sync --target all --write
cargo run -q -p vox-cli -- ci operations-verify
cargo run -q -p vox-cli -- ci ssot-drift
cargo test -p vox-orchestrator-mcp --lib history_tools
cargo test -p vox-orchestrator-mcp --lib graph_tools
cargo clippy -p vox-orchestrator-mcp -- -D warnings
```

Expected: green. If `ssot-drift` names another generated artifact (for example `contracts/mcp/tool-registry.canonical.yaml`, `contracts/capability/capability-registry.yaml`, or `model-manifest.generated.json`), run the exact `--write` command it prints and include the result. If a test asserts a fixed MCP tool count, update the count and say so in the commit body.

- [ ] **Step 5: Commit**

<!-- AMENDED: #18 -->
```bash
cargo fmt -p vox-orchestrator-mcp
git add crates/vox-orchestrator-mcp contracts/operations/catalog.v1.yaml
git status --short contracts   # stage only generated files whose diff mentions vox_search_history
git commit -m "feat(mcp): add vox_search_history tool over repo history facade"
```

---

### Task 10: Verification, subject-hint verdict, gated SessionStart hook

<!-- AMENDED: G6, G11, G12, G16 — measured brief latency gate, real common-dir cold store, ignored regressions run by name, subject_hint report, hook gated on owner approval -->

**Files:**
- Modify: `docs/superpowers/specs/2026-09-22-repo-history-graph-design.md` (the §2 verdict)
- Modify, **only with owner approval**: `.claude/settings.json`

**Interfaces:** Consumes: `vox graph history …` (Task 8) and the ignored regressions (Task 6).

<!-- AMENDED: #13, #14 — every Bash call is a fresh shell, so each step starts with this preamble. Deletion is limited to THIS version's directory: the old `rm -rf "$STORE"` wiped every version's store for every worktree, and it wasn't guarded against an empty path -->
**Preamble: run it at the start of every step below.**

```bash
set -euo pipefail
cd /Users/brbrainerd/dev/vox/.worktrees/repo-history-graph
V=./target/release/vox
ROOT="$(git rev-parse --path-format=absolute --git-common-dir)/vox-cache/repo-history"
EXT=$(sed -n 's/^pub const EXTRACTOR_VERSION: &str = "\(.*\)";/\1/p' crates/vox-graph-reader/src/ast.rs)
SCHEMA=$(sed -n 's/^pub const SCHEMA_VERSION: u32 = \([0-9]*\);/\1/p' crates/vox-graph-reader/src/history/model.rs)
DIR="$ROOT/s${SCHEMA:?}-e${EXT:?}"
case "$DIR" in /*/vox-cache/repo-history/s*-e*) ;; *) echo "refusing unexpected store path: $DIR"; exit 1;; esac
```

- [ ] **Step 1: Cold `brief` is bounded and honest**

```bash
cargo build -q -p vox-cli --release
rm -rf "$DIR"
out=$( { /usr/bin/time -p "$V" graph history brief; } 2>&1 ); echo "$out"
echo "$out" | grep -q '^real' || echo "FAIL: no timing captured"
echo "$out" | grep -q "commits behind" || echo "FAIL: cold brief must report commits behind"
echo "$out" | awk '/^real/{exit !($2 <= 1.0)}' || echo "FAIL: cold brief took over 1 s"
```

Expected: one `history: … · N commits behind …` line, no FAIL lines.

- [ ] **Step 2: Backfill, then check that a warm `brief` stays under 1 s**

```bash
time "$V" graph history focus --since-days 30 --limit 8
out=$( { /usr/bin/time -p "$V" graph history brief; } 2>&1 ); echo "$out"
echo "$out" | grep -q "commits behind" && echo "FAIL: warm brief still behind" || true
echo "$out" | awk '/^real/{exit !($2 <= 1.0)}' || echo "FAIL: warm brief took over 1 s"
```

Record the backfill wall time and both `brief` timings in the final report. If either `brief` fails the 1 s gate, profile it (usually `ls-tree` or JSONL load) before continuing.

- [ ] **Step 3: Real-history regressions, run by name**

```bash
cargo test -p vox-graph-reader --test history_ingest_tests -- --ignored real_history
```

Expected: PASS against the committed golden file.

- [ ] **Step 4: Spot-check against spec §10**

```bash
"$V" graph history focus --since-days 30 --limit 8
"$V" graph history log crates/vox-plugin-browser/src/host.rs --limit 5
"$V" graph history forgotten --limit 10
"$V" graph history search hakari --limit 5
```

Expected:
- `crates/vox-gui` near the top of `focus`.
- `log` reaches `engine.rs` via lineage, and the merge row lists "refactor: split browser engine under the god-object cap" among its inner subjects.
- `forgotten` lists existing crates, with no `vox-integration-tests` inflation.

Paste all four outputs into the final report.

- [ ] **Step 5: Subject-hint verdict (spec §2 "measure before deciding")**

```bash
C="$DIR/commits.jsonl"   # this version's store, not whichever directory sorts first
SAMPLE=$(mktemp)
jq -s 'unique_by(.sha) | map(select(.subject_hint != null)) | group_by(.subject_hint)
  | map({hint: .[0].subject_hint, commits: length, mechanical_share: ((map(select(.mechanical)) | length) / length)})' "$C"
jq -rs 'unique_by(.sha) | .[] | select(.subject_hint == "lint" and (.mechanical | not)) | .sha' "$C" | sort -R | head -20 > "$SAMPLE"
while read -r s; do git show --stat --format='%h %s' "$s" | head -15; echo ---; done < "$SAMPLE"
```

`unique_by(.sha)` removes duplicate rows. Rows orphaned by a rollback can only exist after an `origin/main` rewrite, so they're rare. If any exist, the counts are approximate.

Classify each of the 20 as a "pure lint fix, no behaviour change" or not. **Rule:** 18 or more of 20 → lint-hinted commits become mechanical in v2, which needs a `SCHEMA_VERSION` bump in a follow-up. Otherwise they stay real. Replace the spec §2 table cell "Real in v1; subject hint recorded separately" with the per-hint numbers, the sample count, and the verdict.

- [ ] **Step 6: SessionStart hook — ONLY with explicit owner approval in chat**

If the owner has not approved it, skip this step and put `hook: pending owner approval` in the final report. If they have approved, append this in `.claude/settings.json` under `"SessionStart"` → the existing `"hooks"` array, after the `vox ci queue --brief --from-snapshot` entry:

```json
          {
            "type": "command",
            "command": "vox graph history brief"
          }
```

- [ ] **Step 7: Run the full local gate**

<!-- AMENDED: #17 — `fmt.vox -- --all` rewrites the whole workspace; check only, and fix this branch's crates with `cargo fmt -p` -->
```bash
VOX_FMT_CHECK=1 vox run scripts/fmt.vox -- --all || cargo fmt -p vox-graph-reader -p vox-cli -p vox-orchestrator-mcp
vox ci pre-push --complete
```

Expected: green. If the fresh-worktree `vox-gui` sidecar or `ui/dist` failure appears, follow AGENTS.md (`vox run scripts/gui-build.vox`; `pnpm install && pnpm build` in `crates/vox-gui/ui`). Don't commit anything to "fix" it.

- [ ] **Step 8: Commit**

```bash
git add docs/superpowers/specs
git commit -m "docs(spec): record repo-history subject_hint verdict"
# only if Step 6 ran:
git add .claude/settings.json && git commit -m "chore(claude): print repo history brief at session start"
```

- [ ] **Step 9: Self-review the whole branch before proposing a merge** (AGENTS.md PR discipline: nothing else reviews it)

```bash
git log --oneline origin/main..HEAD
git diff origin/main..HEAD --stat -- . ':(exclude)Cargo.lock' ':(exclude)contracts/reports/*.v1.json'
```

Run `/code-review high` over the range from `main` to `HEAD`. Do not push.

---

## Deferred Minor Issues

Found by the pre-execution review (`.superpowers/review/`). None blocks execution; handle them in follow-ups or opportunistically.

- **M1 (Track A #10):** `search` scores mechanical commits and paths, and only `log`/`focus` expose `--include-mechanical`. Filter mechanical rows in `search`, and add the flag to `forgotten`/`search`/`timeline` for D6 parity.
- **M2 (A #11):** every `M` `.rs` file is parsed twice: `file_symbols` via the extractor, then `rust_normalized` via `syn`. Reuse one parse if backfill time matters.
- **M3 (A #12):** `BlobReader::read` still buffers oversize blobs before discarding them. Drain with `io::copy(&mut (&mut self.stdout).take(size + 1), &mut io::sink())` to keep memory flat.
- **M4 (A #15, B #11):** Tasks 8 and 9 edit files already over 500 lines with no suppression: `graphify/mod.rs` (1444), `dispatch.rs` (2110), `input_schemas.rs` (1109). Add "Legacy: already N lines" entries to `contracts/toestub/suppressions.v1.json` if the TOESTUB ratchet flags them.
- **M5 (A #17):** the commit snippets leave out the `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>` trailer that Global Constraints require. Append it to every commit.
- **M6 (A #18):** there's no ingest-level test that a lockfile-only commit and an fmt-only commit each produce `commit.mechanical == true`.
- **M7 (A #20):** `{"query":"log"}` passes the MCP JSON schema but fails serde, because `path` is required. Add `oneOf`/`if-then` for per-query required fields, or map the serde error to a remediation hint.
- **M8 (B #12):** `brief` loads every JSONL row, including symbol lists. If Task 10's warm timing is near 1 s, add a lean loader that skips lineage and symbols.
- **M9 (B #13):** `BlobReader` parses the header with `split_whitespace`, so a missing path like `x 5 y` could be read as a size. Parse from the right: treat a last token of `missing`/`ambiguous` as "no blob".
- **M10 (B #10):** Task 8 Step 9 times a debug `cargo run`, which can falsely trip the 5-minute profiling gate. Time the `--release` binary instead.
- **M11 (C cuts):** merge `load`/`load_checked`; drop `State`'s version fields (the directory name encodes them); drop `CatchUp::OverBudget.behind` and `Done.rebuilt`. That saves about 25 lines. Take it only if a file approaches the cap.
- **M12 (B #5):** no tests yet for the `GIT_*` env stripping, `lock.touch()` inside `catch_up`, the F1 ancestor time bound, the F4 mechanical-only area, the F2 `+N more` suffix, or the foxtrot-merge reset (#8).
- **M13 (C):** `cargo hakari generate` (Task 4, conditional) modifies `workspace-hack`. Call it out in that commit's body.

## Execution Order

**Sequential constraints (files shared between tasks, so these can't run in parallel):**
- Task 0 → everything: the branch is created from `origin/main` plus `ff50ba1e7`.
- Task 1 → Task 2: both modify `crates/vox-graph-reader/src/lib.rs`.
- Task 2 → Tasks 3, 4, 5, 6, 7, 8: each appends to `crates/vox-graph-reader/src/history/mod.rs`, which Task 2 creates.
- Task 1 → Task 5: `lineage::symbol_suffixes` depends on the moved `ast.rs` staying stable.
- Task 3 → Task 6 → Task 8: `tests/common/mod.rs` (created in 3) is used by 6 and 8. Task 6 needs `git.rs`, and Task 8 needs `ingest.rs`.
- Task 8 → Task 9: both modify `contracts/operations/catalog.v1.yaml` and run `operations-sync`. Task 8 uses `--target cli`; Task 9 regenerates the MCP artifacts.
- Task 0 → Task 10: both modify `docs/superpowers/specs/2026-09-22-repo-history-graph-design.md`.
- Task 6 → Task 10: Task 10 reads the golden `history_carrier_pairs.txt`.

**Pre-flight checklist:**
- [ ] Worktree isolated at `.worktrees/repo-history-graph` (superpowers:using-git-worktrees)
- [ ] Target history confirmed: `git log --oneline origin/main..HEAD` shows only the cherry-picked `ff50ba1e7` (plus the docs commit)
- [ ] No schema migrations. The store is JSONL, versioned by `SCHEMA_VERSION`; `vox-db` is untouched.
- [ ] No test database needed. Tests use tempdir git repos; the real-history test reads this full clone.
- [ ] `git --version` ≥ 2.40 on `PATH` (`/opt/homebrew/bin/git` 2.55; Apple's `/usr/bin/git` 2.39 is too old for `check-attr --source`)
- [ ] Fresh-worktree prerequisites for Task 10's `pre-push --complete`: the `vox-gui` sidecar and `crates/vox-gui/ui/dist` (AGENTS.md)

**Recommended task sequence:** 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10.
- Batch candidates: Tasks 4, 5 and 7 each depend only on `model.rs`, plus Task 1 for Task 5. They can be implemented in parallel if the one-line `history/mod.rs` edits are merged by hand. As written, run them in order.
- Owner gates: Task 6 Step 5 (new golden pairs, only if an unexpected pair appears) and Task 10 Step 6 (SessionStart hook).

**SDD ledger pre-population (copy into progress.md):**
```text
[settled] Grill rulings G1–G16 (see .superpowers/review/2026-09-22-repo-history-graph-grill.md): shared common-dir store, versioned dirs, origin/main tip + rollback, chain-walk validity, token lock, check-attr --source, whitespace allowlist, lineage eligibility/filters, brief never ingests, MCP 60 s + {complete,behind,rows}, fan-in allowlist + cache, merge inner subjects per path, hook gated, subject_hint 18/20 rule — ruling: settled
[settled] Conflict on crates/vox-graph-reader/src/lib.rs: Task 1 → Task 2 sequential — ruling: settled
[settled] Conflict on crates/vox-graph-reader/src/history/mod.rs: Tasks 2→3→4→5→6→7→8 sequential — ruling: settled
[settled] Conflict on contracts/operations/catalog.v1.yaml + operations-sync outputs: Task 8 (--target cli) → Task 9 — ruling: settled
[settled] Conflict on docs/superpowers/specs/…design.md: Task 0 → Task 10 — ruling: settled
[settled] #1 Critical: branch from origin/main + cherry-pick ff50ba1e7; do NOT start from the stale fix branch — ruling: settled
[settled] #2: command-sync needs --write; paths baseline via UPDATE_CLI_CATALOG_BASELINE=1 test — ruling: settled
[settled] #3: graphify/history.rs keeps an in-file to_query test (tdd-guard) — ruling: settled
[settled] #4: classify(path, status, …): whitespace/symbol_neutral only for status "M" — ruling: settled
[settled] #5: per-source lineage weights scaled to sum ≤ 1 — ruling: settled
[settled] #6: append_rows prepends "\n" after a torn line — ruling: settled
[settled] #7: impl Git ≤ 12 methods; TOESTUB gate is file size only (fn-line count is pre-existing, non-blocking) — ruling: settled
[settled] #8: foxtrot guard in catch_up; complete = intact && behind == 0 — ruling: settled
[settled] #9: add `pub mod …;` in Step 1 of every task so red steps really fail — ruling: settled
[settled] #10: git_cmd strips all GIT_* env and pins output-affecting config; check-attr exit status checked; ls-tree -z — ruling: settled
[settled] #11: facade tests live in tests/history_api_tests.rs on common::Repo — ruling: settled
[settled] #12: golden pairs hand-seeded; bless writes .new only; additions need owner review — ruling: settled
[settled] #13/#14: Task 10 preamble per step; delete only $ROOT/s<SCHEMA>-e<EXT> with path guard — ruling: settled
[settled] #15: no Paths.store_dir override; prune is best-effort with strict s<digits>-e<alnum> names — ruling: settled
[settled] #16: unparseable state.json → reset — ruling: settled
[settled] #17: Task 10 fmt is check-only — ruling: settled
[settled] #18: stage explicit paths; never `git add contracts` — ruling: settled
[settled] #19: no `repair`; rollback = commit_state only — ruling: settled
[settled] #20: mutation-sensitive tests for Rust-only symbol check, generated-file skip, stale-lock takeover — ruling: settled
[settled] #21: Task 1 resolve_edges span is ~83–206 — ruling: settled
```
