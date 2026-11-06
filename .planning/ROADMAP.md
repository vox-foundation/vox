# Roadmap: Vox Unified Workstream

# Roadmap: v5.7 — Vox ↔ Python Bridge & Tolerant CLI Ergonomics

**Milestone:** v5.7
**Status:** Phase 324 verified 2026-11-03 (5/5). `phase.complete` did not write this file: workstream mode has no active workstream, and v5.7 lives on this root roadmap.
**Updated:** 2026-11-01
**Design SSOT:** `docs/src/architecture/vox-python-interop-design-2026-11-01.md`; `docs/src/architecture/cli-semantic-tolerance-research-2026.md`
**Paper-code rule (all phases):** a phase exits only on an end-to-end `vox run` golden through the **real** worker (or, for compiler-only phases, a compiler golden through the real pipeline) — never on unit tests or the fake worker alone. Feature-matrix cells flip to `Implemented` only with a running ladder entry.

### Phase 324: Effect Gate & Determinism (`EFF-01..03`)

**Goal:** The effect and capability gate that every later Python call depends on exists and cannot be silently bypassed.
**Depends on**: Nothing (first phase of v5.7)
**Requirements**: EFF-01, EFF-02, EFF-03
**Success Criteria** (what must be TRUE):

  1. `@uses(nonexistent)` and `uses nonexistent` both produce a compile error; `@uses(mcp(tool))` appears in HIR as `Mcp(tool)` (golden).
  2. `@uses(python_unverified)` survives into HIR and `HirCapability`; a materializer-shaped call without the effect fails typecheck (golden).
  3. `vox run --caps` without `py` refuses a Python-capability call at runtime with a caps error.
  4. A `workflow` calling a Python-capability fn is rejected by capability (not name); the same call in an `activity` compiles.

**Plans**: 2/2 complete (324-01, 324-02). Verification passed 2026-11-03.

### Phase 325: Bridge Runtime & Environment (`BR-01..04`, `GOV-01`)

**Goal:** `vox-python-bridge` spawns a real uv-managed worker, speaks framed Arrow IPC, survives crashes/timeouts, and lands with governance rows.
**Depends on**: Phase 324
**Requirements**: BR-01, BR-02, BR-03, BR-04, GOV-01
**Success Criteria** (what must be TRUE):

  1. `vox py sync` on a `[python]` project creates the env and records the `uv.lock` hash in `vox.lock`; `vox py doctor` reports status; a project without `[python]` gets `py-env:` and spawns no process.
  2. A one-op golden runs end-to-end under `vox run` through the real worker; a worker op that prints to stdout still returns a correct result.
  3. A deliberately hung op hits `timeout_s`, the worker restarts, and a pre-restart handle returns `py: stale handle`; the host process survives a worker crash.
  4. An over-cap fetch returns `py: fetch exceeds cap`; sandboxed spawn goes through `vox-skill-runtime`, unsandboxed warns.
  5. `vox-arch-check` and `vox ci crate-edges` pass with the new crate; AGENTS.md exemption and stale-citation fixes are in the diff.

**Plans**: 2/2 complete (325-01, 325-02)

### Phase 326: `std.array` on numpy (`ARR-01..04`, `K-01`)

**Goal:** Vox code computes with numpy through a closed, typed, standards-named vocabulary and pure lazy plans.
**Depends on**: Phase 325
**Requirements**: ARR-01, ARR-02, ARR-03, ARR-04, K-01
**Success Criteria** (what must be TRUE):

  1. `examples/golden/py_array_stats.vox` runs under `vox run` with asserted output, using one round-trip per materialization.
  2. Mixed int/float without `.astype`, `std` without `correction=`, and `sort` without `nulls=` are compile errors.
  3. NaN, ±Inf and i64::MIN/MAX round-trip through `to_list`; uint64 overflow errors.
  4. `vox ci python-bridge-parity` fails when an allowlisted op is missing its signature, handler or conformance test (negative test).
  5. Ladder entry + complexity-budget row exist for the golden; `LEXER_KEYWORDS` unchanged; K2 ceiling enforced.

**Plans**: 8/8 executed. Verified 2026-11-04 (8/8 must-haves). Code review left 3 critical findings open.

### Phase 327: `std.frame` on polars (`FRM-01..03`)

**Goal:** Vox code runs lazy dataframe pipelines on polars and fetches typed `Records` with exact type mapping and host-side path control.
**Depends on**: Phase 326
**Requirements**: FRM-01, FRM-02, FRM-03
**Success Criteria** (what must be TRUE):

  1. `examples/golden/py_frame_report.vox` (scan_csv → filter → group_by/agg → sort → collect) passes with one round-trip per `collect`.
  2. A fixture with decimal, tz-aware timestamp, date, duration and nullable columns fetches as `Decimal`, UTC `DateTime`, `Date`, `Duration`, `Option[T]`; a naive timestamp errors with the fix-it.
  3. `scan_csv` on a path outside `--caps fs:ro=` is refused before any worker request (asserted via fake-worker request log).

**Plans**: TBD

### Phase 328: `std.plot` & `std.tensor` (`PLT-01`, `TNS-01..02`)

**Goal:** Vox code renders matplotlib charts and runs exported PyTorch models without data or tensors leaving the worker unnecessarily.
**Depends on**: Phase 327
**Requirements**: PLT-01, TNS-01, TNS-02
**Success Criteria** (what must be TRUE):

  1. A plot golden writes a non-empty PNG with expected dimensions; the request log shows no frame fetch to host.
  2. `examples/golden/py_tensor_infer.vox` loads a checked-in tiny `.pt2` and returns expected logits on CPU; a pickle `.pt` is refused.
  3. Two seeded runs produce identical numpy/torch outputs; the activity journal entry contains library versions and lock hash.

**Plans**: TBD

### Phase 329: Tier 2 Escape Hatch (`PYX-01..04`)

**Goal:** Vox code reaches the long tail (scipy, scikit-learn, pillow, pandas, any bound package) through a typed, gated, counted escape hatch.
**Depends on**: Phase 328
**Requirements**: PYX-01, PYX-02, PYX-03, PYX-04
**Success Criteria** (what must be TRUE):

  1. `examples/golden/py_escape_psd.vox` (`import py sig from "scipy.signal"`) passes; an unlisted name, a stale sidecar, and `PyObject.field` are compile errors.
  2. Curated sidecars + goldens for scipy, scikit-learn, pillow and pandas pass in the opt-in Python CI job.
  3. Each Tier 2 call site yields `vox/python/unverified-call` and a `python.escape_call` event; syntax-K support metrics show `py_tier1_calls` / `py_tier2_calls`.
  4. Attempting to bind `pickle` or `subprocess` fails with a Denied-policy error.

**Plans**: TBD

### Phase 330: DB & Backend Interop (`DBI-01..03`)

**Goal:** Vox apps share databases owned by Drizzle/SQLAlchemy/Alembic and serve Python clients, using existing introspection and OpenAPI surfaces.
**Depends on**: Phase 324 (independent of 325–329; may run in parallel)
**Requirements**: DBI-01, DBI-02, DBI-03
**Success Criteria** (what must be TRUE):

  1. `vox db introspect --emit vox` on a sqlite fixture emits `table` decls that `vox db verify` accepts against the same DB (round-trip test; postgres/mysql behind existing features).
  2. With `[db] schema_owner = "external"`, `vox db migrate` refuses to apply and `--dry-run` prints the plan.
  3. The how-to's Python client example is generated from a golden server app's OpenAPI and checked in CI docs lint.

**Plans**: 1/1 complete (330-01)

### Phase 331: Tolerant CLI Ergonomics & Precomputed Semantic Routing (`CLI-TOL-01..04`)

**Goal:** The `vox` CLI tolerates human memory gaps (unambiguous prefix completions, global flag hoisting, verb↔noun token permutations, unique leaf elision, and precomputed compositional synonyms) in `<1.5 ms` with zero runtime LLM calls, while enforcing `side_effect_class` safety gates and returning structured 1-turn self-repair redirects for LLM agents and non-TTY callers.
**Depends on**: Independent of Phases 324–330 (may run in parallel)
**Requirements**: CLI-TOL-01, CLI-TOL-02, CLI-TOL-03, CLI-TOL-04
**Success Criteria** (what must be TRUE):

  1. `VoxCliRoot` enables `infer_subcommands(true)` and `infer_long_args(true)`, and all `GlobalOpts` flags (`--json`, `--quiet`, `--verbose`, `--color`) carry `global = true` so `vox sec doc` and `vox secrets list --json` parse natively at `0 ms` overhead.
  2. When `VoxCliRoot::try_parse_from` fails, `argv_normalize` resolves order-agnostic verb↔noun permutations (`vox list secrets` → `vox secrets list`), unique leaf elision (`vox command-compliance` → `vox ci command-compliance`), and wrong-subcommand flags (`vox build --release` → suggests `vox compile --release`) in `<1.5 ms` over `CommandCatalog` and `CapabilityIndex`.
  3. Execution is gated by `side_effect_class` and TTY/agent detection: `read_only` permutations/synonyms auto-execute on interactive TTYs with a dim `stderr` canonical echo, mutating commands prompt `[Y/n]` on TTYs, and non-TTY/CI/LLM callers (`--json`, `CI=true`, `VOX_MCP=1`) never hang on prompts or silently auto-execute fuzzy guesses—returning exit code `2` with structured `aci.error_recovery` canonical redirects.
  4. `vox ci cli-synonyms --write` compiles compositional `verb_aliases`, `noun_aliases`, and `foreign_idioms` from `contracts/operations/catalog.v1.yaml` (using `vox_actor_runtime::llm` + `gemini-3.5-flash-lite` structured output and `gemini-embedding-2` / `local_embed_text` cosine margin $\ge 0.08$ verification) into `contracts/cli/semantic-aliases.v1.json`, and `vox ci command-compliance` fails on any alias collision or antonym banlist violation (`add` $\leftrightarrow$ `remove`, `start` $\leftrightarrow$ `stop`, `lock` $\leftrightarrow$ `unlock`, `set` $\leftrightarrow$ `get`).

**Plans**: 3 plans

Plans:

- [ ] `331-01-PLAN.md` — Prefix unfolding (`infer_subcommands`/`infer_long_args`), global flag hoisting, and `<1.5 ms` deterministic `argv_normalize::classify_recovery` (`CLI-TOL-01`, `CLI-TOL-02`)
- [ ] `331-02-PLAN.md` — Dual-persona (`InteractiveHuman` vs `NonInteractiveOrAgent`) & `side_effect_class` safety gate + structured `aci.error_recovery` diagnostics (`CLI-TOL-03`)
- [ ] `331-03-PLAN.md` — Precomputed semantic synonym compiler (`vox ci cli-synonyms`), `contracts/cli/semantic-aliases.v1.json` lockfile, and antonym/collision firewall (`CLI-TOL-04`)

### Phase 333: Gated training for every model size

**Goal:** Every published Qwen3 size finishes the same release review, then a merged complete model is stored and distributed, and only then does its Hugging Face card call it ready. 0.6B leaves the smoke recipe. 4B is already a full shared-recipe run. Cards stay in plain language and describe the trained add-on until the complete model is the file being distributed.
**Depends on**: Independent of Phases 324–331
**Requirements**: REL-01, REL-02, REL-03, REL-04
**Success Criteria** (what must be TRUE):

  1. 0.6B, 1.7B, 4B, and 8B each have a published run on the shared recipe (rank 16, alpha 32, gradient accumulation 8). The 0.6B smoke preset (`qwen3_dev_cpu`, rank 8, accumulation 1) is not the published 0.6B adapter. The published 4B run stays classified as a full shared-recipe run that was uploaded before the checklist passed. A resumed run reports token and step counts for the whole run, not only the last process.
  2. Each of those four sizes has a passing collateral check and a release receipt with `overall_passed: true`. 14B and 32B stay without a distributed Vox model until a run clears that same review.
  3. After that receipt, `vox mens merge-qlora` writes a complete checkpoint from the local Qwen base plus the trained adapter, and that checkpoint is the file stored and distributed. The local cache already holds original Qwen bases for 0.6B, 1.7B, 4B, 8B, and 14B. 32B is not downloaded. 14B has no trained adapter yet.
  4. The Hugging Face card and the size index change in the same step as that distribution. Until then they say the page is the trained add-on and the review is unfinished. They do not use the word "ungated". A reader can download the distributed file from the Files tab or with one `vox mens hub download` command.

