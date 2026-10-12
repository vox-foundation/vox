---
title: "Handoff: make the nightly green (Antigravity + Gemini Flash 3.8)"
description: "Self-contained brief for an Antigravity/Gemini Flash session to finish the nightly-repair work: state as of 2026-10-12, 15 failing steps with root causes, ordered tasks with verify commands, rules, and the owner decisions to ask about."
category: "Contributors"
status: "current"
---

# Handoff: make the nightly green

**Written:** 2026-10-12 by Claude Code (Opus) at the end of a long session.
**For:** an Antigravity session running Gemini Flash 3.8, working on its own (no Claude reviewer).
**Goal:** one scheduled or dispatched `nightly.yml` run on `main` in which **every job runs to completion and
concludes `success`** (advisory `continue-on-error` steps excepted). That is also the remaining success criterion of
GSD phase 06.1 ("three consecutive green nightlies"); after the first green run, keep watching until three are green
in a row, then mark 06.1 complete (see §8).

Read this whole file before touching anything. Then read `AGENTS.md` sections **CI Contract**, **Dependency
Discipline**, **Test-First Policy**, **PR & Review Discipline** and **Build Broker** in full (AGENTS.md is long; if your
harness truncates it, open it with a file read rather than relying on the auto-loaded copy).

---

## 0. Before you start (for the human launching the session)

- Work in a **fresh git worktree** off `origin/main` (e.g. `git worktree add ../vox-nightly-green origin/main`), not in
  `~/dev/vox` (that checkout belongs to another session on `feat/research-trial-flywheel`).
- The agy driver kit (`docs/agents/agy-driver-kit/`, installed under `.agents/`) **denies all git index commands** by
  design, because in the Claude-drives-agy setup Claude commits. This handoff asks the agent to commit, push and open PRs
  itself. Either run Antigravity **without** the kit's `hooks.json` guard for this session, or keep the guard and do the
  commits yourself when the agent reports `READY`.
- `gh` is authenticated as `brbrainerd` on this machine; the agent uses it for PRs, run logs and dispatches.

---

## 1. Where things stand (2026-10-12)

**Landed on `main`** (all via the merge queue):

| PR | What |
|---|---|
| #610 | v1.0 close-out: Phase 6, 06.1, dependency advisories, workflow codegen, quarantine burn-down |
| #621 | config-parity false positive (`gui/drive.rs`) + test inventory |
| #634 | nightly hardening: every job `timeout-minutes: 180`, every `run:` step `timeout-minutes: 90` and `if: ${{ !cancelled() }}`, `cancel-in-progress: false`; crate blast-radius gate advisory on cross-host data; CodeRabbit on demand (free tier), `review_status: false` |

Because of #634 a nightly now **runs every step even after one fails**, so it shows the whole picture. The latest
completed nightly, **run `38139188025`** (scheduled, 2026-10-11, `main` @ `a153a2d53`), ran all jobs to completion
and **failed 15 steps in 4 jobs**, listed in §4. Many of these were *always* broken but never ran before, because
they sat behind an earlier failing step. A newer scheduled run (`38266427089`, 2026-10-12) was in progress when this
was written; check it first (§3) since `main` has moved.

**Unfinished local work** (committed locally, **not pushed**): branch **`wip/fmt-safety-2026-10-12`** (off
`origin/main` @ `a29ec98b0`) carries the partial `vox fmt` fix in `crates/vox-compiler/src/fmt/{expr.rs,mod.rs}`
and this handoff. Three of its tests fail on purpose until Task 1 is finished. The branch is visible from every
worktree of this repo: `git switch -c fix/vox-fmt-safety wip/fmt-safety-2026-10-12`.

---

## 2. Rules (non-negotiable)

1. **Foreground only.** Never background a command and end your turn "waiting": a headless run has no wake-up
   (a 57-minute hang happened this way). Prefix every build/test with GNU `timeout` (on PATH), e.g.
   `timeout 1800s cargo test -p vox-compiler --lib fmt::tests`. Exit 124 = timed out: report it, don't retry blindly.
