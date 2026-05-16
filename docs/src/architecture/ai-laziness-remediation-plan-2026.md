# AI-Laziness Remediation Plan (2026-05-16)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut aspirational/stub code from the Vox codebase in four tracks (retirements → telemetry/dedup → vox-code-audit stub strip → MENS Batch 3), beginning with a small, low-risk set of verified retirements that rebuild trust in the codebase signal.

**Architecture:** This plan was authored after a parallel multi-agent audit on 2026-05-16 (9 agents across mesh/MENS, codegen, orchestrator, telemetry, plugins, scientia, GUI/Tauri, oddballs, and docs). Agent claims were spot-checked by hand; two false-positive retirements (`vox-orchestrator-d`, `vox-plugin-mens-candle-metal`) were removed from this plan after verification. Subsequent tracks (Telemetry trace propagation + dedup, `vox-code-audit` stub strip, MENS Batch 3) are scoped at outline only and will get their own detailed plans after Phase 1 ships.

**Tech Stack:** Rust 1.83 workspace (cargo), 112 crates, Vox language (`.vox` scripts for automation per AGENTS.md), TypeScript/React in `apps/`, mdBook docs in `docs/src/`.

**Companion audit findings:** See `comprehensive-audit-v2-2026.md` (April 2026) for the governance crisis diagnosis this plan partly responds to.

---

## Phase 1 (Track 1): Verified Retirements

**Goal of phase:** Remove 7 items from the codebase that audit + verification confirmed are zero-value, broken, or misfiled. Each task is independent, small, and reversible via `git revert`. After this phase ships, the codebase has fewer false positives in `cargo metadata`, `vox arch-check`, and the catalog — making the remaining audits sharper.

**Out of scope for Phase 1 (audit-agent false positives found during verification):**
- `vox-orchestrator-d` — verified real; ADR 022 Phase B daemon with 81 cross-workspace refs.
- `vox-plugin-mens-candle-metal` — verified real `MlBackend` plumbing; SP3 stub state matches CUDA; revisit in MENS Batch 3 plan.
- `apps/interop/marquee_app` — **canonical Slot 1 v1.0 marquee app**, ratified by council 2026-05-15 (`contracts/marquee/manifest.v1.yaml`); load-bearing for CR-P1/CR-P3/CR-E2/CR-L0/CR-L7. The audit agent fabricated a "broken JSX in dist/" claim — there is no `dist/` directory, and the `.vox` source's placeholder endpoint returns are deliberate (canonical showcase of `@table`, `@endpoint(kind: query|mutation)`, components, routes).

### Task 1.0: [removed — marquee_app verified as canonical v1.0 marquee app, not a retirement target]

**Goal:** Confirm the audit's claim that marquee_app is a broken demo with emitted JSX that doesn't compile and no callers. If a consumer exists, demote to "fix rather than delete."

**Files:**
- Read: `apps/interop/marquee_app/src/main.vox`
- Read: `apps/interop/marquee_app/dist/Dashboard.tsx`
- Search: workspace-wide for `marquee_app` / `marquee-app`

- [ ] **Step 1: Grep for callers**