**Plans**: 0 plans

Plans:

- [ ] TBD (run `/gsd-plan-phase 333` to break down)

## Progress Table (`v5.7`)

| Phase | Requirements | Status |
|---|---|---|
| 324 Effect Gate & Determinism | EFF-01..03 | Verified 2026-11-04 |
| 325 Bridge Runtime & Environment | BR-01..04, GOV-01 | Verified 2026-11-04 |
| 326 `std.array` on numpy | ARR-01..04, K-01 | Verified 2026-11-04 |
| 327 `std.frame` on polars | FRM-01..03 | Not started |
| 328 `std.plot` & `std.tensor` | PLT-01, TNS-01..02 | Not started |
| 329 Tier 2 Escape Hatch | PYX-01..04 | Not started |
| 330 DB & Backend Interop | DBI-01..03 | Verified 2026-11-04 |
| 331 Tolerant CLI Ergonomics & Precomputed Semantic Routing | CLI-TOL-01..04 | Planned (3 plans) |
| 333 Gated training for every model size | REL-01..04 | Not planned |

### Phase 342: Docs Authoring Parity and Starlight 0.42 Upgrade

**Directory:** `phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/`
**Goal:** Close the verified remainder of the AsciiDoc-parity blueprint for `voxlang.org`: upgrade `docs-astro` to Starlight ≥0.42 / Astro ≥7.2.10 (the gate for the ecosystem plugins), render the 24 GitHub-alert docs as real asides, and add build-time internal-link validation — without new contracts, new `.mdx` churn, or a parallel status system.
**Requirements**: DOCS-PARITY-01..04
**Depends on:** none (independent of v5.6 lanes; coordinate banners with `docs-freshness` Phase 32)
**SSOT:** `docs/src/architecture/docs-site-asciidoc-parity-findings-2026.md`
**Plans:** 4/4 plans complete — ✅ verified 2026-11-06 (7/7, `342-VERIFICATION.md`, `09c272e99`). Resolved stack: astro 7.4.5, starlight 0.42.9 (mature-only install, no release-age exceptions). Playwright 20 / unit 3 failures are the frozen pre-upgrade baseline, unchanged.

- **DOCS-PARITY-01** (✅ committed `9829418e9`): `remark-vox-include.mjs` supports `tag::x[]`/`end::x[]`, `#` leaders, comma multi-select, `N..M` line ranges, exact names, dedent; unit tests in `docs-astro/tests/unit/remark-vox-include.test.mjs`; `llms.txt` version derived from root `Cargo.toml` + namespace disambiguation. Build green (906 pages).
- **DOCS-PARITY-02:** Starlight `^0.38` → `≥0.42`, Astro `^6` → `≥7.2.10`; fix overrides (`Banner`, `Sidebar`, `PageSidebar`, `TableOfContents`, `SiteTitle`), `routeData.ts`, and the `@astrojs/starlight/components` Vite alias shims; `pnpm build` + `tests/unit` + Playwright `reader-layout`/`trust-signals` stay green.
- **DOCS-PARITY-03:** `starlight-github-alerts` renders the 24 `> [!NOTE|TIP|IMPORTANT|WARNING|CAUTION]` docs as asides (identical on GitHub and the site).
- **DOCS-PARITY-04:** `starlight-links-validator` in the build; fix the two current `remark-doc-links` failures (`t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md` out-of-root link, missing `api/vox-cargo-shim.generated.md` from `l5-binaries.generated.md`); feeds nightly Docs Link Checker (#578).

**Out of scope (rejected after verification — see SSOT):** reality-audit-driven banners (inventory has no status field; banners already ledger-driven, owned by `docs-freshness` Phase 32), `stability-matrix.v1.json` + Content Layer collections (invented contract; generated-`.md` pattern is canonical), `remark-vox-attributes` tokens / `::include{}` directives / `// !hide` / `examples/golden/anti/` titles (zero call sites), `starlight-markdown-blocks` custom asides.

Plans:
- [x] 342-01-PLAN.md — Fix both remark-doc-links failures at source (t3 page links; hub generator links only existing API pages, test-first) and freeze the pre-upgrade test baseline (wave 1)
- [x] 342-02-PLAN.md — Package-legitimacy gate, then astro 7 / Starlight 0.42 upgrade (unified() processor, alias via import.meta.resolve, llms-txt 0.12 patch), dependabot unpin, agent-safe Playwright (wave 2)
- [x] 342-03-PLAN.md — starlight-github-alerts renders every alert as an aside; un-nest the two list-nested alerts for GitHub parity (wave 3)
- [x] 342-04-PLAN.md — starlight-links-validator on default fatal options with a unit-tested exclude predicate, mutation probe, findings SSOT update (wave 4)

### Phase 343: Docs System SSOT and Automated Drift Control

**Goal:** Every kind of documentation divergence is either prevented (generated from a contract), detected (a gate that fails), or auto-corrected (regenerated by the `ssot-drift` / `ssot-autoregen` loop). No documented gap stays manual and nobody notices. The docs-system map `docs/src/architecture/docs-system-ssot.md` (landed 2026-11-06) is the SSOT; its §6 "Known gaps" list is this phase's backlog. Each gap closed removes its bullet in the same PR.
**Requirements**: DOCS-DRIFT-01..06
**Depends on:** Phase 342
**SSOT:** `docs/src/architecture/docs-system-ssot.md`
**Plans:** 0 plans

- **DOCS-DRIFT-01 (prevent, regen-list parity):** derive `scripts/ssot-regen.vox`'s regenerator list from `run_ssot_drift`'s stage table (or add a parity check that fails on mismatch). Removes the "keep in sync" hand list.
- **DOCS-DRIFT-02 (detect, overlay parity):** a `vox ci` check that extracts normative claims (forbidden/required frontmatter keys, retired symbols, banned commands) from `AGENTS.md` and fails when `.cursor/rules/*.mdc`, `GEMINI.md`, `CLAUDE.md` or `.github/copilot-instructions.md` contradict them. First fix: the `last_updated` contradiction in `documentation-policy.mdc`.
- **DOCS-DRIFT-03 (detect, prose vs code):** scan docs and agent files for `vox <cmd> [--flag]` mentions and "planned per …" claims. Fail when a mentioned command or flag is absent from `contracts/cli/command-registry.yaml`, or when a "planned" surface already exists there. First fixes: AGENTS.md `vox audit retirement` / `vox audit effort`.
- **DOCS-DRIFT-04 (detect, dead guards and orphans):** make guards that silently no-op when their input file is missing fail loudly (`check_docs_ssot` orphan inventory). Add an orphan-page report: a `docs/src` page no other page or nav entry links to, with a shrink-only baseline.
- **DOCS-DRIFT-05 (auto-correct, scheduled loops):** a scheduled job runs `vox audit docs --update-ledger` and the external link checker. It opens one draft PR with ledger refreshes and proposed `.lycheeignore` / link fixes, instead of a nightly-failure issue (#578). Decide whether to revive or retire the dormant docs-reality-audit program and the self-heal bot's draft-PR stage.
- **DOCS-DRIFT-06 (meta-gate):** a test asserts that every gate listed in docs-system-ssot §4 is wired where the table says (pre-push tier / `docs-quality.yml` / nightly). It also asserts §6 lists only gaps with no matching gate. The map can then never silently disagree with the machinery.

Plans:
- [ ] TBD (run /gsd-plan-phase 343 to break down)

---

# Roadmap: v5.6 — Reasoning Where the Lane Needs It

**Milestone:** v5.6
**Status:** Phases 313–323 complete (25/25 requirements implemented and verified).
**Updated:** 2026-11-01

### Phase 313: Lane Prefix (`LANE-01`, `LANE-02`)

**Goal:** Codegen requests and codegen training rows use the non-thinking prefix, and hub and agent requests use the bare thinking-mode prefix.
**Depends on**: Nothing (first phase of v5.6)
**Requirements**: LANE-01, LANE-02
**Success Criteria** (what must be TRUE):

  1. A codegen serve prompt contains ` /no_think` and ends with the empty think block.
  2. A hub or agent serve prompt ends at a bare `assistant` turn and does not contain `/no_think`.
  3. A codegen direct-answer training row keeps the empty think block outside the loss, and a row that already has a non-empty think body is not rewritten with `/no_think`.

**Plans**: 4 plans

Plans:

**Wave 1**

- [x] `313-01`: Tracer: raw `/generate` carries `lane` through to the plugin payload, and an unknown lane is a 400 before any worker call (completed 2026-10-26)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] `313-02`: CUDA and Metal `assemble_prompt` wrap raw prompts by lane through one shared builder (completed 2026-10-26)
- [x] `313-03`: Chat completions accept `lane` and flatten the last user turn plus the assistant opener (completed 2026-10-26)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] `313-04`: Lock the existing codegen training fusion with tests only (completed 2026-10-26)

### Phase 314: Held-out Reasoning Set (`BENCH-02`, `BENCH-03`)

**Goal:** The same GSM8K, MATH-500, and Vox items exist for a before/after, and a missing model cannot write a passing score.
**Depends on**: Phase 313
**Requirements**: BENCH-02, BENCH-03
**Success Criteria** (what must be TRUE):

  1. A checked-in Vox slice has a checkable answer on every item and does not reuse the fixed Plan / Syntax / Implementation template.
  2. The GSM8K and MATH-500 slices used for the comparison are the checked-in files, not a new sample.
  3. A reasoning eval that cannot load the model writes no passing score and records how many items were scored.

**Plans**: 2 plans

Plans:

- [x] `314-01` (Wave 1): Check in the gsm8k_think, math500_think, and vox_think 32-item slices with a pinned loader and an interpreter oracle for Vox golds (completed 2026-10-26)
- [x] `314-02` (Wave 2, after 314-01): Fail closed when the model does not load or any inference error occurs; write `not_scored` with `n: 0`, and commit the scorer (completed 2026-10-26)

### Phase 315: Serve Gate (`GATE-01`, `GATE-02`)

**Goal:** Serve refuses an adapter whose collateral report is missing or worse on accuracy or think-initiation, and the report labels the benches as slices.
**Depends on**: Phase 314
**Requirements**: GATE-01, GATE-02
**Success Criteria** (what must be TRUE):

  1. Serve refuses to bind an adapter when the collateral report is missing.
  2. Serve refuses when accuracy or think-initiation on the reasoning keys drops past the configured threshold.
  3. The report states that the benches are slices and records the item count, so the 5% bar is not read as a full GSM8K or MATH-500 result.

**Plans**: 2 plans

Plans:

- [x] `315-01` (Wave 1): Strict per-slice evaluator in vox-eval; eval-collateral-damage requires all six reasoning keys, removes a stale report, and labels the slices with n (completed 2026-10-26)
- [x] `315-02` (Wave 2, after 315-01): Serve re-validates the on-disk report and the per-slice reasoning reports and refuses a missing, stale, edited, or unscored one (completed 2026-10-26)

### Phase 316: Before/After Comparison (`BENCH-01`)

**Goal:** A pinned Qwen3 base of any ladder size and its adapter are scored on the same slices when the weights exist, and no score is invented when they do not.
**Depends on**: Phase 315
**Requirements**: BENCH-01
**Success Criteria** (what must be TRUE):

  1. When the requested pinned Qwen3 snapshot (default `Qwen/Qwen3-8B`) and an adapter trained on it are on disk, one report holds accuracy and think-initiation for the GSM8K slice, the MATH-500 slice, and the Vox slice on both sides.
  2. When the snapshot is absent, the run records that fact and does not write numeric scores.
  3. Both sides use the checked-in items.

**Plans**: 1/1 plans executed

Plans:

- [x] 316-01-PLAN.md
- [x] `316-01`: Run the before/after on the checked-in slices, or record that the snapshot is absent (completed 2026-10-28)

### Phase 317: Size Ladder (`SIZE-01`)

**Goal:** Every Qwen3 dense size is a pinned, licensed rung that the trainer and the harness can name.
**Depends on**: Phase 316
**Requirements**: SIZE-01
**Success Criteria** (what must be TRUE):

  1. The ladder has Qwen3-0.6B, 1.7B, 4B, 8B, 14B, and 32B, each pinned to a real Hugging Face commit and carrying a license.
  2. A rung whose memory floor has not been measured on hardware says so.

