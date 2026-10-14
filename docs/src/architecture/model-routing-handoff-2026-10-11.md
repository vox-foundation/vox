---
title: "Model Routing Audit and Live Proof — Handoff (2026-10-11)"
description: "State of the measured-quality model routing work on feat/research-trial-flywheel: what shipped, what was verified live, and the remaining manual steps with exact commands."
category: "Architecture SSOTs"
status: "research"
training_eligible: false
---

# Model Routing Audit and Live Proof — Handoff

**Written for:** whoever finishes this work next, human or agent, in any IDE. It assumes
no memory of the sessions that produced it (2026-10-08 to 2026-10-11).

**Design record:** [`model-autonomic-system-2026.md`](model-autonomic-system-2026.md)
§5.1 (how benchmarks feed routing) and §5.2 (live proof results). This page covers
state and next steps only; it does not restate the design.

## 1. Where things stand

The code work is done and committed. What remains is manual: a proof that needs a
vendor key nobody has configured yet, rolling the new code into the running daemon,
the pre-push gates, and the push/PR decision.

| | |
|---|---|
| Branch | `feat/research-trial-flywheel`, 60 commits ahead of `origin`, **not pushed** |
| Commits from this work | `977ef988f`, `139bdd20e`, `42c743f80`, plus the commit that adds this page |
| Other agents' commits on the same branch | Phase `12-*` work (Candle samplers, eval-corpus repair turns, typed report). Not part of this work. |
| Live spend | ≤ $1.83 of a $10 cap on the final proof run (earlier runs were also well under cap) |
| Plan file (do not edit) | `~/.cursor/plans/model_routing_audit_+_live_proof_01963215.plan.md`; every todo is complete except the direct-key leg |

### Commits

- **`977ef988f` fix(clippy).** Three lints that the Rust 1.98 toolchain flags in older
  code: a nested `if` in `vox-gui` `commands/knowledge.rs` (`check_robots_allowed`) and
  a `vec!` in a `vox-cli` `commands/model/scoreboard.rs` test.
- **`139bdd20e` feat(models).** The routing work itself, 63 files:
  - **Direct vendor wires.** Every `ProviderType` maps to its own endpoint and key in
    `llm_config_for_spec` and `vox-config` `resolve_egress.rs` / `inference.rs`.
    `key_guard` checks the key for the wire that will actually be used.
  - **Real quality signal.** `vox-actor-runtime` `llm/chat.rs` no longer writes
    success as `quality_score`. Only graded benchmarks write quality.
  - **`vox model bench`** (`crates/vox-cli/src/commands/model/bench/`). Graded suites
    measure intelligence, responsiveness (streaming TTFT, tok/s, p50/p95), efficiency
    (cost per solve, tokens per solve) and reliability. `--max-usd` is enforced: the
    cost is estimated before anything runs, and the run aborts once reconciled spend
    crosses the cap. `--latest N` benches `-latest` alias targets.
  - **Persistence.** The `model_benchmark_results` table (`vox-db`
    `store/ops_model_bench.rs`, schema in `schema/domains/scientia.rs`). It is read
    back as Wilson-scored `BenchEvidence` (`vox-orchestrator` `models/bench_evidence.rs`).
  - **Measurement.** Resolved model ids for aliases, OpenRouter `/generation` cost
    reconciliation, and streaming TTFT (`vox-llm-egress` `measured.rs`).
  - **Ranking fixes:**
    - Ollama entries exist only if the live `/api/tags` listing has them
      (`catalog::sync_installed_ollama`).
    - Undated hosted specs yield to dated, newer family members (`family::is_superseded`).
    - Tagged models yield to untagged successors (`ranking::rank_pass`).
  - **Adaptation loop** (`models/adaptation.rs`). Discover, bench, promote, and
    re-bench when a `-latest` alias moves to a new target.
  - **GUI.** Per-mode leaderboard on the Axis Models surface
    (`ModelLeaderboard.tsx`) and an Auto preview in the chat model picker. Playwright
    spec: `crates/vox-gui/ui/e2e/models-leaderboard.spec.ts`.
  - **Live proof script:** `scripts/model-routing-live-proof.vox`.
  - **Contracts regenerated:** CLI registry, operations catalog, capability registry,
    CLI command surface.
- **`42c743f80` test(models).** Two real tests in place of test-first suppressions:
  - The daemon's `refresh_model_scoreboard` loads persisted bench evidence into the
    router. Checked by mutation: removing the injection line makes the test fail.
  - `classify --measure` extracts its summary through a tested `first_summary` helper.

### Routing paths that now share one evidence builder