Run: `rg -nF "marquee_app" -g "!apps/interop/marquee_app/**"`
Expected: zero hits outside its own dir (CI configs, .vox/audit/ artifacts, doc references don't count as consumers).

- [ ] **Step 2: Visually confirm broken JSX**

Open `apps/interop/marquee_app/dist/Dashboard.tsx`. Lines ~12–18 should contain an arrow function `(() => { <li>...</li>; })()` that doesn't return its JSX. If the JSX is well-formed, abort Phase 1.4 and re-plan.

- [ ] **Step 3: Confirm build artifacts are stale**

Run: `ls -la apps/interop/marquee_app/dist/`
Expected: file mtimes from April 2026 or earlier; nothing rebuilt this month.

- [ ] **Step 4: Record verdict**

If steps 1–3 confirm: proceed to Task 1.4 (delete). If any conflict: stop and re-scope with the user.

---

### Task 1.1: Delete `crates/vox-integration-tests`

**Files:**
- Delete: `crates/vox-integration-tests/` (entire directory)
- Modify: `Cargo.toml` (workspace) — remove member entry + `[workspace.dependencies]` path entry

- [ ] **Step 1: Confirm zero callers**

Run: `rg -nF "vox-integration-tests" -g "**/Cargo.toml"`
Expected: only `Cargo.toml` (workspace) line 87 and the crate's own `Cargo.toml`.

Run: `rg -nF "vox_integration_tests"`
Expected: zero hits.

- [ ] **Step 2: Delete the crate**

```bash
rm -rf crates/vox-integration-tests
```

- [ ] **Step 3: Remove workspace references**

Edit `Cargo.toml` (workspace root):
- Remove the line `"crates/vox-integration-tests",` from `members = [...]`
- Remove the line `vox-integration-tests     = { path = "crates/vox-integration-tests" }` from `[workspace.dependencies]` (line ~87)

- [ ] **Step 4: Verify workspace still resolves**

Run: `cargo metadata --no-deps --format-version 1 > NUL 2>&1` (PowerShell uses `$null`; Bash use `/dev/null`)
Expected: exit code 0, no errors.

- [ ] **Step 5: Build smoke check**

Run: `cargo check --workspace --all-targets`
Expected: success or only pre-existing warnings (no new errors).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: retire vox-integration-tests (empty 4-LoC harness, 0 callers)"
```

---

### Task 1.2: Delete `crates/voxup`

**Files:**
- Delete: `crates/voxup/` (entire directory)
- Modify: `Cargo.toml` (workspace) — remove member + workspace.dependencies entry
- Search: workspace for any `voxup` binary invocations in CI / scripts / docs

- [ ] **Step 1: Confirm zero callers**

Run: `rg -nF "voxup" -g "!crates/voxup/**" -g "!docs/**" -g "!.vox/audit/**"`
Expected: only entries in `Cargo.toml` (workspace).

Run: `rg -nF "voxup" docs/src/`
Document any doc references; they'll need cleanup in step 4.

- [ ] **Step 2: Delete crate**

```bash
rm -rf crates/voxup
```

- [ ] **Step 3: Remove workspace references**

Edit `Cargo.toml` (workspace root):
- Remove `"crates/voxup",` from `members`
- Remove `voxup = { path = "crates/voxup" }` if present in `[workspace.dependencies]`

- [ ] **Step 4: Clean up doc references**

For each doc match from step 1, either remove the reference or replace with a one-line note that `voxup` is retired.

- [ ] **Step 5: Verify workspace resolves + builds**

Run: `cargo metadata --no-deps --format-version 1`
Run: `cargo check --workspace --all-targets`
Expected: success.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: retire voxup (unfinished toolchain multiplexer, 0 callers)"
```

---

### Task 1.3: Delete `crates/vox-plugin-cloud`

**Goal:** This plugin's entire `sync.rs` is an SP7 scaffold returning `"not yet implemented; SP7 scaffold"` on `upload`/`download` and `"[]"` on `list_remote_json`. Verified by reading the source. Bundling it in `vox-dev` means users hit a hardcoded error.

**Files:**
- Delete: `crates/vox-plugin-cloud/` (entire directory)
- Modify: `Cargo.toml` (workspace) — remove member + path entry
- Modify: `crates/vox-plugin-catalog/catalog.toml` — remove the plugin entry and remove from any `bundled-in` arrays
- Modify: any consumers found by grep

- [ ] **Step 1: Inventory callers**

Run: `rg -nF "vox-plugin-cloud" -g "**/Cargo.toml"`
Expected: only workspace + the crate itself.

Run: `rg -nF "vox_plugin_cloud"`
Expected: zero or just internal references inside `vox-plugin-cloud/`.

Run: `rg -nF "\"cloud\"" crates/vox-plugin-catalog/catalog.toml`
Document the entry's location and any bundle membership.

- [ ] **Step 2: Verify the `as_cloud_sync` extension point has no production callers**

Run: `rg -nF "as_cloud_sync\|cloud_sync"`
If any production crate (not just tests) depends on `CloudSync` being resolvable, abort and re-scope.

- [ ] **Step 3: Delete crate**

```bash
rm -rf crates/vox-plugin-cloud
```

- [ ] **Step 4: Remove workspace references**

Edit `Cargo.toml` (workspace):
- Remove `"crates/vox-plugin-cloud",` from `members`
- Remove the path entry from `[workspace.dependencies]`

- [ ] **Step 5: Remove from plugin catalog**

Edit `crates/vox-plugin-catalog/catalog.toml`:
- Delete the entire `[[plugins]]` block for `id = "cloud"`
- Remove `"cloud"` from any `bundled-in = [...]` arrays (e.g., `vox-dev`)

- [ ] **Step 6: Regenerate the catalog doc**

Run: `vox run scripts/regenerate-plugin-catalog-doc.vox` (or the documented regen command — search `crates/vox-plugin-catalog/README.md` for the canonical invocation; do NOT hand-edit `docs/src/reference/plugin-catalog.generated.md`).

If no regen script exists, file a follow-up issue rather than hand-editing the generated doc (per project memory rule).

- [ ] **Step 7: Run catalog validation tests**

Run: `cargo test -p vox-plugin-catalog`
Expected: pass. If a test asserts the cloud plugin is listed, update it.

- [ ] **Step 8: Verify workspace builds**

Run: `cargo check --workspace --all-targets`
Expected: success.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore: retire vox-plugin-cloud (entire crate is SP7 scaffold; bundled but non-functional)"
```

---

### Task 1.4: Delete `crates/vox-plugin-script-execution`

**Goal:** Identical situation to Task 1.3. Both `execute()` and `validate()` return `"not yet implemented; SP7 scaffold"`. The plugin is in the `vox-dev` bundle; `vox run` users hit the error.

**Files:**
- Delete: `crates/vox-plugin-script-execution/` (entire directory)
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/vox-plugin-catalog/catalog.toml`

- [ ] **Step 1: Inventory callers**

Run: `rg -nF "vox-plugin-script-execution" -g "**/Cargo.toml"`
Run: `rg -nF "vox_plugin_script_execution"`
Run: `rg -nF "as_script_executor"` — confirm no production consumer.
Expected: zero production callers.

- [ ] **Step 2: Delete crate**

```bash
rm -rf crates/vox-plugin-script-execution
```

- [ ] **Step 3: Remove workspace references**

Edit `Cargo.toml`:
- Remove member entry
- Remove path entry from `[workspace.dependencies]`

- [ ] **Step 4: Remove from catalog**

Edit `crates/vox-plugin-catalog/catalog.toml`:
- Delete the `[[plugins]]` block for `id = "script-execution"`
- Remove `"script-execution"` from any `bundled-in` arrays

- [ ] **Step 5: Regenerate catalog doc**

Same as Task 1.3 step 6.

- [ ] **Step 6: Run catalog tests**

Run: `cargo test -p vox-plugin-catalog`
Expected: pass.

- [ ] **Step 7: Verify build**

Run: `cargo check --workspace --all-targets`

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: retire vox-plugin-script-execution (entire crate is SP7 scaffold; bundled but non-functional)"
```

---

### Task 1.5: Remove the LSP "proximity alert" stub

**Goal:** `crates/vox-lsp/src/main.rs:105-118` contains a hardcoded `if word == "resolveArenaRound" || word == "combatRoundResolver"` block that returns boilerplate markdown about "Knowledge Conflating Hallucinations." It appears to be dev/debug leftover, not a real LSP capability.

**Files:**
- Modify: `crates/vox-lsp/src/main.rs:105-118`
- Test: `crates/vox-lsp/tests/` (verify hover tests still pass)

- [ ] **Step 1: Read and confirm**

Open `crates/vox-lsp/src/main.rs` at lines 105–118. Confirm the block matches:

```rust
// Wave 5: Semantic Proximity Hover
// Surface proximity hints from search execution directly in the editor.
if word == "resolveArenaRound" || word == "combatRoundResolver" {
    let md = format!(
        "**Proximity Alert:** `{word}` shares semantic overlap with a similar symbol. ..."
    );
    return Ok(Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: md,
        }),
        range: None,
    }));
}
```

If the block is different, re-read the file and update this step before continuing.

- [ ] **Step 2: Search for tests of this behavior**

Run: `rg -nF "resolveArenaRound\|combatRoundResolver\|Proximity Alert" crates/vox-lsp/`
Expected: only the one source location. If a test asserts this behavior, delete the test too.

- [ ] **Step 3: Delete the block**

Edit `crates/vox-lsp/src/main.rs`: remove lines 105–118 (the `// Wave 5:` comment block and the entire `if word == ...` body, leaving the surrounding handler intact). Verify the resulting `hover` handler ends with `Ok(None)` after the prior `if let Some(md) = builtin_hover_markdown_in_line(...)` block.