**Plans:** 1/1 plans executed

Plans:

- [x] 317-01-PLAN.md — Pin 1.7B and 4B on the qwen3_code ladder and name every dense size from the overlay

### Phase 318: Live VRAM Selector (`SIZE-02`)

**Goal:** One selector reads current free memory, titrates to the largest Qwen3 size that fits, and picks the hub and its spokes together. Training, local serve, and chat all use it.
**Depends on**: Phase 317
**Requirements**: SIZE-02
**Success Criteria** (what must be TRUE):

  1. The selector reads current free memory (Metal live budget, CUDA free VRAM), not total memory, and picks the largest ladder size whose predicted footprint fits.
  2. If free memory drops between the pick and the load, it steps down a size and says why, instead of loading and running out of memory.
  3. Every spoke's base and preset follow the picked size. An explicit size overrides the pick, and a size that does not fit is refused rather than silently shrunk.
  4. The inference auto-selector and the training base pick both use this selector. The old fixed tiers that point at run directories that do not exist are gone.

**Plans:** 3/3 plans executed

Plans:

- [x] 318-01-PLAN.md — Measured QLoRA picker, pre-load step-down, and free-memory adapter (completed 2026-10-29)
- [x] 318-02-PLAN.md — One size for every spoke, and a re-read immediately before training load (completed 2026-10-29)
- [x] 318-03-PLAN.md — Chat and the GUI host call the selector; directory tiers are removed (completed 2026-10-29)

### Phase 319: Mesh-Wide Selection (`MESH-01`)

**Goal:** Across a mesh, the selector picks the largest size that fits on any node's current free memory and routes the work there.
**Depends on**: Phase 318
**Requirements**: MESH-01
**Success Criteria** (what must be TRUE):

  1. Node heartbeats carry current free accelerator memory, not only total VRAM.
  2. The mesh pick chooses the largest size that fits on some node, routes the job to that node, and titrates down when no node fits the larger size.
  3. A stale or missing node reading is not treated as free memory, and the local fallback still works with no mesh.

**Plans:** 3/3 plans executed

Plans:

- [x] 319-01-PLAN.md — Largest measured QLoRA floor, routed to a node that holds it (completed 2026-10-29)
- [x] 319-02-PLAN.md — Stale window, explicit-size refusal, and heartbeat free memory (completed 2026-10-29)
- [x] 319-03-PLAN.md — Local fallback when no node has a fresh reading (completed 2026-10-29)

### Phase 320: Train the Sizes (`SIZE-03`)

**Goal:** Each Qwen3 size that fits this Mac is trained with the lane prefix, and each size that does not is recorded as not trained.
**Depends on**: Phase 319
**Requirements**: SIZE-03
**Success Criteria** (what must be TRUE):

  1. Each size that fits produces a Metal QLoRA adapter whose manifest names its pinned base.
  2. A size that does not fit has a record with the measured memory and no adapter.

**Plans:** 2/2 plans executed

Plans:

- [x] 320-01-PLAN.md — One size_train.v1 record per qwen3_code QLoRA size, from an injected free-memory reading (completed 2026-11-01)
- [x] 320-02-PLAN.md — Register vox mens size-train and take that reading once (completed 2026-11-01)

### Phase 321: Harness Before/After (`HARN-01`, `BENCH-04`)

**Goal:** The harness serves a chosen size, and one report shows before and after reasoning and intelligence scores for every trained size.
**Depends on**: Phase 320
**Requirements**: HARN-01, BENCH-04
**Success Criteria** (what must be TRUE):

  1. The harness serves a chosen size's base or adapter through the existing serve path, behind the collateral gate.
  2. For every trained size, the report holds base and adapter reasoning accuracy and think-initiation on the three slices, plus an intelligence score on a codegen or agent bench, with n recorded.
  3. At least two sizes have real trained scores. Sizes without them are listed as not trained, with no numbers.

**Plans**: 1/1 plans executed

Plans:

- [x] `321-01`: Chosen-size serve resolution behind collateral gate (`HARN-01`) and multi-size `harness_before_after.v1` reasoning + intelligence report (`BENCH-04`) (completed 2026-11-01)

### Progress

| Phase | Name | Plans | Status |
|---|---|---|---|
| **Phase 313** | Lane Prefix | `4/4` | Complete |
| **Phase 314** | Held-out Reasoning Set | `2/2` | Complete |
| **Phase 315** | Serve Gate | `2/2` | Complete |
| **Phase 316** | Before/After Comparison | `1/1` | Complete    2026-10-28 |
| **Phase 317** | Size Ladder | `1/1` | Complete    2026-10-28 |
| **Phase 318** | Live VRAM Selector | `3/3` | Complete    2026-10-29 |
| **Phase 319** | Mesh-Wide Selection | `3/3` | Complete    2026-10-29 |
| **Phase 320** | Train the Sizes | `2/2` | Complete    2026-11-01 |
| **Phase 321** | Harness Before/After | `1/1` | Complete    2026-11-01 |
| **Phase 322** | Calibrated turn decisions + ChatARC | `1/1` | Complete    2026-11-01 |
| **Phase 323** | High-Value Starlight MDX Conversions + Ingest Parity | `2/2` | Complete    2026-11-01 |

### Phase 322: Calibrated turn decisions and dynamic action catalog (`DEC-01..04`, `CAT-01..02`)

**Directory:** `phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/`
**Goal:** Fully plumb System One / RLCD-style calibrated decisions into the live model-route and agent-loop control points (no facade). ChatARC composes the catalogs the harness already discovers (skills, MCP, clap commands) and chooses among shortlists. Complements `models::select`, clutch/risk axes, SIZE/MESH selectors, and MENS spokes — does not replace them or pretend `allow_free_in_performance_mode` is PreferFree.
**Depends on**: Nothing in 317–321 (parallel). Must not fork SIZE-02 / MESH-01. Primary insertion is `llm_bridge/model_route_policy/resolve.rs`, plus repair escalation and stop/post-tool hooks.
**Requirements**: DEC-01, DEC-02, DEC-03, DEC-04, CAT-01, CAT-02
**Success Criteria** (what must be TRUE):

  1. Turn-start pack feeds `build_selection_request` without overriding clutch-derived axes or `allow_free` when a clutch profile is set; flag off matches today's request bytes-for-bytes on fixtures.
  2. `allow_free_in_performance_mode` is documented and tested as scorer eligibility only — `free_floor` slug order stays in `bootstrap_inference` / `FREE_FLOOR`.
  3. Post-tool low confidence reuses `apply_repair_escalation_to_config`; stop/verify stay on `evaluate_stop_hook` / `evaluate_post_tool_batch_hook`.
  4. ChatARC shortlist Choice is plumbed into an existing activation path and stays correct when a new skill, MCP tool, or clap command appears; selective-prediction metrics published; hosted Jev behind local-only privacy.

**Plans**: 1/1 plans executed

**Research / prior art:** [`docs/src/architecture/jev-system-one-harness-integration-research-2026.md`](../docs/src/architecture/jev-system-one-harness-integration-research-2026.md) (§§11–16); cursor plan `labeled_router_plus_jev` folded here (do not execute as a standalone facade).

Plans:

- [x] `322-01`: Pre-turn `TurnDecisionPack` (`DEC-01`), post-tool confidence escalation (`DEC-02`), flag-off byte-identity tests (`DEC-03`), selective-prediction & privacy gate (`DEC-04`), and ChatARC top-K shortlist + calibrated Choice (`CAT-01..02`) (completed 2026-11-01)

### Phase 323: High-Value Starlight MDX Conversions and Ingest Parity (`MDX-INGEST-01..03`, `MDX-CONV-01..03`)

**Directory:** `phases/323-high-value-starlight-mdx-conversions-and-ingest-parity/`
**Goal:** Make `.mdx` a first-class, zero-regression citizen across `vox-kb-facets`, `vox-search` (Library index, `kb_ids`, RAG `ingest_markdown_tree`, `chunk_document`), `vox-doc-pipeline`, and MENS training extraction; then convert high-value tutorials, onboarding guides, and flagship language reference pages to Starlight MDX (`<Steps>`, `<Tabs>`, `<CardGrid>`, `<VoxPlayground>`) while keeping prose-heavy architecture/ADR/research docs in `.md`.
**Depends on:** Nothing (parallel documentation and search track)
**Requirements:** `MDX-INGEST-01`, `MDX-INGEST-02`, `MDX-INGEST-03`, `MDX-CONV-01`, `MDX-CONV-02`, `MDX-CONV-03`
**Success Criteria** (what must be TRUE):

  1. `vox-kb-facets` (`inspect_summary`, `is_summary_marker_line`, `strip_summary_blocks`) accepts both HTML (`<!-- kb:summary -->`) and MDX JSX (`{/* kb:summary */}`) markers and ignores top-level ESM `import` lines between `# H1` and the summary block.
  2. `vox-search::kb_ids::is_kb_corpus_path`, `doc_title`, and `ingest::ingest_markdown_tree` index both `.md` and `.mdx` under `docs/src/`; `library::chunk::first_paragraph` and MENS corpus extraction strip ESM `import`/`export` statements and JSX layout wrapper tags (`<Tabs>`, `<TabItem>`, `<Steps>`, `<CardGrid>`, `{/* ... */}`) so Tantivy cards and training chunks never leak JSX boilerplate.
  3. `vox-doc-pipeline` validates `.mdx` files (frontmatter, facets, `vox` doctest fences, internal `.md`/`.mdx` links) and upgrades `docs/src/index.mdx` to clean Starlight components (`<CardGrid>`, `<LinkButton>`) without inline-style HTML soup.
  4. High-value tutorials (`docs/src/tutorials/**`) and flagship reference/getting-started pages are converted to `.mdx` using Starlight `<Steps>`, `<Tabs>` (showing Vox source alongside generated SQL/Rust/TSX), and embedded `<VoxPlayground>`, passing `pnpm --dir docs-astro build`, `vox-doc-pipeline --lint-only`, and `cargo test -p vox-search -p vox-kb-facets -p vox-doc-pipeline`.

**Plans:** 2/2 plans executed

Plans:

- [x] `323-01`: Dual HTML/MDX summary markers and L0 MDX boilerplate stripping across `vox-kb-facets`, `vox-search`, `vox-doc-pipeline`, and `vox-corpus` (`MDX-INGEST-01..03`) (completed 2026-11-01)
- [x] `323-02`: Upgrade `docs/src/index.mdx`, add `.md` ↔ `.mdx` link fallback, and ship interactive Starlight MDX tutorial `tut-fullstack-walkthrough.mdx` (`MDX-CONV-01..03`) (completed 2026-11-01)

---

# Roadmap: v5.5 — T3 CI/TOESTUB Deep Enforcement (`Phases 307–312`) — SHIPPED

**Milestone:** v5.5
**Status:** Planning complete — ready for `/gsd-plan-phase 307`
**Updated:** 2026-10-26T01:59:06Z

### Phase 307: PR Vouch & Contributor Trust Gate (`T3-TRUST-01..04`)

**Directory:** `phases/307-pr-vouch-and-contributor-trust/`
**Goal:** Add `.github/VOUCHED.td` (or `contracts/ci/vouched.v1.yaml`) SSOT for trusted/denounced GitHub logins, documented in AGE…

- [x] `307-01`: Implement T3-TRUST requirements

### Phase 308: Preview/Privileged Workflow Trust-Split Policy (`T3-PREVIEW-01..04`)

**Directory:** `phases/308-preview-trust-split-policy/`
**Goal:** Author `docs/src/ci/privileged-preview-trust-split.md` capturing t3code relay pattern: deploy may run PR code only under…

- [x] `308-01`: Implement T3-PREVIEW requirements

### Phase 309: Live Transfer-Budget Measurement & Job-Matrix Change Detector (`T3-LIVE-CI-01..04`)

**Directory:** `phases/309-live-transfer-budget-and-job-matrix/`
**Goal:** Replace fixture-only `transfer-budget-report` CI path with artifact emission from real `vox-orchestrator-mcp` transfer-b…

- [x] `309-01`: Implement T3-LIVE-CI requirements