Measured evidence reaches every selector through `catalog::sync_installed_ollama` plus
the registry's evidence injection:

- the daemon catalog refresh (`orchestrator/catalog_refresh.rs`) and its periodic
  scoreboard refresh (`orchestrator/core/telemetry.rs`)
- `vox model bench`
- `vox chat --model auto` (`vox-cli` `commands/chat.rs` `route_auto`)
- the Axis Models surface (`vox-gui` `commands/models.rs` `registry_with_scoreboard`)

## 2. What was verified

- **Live proof:** `vox run scripts/model-routing-live-proof.vox` exited 0. Full numbers
  are in [§5.2](model-autonomic-system-2026.md). In brief:
  - OpenRouter: 11 models × 22 cases for $0.33, scores from 0.65 to 0.98.
  - All 24 mode × category routes ranked on measured quality.
  - Local `qwen3:8b` scored 0.25. `auto-local` failed closed on a dead endpoint.
  - `vox chat --model auto` routed to `deepseek/deepseek-v4.1-flash` and answered.
- **Tests:**
  - `vox-orchestrator` `models::`: 309 pass, plus the new daemon-refresh test.
  - `vox-cli` bench tests: 19 pass, plus the classify test.
  - `vox-config` direct-provider tests and the GUI bench-row test pass.
  - Vitest for `Models/` and `ChatModelPicker` passes, as does the Playwright spec
    `models-leaderboard.spec.ts`. Screenshots: `crates/vox-gui/ui/review-bundle/latest/models-leaderboard-{900,1440,newcomer-1440}.png`.
  - UI `typecheck` was clean at the time it ran.
- **Gates:** doc lint, `vox ci line-endings`, `operations-verify` and
  `command-compliance` pass. Clippy (`-D warnings`) was clean on `vox-llm-egress`,
  `vox-orchestrator`, `vox-cli` and `vox-gui` before `42c743f80`; see item 3.5 for the
  rerun after it.

## 3. Remaining manual work, in priority order

### 3.1 Direct vendor wire proof (blocked on a key)

Only the OpenRouter key resolves on this machine, so the proof script's `direct` leg
skipped itself. A person has to add at least one vendor key; agents never enter keys.

```bash
pbpaste | vox secrets set GEMINI_API_KEY --stdin    # or ANTHROPIC_API_KEY, OPENAI_API_KEY, GROQ_API_KEY, ...
vox secrets doctor                                   # confirm it resolves
vox run scripts/model-routing-live-proof.vox         # the direct leg now runs; $10 total cap
```

The plan also asks for each direct call to succeed **without** an OpenRouter key, which
proves the call went over the vendor's own wire. The script does not do that today.
Either temporarily remove the OpenRouter key from the vault for one run, or extend the
`direct` leg (around line 244) to run with the OpenRouter key masked. If you mask it
through the environment, use a mechanism `vox-secrets` already honours; do not add a new
env var without updating `contracts/config/env-vars.v1.yaml`. When it passes, update
§5.2 of the autonomic doc in place.

### 3.2 Roll the new routing into the running daemon

The daemon at `~/.vox/bin/vox-orchestrator-d` (it was PID 52527 at handoff) runs code
from before this work. The sessions were told not to restart it, so the daemon, MCP and
GUI-over-daemon paths still route the old way until someone does:

```bash
cargo build -p vox-orchestrator-d --release
cp ~/.vox/bin/vox-orchestrator-d ~/.vox/bin/vox-orchestrator-d.bak-2026-10-11
cp target/release/vox-orchestrator-d ~/.vox/bin/vox-orchestrator-d
# stop the running daemon (check nothing important is mid-task first), then start it however you normally do
```

Make sure no other agent depends on the running daemon before you stop it.

### 3.3 Daemon-path routing proof (plan step 5, not done)

The live proof computes the 24 mode routes **in-process** inside `vox model bench`. It
never sent chat turns through the daemon, and the GUI evidence is Playwright against
mocked IPC. To close plan step 5, after 3.2:

1. Send one `vox_chat_message` per clutch mode (Free / Efficiency / Balanced / Genius)
   and per research mode through the daemon. Check that the chosen model matches that
   mode's leaderboard winner: either the `routes` table in a real bench report
   (`vox model bench ... --report <path>`; `--dry-run` only prints a cost estimate) or
   the Axis Models leaderboard. Note that the daemon routes on the user store's
   evidence, not the proof's isolated store (see §4), so bench into the user store
   first or compare against the daemon's own leaderboard.
2. Run one real Axis chat turn per mode and save a screenshot to
   `crates/vox-gui/ui/review-bundle/latest/`.