- [ ] **Step 4: Build and test**

Run: `cargo test -p vox-lsp`
Expected: all hover tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-lsp/src/main.rs
git commit -m "chore(lsp): remove dev-leftover proximity-alert stub for two hardcoded symbols"
```

---

### Task 1.6: Relocate `vox-plugin-noop-skill` to test fixtures

**Goal:** This plugin is a real fixture used by `vox-plugin-host/tests/load_noop_skill.rs` and `vox-cli/tests/plugin_commands_smoke.rs`, but is currently in `crates/` and *listed in the production catalog* (`vox-plugin-catalog/catalog.toml:188`). Move it next to its peer test fixtures (`vox-plugin-host/tests/fixtures/noop-code/`) and remove it from the catalog so it stops showing up as a real distribution plugin.

**Files:**
- Move: `crates/vox-plugin-noop-skill/` → `crates/vox-plugin-host/tests/fixtures/noop-skill/`
- Modify: `Cargo.toml` (workspace) — remove `crates/vox-plugin-noop-skill` from members
- Modify: `crates/vox-plugin-catalog/catalog.toml` (lines ~188–192) — remove `noop-skill` entry
- Modify: `crates/vox-plugin-host/tests/load_noop_skill.rs` (line 18) — fix the path
- Modify: `crates/vox-cli/tests/plugin_commands_smoke.rs` (lines 54–57, 64) — fix the path

- [ ] **Step 1: Inventory references**

Run: `rg -nF "vox-plugin-noop-skill\|noop-skill" -g "!docs/src/architecture/plugin-system-redesign*" -g "!.vox/**" -g "!contracts/reports/**"`
Note every match. Expected sites:
- `crates/vox-plugin-host/tests/load_noop_skill.rs`
- `crates/vox-cli/tests/plugin_commands_smoke.rs`
- `crates/vox-plugin-catalog/catalog.toml`
- `crates/vox-cli/src/commands/ci/agentskills_compliance.rs`
- `Cargo.toml` (workspace)

- [ ] **Step 2: Move the directory**

```bash
mkdir -p crates/vox-plugin-host/tests/fixtures
git mv crates/vox-plugin-noop-skill crates/vox-plugin-host/tests/fixtures/noop-skill
```

(Using `git mv` preserves history.)

- [ ] **Step 3: Remove from workspace members**

Edit `Cargo.toml` (workspace): remove line `"crates/vox-plugin-noop-skill",`.

The fixture directory contains no `Cargo.toml` of its own (only `Plugin.toml` + `noop.skill.md`), so it doesn't need to be a workspace member.

- [ ] **Step 4: Update fixture paths in tests**

`crates/vox-plugin-host/tests/load_noop_skill.rs` line ~18:
- Was: `.join("vox-plugin-noop-skill")`
- Now: `.join("tests").join("fixtures").join("noop-skill")` (verify the path resolves relative to `CARGO_MANIFEST_DIR`)

`crates/vox-cli/tests/plugin_commands_smoke.rs` lines ~54–64:
- Was: `workspace_root.join("crates").join("vox-plugin-noop-skill")`
- Now: `workspace_root.join("crates").join("vox-plugin-host").join("tests").join("fixtures").join("noop-skill")`

- [ ] **Step 5: Remove from production catalog**

Edit `crates/vox-plugin-catalog/catalog.toml`:
- Delete the `[[plugins]]` block at line 188 (`id = "noop-skill"`, `default-source = "local:crates/vox-plugin-noop-skill"`)
- Remove `"noop-skill"` from any `bundled-in` arrays

- [ ] **Step 6: Decide on `agentskills_compliance.rs:212` assertion**

`crates/vox-cli/src/commands/ci/agentskills_compliance.rs:212` asserts `is_valid_name("noop-skill")`. That's a name-validation test using `"noop-skill"` as a known-good example string, NOT a reference to the catalog entry. Leave the line alone.

- [ ] **Step 7: Regenerate catalog doc**

Same as Task 1.3 step 6 — do not hand-edit the generated `.md`.

- [ ] **Step 8: Run affected tests**

Run: `cargo test -p vox-plugin-host -- load_noop_skill`
Expected: pass with new path.

Run: `cargo test -p vox-cli -- plugin_commands_smoke`
Expected: pass with new path. (This test may require the workspace to be built; do `cargo build -p vox-cli` first if it complains.)

Run: `cargo test -p vox-plugin-catalog`
Expected: pass.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore(plugins): move noop-skill from crates/ to vox-plugin-host test fixtures; remove from production catalog"
```