### Phase 310: TOESTUB Test-Discipline Detectors (Wall-Clock, Real-Wait, Loop, Frame) (`T3-TEST-DET-01..04`)

**Directory:** `phases/310-test-discipline-detectors/`
**Goal:** Extend `test_clock_and_skip` (or new detector) to flag wall-clock reads in tests: `Instant::now`, `SystemTime::now`, `Da…

- [x] `310-01`: Implement T3-TEST-DET requirements

### Phase 311: Host-Runtime Boundary, Schema Hoist, Knip Dual & DOM-Env Discipline (`T3-HOST-SCHEMA-01..04`)

**Directory:** `phases/311-host-runtime-schema-knip-dom/`
**Goal:** New detector banning direct host platform/arch probes outside allowlisted host crates (`std::env::consts::{OS,ARCH}`, `p…

- [x] `311-01`: Implement T3-HOST-SCHEMA requirements

### Phase 312: Sidecar Fingerprint Hash Compare, Surface Graph Regen & v5.5 Scorecard (`T3-FP-GRAPH-01..04`)

**Directory:** `phases/312-fingerprint-hash-graph-scorecard/`
**Goal:** Upgrade `gui-sidecar-fingerprint.yml` from path-diff labeling to content hash of native/sidecar inputs (Cargo.lock subse…

- [x] `312-01`: Implement T3-FP-GRAPH requirements

### Progress

| Phase | Name | Plans | Status |
|---|---|---|---|
| **Phase 307** | PR Vouch & Contributor Trust Gate | `1/1` | Complete |
| **Phase 308** | Preview/Privileged Workflow Trust-Split Policy | `1/1` | Complete |
| **Phase 309** | Live Transfer-Budget Measurement & Job-Matrix Change Detector | `1/1` | Complete (Partials) |
| **Phase 310** | TOESTUB Test-Discipline Detectors (Wall-Clock, Real-Wait, Loop, Frame) | `1/1` | Complete |
| **Phase 311** | Host-Runtime Boundary, Schema Hoist, Knip Dual & DOM-Env Discipline | `1/1` | Complete (Partials) |
| **Phase 312** | Sidecar Fingerprint Hash Compare, Surface Graph Regen & v5.5 Scorecard | `1/1` | Complete (Partials) |

---

## Milestones

- ✅ **v5.3 (`Phases 295–300`, `24/24` requirements — Complete 2026-10-25; CV residual):** Photorealistic Edge-Softening, Continuous Monumental Aqueduct-to-Sewer Architecture, Sub-Surface Ground Integration & CV Flywheel Score Maximization — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-photorealistic-edge-softening-and-cv-maximization-spec-2026.md)
- ✅ **v5.2 (`Phases 289–294`, `24/24` requirements — Complete 2026-10-25):** Smooth Physically-Grounded Lighting, High-Detail Vitruvian Architecture & Grounds, Integrated Roman Hydraulic/Sewer Grid (`Aquaeductus`, `Castellum Divisorium`, `Fistulae` & `Cloaca Maxima`) & Animated Shape+Color Activity Conduits — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-hydraulic-plumbing-and-detailed-architecture-spec-2026.md)
- ✅ **v5.1 (`Phases 283–288`, `24/24` requirements — Complete 2026-10-25):** T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure: System Prompt `$variable` & Worktree Branch Fix, Full `context_sanitizer` & `ToolRefusalDetector`, Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` & `ModelSpec` Cost Accounting, Async Worktree Lease & Monotonic Sparse Checkpoints, Real `MultiVariantWorktreePool`, Multi-Provider Consensus Synthesizer, Depth-4 External Session Discovery, Unified Tauri/MCP `message_id` & `oxlint-plugin-t3code` Zero-Violation Audit Parity — [Requirements](REQUIREMENTS.md) | [Architecture Audit](../docs/src/architecture/t3-vox-wave5-deep-review-and-parity-closure-2026.md)
- ✅ **v5.0 (`Phases 277–282`, `24/24` requirements — Complete 2026-10-25):** Photorealistic Vitruvian Material, Depth-Shading, Atmospheric Lighting & FX Engine (`pbrMaterials.ts`, Projected Cast Shadows, Vertical AO, Specular Chamfer Bevels, Bathymetric Sea + Surf Foam, Crepuscular God-Rays, Additive GI Pools, Particle FX & Diegetic Cliff-Apron Plaques) — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-photorealistic-lighting-and-materials-spec-2026.md)
- ✅ **v4.9 (`Phases 271–276`, `24/24` requirements — Complete 2026-10-25):** Vitruvian Roman Tactical Developer Controller: Zero-Hop Diff/Diagnostic Loupe (*Speculum Codicis*), Multi-Agent Write-Set Collision Cones (*Concursus Legionum*), Clickable Viaduct Contract Inspector (*Pons Foederis*), Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*), Milestone Road Turn Time-Scrubber (*Via Temporis*) & Split-Dock Workbench Integration (*Tabula Praetoria `⌘M`*) — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-tactical-developer-controller-spec-2026.md)
- ✅ **v4.8 (`Phases 265–270`, `24/24` requirements — Complete 2026-10-25):** T3 Wave 4 Live Pipeline End-to-End Closure: Real LLM Consensus, Stateful Session/Guest/Checkpoint Persistence, Live Agent-Loop Guards & Tauri/MCP Envelope Unwrapping — [Requirements](REQUIREMENTS.md) | [Architecture Audit](../docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md)
- ✅ **v4.7 (`Phases 259–264`, `24/24` requirements — Shipped 2026-10-25):** Universal Repo-Agnostic Codebase Civilization Simulator & Attention-Weighted Developer Control Surface — [Archive](milestones/v4.7-ROADMAP.md) | [Requirements](milestones/v4.7-REQUIREMENTS.md) | [Audit](v4.7-MILESTONE-AUDIT.md)
- ✅ **v4.6 (`Phases 253–258`, `24/24` requirements — Shipped 2026-10-25):** T3 Wave 3 Live Surface & Daemon RPC Wiring, Router/DPO Quality-Prior Flywheel & Workspace-Wide TOESTUB Remediation — [Archive](milestones/v4.6-ROADMAP.md) | [Requirements](milestones/v4.6-REQUIREMENTS.md) | [Audit](v4.6-MILESTONE-AUDIT.md)
- ✅ **v4.5 (`Phases 242–252`, `44/44` requirements — Shipped 2026-10-25):** T3 Ecosystem Wave 2 Advancements, Consensus Engine, Git Plumbing Checkpoints & TOESTUB Static Enforcement — [Archive](milestones/v4.5-ROADMAP.md) | [Requirements](milestones/v4.5-REQUIREMENTS.md)
- ✅ **v4.4 (`Phases 236–241`, `24/24` requirements — Shipped 2026-10-25):** Concept-Art Pixel & Structural Convergence, Live Animated Roman Civilization Simulator (`VRBS ROMA`), Interactive DAG/Diff Workbench & Before/After Visual Proof — [Archive](milestones/v4.4-ROADMAP.md) | [Requirements](milestones/v4.4-REQUIREMENTS.md)
- ✅ **v4.3 (`Phases 230–235`, `24/24` requirements — Shipped 2026-10-25):** Roman Civilization Codebase Simulator (`VRBS ROMA`), Strategic Minimap Radar & Cross-Surface Readability/Occlusion Polish — [Archive](milestones/v4.3-ROADMAP.md) | [Requirements](milestones/v4.3-REQUIREMENTS.md)

---

## Phases (`v4.8`)

### Phase 265: `tool:<name>` Fence Salvage Fix, Live `@rule:<name>` Mention Resolution & `$variable` System Prompt Interpolation (`T3-SALVAGE-RULES-01..04`)

**Goal:** Eliminate the `tool:<name>` stream-suppression vs. salvage mismatch in `salvage_and_stream.rs`, wire `resolve_user_rule_mentions` (`@rule:<name>` + `Always` rules) into `resolve_mentions` before file-path matching, and wire `interpolate_system_prompt_variables` into live turn preparation in `preamble_and_retrieval.rs`.
**Requirements:** `T3-SALVAGE-RULES-01`, `T3-SALVAGE-RULES-02`, `T3-SALVAGE-RULES-03`, `T3-SALVAGE-RULES-04`
**Plans:**

- [x] `265-01`: Fix `tool:<name>` fence salvage in `salvage_and_stream.rs`, wire `@rule:<name>` resolution in `mentions.rs`, wire `$variable` interpolation in `preamble_and_retrieval.rs`, and add integration tests (`phases/265-salvage-rules-and-prompt-vars/265-01-PLAN.md`)

### Phase 266: Live `context_sanitizer` History Stripping, `vox_chat_compact_up_to`, `BatteryCostLedger` Accounting, `VoxIdAllocatorV2` Message IDs & Real `ResponseQualityAnalyzer` Telemetry (`T3-HIST-BATTERY-01..04`)

**Goal:** Wire `sanitize_assistant_for_history` and `escape_prompt_injection_tokens` into `transcript_entry_to_llm_message` (`conversation.rs`), implement `vox_chat_compact_up_to` (`history.rs`), and wire live `BatteryCostLedger` (`check_preflight_balance` + `record_turn_usage`), `VoxIdAllocatorV2` deterministic message IDs, and real `ResponseQualityAnalyzer` scores into `message.rs` and `persist_and_envelope.rs`.
**Requirements:** `T3-HIST-BATTERY-01`, `T3-HIST-BATTERY-02`, `T3-HIST-BATTERY-03`, `T3-HIST-BATTERY-04`
**Plans:**

- [x] `266-01`: Wire `context_sanitizer` in `conversation.rs`, implement `vox_chat_compact_up_to` in `history.rs`, and wire `BatteryCostLedger`, `VoxIdAllocatorV2`, and `ResponseQualityAnalyzer` in `message.rs` & `persist_and_envelope.rs` (`phases/266-history-sanitizer-and-battery-telemetry/266-01-PLAN.md`)

### Phase 267: Live `agent_loop` Integration: `CommandOutputHub` `\r` Normalization, `20 KiB` DOM Shedding, Pre-Write Sparse Git Checkpoints, `ResumableTurnStreamRegistry` & Mid-Turn Steer Draining (`T3-LOOP-CHECKPOINT-01..04`)

**Goal:** Wire `normalize_terminal_output` (`\r` folding), `bound_snapshot_metadata` (`20 KiB` DOM shedding), automatic pre-write sparse Git checkpoints (`should_sparse_checkpoint`), `ResumableTurnStreamRegistry` (`vox_chat_stream_resume`), and mid-turn `ThreadRunQueue` (`vox_chat_queue_steer`) into `agent_loop.rs` and `salvage_and_stream.rs`.
**Requirements:** `T3-LOOP-CHECKPOINT-01`, `T3-LOOP-CHECKPOINT-02`, `T3-LOOP-CHECKPOINT-03`, `T3-LOOP-CHECKPOINT-04`
**Plans:**

- [x] `267-01`: Wire `\r` normalization, `20 KiB` DOM shedding, pre-write sparse Git checkpoints, `ResumableTurnStreamRegistry`, and mid-turn `ThreadRunQueue` steer draining into `agent_loop.rs` & `salvage_and_stream.rs` (`phases/267-agent-loop-checkpoints-and-resumable-streams/267-01-PLAN.md`)

### Phase 268: Stateful `t3_wave3_tools.rs`: Concurrent `join_all` LLM Consensus Fan-Out, Live Transcript Hydration/Persistence for `vox_context_handoff` & `vox_session_*` & Persistent `GuestShareService` (`T3-STATEFUL-TOOLS-01..04`)

**Goal:** Upgrade `t3_wave3_tools.rs` so `vox_consensus_run` dispatches concurrent `join_all` LLM calls across models, `vox_context_handoff` and `vox_session_*` hydrate and persist real transcripts in `ServerState` (`chat_history:{session_id}` + real `~/.claude`/`~/.codex`/`~/.cursor` files), and `vox_guest_share_*` uses a persistent `GuestShareService` on `ServerState`.
**Requirements:** `T3-STATEFUL-TOOLS-01`, `T3-STATEFUL-TOOLS-02`, `T3-STATEFUL-TOOLS-03`, `T3-STATEFUL-TOOLS-04`
**Plans:**

- [x] `268-01`: Upgrade `t3_wave3_tools.rs` to dispatch concurrent `join_all` LLM consensus legs, hydrate/persist live `ServerState` transcripts in `vox_context_handoff` & `vox_session_*`, and persist `GuestShareService` on `ServerState` (`phases/268-stateful-consensus-handoff-and-session-tools/268-01-PLAN.md`)

### Phase 269: `vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools, `DelegationWorktree` Lease/Env Wiring, `submit_webhook_events` Dedupe/Rate Gate & Compact `vox_orchestrator_status` (`T3-REGEN-WORKTREE-01..04`)