2. **Test first, and prove the guard.** For every behaviour change: write the failing test, run it, see it fail, then
   implement. For every guard/gate: **mutate the guarded code, confirm the test fails, restore, confirm green**
   (check the file content before and after so a concurrent edit can't hand you a meaningless pass).
3. **Local verification is crate-scoped.** `cargo check -p <crate>`, `cargo test -p <crate> <filter>` or
   `cargo nextest run -p <crate>`. **Never** `cargo clippy --workspace`, never a workspace-wide test, **never
   `cargo fmt --all`** (use `rustfmt --edition 2024 --config-path . <files>` or `cargo fmt -p <crate>`). Hosted CI is
   the gate.
4. **Never raise a job cap** (`timeout-minutes` beyond the policy max: 30 for PR/push workflows, 180 for
   scheduled/dispatch ones; `workflow_policy_guard` enforces it). Over budget means cache, shard, or split.
5. **Owner-only, never do these yourself:** add `exceptions` to `contracts/ci/crate-edges.allow.v1.json`; regenerate
   `crate-edges.allow.v1.json` or `fan-in-snapshot.v1.json` to admit an edge; edit
   `contracts/crypto/transport-providers.v1.json`; raise or re-derive `contracts/ci/crate-budget.v1.json` ceilings;
   add a test to the `ci-gate` quarantine in `.config/nextest.toml`. If a fix needs one of these, **stop and ask**.
6. **Don't hand-regenerate SSOT after a merge** (AGENTS.md). Fix generators/inputs. Inventories that a gate checks
   for exact equality (`contracts/reports/test-inventory.v1.json`, `contracts/reports/safety-inventory/baseline.v1.json`)
   are regenerated in the same PR that changes the tests: `vox ci test-inventory --output contracts/reports/test-inventory.v1.json`,
   then `--check`.
7. **Big generated/contract files: one targeted edit.** Never re-serialize a whole JSON/YAML contract. Check
   `git diff --numstat` before committing; anything over ~400 changed lines you didn't expect is a red flag.
8. **Commit by pathspec** (`git add -- <files>`; `git commit -- <files>`); imperative subject < 72 chars, body says
   why. End each message with a `Co-Authored-By:` line naming your model.
9. **Prefer a stop to a silently shipped mistake.** When a task needs a product/owner decision (§7) or the evidence
   contradicts this brief, stop and ask the user. Do not guess.
10. **Stale installed `vox`.** Gates refuse to run when `~/.cargo/bin/vox` is older than the tree. Use
    `cargo run -q -p vox-cli -- ci <gate>` (builds current source) or set `VOX_SKIP_FRESHNESS_CHECK=1` when you have
    just built `target/debug/vox` yourself.
11. **macOS shell gotchas:** BSD `sed` has no `\t` and needs `sed -i ''`; no `cmp` in some allowlists (use `diff -q`);
    zsh does not word-split `$VAR` (use arrays or `${=VAR}`).

---

## 3. First steps

```bash
git fetch origin
gh run list --workflow nightly.yml -L 5 --json databaseId,event,status,conclusion,createdAt,headSha
# Failing steps of a completed run:
gh run view <RUN> --json jobs -q '.jobs[]|select(.conclusion=="failure")|"## \(.name)\n" + ([.steps[]|select(.conclusion=="failure")|"  - \(.name)"]|join("\n"))'
# One job's log (completed run):
gh run view --job <JOB_ID> --log > /tmp/job.log
# One job's log while the run is still in progress:
gh api --allow-escape-sequences repos/vox-foundation/vox/actions/jobs/<JOB_ID>/logs > /tmp/job.log
```

Compare the newest completed run against §4. Drop tasks that `main` already fixed (other sessions, including the
owner, land nightly fixes in parallel; **check `git log origin/main` for the area before starting a task** and merge
`origin/main` into your branch before pushing, to avoid conflicts).

---

## 4. The 15 failing steps (run `38139188025`) and their root causes

| # | Job / step | Root cause (from the log) | Task |
|---|---|---|---|
| 1 | Compiler gates / **Golden examples fmt idempotency (A.12)** | `vox fmt` cannot round-trip `examples/golden/adt_multi_field.vox` (printer emitted `??` for `>`/`<` and `->` for match arms). Deeper: the printer is partial and **destructive** — it deletes comments and whole declarations (`db_native_ir.vox`: 31 → 6 lines, a `query` vanished) | 1 |
| 2 | Guards / **Config hygiene + registry parity + GUI codegen drift** | `crates/vox-gui/build.rs:148-149` reads `CARGO_CFG_TARGET_OS` / `CARGO_CFG_TARGET_ENV` (Cargo build-script env), not in the registry or allowlist | 2 |
| 3 | Guards / **OpenClaw + scaling policy SSOT** | `contracts/reports/scaling-audit/findings-array.v1.schema.json` is invalid JSON (`expected , or } at line 9 column 11`): every `{` after a `:` was stripped by an old bulk commit (`a9d9641de`, 2026-04-23) | 3 |
| 4 | Guards / **Secrets SSOT parity + secret direct-read guard** | `secrets-parity OK`, then `git diff failed while checking secret env usage (range=HEAD~1...HEAD)`: shallow checkout has no `HEAD~1` | 4 |
| 5 | Guards / **Vox script corpus check** | 4 scripts fail `vox check`, e.g. `scripts/plugin-candidacy.vox:49` "Cannot assign to immutable variable" (missing `let mut`), `scripts/docs-corpus-census.vox` FAIL | 5 |
| 6 | Guards / **Workflow scripts allowlist** | workflows reference scripts missing from the allowlist: `scripts/fmt.vox`, `clean-build-artifacts.vox`, `crate-build-audit.vox`, `plugin-candidacy.vox`, `coverage-graph/*.py`, `scripts/*README.md`, `scripts/install.sh` | 6 |
| 7 | Guards / **Script hygiene (.vox-only automation)** | non-`.vox` files under `scripts/`: `broker-bypass-fixtures/*.sh` (test fixtures), `coverage-graph/*.py`, `install.sh` / `install.ps1` (bootstrap installers) | 6 |
| 8 | Guards / **Determinism Audit** | **timed out at the 90-minute step cap** (`vox ci determinism-audit` builds every golden twice into `target/determinism-audit-{1,2}`) | 7 |
| 9 | Guards / **Dependency Sprawl Check** | `vox ci dep-sprawl --cap 25`: 8 crates over 25 direct deps (vox-cli 137, vox-orchestrator 85, vox-populi 82, vox-ml-cli 61, vox-compiler 52, vox-db 34, vox-gamify 31) — a cap that never matched reality; the step never ran before | 8 (decision) |
| 10 | Guards / **Dynamic Workspace Health Audit** | 7 issues: "BLOAT: runtime crate has N direct deps (limit 40)" (vox-orchestrator 85, vox-orchestrator-mcp 97, …) and "LEAK: foundational crate `vox-plugin-mens-candle-core` pulls in ML dependencies"; also `vox ci profile-parity`: "lean: forbidden crate `vox-gamify` present in lean build" | 8 (decision) + 9 |
| 11 | Guards / **Determinism / health** also print `unused workspace dependency …` (cargo-shear) | informational (`continue-on-error`), **not** a failure; ignore unless asked | — |
| 12 | Tests / **Run slow test partition** | runner **out of disk** (`No space left on device` building `vox-orchestrator`) during the llvm-cov job | 10 |
| 13 | Tests / **Serving path unit tests** | same disk exhaustion (`failed to create directory … No space left on device`) | 10 |
| 14 | Audits / **Mens corpus mix (strict required sources)** | `[mix] strict: required source missing: target/dogfood/train.jsonl`; the step's escape hatch only tolerates the known-absent `validated_mixed.jsonl` | 11 |
| 15 | Audits / **Populi CI gate matrix** | `vox ci mens-gate --profile ci_full` runs `cargo nextest run -p vox-plugin-mens-candle-cuda --lib merge_v2 --no-tests=fail`: **0 tests matched** (39 skipped) — the filter in `scripts/populi/gates.yaml:20,45,71` no longer matches anything without the `cuda` feature | 12 |

Also note (not failing, advisory): the **crate blast-radius gate** now prints OVER lines with "not enforced": the
committed `contracts/ci/crate-build-map.v1.json` was measured on Windows `cargo check`, the ceilings on Linux
`cargo build`. Re-measuring on Linux is an owner action (§7).

---

## 5. Tasks (do them in this order; one PR may batch several small ones)

Each task: **Problem → Fix → Verify** (every verify command lists how it fails).

### Task 1 — `vox fmt` must never corrupt a file (branch `wip/fmt-safety-2026-10-12`)

**Problem.** `crates/vox-compiler/src/fmt/` is a partial printer: `print_expr` handles ~9 of 31 `Expr` kinds and
writes `...` for the rest; `print_pattern` handles 2 of 5 and writes `pat`; `print_type` writes `Any`; the statement
printer drops unknown statements (`_ => {}`); comments are never printed. `try_format` only re-parses the output, so
anything that still parses (a dropped declaration, `Any`) is written back to disk. **Worse, the LSP
(`crates/vox-lsp/src/server.rs:228`) calls the soft `fmt::format()` on format-on-save**, so editors silently delete
user code.

**Already done on the WIP branch (uncommitted):**
- `print_binop` is exhaustive over all 14 `BinOp` variants (no wildcard; `Mod %`, `Lt/Gt/Lte/Gte`, `And/Or`, `Pipe |>`).
- Match arms print `=>` instead of the deprecated `->`.
- Tests in `fmt/mod.rs`: `every_binop_round_trips` (passes), `match_arms_print_fat_arrow` (fails: literal pattern
  `0` prints as `pat`), `formatter_refuses_to_drop_comments` and `formatter_refuses_to_drop_declarations` (fail: not
  implemented yet).

**Fix.**
1. In `fmt/mod.rs`, add a private `fn layout_only(source: &str, out: &str) -> bool` that lexes both with
   `crate::lexer::lex_preserving` (keeps `Token::Comment`), drops `Token::Newline` and `Token::Eof`, and compares the
   two `Vec<Token>` for equality (`Token: PartialEq`; payloads like `IntLit(5)` compare too).
2. `try_format`: after printing and re-parsing, if `!layout_only(source, &out)` return
   `Err(vec![ParseError::new(Span::new(0, 0), "formatter would change more than layout (comments or constructs it cannot print yet); file left unchanged", vec![], None)])`.
   The test asserts the message contains `"layout"`. `Span` is `crate::ast::span::Span` (`Span::new(start, end)`).
3. `format` (LSP soft mode): if `!layout_only(source, &out)` return `source.to_string()`.
4. Make `print_pattern` exhaustive (no wildcard): `Tuple` → `(a, b)`, `Constructor { name, fields }` → `Name(f1, f2)`
   (just `Name` when `fields` is empty), `Literal { value }` → `print_expr(value)`. Pattern enum:
   `crates/vox-ast/src/pattern.rs` (5 variants).
5. Do **not** add an equivalence for `->`/`=>` in `layout_only`: `->` is also state-machine syntax. A file that still
   uses deprecated `->` arms is refused, which is acceptable.

**Verify.**
- `timeout 2400s cargo test -p vox-compiler --lib fmt::tests` — fails when: any `FAILED` line, or the summary is not
  `0 failed`.
- Mutation: make `layout_only` return `true` unconditionally → `formatter_refuses_to_drop_comments` and
  `formatter_refuses_to_drop_declarations` must FAIL; restore; green again.
- `timeout 2400s cargo run -q -p vox-cli -- fmt <copy of examples/golden/db_native_ir.vox>` — fails when: exit 0 and the
  copy shrinks (it must now exit non-zero with the "layout" message and leave the file byte-identical: `diff -q`).
- `cargo test -p vox-lsp` (the LSP uses `format`) — fails when: non-zero exit.
- Regenerate the test inventory (rule 6) because you added tests.

**Then A.12 still fails** (the formatter now refuses most goldens: they all carry comment frontmatter). Changing what
A.12 asserts is an **owner decision** (§7, D1). Do not change `nightly.yml`'s A.12 step until the user answers.

### Task 2 — config-hygiene: Cargo build-script env vars

`crates/vox-cli-ci/src/config_hygiene.rs` `THIRD_PARTY_ALLOWLIST`: add `CARGO_CFG_TARGET_OS` and
`CARGO_CFG_TARGET_ENV` under the existing `// Cargo / Rust runtime names.` comment (they are set by Cargo for build
scripts). Better and still small: allowlist the `CARGO_CFG_` prefix if the check supports prefixes (read
`check_env_reads_registered`); if not, the two names.
**Verify:** `timeout 2400s cargo run -q -p vox-cli -- ci config-hygiene` — fails when: output contains `NEW violation`.
`cargo test -p vox-cli-ci --lib config_hygiene` — fails when: non-zero exit.

### Task 3 — repair the three invalid JSON contracts, and guard against it recurring

Invalid on `main` (check: `for f in $(git ls-files contracts | grep '\.json$'); do jq empty "$f" >/dev/null 2>&1 || echo "$f"; done`):
- `contracts/reports/scaling-audit/findings-array.v1.schema.json`
- `contracts/reports/toestub-remediation/delta-after-remediation.v1.schema.json`
- `contracts/reports/completion-task-ledger.v1.json` (check first: if it is meant to be JSONL, it is not corrupted; read
  its consumer before editing)

The two schemas lost every `{` that followed a `:` (`"items": ` then a newline, `"rule_id":  "type": "string" }`).
Restore the braces by hand (one targeted edit per file, rule 7); keep content otherwise identical. Then add a
recurrence guard: a test in `crates/vox-cli-ci` (next to an existing contracts test) that walks `contracts/**/*.json`
and asserts each parses with `serde_json` (skip files documented as JSONL). Test first: it must fail on the current
tree, pass after the repair; mutation: re-break one brace → fails.
**Verify:** the `jq` loop prints nothing; `timeout 2400s cargo run -q -p vox-cli -- ci` *(the OpenClaw + scaling
policy step's command from `nightly.yml`)* exits 0; the new test passes.

### Task 4 — secret direct-read guard on a shallow checkout

The guard runs `git diff HEAD~1...HEAD`, which fails when the checkout has depth 1. Fix in the guard (find it from
`nightly.yml`'s "Secrets SSOT parity + secret direct-read guard" step → `vox ci secret-env-guard` or similar in
`crates/vox-cli-ci/src`): when `HEAD~1` does not resolve (`git rev-parse --verify HEAD~1` fails), fall back to
scanning the files at `HEAD` (or skip the diff-scoped check with a clear `note:`), instead of erroring. Test first with a
temp repo containing a single commit. Do **not** just add `fetch-depth: 2` (it hides the bug for other callers).
**Verify:** the guard's unit test; running the step's command locally exits 0.

### Task 5 — make every `scripts/**/*.vox` pass `vox check`

Run the step's exact command from `nightly.yml` ("Vox script corpus check") locally to list the 4 failing scripts.
Known: `scripts/plugin-candidacy.vox` (lines 49, 79, 83, 88, 91 assign to bindings declared without `mut` → declare
them `let mut`), `scripts/docs-corpus-census.vox`. Fix the scripts, not the checker. Warnings (e.g. trailing `;` in
`vendor-skills.vox`) don't fail the step; leave them.
**Verify:** `for f in $(git ls-files 'scripts/*.vox' 'scripts/**/*.vox'); do timeout 300s cargo run -q -p vox-cli -- check "$f" >/dev/null 2>&1 || echo "FAIL $f"; done` prints nothing (or run the step's command).

### Task 6 — workflow-scripts allowlist and script hygiene

Two guards (`vox ci workflow-scripts`, `vox ci script-hygiene`, in `crates/vox-cli-ci/src`). Read each one's allowlist
mechanism first.
- **Legitimate, allowlist with a reason:** `scripts/install.sh` / `scripts/install.ps1` (canonical installers named in
  AGENTS.md), `scripts/broker-bypass-fixtures/*.sh` (test fixtures for `broker-bypass-lint.vox`, never executed),
  `scripts/**/README.md` references, and the `.vox` scripts the workflows really call (`fmt.vox`,
  `clean-build-artifacts.vox`, `crate-build-audit.vox`, `plugin-candidacy.vox`).
- **Real violations:** `scripts/coverage-graph/*.py` (called from `nightly.yml`). AGENTS.md forbids new `.py` glue;
  porting four Python scripts to `.vox` is real work. **Owner decision D3** (§7): port now, or allowlist with a
  `retire-by` and a tracking issue.
**Verify:** both commands exit 0 locally.

### Task 7 — Determinism Audit must fit (it hit the 90-minute step cap)

`crates/vox-cli-ci/src/determinism_audit.rs` compiles every golden twice (`target/determinism-audit-1` and `-2`) and
diffs the outputs. Profile it locally on a few goldens. Options, cheapest first:
1. Reuse one compiled `vox` and run the two codegen passes in-process (codegen determinism does not need a full
   `cargo build` of the generated crate — confirm what the audit actually compares: the emitted `.ts`/`.rs` text).
2. Parallelize the per-golden loop (derive parallelism from `std::thread::available_parallelism`, no literal counts).
3. Split it into its own nightly job (no cap change; still ≤ 180) so it no longer competes with the Guards job.
Do not just raise the step timeout.
**Verify:** time the step's command locally on the full golden set; it must finish well under 90 minutes on a
4-core runner (state the measured time in the commit body).

### Task 8 — dependency sprawl and workspace health (needs a decision first)

`dep-sprawl --cap 25` and the health audit's `limit 40` have never matched the workspace; they surfaced only now.
Do not delete dependencies to hit 25. **Ask the owner (D2)**. The usual fix in this repo is a **ratchet**: record
today's counts as per-crate ceilings in a committed baseline and fail only on increases. If the owner approves, add the
baseline (the tool must read it), tests first, mutation-checked.

### Task 9 — profile-parity: `vox-gamify` in the lean build

`vox ci profile-parity`: "lean: forbidden crate `vox-gamify` present". Find who pulls it into the lean profile
(`cargo tree -p vox-cli --no-default-features --features <lean features> -i vox-gamify -e features`; the lean feature
set is in the profile-parity source). Make the dependency optional/feature-gated at the pulling crate. If it needs a
new crate edge or feature redesign, stop and ask.
**Verify:** `timeout 2400s cargo run -q -p vox-cli -- ci profile-parity` exits 0.

### Task 10 — Tests job runs out of disk

The `Tests (nextest + llvm-cov + doctests)` job fills the hosted runner's disk while building instrumented binaries.
Fix in `nightly.yml`, in that job:
1. Add a first step that frees space (well-known safe removals on `ubuntu-latest`: `/usr/share/dotnet`,
   `/usr/local/lib/android`, `/opt/ghc`, `/usr/local/share/boost`, and `docker image prune -af`), followed by `df -h /`.
2. Reduce debuginfo for this job: `CARGO_PROFILE_DEV_DEBUG: line-tables-only` in the job `env:` (coverage needs line
   tables, not full debuginfo). Confirm llvm-cov still reports line coverage.
Keep the `!cancelled()`/timeout pattern on new steps. Update `crates/vox-cli/tests/ci_workflow_contract.rs` if a
pinned string changes.
**Verify:** `actionlint .github/workflows/nightly.yml` (no new structural errors);
`cargo test -p vox-cli --test ci_workflow_contract`; `cargo run -q -p vox-cli -- ci ssot-drift` (includes the
workflow-policy guard). Real proof is the next nightly.

### Task 11 — Mens corpus mix needs `target/dogfood/train.jsonl`

The strict mix requires `target/dogfood/train.jsonl`, which nothing in the nightly produces (producers:
`crates/vox-corpus/src/training/mix_prepare.rs`, `crates/vox-ml-cli/src/commands/mens/pipeline.rs`,
`scripts/mens/full-pipeline.vox`). Either run the cheap producer in an earlier step of the Audits job, or (if it needs
model/network work) extend the step's known-absent allowance to `train.jsonl` with the same warning style it uses
for `validated_mixed.jsonl`. Prefer producing it; ask if it needs network or secrets.

### Task 12 — mens-gate filter matches zero tests

`scripts/populi/gates.yaml` lines 20, 45, 71: `cargo test -p vox-plugin-mens-candle-cuda --lib merge_v2`; under
nextest with `--no-tests=fail`, 0 of 39 tests match (the `merge_v2` tests are probably behind the `cuda` feature or
were renamed). Find the tests (`rg -n "merge_v2" crates/vox-plugin-mens-candle-*`), then point the gate at the crate
and filter where they now run without CUDA (likely `vox-plugin-mens-candle-core`), or drop the entry if the CUDA
health job (`ml-cuda-health`) already covers it — say which in the commit body.
**Verify:** `timeout 2400s cargo run -q -p vox-cli -- ci mens-gate --profile ci_full` exits 0.

---

## 6. Landing each batch

1. Branch from current `origin/main` (`git switch -c fix/nightly-<topic> origin/main`), batch related tasks.
2. Before pushing: merge `origin/main` again; run the verify commands of every task in the batch;
   `rustfmt --check` on touched `.rs` files; `cargo clippy -p <touched crate> --all-targets -- -D warnings`.
3. `git push -u origin HEAD:fix/nightly-<topic>`; `gh pr create --draft` with a body that lists each fixed nightly
   step and its evidence; read the PR's CI; fix what your change broke.
4. Self-review the full diff (`git diff origin/main...HEAD`), mark ready (`gh pr ready <N>`), then comment
   `@coderabbitai review` **once** (free tier, on demand; advisory only; never tick its Autofix/Autopilot boxes).
   Triage its findings if any arrive.
5. Merge through the queue: `gh pr merge <N> --squash` (the repo's merge queue sets the method). A PR that conflicts
   after another merges **does not enter the queue** even though the command prints success; confirm with
   `gh api graphql -f query='query{repository(owner:"vox-foundation",name:"vox"){pullRequest(number:<N>){isInMergeQueue}}}'`.
   Every push resets auto-merge; re-arm it after pushing (`gh pr merge <N> --auto --squash`).

---

## 7. Owner decisions (ask the user; do not decide alone)

| ID | Question | Recommendation from the previous session |
|---|---|---|
| D1 | What should A.12 (golden fmt idempotency) assert now that `vox fmt` refuses rather than corrupts? Options: **(a)** a *safety* gate — `vox fmt` either refuses or leaves the file layout-only-changed, never corrupts; **(b)** build a full comment-preserving formatter (all 31 `Expr` kinds, statements, types, patterns, JSX) so A.12 passes as written (multi-day); **(c)** disable `vox fmt` + LSP formatting and drop A.12 until a real formatter exists | (a) now, with (b) as a tracked follow-up. The user interrupted this question last session without answering, so **ask again before changing A.12**. |
| D2 | `dep-sprawl --cap 25` and health-audit `limit 40` never matched reality (vox-cli has 137 direct deps). Ratchet at today's counts, raise the caps, or keep them as an aspirational advisory? | Ratchet (fail on increase), tighten over time. |
| D3 | `scripts/coverage-graph/*.py` violate the `.vox`-only rule and are called from `nightly.yml`. Port to `.vox` now, or allowlist with `retire-by` + issue? | Allowlist with a dated retire-by now; port in a separate PR. |
| D4 | Re-arm the crate blast-radius gate: re-measure `crate-build-map.v1.json` on `x86_64-unknown-linux-gnu (cargo build)` (or re-derive the ceilings on the map's host). | Re-measure on Linux; ceilings unchanged. |
| D5 | `contracts/reports/completion-task-ledger.v1.json` fails `jq`: corrupted or JSONL by design? | Check its consumer; repair only if corrupted. |

---

## 8. Definition of done

1. Dispatch a nightly on `main` after the last fix lands: `gh workflow run nightly.yml --ref main`, then watch it with a
   foreground loop that polls every few minutes **with a `timeout`** (e.g. a 4-hour `timeout` around
   `until gh run view <RUN> --json status -q .status | grep -q completed; do sleep 300; done`), not a background job.
2. **Done** = a nightly where every job has `conclusion: success` and no job is `cancelled`/`timed_out`, and no
   `run:` step hit its 90-minute timeout. Report the run URL and the job list.
3. Then keep the next two scheduled runs green (06.1 requires three consecutive). When they are, update
   `docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md` Task 0.4 status and the GSD state
   (`.planning/ROADMAP.md` Phase 06.1 → complete) in one docs PR.
4. Record anything durable and non-obvious you learned in `~/dev/memory/` (format in `~/dev/AGENTS.md`), not in an
   IDE-private store.

---

## 9. Reference

- Hardening that makes failures visible: PR #634 (`nightly.yml`: job caps 180, step caps 90, `!cancelled()`,
  `cancel-in-progress: false`).
- Previous verification nightly (all jobs ran to completion): run `37905219290`.
- Latest analysed nightly: run `38139188025` (job IDs: Audits `114471558454`, Compiler gates `114471558499`,
  Tests `114471558501`, Guards `114471558518`).
- agy/Antigravity operating manual and field-tested hazards: `docs/src/contributors/antigravity-driven-execution.md`.
- CI contract and policy: `AGENTS.md` §CI Contract; plan `docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md`.