---

### Task 1.7: Move `apps/experimental/visualizer` to `scratch/visualizer-prototype/`

**Goal:** Polished UI prototype with hardcoded data ("Active Agents: 4", "Queue Depth: 12"); no `invoke` calls or `useEffect` data fetches. Per user direction, keep as a design prototype (not delete, not extract) under `scratch/`.

**Files:**
- Move: `apps/experimental/visualizer/` → `scratch/visualizer-prototype/`
- Possible: a small README at `scratch/visualizer-prototype/README.md` explaining provenance
- Search: any CI/build references to the old path

- [ ] **Step 1: Confirm no build references**

Run: `rg -nF "apps/experimental/visualizer\|apps\\experimental\\visualizer"`
Expected: zero hits outside the directory itself.

Run: `rg -nF "experimental/visualizer" .github/ scripts/ infra/ docker/ 2>&1`
Expected: zero.

- [ ] **Step 2: Move the directory**

```bash
mkdir -p scratch
git mv apps/experimental/visualizer scratch/visualizer-prototype
```

- [ ] **Step 3: If `apps/experimental/` is now empty, remove it**

Run: `ls apps/experimental/`
If empty: `rmdir apps/experimental` and `git add -A` to capture deletion.

- [ ] **Step 4: Drop a 6-line README in the new location**