**Goal:** Implement `vox_chat_regenerate` and `vox_chat_edit_message_in_place` in `history.rs`, wire `WorkspaceLeaseMap` + `provision_untracked_env_files` in `DelegationWorktree::create`, wire `WebhookDedupeAndRateGate` in `submit_webhook_events`, and wire `compact` in `vox_orchestrator_status` + `transfer_budget.rs`.
**Requirements:** `T3-REGEN-WORKTREE-01`, `T3-REGEN-WORKTREE-02`, `T3-REGEN-WORKTREE-03`, `T3-REGEN-WORKTREE-04`
**Plans:**

- [x] `269-01`: Implement `vox_chat_regenerate` & `vox_chat_edit_message_in_place`, wire `WorkspaceLeaseMap` & `.env` provisioning in `agy_worktree.rs`, wire `WebhookDedupeAndRateGate` in `webhook_intake.rs`, and wire `compact` in `vox_orchestrator_status` & `transfer_budget.rs` (`phases/269-regenerate-worktree-lease-and-webhook-gate/269-01-PLAN.md`)

### Phase 270: `t3Wave3Rpc.ts` Nested Tauri Envelope Unwrapping, Live `ChatTranscript.tsx` In-Place Edit & Posture-Aware Regeneration & Full-Stack Verification (`T3-GUI-ENVELOPE-01..04`)

**Goal:** Fix `unwrapMcpData` in `t3Wave3Rpc.ts` to unwrap nested Tauri `{ is_error, result: { success, data } }` and stringified MCP envelopes, expose typed `editChatMessageInPlace` / `regenerateChatTurn` / `compactChatUpTo` / `resumeChatStream` / `steerChatQueue` client helpers, wire in-place edit and posture regeneration in `ChatTranscript.tsx`, and verify all Rust, Vitest, TypeScript, and audit gates.
**Requirements:** `T3-GUI-ENVELOPE-01`, `T3-GUI-ENVELOPE-02`, `T3-GUI-ENVELOPE-03`, `T3-GUI-ENVELOPE-04`
**Plans:**

- [x] `270-01`: Fix `unwrapMcpData` nested Tauri/MCP envelope unwrapping in `t3Wave3Rpc.ts`, wire in-place edit & regenerate in `ChatTranscript.tsx`, and run full-stack verification (`phases/270-tauri-envelope-unwrapping-and-transcript-actions/270-01-PLAN.md`)

---

## Progress Table (`v4.8`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 265** | `tool:<name>` Fence Salvage Fix, Live `@rule:<name>` Mention Resolution & `$variable` System Prompt Interpolation | `1/1` | Complete (`2026-10-25`) |
| **Phase 266** | Live `context_sanitizer` History Stripping, `vox_chat_compact_up_to`, `BatteryCostLedger` Accounting, `VoxIdAllocatorV2` Message IDs & Real `ResponseQualityAnalyzer` Telemetry | `1/1` | Complete (`2026-10-25`) |
| **Phase 267** | Live `agent_loop` Integration: `CommandOutputHub` `\r` Normalization, `20 KiB` DOM Shedding, Pre-Write Sparse Git Checkpoints, `ResumableTurnStreamRegistry` & Mid-Turn Steer Draining | `1/1` | Complete (`2026-10-25`) |
| **Phase 268** | Stateful `t3_wave3_tools.rs`: Concurrent `join_all` LLM Consensus Fan-Out, Live Transcript Hydration/Persistence for `vox_context_handoff` & `vox_session_*` & Persistent `GuestShareService` | `1/1` | Complete (`2026-10-25`) |
| **Phase 269** | `vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools, `DelegationWorktree` Lease/Env Wiring, `submit_webhook_events` Dedupe/Rate Gate & Compact `vox_orchestrator_status` | `1/1` | Complete (`2026-10-25`) |
| **Phase 270** | `t3Wave3Rpc.ts` Nested Tauri Envelope Unwrapping, Live `ChatTranscript.tsx` In-Place Edit & Posture-Aware Regeneration & Full-Stack Verification | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v4.9` — Vitruvian Roman Tactical Developer Controller)

### Phase 271: Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*) (`URBS-LOUPE-01..04`)

**Goal:** Eliminate context-switching between `VRBS` and external diff/terminal tabs by adding `workspace_file_diff_and_diagnostics` in `workspace_town.rs` and rendering `SpeculumLoupePopover.tsx` directly above any clicked or hovered Treadwheel Crane (`+LoC / -LoC`) or red error building (`N ERR`) with real unified Git diff hunks, `file:line:col` compiler diagnostics, and 1-click `[✓ Stage File]`, `[↺ Revert File]`, and `[⚒ Fix Exact Error]` actions.
**Requirements:** `URBS-LOUPE-01`, `URBS-LOUPE-02`, `URBS-LOUPE-03`, `URBS-LOUPE-04`
**Plans:**

- [x] `271-01`: Implement `workspace_file_diff_and_diagnostics` + `workspace_stage_file` / `workspace_revert_file` in `workspace_town.rs`, build `SpeculumLoupePopover.tsx`, wire into `LudusSandbox.tsx`, and add Rust/Vitest tests (`phases/271-inline-diff-and-diagnostic-loupe/271-01-PLAN.md`)

### Phase 272: Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*) (`URBS-COLLISION-01..04`)

**Goal:** Prevent concurrent multi-agent merge conflicts and semantic clobbering by computing each active agent's write-set + 1-hop blast-radius cone (`collisionRadar.ts`), rendering intersecting Roman surveyor groma cones and a 3D **Hammered-Bronze Crossed-Swords Collision Standard (`⚠ WRITE-SET COLLISION`)** in `worldRenderer.ts`, and wiring 1-click `[⇄ Serialize Agents]` and `[🌿 Split Worktree]` actions.
**Requirements:** `URBS-COLLISION-01`, `URBS-COLLISION-02`, `URBS-COLLISION-03`, `URBS-COLLISION-04`
**Plans:**

- [x] `272-01`: Build `collisionRadar.ts`, render surveyor groma cones & Crossed-Swords Collision Standard in `worldRenderer.ts`, wire `[⇄ Serialize Agents]` & `[🌿 Split Worktree]` in `LudusSandbox.tsx` / `ImperiumInspector.tsx`, and add Vitest tests (`phases/272-multi-agent-collision-cone-radar/272-01-PLAN.md`)

### Phase 273: Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*) (`URBS-VIADUCT-01..04`)

**Goal:** Turn stone viaducts between districts into clickable first-class architectural contract boundaries by scanning cross-module dependencies and `layers.toml` layer assignments in `workspace_town.rs` (`TownDependencyEdge`), adding viaduct hit-testing in `layout.ts`, rendering a **Cracked Red Arch Keystone (`⚠ LAYER VIOLATION`)** in `worldRenderer.ts` on illegal upward dependencies, and adding the `PONS CONTRACTVS` inspector panel in `ImperiumInspector.tsx`.
**Requirements:** `URBS-VIADUCT-01`, `URBS-VIADUCT-02`, `URBS-VIADUCT-03`, `URBS-VIADUCT-04`
**Plans:**

- [x] `273-01`: Add `TownDependencyEdge` + `layers.toml` violation scanner in `workspace_town.rs`, viaduct hit-testing in `layout.ts`, cracked keystone rendering in `worldRenderer.ts`, and `PONS CONTRACTVS` inspector panel in `ImperiumInspector.tsx` (`phases/273-clickable-viaduct-contract-inspector/273-01-PLAN.md`)

### Phase 274: Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) (`URBS-QUARANTINE-01..04`)

**Goal:** Give developers a physical spatial guardrail that prevents agents from modifying off-limits modules by storing `quarantined_paths` on `ServerState`, exposing `vox_quarantine_set` / `vox_quarantine_list` MCP & Tauri RPCs, enforcing pre-execution write rejection in `agent_loop.rs`, and rendering a 3D **Wrought-Iron Perimeter Portcullis & Bronze Padlock (`🔒 QUARANTINED: READ-ONLY`)** toggled via `Alt+Click` on any district.
**Requirements:** `URBS-QUARANTINE-01`, `URBS-QUARANTINE-02`, `URBS-QUARANTINE-03`, `URBS-QUARANTINE-04`
**Plans:**

- [x] `274-01`: Add `quarantined_paths` to `ServerState`, expose `vox_quarantine_set` / `vox_quarantine_list`, enforce write rejection in `agent_loop.rs`, and render 3D perimeter portcullis fencing in `worldRenderer.ts` (`phases/274-hard-agent-quarantine-fencing/274-01-PLAN.md`)

### Phase 275: Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`) (`URBS-SCRUBBER-01..04`)

**Goal:** Turn the Stone Milestone Road (`I, II, III, IV`) at `PORTA GIT` into an interactive visual `git bisect` timeline bar (`ViaTemporisScrubber.tsx`) with a draggable Bronze Sundial Gnomon Scrubber that previews per-checkpoint building diffs (`+LoC / -LoC`), test shield state flips (`PASS` → `FAIL`), and token/BU spend, with 1-click `[↺ Rollback Worktree to Milestone]` wired to `vcsCheckpointRestore`.
**Requirements:** `URBS-SCRUBBER-01`, `URBS-SCRUBBER-02`, `URBS-SCRUBBER-03`, `URBS-SCRUBBER-04`
**Plans:**

- [x] `275-01`: Add `vox_vcs_checkpoint_diff_summary` RPC, build `ViaTemporisScrubber.tsx` sundial timeline bar, wire historical building diff preview & 1-click rollback in `LudusSandbox.tsx`, and add Vitest tests (`phases/275-milestone-road-time-scrubber/275-01-PLAN.md`)

### Phase 276: Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*) (`URBS-DOCK-01..04`)

**Goal:** Promote `VRBS` from `Settings → Gamify` to a 1-click split-dock Tactical Map (`[🏛 Tactical Map ⌘M]`) inside `DockWorkspaceShell.tsx` and `ChatSurface.tsx`, wiring bi-directional district context filtering between the map and Chat/Review plus real-time tool-call mechanism animations (`read_file` → Screw Pump, `write_file` → Treadwheel Crane, `test` → Shield Wall, approval → Portcullis Gate).
**Requirements:** `URBS-DOCK-01`, `URBS-DOCK-02`, `URBS-DOCK-03`, `URBS-DOCK-04`
**Plans:**

- [x] `276-01`: Wire `[🏛 Tactical Map ⌘M]` split-dock panel in `DockWorkspaceShell.tsx` & `ChatSurface.tsx`, bi-directional district filter in `useAppStore.ts`, live tool-call mechanism pulses in `LudusSandbox.tsx`, and Playwright CV Flywheel verification (`phases/276-split-dock-tactical-map-sync/276-01-PLAN.md`)

---

## Progress Table (`v4.9`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 271** | Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 272** | Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 273** | Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 274** | Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 275** | Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`) | `1/1` | Complete (`2026-10-25`) |
| **Phase 276** | Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*) | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v5.0` — Photorealistic Vitruvian Material, Depth-Shading, Atmospheric Lighting & FX Engine)

### Phase 277: Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`) (`URBS-MAT-01..04`)

**Goal:** Eliminate flat solid-color polygon fills by creating a deterministic offscreen procedural material synthesis atlas (`pbrMaterials.ts`) producing tileable `CanvasPattern`s and metallic gradients for Pitted Roman Travertine Ashlar, Scalloped Terracotta Roof Tiles, Opus Incertum Flagstone Plazas, Stratified Basalt Cliffs, and Anisotropic Hammered Bronze.
**Requirements:** `URBS-MAT-01`, `URBS-MAT-02`, `URBS-MAT-03`, `URBS-MAT-04`
**Plans:**

- [ ] `277-01`: Create `pbrMaterials.ts` procedural natural surface material atlas + unit tests (`phases/277-procedural-natural-materials/277-01-PLAN.md`)