### 3.4 Bench frontier models (needs a bigger budget)

Frontier models were outside the $10 sweep. Genius mode sees them only through their
public intelligence index, so a measured 0.98 flash model outranks them. Bench them
before trusting genius routes, for example:

```bash
vox model bench --model openrouter:<frontier-id>,... --max-usd 25
```

### 3.5 Re-run clippy on `vox-cli`

After `42c743f80`, `cargo clippy -p vox-orchestrator -p vox-cli --lib --tests -- -D warnings`
failed on one unused `ctx` binding at `crates/vox-cli/src/commands/model/eval_corpus.rs:761`.
That is another agent's phase-12 eval-corpus work, and it has since been committed
(`871cee721`, `a5dc95191`). The failure stopped the lint before it reached
`classify.rs`, so re-run it. If it is still red, the fix belongs with the eval-corpus
work, not here.

### 3.6 Pre-push gates before any push

These were not run on the final tree:

```bash
vox ci pre-push --complete             # includes clippy; the default fast tier does not
pnpm --dir crates/vox-gui/ui typecheck
pnpm --dir crates/vox-gui/ui test:e2e  # only models-leaderboard.spec.ts was run, not the full suite
```

Other agents' uncommitted GUI files (BreadcrumbBar, Sidebar, ChatSurface, Loquela,
Popover, Tooltip) can make the UI gates fail for reasons unrelated to this work. Check
`git status` before attributing a failure.

### 3.7 Push and PR (the user's call)

Nothing is pushed. The branch also carries other agents' phase-12 commits, so decide
whether to PR the whole branch or split it (for example, cherry-pick the four commits
from this work onto a fresh branch off `main`). CodeRabbit is retired, so review the
full branch diff yourself before asking for a merge (AGENTS.md §PR & Review Discipline).
Cache regeneration after merge is handled by the `ssot-autoregen` CI job; do not
hand-regenerate.

### 3.8 Small loose ends

- **`Cargo.lock`:** an unstaged line adds `reqwest 0.12.28` to `vox-gui`'s
  dependencies. It does not come from this work; its owner is unknown. Leave it to
  whoever changed `vox-gui`'s manifest.
- **`VOX_MODEL_FORCE`:** the proof script sets it (as the research shim does), but it
  has no entry in `contracts/config/env-vars.v1.yaml`. The gap predates this work (the
  variable dates from 2026-05-15); add an entry when convenient.
- **Optional refactor:** `Orchestrator::refresh_model_scoreboard` (`telemetry.rs`)
  repeats what `ModelRegistry::inject_evidence_from_db` (`models/registry.rs`) does,
  adding per-source warning logs. Folding the daemon onto the shared builder, with the
  logging kept, would remove a place where the two can drift. The new daemon-refresh
  test would guard that change.
- **Leave alone:** the known `vox-orchestrator-mcp` test errors (`visus_review`,
  `INFERENCE_PROFILE_TEST_LOCK`) were deliberately out of scope.

## 4. Environment notes for the next session

- **Build broker:** route every `cargo` call through the broker; never call
  `~/.cargo/bin/cargo` directly. During this work another session ran
  `cargo clean --profile dev` on the shared `target/`, which caused stale
  `icu_locale_core` (E0460) errors and a missing `target/debug/vox`. If you see those,
  wait for the clean to finish, then rebuild. Do not kill the other session.
- **Live bench state:** the proof writes its reports to `target/model-bench/*.json` and
  its database to `target/model-bench/data` (`VOX_DATA_DIR`), apart from the user store,
  because the running daemon holds the user database's lock. A later `cargo clean`
  by another session has already deleted that directory, so the raw reports from the
  2026-10-09 run are gone; the numbers survive only in §5.2 of the autonomic doc. A
  fresh proof run regenerates both.
- **Other agents:** several agents work in this tree at once and keep their own staged
  files. Commit by path (`git commit -- <paths>`); do not commit `Cargo.lock` or files
  you did not change. If `.git/index.lock` exists, wait rather than deleting it.
- **Test-first guard:** the pre-commit `tdd-guard` rescans every touched file, so an old
  untested `pub fn` in a file you edit will block the commit. Prefer a real test.
  Suppress (in `contracts/toestub/suppressions.v1.json`, with a truthful reason) only
  when a test would add nothing, as with the `vox model` dispatcher.
- **Formatting:** run `rustfmt` on specific files, or `vox run scripts/fmt.vox`; never
  `cargo fmt --all`.
- **`vox-cli` tests** live in the library target: `cargo test -p vox-cli --lib <filter>`.
  `--bin vox` runs zero of them.