Create `scratch/visualizer-prototype/README.md`:

```markdown
# Visualizer Prototype

Design prototype moved from `apps/experimental/visualizer/` on 2026-05-16
during AI-laziness remediation Phase 1 (see
`docs/src/architecture/ai-laziness-remediation-plan-2026.md`).

This is a UI sketch with hardcoded data — not a runnable orchestrator
dashboard. Salvage components from here if useful, but don't restore
the standalone app without a real backend.
```

- [ ] **Step 5: Verify nothing else broke**

Run: `cargo check --workspace --all-targets`
Expected: success (this is a TS app; cargo shouldn't notice, but confirm.)

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: move apps/experimental/visualizer to scratch/visualizer-prototype (UI sketch with no backend)"
```

---

### Task 1.8: [removed — marquee_app is the canonical v1.0 Slot 1 marquee app, not a retirement target]

**Goal:** Hollow demo with emitted JSX that doesn't compile and stub endpoints returning hardcoded `[]`/`Ok("")`. Conditional on Task 1.0 verification.

**Files:**
- Delete: `apps/interop/marquee_app/`
- Search: workspace for any references

- [ ] **Step 1: Final caller sweep**

Run: `rg -nF "marquee_app\|marquee-app" -g "!apps/interop/marquee_app/**" -g "!.vox/**" -g "!contracts/reports/**"`
Expected: zero.

- [ ] **Step 2: Delete**

```bash
git rm -r apps/interop/marquee_app
```

- [ ] **Step 3: If `apps/interop/` is now empty, remove it**

```bash
ls apps/interop/
# If empty:
rmdir apps/interop
git add -A
```

- [ ] **Step 4: Verify**

Run: `cargo check --workspace --all-targets`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: retire apps/interop/marquee_app (broken JSX, stub endpoints, no consumers)"
```

---

### Task 1.9: Phase 1 verification gate

**Goal:** Before declaring Phase 1 done, run the full workspace check + tests one last time and confirm the architecture-check still passes.

- [ ] **Step 1: Full workspace build**

Run: `cargo check --workspace --all-targets`
Expected: clean compile (or only pre-existing warnings).

- [ ] **Step 2: Workspace test (no execution, just compilation)**

Run: `cargo test --workspace --no-run`
Expected: success.

- [ ] **Step 3: Architecture check**

Run: `cargo run -p vox-arch-check`
Expected: pass, or only pre-existing violations (none newly introduced).

- [ ] **Step 4: Doc/link checks**

Run the project's documented link checker (search `lefthook.yml` or `.github/workflows/` for the canonical invocation).
Expected: no new broken links introduced by removed paths.

- [ ] **Step 5: Append summary to CHANGELOG**

Add an entry to `CHANGELOG.md` under the next unreleased section summarizing the seven retirements with the rationale "AI-laziness remediation Phase 1." Do NOT generate the changelog by hand if the project uses `git-cliff` or similar — use the documented regen command instead.

- [ ] **Step 6: Commit changelog**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): record AI-laziness remediation Phase 1 retirements"
```

- [ ] **Step 7: Open PR**

The branch is `cc_bdesktop2/fervent-albattani-e65953`. Push and open a PR titled:
`chore: AI-laziness remediation Phase 1 (verified retirements)`

PR body should link to this plan doc and list the seven retired/relocated items.

---

## Phase 2 (Track 4): Telemetry Trace Propagation + Documented Dedup

**Status:** Outline only — gets its own detailed plan after Phase 1 lands.

**Scope:**
1. Wire `tokio::task_local!` span context (already defined in `crates/vox-telemetry/src/span.rs`) into `crates/vox-orchestrator-mcp/src/llm_bridge/infer.rs` so that `ModelCallEvent` emissions populate `task_id`, `trace_id`, `caller_agent_id` instead of `None`.
2. Inventory the 8 `vox-db` legacy telemetry wrappers (`benchmark_telemetry.rs`, `syntax_k_telemetry.rs`, `exec_time_telemetry.rs`, etc.) that still write to `research_metrics` outside the new L1 facade; route them through `vox_telemetry::record_event!` or retire each one if its emit site is dead.
3. Fix the arXiv `.tex` dual-brain: `crates/vox-publisher/src/submission/arxiv.rs:26` generates a placeholder `\documentclass{article}` instead of using `vox-manuscript-latex::render_latex()`. Either merge to a single pass-through or replace `pack_arxiv_staging_tar_gz` with a typed `SubmissionArtifact { manifest, rendered_tex, arxiv_bundle }`.

**Decision points for the Phase 2 plan:**
- Should the 8 legacy `vox-db` telemetry wrappers be deleted in one PR or migrated one at a time?
- Should we add a CI guard (lint rule in `vox-rule-pack` or a `vox-arch-check` rule) that forbids new direct writes to `research_metrics`?
- For the arXiv submission, is "operator-assist" the long-term answer or do we want to design an actual arXiv API client now?

---

## Phase 3 (Track 2): `vox-code-audit` Stub Strip

**Status:** Outline only.

**Scope:** `crates/vox-code-audit/` (19,375 LoC, 83 files) contains 27 `todo!()` / `unimplemented!()` / `panic!("not implemented")` call sites. The crate is a lint-rule engine; panics in audit rules turn audit runs into crashes instead of reporting findings. Replace each panic with either a real implementation or a graceful skip/warn.

**Approach (TBD in detailed plan):**
1. Inventory the 27 stubs with file:line + which rule they belong to.
2. Group by rule family.
3. For each group: decide implement-now / convert-to-warn / delete-rule.
4. Add a CI lint that bans `todo!()` / `unimplemented!()` in `vox-code-audit/src/**` going forward.

**Decision points:** How aggressive on the convert-to-warn vs. implement-now split? Some rules are aspirational (e.g., detect a non-trivial code-smell pattern) and may never be worth implementing — those should be deleted, not warned.

---

## Phase 4 (Track 3): MENS Batch 3 + Plugin SSOT Consolidation

**Status:** Outline only. Highest risk; deserves its own detailed plan with explicit verification on real GPUs.

**Scope:**
1. **Wire the four currently-stubbed entry points in `vox-plugin-mens-candle-cuda/`:**
   - `crates/vox-plugin-mens-candle-cuda/src/checkpoint.rs::save_checkpoint` (currently `bail!("not yet wired (SP3 stub)")`)
   - `crates/vox-plugin-mens-candle-cuda/src/model.rs::load_from_path` (stubbed)
   - `crates/vox-plugin-mens-candle-cuda/src/inference.rs` (stub)
   - `crates/vox-plugin-mens-candle-cuda/src/merge.rs::merge_qlora_adapter` (stub)
2. **Wire `vox-ml-cli` eval-local:** `crates/vox-ml-cli/src/commands/mens/eval_local.rs:77-80` currently sets `engine = Option<()>::None` pending plugin-host dispatch. Plumb the dispatch.
3. **Plugin SSOT consolidation:** Purge lingering imports / feature gates in `vox-populi/src/mens/tensor/*` that still reference deleted `candle_model_qwen` / `candle_inference_serve` / `backend_candle_qlora` modules. Plugin owns these now.
4. **Metal plugin decision:** `vox-plugin-mens-candle-metal/` mirrors CUDA structurally and has real `MlBackend` plumbing in `backend.rs`, but `training.rs` carries the same SP3 stub messages. Per `mesh-and-language-distribution-ssot-2026.md §0.2` (claimed by audit; verify), training is CUDA-only for v0.6+. Options:
   - **(a)** Apply Batch 3 wiring symmetrically to Metal (more code to maintain, but supports macOS dev).
   - **(b)** Mark Metal plugin's training entry points as `bail!("Metal training intentionally not supported; use CUDA")` and keep inference/merge support.
   - **(c)** Delete Metal training-related modules entirely; keep Metal inference + tokenizer-only support.

**Decision points for the Phase 4 plan:**
- Which Metal option (a / b / c)?
- What does "wired" mean for verification? (Saved checkpoint must round-trip via load + eval; or: training run for N steps with non-NaN loss; or: external benchmark match)
- Does Batch 3 ship under a feature flag (`experimental-training`) initially, or full default?

---

## Cross-cutting principles

- **No hand-edits to auto-generated files.** Per project memory: `SUMMARY.md`, `architecture-index.md`, `feed.xml`, `*.generated.md`, `.cursorignore` are regenerated by tools; always rerun the generator.
- **Automation scripts are `.vox`** (per AGENTS.md and CLAUDE.md). Do not introduce `.ps1`, `.sh`, or `.py` scripts.
- **Each PR small.** Phase 1 should produce ~8 commits; group them into 1–3 PRs based on what reviewers prefer.
- **Verify before delete.** Two of the audit's "delete" candidates (`vox-orchestrator-d`, `vox-plugin-mens-candle-metal`) turned out to be real systems. Always grep + read before removing.

## Companion docs

- `docs/src/architecture/comprehensive-audit-v2-2026.md` — prior governance-crisis diagnosis
- `docs/src/architecture/v1-release-criteria.md` — v1.0 gates (currently mostly undefined operationally)
- `docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md` — ongoing tombstone tracker
- `AGENTS.md` — workspace policy surface