### Phase 278: True Isometric Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels (`URBS-DEPTH-01..04`)

**Goal:** Ground every 3D structure with physical depth by adding projected directional twilight cast shadows (`drawIsoCastShadow`), vertical height-shaded face gradients (`contact AO` at base → `golden-hour rim light` at cornice) overlaid with travertine ashlar texture, and physical specular chamfer ridge highlights replacing flat black cartoon outlines.
**Requirements:** `URBS-DEPTH-01`, `URBS-DEPTH-02`, `URBS-DEPTH-03`, `URBS-DEPTH-04`
**Plans:**

- [ ] `278-01`: Implement projected directional cast shadows, vertical AO/rim gradients, and specular chamfer bevels in `worldRenderer.ts` (`phases/278-depth-shading-and-cast-shadows/278-01-PLAN.md`)

### Phase 279: Sculpted Coastal Bathymetry, Surf Foam Ribbons, Craggy Cliff Geology & Cobblestone Terraces (`URBS-TERRAIN-01..04`)

**Goal:** Transform the flat background and terraces into a natural Mediterranean peninsula with multi-stop bathymetric sea depth gradients, coastal surf foam ribbons, stratified basalt cliff geology with vertical fissures and moss ledges, and cobblestone flagstone plazas.
**Requirements:** `URBS-TERRAIN-01`, `URBS-TERRAIN-02`, `URBS-TERRAIN-03`, `URBS-TERRAIN-04`
**Plans:**

- [ ] `279-01`: Upgrade `paintFullBleedRomanLandscape` with bathymetric sea depth, coastal surf foam, stratified cliff textures, and cobblestone terrace paving (`phases/279-bathymetry-cliffs-and-terrace-geology/279-01-PLAN.md`)

### Phase 280: Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric God-Rays, Elevation Mist & Additive GI Pools (`lightingAndFx.ts`) (`URBS-LIGHT-01..04`)

**Goal:** Deliver cinematic atmospheric lighting by creating `lightingAndFx.ts` with inter-terrace atmospheric elevation mist, diagonal golden-hour crepuscular god-rays, additive point-light GI bounce pools around braziers/forges/windows/aqueducts, and a cinematic vignette & warm highlight bloom pass.
**Requirements:** `URBS-LIGHT-01`, `URBS-LIGHT-02`, `URBS-LIGHT-03`, `URBS-LIGHT-04`
**Plans:**

- [ ] `280-01`: Create `lightingAndFx.ts` atmospheric mist, volumetric god-rays, additive GI light pools, and cinematic post-processing (`phases/280-atmospheric-lighting-and-god-rays/280-01-PLAN.md`)

### Phase 281: Live Particle FX Engine (Forge Embers, Volumetric Smoke Plumes, Waterfall Spray & Mason Chisel Sparks) (`URBS-FX-01..04`)

**Goal:** Bring the 3D mechanisms to life with deterministic real-time particle emitters in `lightingAndFx.ts` and `drawAnimatedWorldEffects`: rising glowing forge/error embers, drifting volumetric smoke plumes, waterfall mist spray, harbor water caustics, and mason chisel sparks on active Treadwheel Cranes.
**Requirements:** `URBS-FX-01`, `URBS-FX-02`, `URBS-FX-03`, `URBS-FX-04`
**Plans:**

- [ ] `281-01`: Implement live forge ember, smoke plume, waterfall spray, water caustic, and mason chisel spark particle emitters (`phases/281-live-particle-fx-engine/281-01-PLAN.md`)

### Phase 282: Diegetic Cliff-Apron Architrave Plaques, Architecture-First HUD Decluttering & Playwright CV Flywheel Convergence (`URBS-DIEGETIC-01..04`)

**Goal:** Uncover 85%+ of the 3D architectural diorama by mounting district plaques as chiseled bronze/travertine inscriptions on the front cliff retaining wall apron, progressive-disclosing secondary file slabs on hover/selection, upgrading `sprites.ts` with photorealistic gradients/shadows, and verifying via the Playwright CV Flywheel.
**Requirements:** `URBS-DIEGETIC-01`, `URBS-DIEGETIC-02`, `URBS-DIEGETIC-03`, `URBS-DIEGETIC-04`
**Plans:**

- [ ] `282-01`: Implement diegetic cliff-apron architrave plaques, HUD progressive disclosure, photorealistic sprite upgrades, and Playwright CV Flywheel verification (`phases/282-diegetic-plaques-and-cv-convergence/282-01-PLAN.md`)

---

## Progress Table (`v5.0`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 277** | Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`) | `1/1` | Complete (`2026-10-25`) |
| **Phase 278** | True Isometric Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels | `1/1` | Complete (`2026-10-25`) |
| **Phase 279** | Sculpted Coastal Bathymetry, Surf Foam Ribbons, Craggy Cliff Geology & Cobblestone Terraces | `1/1` | Complete (`2026-10-25`) |
| **Phase 280** | Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric God-Rays, Elevation Mist & Additive GI Pools | `1/1` | Complete (`2026-10-25`) |
| **Phase 281** | Live Particle FX Engine (Forge Embers, Volumetric Smoke Plumes, Waterfall Spray & Mason Chisel Sparks) | `1/1` | Complete (`2026-10-25`) |
| **Phase 282** | Diegetic Cliff-Apron Architrave Plaques, Architecture-First HUD Decluttering & Playwright CV Flywheel Convergence | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v5.1` — T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure)

### Phase 283: System Prompt `$variable` Target Fix, Sanitizer Tag Completeness, `ToolRefusalDetector` & Lazy MCP Meta-Tools (`T3-PROMPT-LOOP-01..04`)

**Goal:** Apply `interpolate_system_prompt_variables` to `system_prompt` with full ISO-8601 UTC `$current_datetime` and worktree `.git` file `gitdir:` branch resolution, expand `escape_user_prompt_special_tokens` (`<tool_call>`, `<tool_response>`, `<think>`, active-turn user prompt) + `compact_historical_web_search` (`<WebFetch>`) + `thought_signature` extraction, wire `pi-t3chat` `ToolRefusalDetector` (`[SYSTEM TOOL CORRECTION]`) and `NotT3Chat` search circuit breaker (`total_searches >= 3`) in `agent_loop.rs`, and wire `LazyMcpGroupRegistry` (`vox_list_mcp_servers`, `vox_list_mcp_tools`, `vox_call_mcp_lazy`) + `sanitize_mcp_tool_search_env`.
**Requirements:** `T3-PROMPT-LOOP-01`, `T3-PROMPT-LOOP-02`, `T3-PROMPT-LOOP-03`, `T3-PROMPT-LOOP-04`
**Plans:**

- [ ] `283-01`: Fix `system_prompt` `$variable` interpolation & worktree branch detection, expand `context_sanitizer` & `thought_signature` persistence, wire `ToolRefusalDetector` & search circuit breaker in `agent_loop.rs`, and wire `LazyMcpGroupRegistry` (`phases/283-prompt-vars-sanitizer-and-loop-guards/283-01-PLAN.md`)

### Phase 284: Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` Lifecycle, Real `ModelSpec` Cost Ledger & `SessionPrivacyPosture` Gate (`T3-STREAM-QUEUE-01..04`)

**Goal:** Replace `ServerState::resumable_streams` tuple map with `ResumableTurnStreamRegistry` (`SequencedTurnChunk`, multi-iteration accumulation, `vox_chat_stream_resume` + `vox_chat_stream_cancel`), wire `ThreadRunQueue` (`pop_next_follow_up`, background work settlement, 429 `LimitRecoveryState`), wire post-resolution preflight check + `ModelRegistry::compute_turn_cost_delta` + 4-hour/monthly quota windows + `vox_battery_status`, and enforce `SessionPrivacyPosture::Ephemeral` disk-persistence gating.
**Requirements:** `T3-STREAM-QUEUE-01`, `T3-STREAM-QUEUE-02`, `T3-STREAM-QUEUE-03`, `T3-STREAM-QUEUE-04`
**Plans:**

- [ ] `284-01`: Wire `ResumableTurnStreamRegistry`, full `ThreadRunQueue` follow-up/settlement/recovery, `ModelRegistry::compute_turn_cost_delta` + `vox_battery_status`, and `SessionPrivacyPosture` persistence gate (`phases/284-resumable-stream-queue-and-cost-ledger/284-01-PLAN.md`)

### Phase 285: Worktree Async Lease Mutex Fix, Persistent Webhook Gate, Monotonic Sparse Git Checkpoints, Real `MultiVariantWorktreePool` & Compact Snapshot Fix (`T3-WORKTREE-CKPT-01..04`)

**Goal:** Hold a `tokio::sync::MutexGuard` across `.await` in `DelegationWorktree::create` and persist `WebhookDedupeAndRateGate` on `ServerState`, fix monotonic per-session sparse checkpoint turn counters + Windows NTFS `temp_index` sanitization + `-p HEAD` parent commits + clean untracked restore + `diff_checkpoints`, materialize real linked worktrees in `MultiVariantWorktreePool` (`vox_muse_pool_spawn` / `vox_muse_pool_adopt`), and fix `orchestrator_status_compact` double-envelope + `parent_agent_id` subagent filtering + `LiveBodyDeduper`.
**Requirements:** `T3-WORKTREE-CKPT-01`, `T3-WORKTREE-CKPT-02`, `T3-WORKTREE-CKPT-03`, `T3-WORKTREE-CKPT-04`
**Plans:**

- [ ] `285-01`: Fix async worktree lease lock & persistent webhook gate, monotonic sparse Git checkpoints & clean restore, real `MultiVariantWorktreePool` worktrees, and unwrapped `orchestrator_status_compact` (`phases/285-worktree-lease-checkpoints-and-muse-pool/285-01-PLAN.md`)

### Phase 286: UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Recursive Session Discovery & Real Guest Share / Diagnostics (`T3-TOOLS-HARDENING-01..04`)

**Goal:** Fix UTF-8 char-boundary truncation panics in `context_handoff.rs` & `mcp_tool_access.rs` + real multi-turn digest in `HandoffStrategy::Summarized`, check all configured LLM providers + run `synthesize_consensus_response` in `handle_consensus_run`, recursively scan depth-4 external sessions (`~/.claude/projects`, `~/.codex/sessions`, `~/.cursor/chats` + `vox_session_list_external`) + expose `vox_guest_share_redeem` / `vox_guest_share_submit`, and enforce server-side access caps in `authorize_wave3_call` + persistent `PullRequestReviewGuideService` + real diagnostics in `handle_file_diff_and_diagnostics`.
**Requirements:** `T3-TOOLS-HARDENING-01`, `T3-TOOLS-HARDENING-02`, `T3-TOOLS-HARDENING-03`, `T3-TOOLS-HARDENING-04`
**Plans:**

- [ ] `286-01`: Fix UTF-8 boundary panics & summarized handoff digest, multi-provider consensus synthesizer, depth-4 external session discovery & guest share submit/redeem, and server-side access / real diagnostics (`phases/286-utf8-safety-consensus-and-session-discovery/286-01-PLAN.md`)

### Phase 287: Unified `message_id` & Telemetry Across Tauri/MCP, Live Transcript Actions, Mid-Turn Steer Composer & `streamWatchdog.ts` (`T3-GUI-PIPELINE-01..04`)

**Goal:** Preserve `backend_message_id` (`usr:...`/`ast:...`) + battery/quality/posture telemetry across `vox-gui` (`chat.rs`/`chat_turn.rs`) and `history.rs` (with turn-index fallback & SQLite sync), wire live user+assistant `✎ Edit`, executing `↻ Regenerate` (`execute: true`), `✂ Compact Here` (`compactChatUpTo`), `/compact` slash command, and real branch comparison in `ChatTranscript.tsx`, fix the `Enter`-key task-abort bug and wire `[⚡ Steer Turn]` / `[+ Queue Next]` (`steerChatQueue`) in `Loquela.tsx`, and implement `streamWatchdog.ts` + `resumeChatStream` + full `ChatExecutionRail` / `SessionInteropModal` / `ConsensusGridView` / `GuestShare` wiring.
**Requirements:** `T3-GUI-PIPELINE-01`, `T3-GUI-PIPELINE-02`, `T3-GUI-PIPELINE-03`, `T3-GUI-PIPELINE-04`
**Plans:**

- [ ] `287-01`: Unify `message_id` & telemetry across Tauri/MCP, wire live edit/regenerate/compact & mid-turn steer composer, and implement `streamWatchdog.ts` + full modal/rail RPC wiring (`phases/287-unified-message-id-and-live-gui-actions/287-01-PLAN.md`)

### Phase 288: `vox-code-audit` T3 Detector Catalog Registration, `oxlint-plugin-t3code` Rule Parity & 43-Violation Workspace Remediation (`T3-AUDIT-PARITY-01..04`)

**Goal:** Register all T3 detector rule IDs in `diagnostics/catalog.rs` (`ALL_KNOWN_IDS`) with `diagnostic_id: Some(...)`, upgrade `unreasoned_suppression.rs`, `test_clock_and_skip.rs`, `mcp_access_and_error_safety.rs`, and `ui_perf_hazards.rs` to full `oxlint-plugin-t3code` parity, refactor raw `Command::new` calls in `t3_wave3_tools.rs`, and remediate all 43 live workspace violations across `vox-gui`, `vox-cli`, and `vox-orchestrator-mcp`.
**Requirements:** `T3-AUDIT-PARITY-01`, `T3-AUDIT-PARITY-02`, `T3-AUDIT-PARITY-03`, `T3-AUDIT-PARITY-04`
**Plans:**

- [ ] `288-01`: Register T3 detectors in `diagnostics/catalog.rs`, upgrade all 4 detectors to `oxlint-plugin-t3code` parity, and remediate all 43 workspace violations (`phases/288-code-audit-parity-and-workspace-remediation/288-01-PLAN.md`)

---

## Progress Table (`v5.1`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 283** | System Prompt `$variable` Target Fix, Sanitizer Tag Completeness, `ToolRefusalDetector` & Lazy MCP Meta-Tools | `1/1` | Complete (`2026-10-25`) |
| **Phase 284** | Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` Lifecycle, Real `ModelSpec` Cost Ledger & `SessionPrivacyPosture` Gate | `1/1` | Complete (`2026-10-25`) |
| **Phase 285** | Worktree Async Lease Mutex Fix, Persistent Webhook Gate, Monotonic Sparse Git Checkpoints, Real `MultiVariantWorktreePool` & Compact Snapshot Fix | `1/1` | Complete (`2026-10-25`) |
| **Phase 286** | UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Recursive Session Discovery & Real Guest Share / Diagnostics | `1/1` | Complete (`2026-10-25`) |
| **Phase 287** | Unified `message_id` & Telemetry Across Tauri/MCP, Live Transcript Actions, Mid-Turn Steer Composer & `streamWatchdog.ts` | `1/1` | Complete (`2026-10-25`) |
| **Phase 288** | `vox-code-audit` T3 Detector Catalog Registration, `oxlint-plugin-t3code` Rule Parity & 43-Violation Workspace Remediation | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v5.2` — Smooth Physically-Grounded Lighting, High-Detail Vitruvian Architecture & Grounds, Integrated Roman Hydraulic/Sewer Grid & Animated Shape+Color Activity Conduits)

### Phase 289: Smooth Ground-Plane-Recessed Lighting, 2D-Feathered Elliptical Sun-Shafts & Periodic Seamless Sea Synthesis (`URBS-SMOOTH-LIGHT-01..04`)

**Goal:** Eliminate alpha-overlay lighting artifacts and repeating water tile seams by moving warm brazier/forge/worksite bounce illumination onto the terrace ground plane before 3D buildings are painted, replacing hard-edged polygon god-rays with 2D-feathered elliptical radial gradients, replacing the hard `ctx.arc` district pulse with soft torch/window rim shimmer, and synthesizing `2π`-periodic seamless water waves in `pbrMaterials.ts`.
**Requirements:** `URBS-SMOOTH-LIGHT-01`, `URBS-SMOOTH-LIGHT-02`, `URBS-SMOOTH-LIGHT-03`, `URBS-SMOOTH-LIGHT-04`
**Plans:**

- [x] `289-01`: Implement pre-building ground-plane illumination, 2D-feathered elliptical sun-shafts, soft torch/window rim shimmer, and periodic seamless sea water synthesis (`phases/289-smooth-ground-lighting-and-seamless-sea/289-01-PLAN.md`)

### Phase 290: High-Detail Terrace Grounds: Orthogonal `Cardo`/`Decumanus` Streets, `Opus Sectile` Marble Plazas, Vaulted `Substructio` Cliff Arcades & Balustrades (`URBS-GROUNDS-01..04`)

**Goal:** Transform flat district terraces and plain cliff retaining walls into richly articulated Roman urban grounds featuring recessed basalt `Cardo`/`Decumanus` streets with raised travertine curbstones and stepping-stone crosswalks, central `Opus Sectile` geometric marble forum plazas around District Wonders, vaulted `Substructio` blind arcades and buttress pilasters on cliff walls, and perimeter stone balustrades with boundary obelisks (`Termini`).
**Requirements:** `URBS-GROUNDS-01`, `URBS-GROUNDS-02`, `URBS-GROUNDS-03`, `URBS-GROUNDS-04`
**Plans:**

- [x] `290-01`: Upgrade terrace grounds and cliff walls in `worldRenderer.ts` with `Cardo`/`Decumanus` streets, `Opus Sectile` plazas, `Substructio` cliff arcades, and stone balustrades (`phases/290-high-detail-terrace-grounds-and-substructio/290-01-PLAN.md`)

### Phase 291: High-Detail Vitruvian Building Architecture (`T0`–`T3`): Rusticated Ground Arcades (`Fornices`), Belt Cornices, Atrium `Impluvia` & `Acroteria` Roofs (`URBS-ARCH-DETAIL-01..04`)

**Goal:** Enrich all `T0`–`T3` file buildings in `worldRenderer.ts` with ground-floor rusticated stone arcades (`Fornices`) and warm shopfront lamplight, projecting inter-story `Marcapiano` belt cornices and corner quoins, recessed atrium `Impluvium` rainwater basins in `T1` Domus courtyards, and sculpted rooftop `Acroteria` finials and brick hypocaust chimneys.
**Requirements:** `URBS-ARCH-DETAIL-01`, `URBS-ARCH-DETAIL-02`, `URBS-ARCH-DETAIL-03`, `URBS-ARCH-DETAIL-04`
**Plans:**

- [x] `291-01`: Upgrade `drawIsoBox`, `drawIsoPitchedRoof`, and `drawIsoFileBuilding` with rusticated ground arcades, belt cornices, atrium impluvia, and acroteria roofs (`phases/291-high-detail-vitruvian-building-architecture/291-01-PLAN.md`)

### Phase 292: Integrated Roman Hydraulic Plumbing & Sewer Network (`hydraulicNetwork.ts`): `Aquaeductus` Trunk, `Castellum Divisorium`, `Fistulae` Supply Pipes & `Cloaca Maxima` Sewer Trenches/Outfalls (`URBS-HYDRAULIC-01..04`)

**Goal:** Create `hydraulicNetwork.ts` integrating the `AQVAE MCP` aqueduct directly into an upper-terrace `Castellum Divisorium` header tank that feeds a SimCity-style Roman hydraulic plumbing (`Fistulae` twin-rail bronze pipes & sluice valves) and `Cloaca Maxima` arched stone sewer trench network across all district terraces, terminating in cliff-base `Cloaca` sea outfalls and stepped cascades (`Cataracta`).
**Requirements:** `URBS-HYDRAULIC-01`, `URBS-HYDRAULIC-02`, `URBS-HYDRAULIC-03`, `URBS-HYDRAULIC-04`
**Plans:**

- [x] `292-01`: Create `hydraulicNetwork.ts` implementing integrated `Aquaeductus` trunk, `Castellum Divisorium`, `Fistulae` supply pipes, `Cloaca Maxima` sewer trenches, and sea outfalls (`phases/292-integrated-roman-hydraulic-and-sewer-network/292-01-PLAN.md`)

### Phase 293: Multi-Channel Color + Geometric Shape Animated Activity Conduits (`AQUA PURA` Chevrons, `IGNIS OPERIS` Diamonds, `CLOACA FAULT` Triangles & `VIA VICTORIAE` Seals) (`URBS-CONDUIT-ANIM-01..04`)

**Goal:** Animate live activities in progress along the Roman hydraulic and sewer conduits using 4 double-coded Color + Geometric Shape channels: Crystalline Cyan Chevron Wave-Crests (`❯❯`) for `AQUA PURA` MCP/provider supply, Molten Gold Diamond Ingots (`◆◆`) for `IGNIS OPERIS` active agent work-in-progress, Crimson Warning Triangles (`▲`) & eddies (`◎`) for `CLOACA FAULT` error backpressure, and Emerald Circular Laurel Medallions (`◉`) for `VIA VICTORIAE` verified CI/Git flows.
**Requirements:** `URBS-CONDUIT-ANIM-01`, `URBS-CONDUIT-ANIM-02`, `URBS-CONDUIT-ANIM-03`, `URBS-CONDUIT-ANIM-04`
**Plans:**

- [x] `293-01`: Implement `drawAnimatedHydraulicActivityFlows` in `hydraulicNetwork.ts` and wire all 4 shape+color activity conduits into `WorldRenderer` (`phases/293-animated-shape-color-activity-conduits/293-01-PLAN.md`)

### Phase 294: Diegetic Hydraulic Sluice-Gate Callouts, Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence (`URBS-SEMIOTICS-CV-01..04`)

**Goal:** Replace ungrounded screen-space straight dependency lines and the floating yellow arrow with diegetic Roman Hydraulic Sluice-Gate (`Cataracta`) callouts and isometric conduit routing, add a shape+color hydraulic flow legend, write comprehensive unit tests (`hydraulicNetwork.test.ts`, `pbrMaterials.test.ts`, `lightingAndFx.test.ts`), and verify convergence via the Playwright `@ngi/platform-ssot` CV Flywheel.
**Requirements:** `URBS-SEMIOTICS-CV-01`, `URBS-SEMIOTICS-CV-02`, `URBS-SEMIOTICS-CV-03`, `URBS-SEMIOTICS-CV-04`
**Plans:**

- [x] `294-01`: Implement diegetic hydraulic sluice-gate callouts, shape+color conduit legend, `hydraulicNetwork.test.ts` unit suite, and Playwright CV Flywheel verification (`phases/294-semiotic-consistency-and-cv-convergence/294-01-PLAN.md`)

---

## Progress Table (`v5.2`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 289** | Smooth Ground-Plane-Recessed Lighting, 2D-Feathered Elliptical Sun-Shafts & Periodic Seamless Sea Synthesis | `1/1` | Complete |
| **Phase 290** | High-Detail Terrace Grounds: Orthogonal `Cardo`/`Decumanus` Streets, `Opus Sectile` Marble Plazas, Vaulted `Substructio` Cliff Arcades & Balustrades | `1/1` | Complete |
| **Phase 291** | High-Detail Vitruvian Building Architecture (`T0`–`T3`): Rusticated Ground Arcades (`Fornices`), Belt Cornices, Atrium `Impluvia` & `Acroteria` Roofs | `1/1` | Complete |
| **Phase 292** | Integrated Roman Hydraulic Plumbing & Sewer Network (`hydraulicNetwork.ts`): `Aquaeductus` Trunk, `Castellum Divisorium`, `Fistulae` Supply Pipes & `Cloaca Maxima` Sewer Trenches/Outfalls | `1/1` | Complete |
| **Phase 293** | Multi-Channel Color + Geometric Shape Animated Activity Conduits (`AQUA PURA` Chevrons, `IGNIS OPERIS` Diamonds, `CLOACA FAULT` Triangles & `VIA VICTORIAE` Seals) | `1/1` | Complete |
| **Phase 294** | Diegetic Hydraulic Sluice-Gate Callouts, Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence | `1/1` | Complete |

---

## Phases (`v5.4` — T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening)

### Phase 301: Change-Gated CI Path Map & `vox ci check-changed` (`T3-CI-CHANGED-01..04`)

**Goal:** Port t3code `check:changed` + INERT/EXTRA_OWNERS ownership mapping into a Vox contract and CLI so local/CI runs only the gates owned by the diff.
**Requirements:** `T3-CI-CHANGED-01`, `T3-CI-CHANGED-02`, `T3-CI-CHANGED-03`, `T3-CI-CHANGED-04`
**Success criteria:**

1. Owner map contract validates under schema and lists crates/UI/workflows/contracts.
2. `vox ci check-changed` (or `vox run scripts/check-changed.vox`) exits 0 on inert planning-only diffs without invoking cargo.
3. PR jobs skip heavy work for inert path sets while required contexts remain honest.
4. Docs in `runner-contract.md` show the local command and ownership rules.

**Plans:**

- [x] `301-01`: Define owner map + implement `check-changed` + CI wiring + docs/tests

### Phase 302: Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding (`T3-CI-BUDGET-01..04`)

**Goal:** Promote transfer-budget coverage into CI artifacts + trusted PR comments, and balance shards with duration weights.
**Requirements:** `T3-CI-BUDGET-01`, `T3-CI-BUDGET-02`, `T3-CI-BUDGET-03`, `T3-CI-BUDGET-04`
**Success criteria:**

1. CI uploads a schema-valid budget artifact on PRs that touch orchestrator-mcp/budget code.
2. Comment publisher runs from default-branch code only and posts delta vs main.
3. Weighted shard file measurably reduces max-shard wall time vs hash-only partitions.
4. Ratchet report lands under `contracts/reports/`.

**Plans:**

- [x] `302-01`: Budget artifact + trusted comment workflow + weighted sharding + ratchet

### Phase 303: Knip Dual Gate, PR Size Labels & Evidence-Asset Ban (`T3-CI-HYGIENE-01..04`)

**Goal:** Catch unused TS exports, label PR blast radius by non-test LOC, and ban committed evidence spam.
**Requirements:** `T3-CI-HYGIENE-01`, `T3-CI-HYGIENE-02`, `T3-CI-HYGIENE-03`, `T3-CI-HYGIENE-04`
**Success criteria:**

1. Knip (or equivalent) fails CI on new unused exports in `vox-gui/ui` outside allowlist.
2. PRs receive exactly one size:* label from the XS–XXL set based on non-test LOC.
3. Committing a disallowed evidence binary fails the hygiene gate.
4. Mutation tests prove each gate fails closed; AGENTS/CONTRIBUTING updated.

**Plans:**

- [x] `303-01`: Knip config + PR size workflow + evidence ban + docs/mutation tests

### Phase 304: Agent-Ops Kill-Safety, Hit-Every-Surface Checklist & Worktree Isolation (`T3-AGENT-OPS-01..04`)

**Goal:** Import t3code agent-ops culture: kill-safety, multi-surface completion checklist, isolated worktree homes, suppression human review.
**Requirements:** `T3-AGENT-OPS-01`, `T3-AGENT-OPS-02`, `T3-AGENT-OPS-03`, `T3-AGENT-OPS-04`
**Success criteria:**

1. AGENTS.md / governance names the kill-safety triad with Vox paths.
2. Checklist is referenced from PR template or `vox doctor` advisory for multi-surface diffs.
3. Worktree setup marker + isolated home documented and scripted via `.vox`.
4. Suppression-only PRs cannot merge without human-review label/CODEOWNERS.

**Plans:**

- [x] `304-01`: Policy docs + checklist advisory + worktree isolation + suppression review gate

### Phase 305: TOESTUB Thin-Transport, Suppression Review Gate & Snapshot Interning (`T3-TOESTUB-ENF-01..04`)

**Goal:** Close the oxlint→TOESTUB gap for transport bypasses, suppression reasons, snapshot interning, and remaining high-value rules.
**Requirements:** `T3-TOESTUB-ENF-01`, `T3-TOESTUB-ENF-02`, `T3-TOESTUB-ENF-03`, `T3-TOESTUB-ENF-04`
**Success criteria:**

1. New detector flags raw RPC/client bypass outside allowlisted files.
2. Unreasoned suppressions in a PR diff fail CI.
3. Snapshot/message-ID interning is live-path covered by tests.
4. Remaining oxlint rules are either detectors or explicit deferred ledger rows.

**Plans:**

- [x] `305-01`: Detectors + suppression ratchet + interning + oxlint mapping ledger

### Phase 306: Tauri Fingerprint Advisory, GUI Cache Layers, Surface Graph Refresh & Scorecard (`T3-CI-CLOSE-01..04`)

**Goal:** Close the milestone with fingerprint labeling, legal GUI caches, a fresh T3 surface graph, and a CI-enforcement scorecard.
**Requirements:** `T3-CI-CLOSE-01`, `T3-CI-CLOSE-02`, `T3-CI-CLOSE-03`, `T3-CI-CLOSE-04`
**Success criteria:**

1. Native/sidecar-affecting PRs get an advisory label; check stays green.
2. GUI jobs restore layered pnpm/vite caches without PR cache saves.
3. Surface graph regenerates from current clone SHAs; inventory script compiles.
4. Scorecard + architecture note list every gem as Shipped/Partial/Deferred with anchors.

**Plans:**

- [x] `306-01`: Fingerprint workflow + cache layers + graph regen + scorecard/docs

---

## Progress Table (`v5.4`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 301** | Change-Gated CI Path Map & `vox ci check-changed` | `1/1` | Complete |
| **Phase 302** | Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding | `1/1` | Complete |
| **Phase 303** | Knip Dual Gate, PR Size Labels & Evidence-Asset Ban | `1/1` | Complete |
| **Phase 304** | Agent-Ops Kill-Safety, Hit-Every-Surface Checklist & Worktree Isolation | `1/1` | Complete |
| **Phase 305** | TOESTUB Thin-Transport, Suppression Review Gate & Snapshot Interning | `1/1` | Complete |
| **Phase 306** | Tauri Fingerprint Advisory, GUI Cache Layers, Surface Graph Refresh & Scorecard | `1/1` | Complete |

## Phases (`v5.3` — Photorealistic Edge-Softening, Continuous Monumental Aqueduct-to-Sewer Architecture, Sub-Surface Ground Integration & CV Flywheel Score Maximization)

### Phase 295: Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression (`URBS-EDGE-PBR-01..04`)

**Goal:** Eliminate hard diamond coastal cutouts, 1px Laplacian PBR pixel chatter (`noiseFloor: 10.7`), and `#1a1613` cartoon ink outlines by introducing feathered elliptical radial coastal shoals, smooth bilinear/radial macro-shaded PBR stone/terracotta/water tiles, full-canvas diagonal ocean swells, and soft tonal masonry/roof edge bevels.
**Requirements:** `URBS-EDGE-PBR-01`, `URBS-EDGE-PBR-02`, `URBS-EDGE-PBR-03`, `URBS-EDGE-PBR-04`
**Plans:**

- [x] `295-01`: Implement feathered coastal bathymetry, low-Laplacian macro-shaded PBR tiles, diagonal ocean swells, and soft tonal edge bevels (`phases/295-feathered-bathymetry-and-smooth-pbr/295-01-PLAN.md`)

### Phase 296: True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums (`URBS-TERRACE-CRAFT-01..04`)

**Goal:** Replace jagged axis-aligned 2D `fillRect` wall merlons and overlapping street alpha rectangles with true `2:1` isometric molded travertine parapet capstones (`Corona`), 3D pyramidal-roofed corner bastions, a unified intersection-free `Cardo`/`Decumanus` street cross-plaza (`Via Silicea`), a recessed multi-ring `Opus Sectile` Forum with 4 corner Corinthian columns, and an overhanging cliff-edge travertine cornice (`Geison`).
**Requirements:** `URBS-TERRACE-CRAFT-01`, `URBS-TERRACE-CRAFT-02`, `URBS-TERRACE-CRAFT-03`, `URBS-TERRACE-CRAFT-04`
**Plans:**

- [x] `296-01`: Upgrade curtain walls, `Cardo`/`Decumanus` street intersections, `Opus Sectile` forum peristyles, and cliff cornices (`phases/296-isometric-walls-and-seamless-streets/296-01-PLAN.md`)

### Phase 297: Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae (`URBS-ARCH-REALISM-01..04`)

**Goal:** Upgrade `Tabularium` and `Pantheon` rotundas with multi-stop cylindrical normal shading and coffered half-dome ribs (`Lacunaria`), replace flat 2D window squares with isometric-skewed recessed stone window reveals and warm interior lamplight, add 2-stepped stylobates and sculpted pediment tympanums to `T2`/`T3` buildings, and upgrade peripheral landmarks (`AQVAE`, `CASTRVM`, `PORTVS`, `PORTA`) to 3D isometric masonry.
**Requirements:** `URBS-ARCH-REALISM-01`, `URBS-ARCH-REALISM-02`, `URBS-ARCH-REALISM-03`, `URBS-ARCH-REALISM-04`
**Plans:**

- [x] `297-01`: Upgrade cylindrical rotunda wonders, isometric window reveals, pediment tympanums, and 3D peripheral landmarks (`phases/297-cylindrical-wonders-and-recessed-windows/297-01-PLAN.md`)

### Phase 298: Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid (`URBS-PLUMBING-3D-01..04`)

**Goal:** Connect `AQVAE MCP` via a continuous 3D multi-arch stone aqueduct viaduct (`Aqua Claudia`) directly into the Acropolis `Castellum Divisorium`, upgrade `drawConduitTrack` with rounded isometric elbow fillets (`quadraticCurveTo`) and 3D cylindrical pipe shading, route all dependency and blast-radius conduits on the **terrace ground / viaduct layer beneath 3D buildings** (`Layer 0.85`), and render stepped stone `Cataracta` cascades between terrace elevations.
**Requirements:** `URBS-PLUMBING-3D-01`, `URBS-PLUMBING-3D-02`, `URBS-PLUMBING-3D-03`, `URBS-PLUMBING-3D-04`
**Plans:**

- [x] `298-01`: Build continuous 3D `Aqua Claudia` aqueduct bridge, rounded pipe elbow fillets, sub-building ground conduit routing, and stepped `Cataracta` cascades (`phases/298-continuous-aqueduct-and-beveled-plumbing/298-01-PLAN.md`)

### Phase 299: Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics (`URBS-FLOW-SEMIOTICS-01..04`)

**Goal:** Add soft radial glow halos and tapered comet trails to all 4 geometric particle channels (`❯` Chevron, `◆` Diamond, `▲` Triangle, `◉` Laurel Seal), scale conduit gauge and particle cadence by live activity load, render glowing bronze manifold rings (`Epistomia`) at district street ports, and upgrade the top-left Semiotic Legend Pill with vector geometric icons and live channel counts.
**Requirements:** `URBS-FLOW-SEMIOTICS-01`, `URBS-FLOW-SEMIOTICS-02`, `URBS-FLOW-SEMIOTICS-03`, `URBS-FLOW-SEMIOTICS-04`
**Plans:**

- [x] `299-01`: Add radial halos and comet tails to activity particles, dynamic load scaling, perimeter manifold rings, and vector-icon legend (`phases/299-halo-particles-and-conduit-semiotics/299-01-PLAN.md`)

### Phase 300: Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization (`URBS-CV-MAX-01..04`)

**Goal:** Expand unit test suites (`pbrMaterials.test.ts`, `lightingAndFx.test.ts`, `hydraulicNetwork.test.ts`, `worldRenderer.test.ts`), achieve positive CV noise reduction (`noiseFloor < 5.1`) and higher `visualAfterScore` (`>= 0.942`) on `urbs-civilization` in `e2e/review/cv-flywheel.spec.ts`, verify 0 regressions across all 32 GUI surfaces, and update the visual blueprint carousel.
**Requirements:** `URBS-CV-MAX-01`, `URBS-CV-MAX-02`, `URBS-CV-MAX-03`, `URBS-CV-MAX-04`
**Plans:**

- [x] `300-01`: Expand unit tests, run Playwright CV Flywheel score maximization, and update visual blueprint (`phases/300-cv-flywheel-score-maximization/300-01-PLAN.md`)

---

## Progress Table (`v5.3`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 295** | Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression | `1/1` | Complete |
| **Phase 296** | True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums | `1/1` | Complete |
| **Phase 297** | Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae | `1/1` | Complete |
| **Phase 298** | Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid | `1/1` | Complete |
| **Phase 299** | Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics | `1/1` | Complete |
| **Phase 300** | Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization | `1/1` | Complete |
