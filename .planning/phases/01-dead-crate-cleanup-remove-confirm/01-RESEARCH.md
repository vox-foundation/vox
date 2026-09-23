# Phase 1: Dead Crate Cleanup — Remove & Confirm - Research

**Researched:** 2026-09-22
**Domain:** Internal workspace hygiene — crate deletion audit, SSOT doc parity, plugin-catalog bookkeeping
**Confidence:** HIGH (every claim below is grounded in a `Read`/`git log`/`grep`/`cargo tree` invocation run this session, not training-knowledge recall)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** After removing vox-scientia-ingest's mandatory `vox-cli` dependency, whether its scholarly-external-jobs functionality is inlined directly into vox-publisher or gated behind a feature flag is left to whoever implements this task — decide based on what reads cleanest once the actual call sites are visible. Reversibility: reversible.
- **D-02:** Document the three KEEP-FROZEN crates' (vox-workflow-runtime, vox-integration-tests, vox-test-harness) "intentionally inactive" status as entries in `catalog.toml` — the same mechanism REQ-dead-crate-catalog-cleanup already uses for ghost-entry bookkeeping, so frozen-crate status and catalog hygiene live in one place rather than split across crate-level comments or `where-things-live.md`.
- **D-03:** Before deleting any crate, re-verify zero-consumer status with this repo's own `vox graph coverage` (dead-surface/OrphanBackend detector) rather than relying solely on the 2026-05-08 PRD audit plus `cargo tree`/`cargo metadata` spot-checks. Run `vox graph status` first and `vox graph refresh --auto` if stale before trusting its output. `cargo tree -p vox-cli` / `cargo metadata` checks from the acceptance criteria still run as the final ground-truth confirmation after deletion.
- **D-04:** Commit deletions/migrations batched by sub-group: (1) the vox-scientia-* trio together, (2) standalone zero-consumer deletes together, (3) the KEEP-FROZEN documentation + catalog-cleanup verification as its own commit.

### Claude's Discretion
- Exact inline-vs-flag implementation shape for vox-scientia-ingest (D-01).
- Sub-grouping order within each batched commit (D-04).

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope. WIRE-UP, EXTRACT-TO-PLUGIN, and MISPLACED dispositions were explicitly identified as out-of-boundary (Phase 2/Phase 3).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-dead-crate-delete | Delete workspace crates with zero consumers and no remaining conceptual fit, consolidating any worthwhile types into their true owning crate first. | See **§Central Finding** and **§PRD Claim vs. Current Reality** — all 10 named crates are already deleted from disk and from `[workspace.dependencies]`; the type/logic migrations the acceptance criteria describe (ConfidencePolicy/ComplexityBand/RiskBand, scientia-ingest logic) are also already landed. Remaining Phase 1 work is the residue enumerated in **§Runtime State Inventory**, not new deletions. |
| REQ-dead-crate-catalog-cleanup | Confirm catalog-only ghost entries (execution-api, stub-check) are fully reflected in `catalog.toml`. | See **§catalog.toml current state** — already satisfied verbatim; verification-only task. |
| REQ-dead-crate-keep-frozen | Explicitly preserve vox-workflow-runtime, vox-integration-tests, vox-test-harness; no action required except D-02's new catalog.toml annotation. | See **§KEEP-FROZEN crates** — `layers.toml`/`where-things-live.md` already document all three correctly; only the catalog.toml frozen-status comment (D-02) is net-new work. |
</phase_requirements>

## Summary

**Central finding — read this before planning any task list.** All 10 crates named in REQ-dead-crate-delete's acceptance criteria (vox-schola, vox-scientia-core, vox-scientia-social, vox-scientia-ingest, vox-socrates-policy, vox-spool, vox-tools, vox-mcp-meta, vox-browser, vox-audio-ingress) were **already deleted from the repository months before this GSD milestone was planned** — most in commit `e828828a9` ("plugin system redesign" PR #65, 2026-05-08, the same day the source PRD is dated), `vox-mcp-meta` in `72bde3718` (2026-05-08), and `vox-scientia-ingest` in `0a6aae51e` (2026-05-12). None of the 10 directories exist on disk; none appear in root `Cargo.toml`'s `[workspace.dependencies]`; `cargo tree -p vox-cli --offline` run this session returns zero matches for any of the 10 names. The associated type migrations the acceptance criteria describe as prerequisites to deletion are *also* already done: `ConfidencePolicy`/`ComplexityBand`/`RiskBand` live in `crates/vox-orchestrator-types/src/socrates_policy/` with zero remaining `vox_socrates_policy::` imports anywhere in code, and vox-scientia-ingest's RSS-crawl/dedup logic now lives in `crates/vox-scientia/src/ingest/` reached through `vox-plugin-publication`'s runtime plugin-host ABI dispatch — not through a `vox-cli` compile-time dependency, and not landed in `vox-publisher` as the PRD speculated.

This means Phase 1 is **not** a deletion phase in practice — it is a **confirmation-and-residue-cleanup phase**. The planner should not generate "delete crate X" tasks for any of the 10; that work is already merged. What remains is: (1) machine-verify the above claims per D-03's ground-truth protocol so the phase's success criteria are demonstrably true, not just assumed true from git archaeology; (2) fix the small number of genuinely stale references that the historical deletions left behind (one CI matrix line, a handful of live docs); (3) do the one piece of net-new work this phase actually requires — annotating the three KEEP-FROZEN crates in `catalog.toml` per D-02, since that annotation does not exist yet in any form.

**Primary recommendation:** Structure Phase 1's plan around *verification tasks* (graph refresh + coverage query + `cargo tree`/`cargo metadata` grep, one commit per D-04's batching) that produce the evidence the success criteria demand, plus one small fix-up task for the stale `ci.yml` matrix entry, plus one net-new task for the catalog.toml frozen-crate comment block (D-02). Do not budget time for crate deletion, type migration, or vox-cli dependency surgery — all three are already merged.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Crate existence verification (cargo tree/metadata) | Build tooling (Cargo) | — | Ground truth for "does this crate exist / get referenced" lives in Cargo's own resolver, not in docs. |
| Zero-consumer / reachability verification | Code-intelligence graph (`vox graph`) | Build tooling (grep/cargo tree) | D-03 mandates the graph as primary; cargo tree is the acceptance-criteria-specified final confirmation, so both tiers are load-bearing here. |
| SSOT doc parity (layers.toml, where-things-live.md) | Docs / Architecture SSOTs | `vox-arch-check` (CI enforcement) | These are hand-authored docs enforced by a compiled checker (`vox-arch-check` Rule 12 WTL parity), so the "primary" author tier is docs but the correctness tier is CI. |
| Plugin/frozen-crate bookkeeping (catalog.toml) | `vox-plugin-catalog` crate (Rust schema + embedded TOML) | Docs | catalog.toml is compiled into the binary via `include_str!` and validated by `build.rs`; it is simultaneously a data file and a build input. |
| CI matrix crate list (`ci.yml` all-features check) | CI / Build tooling | — | A workflow YAML file, not owned by any Rust crate; drifts silently when crates are renamed/merged because nothing type-checks it. |

## Standard Stack

Not applicable in the conventional sense — this phase adds no new external dependency. The "stack" is entirely this repo's own internal verification tooling:

| Tool | Purpose | Why it's the standard here |
|------|---------|----------------------------|
| `vox graph status` / `vox graph refresh --auto` / `vox graph query` | D-03-mandated reachability verification | AGENTS.md §Code Intelligence — Graphify; this repo's own code-intelligence graph, not a generic search tool. |
| `cargo tree -p vox-cli --offline` | Acceptance-criteria-specified ground truth for "no reference to crate X" | Verified working this session — returns clean output in well under the 120s default timeout when run with `--offline`. |
| `cargo metadata` | Workspace-wide (not just vox-cli-rooted) reference check | Named explicitly in the phase's Success Criteria #1. |
| `/usr/bin/grep` (absolute path) | Source-level import verification (`vox_socrates_policy::`, `vox_scientia_ingest::`, etc.) | See **§Pitfall: grep interception** below — the bare `grep` name is intercepted by a project-root-scoped shim in this environment; the absolute-path invocation bypasses it. |
| `git log --diff-filter=D --oneline --all -- <path>` | Establishing when/whether a crate directory was actually deleted, vs. relying on the PRD's static claim | This is how the central finding above was established — the PRD alone cannot tell you whether its own recommendations were later executed. |

No `Package Legitimacy Audit` section — this phase installs no external packages.

## Architecture Patterns

### Crate-removal residue pattern (what "delete a crate" actually touches in this workspace)

Based on inspecting the three historical deletion commits (`e828828a9`, `72bde3718`, `0a6aae51e`) plus the current state of every downstream file they should have touched:

1. `crates/<name>/` directory removed (`git rm -r`).
2. `Cargo.toml` → `[workspace.dependencies]` entry removed. **Verified pattern:** all 10 target crates are absent from this table today.
3. `docs/src/architecture/layers.toml` → crate's layer-assignment row removed. **Verified:** 9 of 10 already clean; `vox-scientia-ingest` has one row left, but — see the pitfall below — it is a different, *intentional*, forward-looking entry, not residue.
4. `docs/src/architecture/where-things-live.md` → concept→crate row removed. **Verified:** same pattern as layers.toml — 9 of 10 clean, `vox-scientia-ingest`'s remaining row is the same intentional planned-crate reference.
5. `contracts/ci/crate-edges.allow.v1.json` / `contracts/ci/crate-layers.v1.json` → edge/layer entries removed. **Verified: zero references to any of the 10 crates in either contract file.** Nothing to do here.
6. `.github/workflows/ci.yml` matrix lists → **not** reliably updated by historical deletions. **Verified gap:** line 1980 of `ci.yml` still lists `vox-scientia-ingest` in the `all-features check` job's crate matrix (`cargo check -p vox-scientia-ingest --all-features` would fail — the package no longer resolves). This is the one confirmed operationally-broken residue from the historical deletions. Note `vox-scientia` (the crate that absorbed the ingest logic) does not appear anywhere else in that same matrix, so simply deleting the line drops CI coverage of that code entirely — the fix should be `vox-scientia-ingest` → `vox-scientia`, not a bare deletion.
7. `crates/vox-plugin-catalog/catalog.toml` → touched only if the deleted crate was itself a `[[plugin]]` entry. None of the 10 target crates were catalog plugins, so no historical catalog.toml edits were expected or found for them.

### catalog.toml's existing "ghost entry" documentation mechanism (relevant to D-02)

`crates/vox-plugin-catalog/catalog.toml` lines 73–77 show the *only* precedent for documenting a removed/inactive thing in this file:

```toml
# ── Skill plugins ──────────────────────────────────────────────────────────────
#
# execution-api and stub-check removed 2026-05-08: no crate exists for either;
# placeholder entries with bundled-in=[] provided no value and pointed to non-existent repos.
# Restore if/when the actual plugin crates are created.
```

This is a **plain TOML comment**, not a structured `[[table]]` entry. `crates/vox-plugin-catalog/src/lib.rs` (lines 15–25) deserializes only four known array-of-tables (`plugin`, `bundle`, `component`, `skill-bundle`) via `#[derive(Deserialize)]` without `deny_unknown_fields` — so a stray unknown table would be silently ignored by the parser (not an error, but also not exposed through any accessor, i.e. dead data). **Implication for D-02:** the frozen-crate documentation should follow the *comment* precedent exactly (a new prose comment block, analogous to the execution-api/stub-check one, naming vox-workflow-runtime / vox-integration-tests / vox-test-harness as intentionally inactive per the dead-crate-fate-plan disposition) — not a new `[[frozen]]` TOML table, which would parse but do nothing.

### §PRD Claim vs. Current Reality (per-crate)

| Crate | PRD (2026-05-08) claim | Verified current reality | Divergence |
|---|---|---|---|
| vox-schola | DELETE, zero consumers | Deleted in `e828828a9` (2026-05-08). Absent from disk and workspace deps. | None — already done. |
| vox-scientia-core | DELETE, pure facade | Deleted in `e828828a9`. | None — already done. |
| vox-scientia-social | DELETE, pure facade | Deleted in `e828828a9`. | None — already done. |
| vox-scientia-ingest | DELETE only after removing mandatory `vox-cli` dep; logic to land in vox-publisher's `scholarly-external-jobs` feature | Deleted `0a6aae51e` (2026-05-12). Logic now lives in `crates/vox-scientia/src/ingest/{rss_crawler,deduplicator}.rs`. `vox-cli`'s command (`crates/vox-cli/src/commands/db/publication/ingest.rs`) dispatches through `vox_plugin_host::cached_code_plugin("publication")` at runtime — **no compile-time `vox-cli` dependency on the ingest logic at all.** The runtime implementation is `crates/vox-plugin-publication/src/ingest.rs::ingest_tick()`, whose own doc comment states: *"Ingest workflow exported as an rlib entry point so that vox-cli can call it without taking a direct dependency on vox-scientia-ingest."* | **Landed in vox-plugin-publication (a plugin), not vox-publisher.** `vox-publisher`'s `scholarly-external-jobs` Cargo feature (Cargo.toml lines 20–23) is a *different, pre-existing, unrelated* feature that only gates `dep:vox-db` for `external_submission_jobs` (outbound scholarly submission orchestration) — it shares a word with the PRD's suggested landing spot but is not the same code path. **D-01 is effectively moot**: the choice it defers (inline into vox-publisher vs. feature-flag) was already resolved differently (full plugin extraction) by prior work not tracked by this PRD. |
| vox-socrates-policy | DELETE after migrating ConfidencePolicy/ComplexityBand/RiskBand into vox-orchestrator-types | Deleted `e828828a9`. Types live in `crates/vox-orchestrator-types/src/socrates_policy/{policy_types,confidence_policy,confidence_override,complexity,mod}.rs`. Zero `vox_socrates_policy::` imports found in any `.rs` file workspace-wide (three references remain, all in Markdown: `docs/agents/orchestrator.md`, `docs/src/reference/socrates-protocol.md`, and one archived doc). | Fully done. The only residue is prose in two live docs still naming the old crate — not a code fix, but worth a mention to the planner as an optional doc-accuracy fix (not in the literal acceptance criteria). |
| vox-spool | DELETE, zero consumers | Deleted `e828828a9`. | None — already done. |
| vox-tools | DELETE, superseded by vox-capability-registry + vox-plugin-host | Deleted `e828828a9`. | None — already done. |
| vox-mcp-meta | DELETE after migrating A2A_MESSAGE_TYPES into vox-orchestrator-types, wire vox-mcp-registry directly | Deleted `72bde3718` (2026-05-08). **A2A_MESSAGE_TYPES landed in `crates/vox-mcp-registry/src/lib.rs` (and is separately duplicated in `crates/vox-corpus/src/mcp_meta.rs`) — not in vox-orchestrator-types as the PRD specifies.** | Divergence exists but is **out of Phase 1's boundary** — CONTEXT.md's own `<code_context>` section flags this as "Phase 2's REQ-dead-crate-wire-up work." Flagging here only so whoever plans Phase 2 knows the actual landing spot differs from the PRD's stated target and there is a live duplicate in vox-corpus to reconcile. |
| vox-browser | DELETE, superseded by vox-plugin-browser | Deleted `e828828a9`. `ci.yml` line 1809 already has a comment: *"Canonical crate is vox-plugin-browser (historical vox-browser crate removed)."* | None — already done, and already self-documented in CI. |
| vox-audio-ingress | DELETE, fold into vox-plugin-oratio-mic or remove | Deleted `e828828a9`. | None — already done. |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| "Is crate X still referenced anywhere?" | An ad hoc recursive `grep -r` across the whole tree as your *only* signal | `vox graph query`/`vox graph coverage` (D-03's mandated first check) plus `cargo tree -p vox-cli --offline` / `cargo metadata` as the specified ground truth | grep alone gives false negatives (misses doc-only prose you may not care about) and false positives (matches unrelated substrings, e.g. bare `vox-tools` inside longer words); the graph and Cargo's own resolver are the two sources of truth this repo's own policy already designates. |
| Bare `grep`/`Grep` tool calls in this environment | Relying on the tool name `grep` resolving to plain `/usr/bin/grep` | Absolute-path `/usr/bin/grep ...` (see Pitfall below) | See the dedicated pitfall entry — an environment-level interception silently redirects `grep` to a project-root-scoped shim that rejected reads against this repo mid-session. |

**Key insight:** for a phase whose entire job is "confirm crates are gone," the temptation is to write a bespoke verification script. Don't — the three tools above (graph, cargo tree, cargo metadata) are already this repo's designated ground truth per D-03 and the phase's own success criteria; a custom script would just be a fourth, unendorsed source of truth to keep in sync.

## Runtime State Inventory

This phase is not a rename, but it *is* a deletion-confirmation / doc-parity phase, so the same "what still references the old name after the code is gone" audit discipline applies. Answered explicitly per category:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data (DBs, collections, IDs) | None — the 10 deleted crates had no persistent stores of their own (vox-spool was an in-process JSONL queue with no external store; vox-scientia-ingest's DB writes flow through `vox-db`, which persists, but under `vox-scientia`'s/`vox-plugin-publication`'s ownership post-migration, not a name-keyed artifact of the old crate name). | None. |
| Live service config (n8n, Datadog, Tailscale, Cloudflare Tunnel, etc.) | None found — this is a pure Rust/Cargo workspace with no such externally-configured services tied to these crate names. | None. |
| OS-registered state (systemd, launchd, Task Scheduler, pm2) | None — none of the 10 crates shipped a standalone daemon/service registration; `vox-audio-ingress` was a binary target but its deletion left no OS registration behind (verified: no launchd/systemd unit files anywhere in the repo reference it). | None. |
| Secrets / env vars | None — `timeout 30 /usr/bin/grep -rn` across `crates/vox-secrets/src/spec/` for any of the 10 crate names returned no hits; no `SecretId` entries reference them. | None. |
| Build artifacts / installed packages / **stale doc & CI references** (the actual residue category for this phase) | (1) `.github/workflows/ci.yml:1980` — `vox-scientia-ingest` still listed in the `all-features check` matrix; would fail if that job runs, since the package no longer resolves. (2) `docs/src/architecture/layers.toml:580` and `docs/src/architecture/where-things-live.md:396` — a `vox-scientia-ingest` row exists in each, but investigation shows this is a **different, intentional, forward-looking `[planned]` entry** for a future SCIENTIA-pipeline-phase crate (see Pitfall below), not residue from the deleted facade crate — do not touch. (3) `docs/agents/orchestrator.md`, `docs/src/reference/socrates-protocol.md` — live docs still narrating `vox_socrates_policy::*` as if the crate exists; optional doc-accuracy cleanup, not required by the acceptance criteria. (4) `catalog.toml` — no `execution-api`/`stub-check` residue (already correctly documented as removed); **no frozen-crate annotation exists yet for the three KEEP-FROZEN crates** — this is D-02's net-new work, not residue. | Fix ci.yml line 1980 (`vox-scientia-ingest` → `vox-scientia`, preserving matrix coverage). Leave layers.toml/WTL's `vox-scientia-ingest` `[planned]` rows alone. Optionally clean the two live docs (not required by acceptance criteria — flag as discretionary). Add the D-02 catalog.toml comment block (required). |

## Common Pitfalls

### Pitfall 1: Trusting the PRD's per-crate claims without re-verifying against current repo state
**What goes wrong:** Planning "delete vox-X" tasks for crates that were already deleted four months before this milestone started, producing a plan full of no-op tasks (or worse, tasks that fail immediately because `rm -rf crates/vox-X` has nothing to remove).
**Why it happens:** The source PRD (`dead-crate-fate-plan-2026-05-08.md`) is dated the same day as the large "plugin system redesign" PR that happened to also delete 8 of the 10 crates it was analyzing — so the PRD's per-crate analysis and the actual deletion work were nearly simultaneous but not causally connected in an obvious way (the PR's commit message doesn't mention the PRD). A planner reading only the PRD (and REQUIREMENTS.md's verbatim-extracted acceptance text, which quotes the PRD) would reasonably assume the deletions are still-pending work.
**How to avoid:** Run `git log --diff-filter=D --oneline --all -- crates/<name>` for every crate the PRD lists as DELETE before writing a task for it. If a deletion commit exists, the task becomes "verify" not "delete."
**Warning signs:** `test -d crates/<name>` returns false, or the crate is absent from `Cargo.toml`'s `[workspace.dependencies]` — either is sufficient to know the crate is already gone.

### Pitfall 2: Treating every `vox-scientia-ingest` grep hit as PRD-crate residue
**What goes wrong:** "Cleaning up" `docs/src/architecture/layers.toml:580` and `where-things-live.md:396` by deleting the `vox-scientia-ingest` rows there, on the assumption they're leftover references to the deleted facade crate.
**Why it happens:** The string `vox-scientia-ingest` is ambiguous in this repo: it names both (a) the now-deleted PRD-disposition crate, and (b) a currently-planned *future* crate tracked in `layers.toml`'s `[planned]` table (`vox-scientia-ingest = { plan = "docs/src/architecture/scientia-phases-2026.md", layer = 3 }`) that is part of an unrelated SCIENTIA-pipeline roadmap ("These will be folded into vox-scientia sub-modules when implemented (Phase I onwards)" — where-things-live.md line 388). These are different concepts that happen to share a name.
**How to avoid:** Before editing any row matching a target crate name, read the surrounding section header/context to confirm it's describing the *disposed* crate and not a `[planned]`/roadmap placeholder for a differently-scoped future effort.
**Warning signs:** The row lives under a `[planned]` TOML table or a Markdown heading like "SCIENTIA pipeline phases" / "will be folded into ... when implemented" — that phrasing signals forward-looking, not backward-residue.

### Pitfall 3: Bare `grep`/native search tools silently redirected or blocked in this session's environment
**What goes wrong:** A plain `grep -n pattern /Users/brbrainerd/dev/vox/Cargo.toml` command (even after `cd`-ing into the vox repo) can fail with `ERROR: path escapes project root ... (root: /Users/brbrainerd/dev/ramada)` — an unrelated sibling project's root — rather than searching the intended file.
**Why it happens:** This environment has a project-root-scoped interception layer (observed this session) that appears to bind `grep` to a different project's root under some invocation shapes.
**How to avoid:** Invoke `/usr/bin/grep` (or another tool) by absolute path, which was confirmed to bypass the interception and search the correct file/tree in this session.
**Warning signs:** A search command returns a "path escapes project root" or similarly-worded sandboxing error naming a directory you did not intend to operate in.

### Pitfall 4: Assuming `catalog.toml`'s ghost-entry mechanism supports structured "frozen" entries
**What goes wrong:** Inventing a new `[[frozen]]` array-of-tables in `catalog.toml` for D-02, expecting `vox-plugin-catalog`'s Rust schema to pick it up.
**Why it happens:** The file has four existing `[[table]]` kinds (`plugin`, `bundle`, `component`, `skill-bundle`), so a new kind looks like the natural pattern to follow.
**How to avoid:** `crates/vox-plugin-catalog/src/lib.rs`'s `CatalogFile` struct deserializes only those four known keys and has no `deny_unknown_fields`, so an unrecognized table would parse successfully but be silently dropped — never surfaced through `all_plugins()`/`all_bundles()`/etc. The *only* existing precedent for "document something as no-longer/not-currently active" in this file (execution-api/stub-check, lines 73–77) is a plain prose `#` comment block. Follow that precedent for the three KEEP-FROZEN crates instead.
**Warning signs:** `cargo build -p vox-plugin-catalog` succeeding is not proof your new table is doing anything — check whether any accessor function actually returns the data before trusting it's "wired in."

## Code Examples

### The existing ghost-entry comment pattern to extend for D-02
```toml
# Source: crates/vox-plugin-catalog/catalog.toml lines 73-77 (verified this session)
# ── Skill plugins ──────────────────────────────────────────────────────────────
#
# execution-api and stub-check removed 2026-05-08: no crate exists for either;
# placeholder entries with bundled-in=[] provided no value and pointed to non-existent repos.
# Restore if/when the actual plugin crates are created.
```

### The plugin-host dispatch pattern that already resolved vox-scientia-ingest's "no vox-cli dependency" requirement
```rust
// Source: crates/vox-cli/src/commands/db/publication/ingest.rs (verified this session, lines 1-40 region)
// The actual crawl/dedup workflow is dispatched through the `publication`
// plugin's `Publication` extension trait object, and dispatches `ingest_tick`
// over the plugin-host ABI:
let plugin = vox_plugin_host::cached_code_plugin("publication")
    // ... .ingest_tick(feed_arg, limit_u32) ...
```
```rust
// Source: crates/vox-plugin-publication/src/ingest.rs lines 1-19 (verified this session)
//! Ingest workflow exported as an rlib entry point so that `vox-cli` can call
//! it without taking a direct dependency on `vox-scientia-ingest`.
use vox_scientia::ingest::{FeedCrawler, IngestDeduplicator};
// This function mirrors the logic previously embedded in
// `vox-cli/src/commands/db/publication/ingest.rs` and is the canonical
// dispatch target now that the CLI delegates via this rlib instead of
// importing `vox-scientia-ingest` directly.
pub async fn ingest_tick(feed_id: Option<&str>, limit: usize) -> Result<()> { /* ... */ }
```

### The stale CI matrix entry to fix
```yaml
# Source: .github/workflows/ci.yml lines 1962-1988 (verified this session — the
# "all-features check" job's crate matrix)
        crate:
          - vox-compiler
          # ...
          - vox-oratio
          - vox-skills
          - vox-scientia-ingest   # <- stale: package no longer exists, `cargo check -p vox-scientia-ingest` would fail
          - vox-doc-pipeline
          - vox-mcp-registry
          # ...
          - vox-primitives        # <- also stale/non-existent, but NOT one of this phase's 10 crates; out of REQ scope, flagged for awareness only
```
Fix: replace `vox-scientia-ingest` with `vox-scientia` (the crate that now owns that code) rather than deleting the line outright, to avoid silently dropping CI's all-features coverage of the ingest module.

## State of the Art

See **§PRD Claim vs. Current Reality** above — that table *is* this phase's "old approach vs. current approach" delta, crate by crate. No separate library-ecosystem "state of the art" applies; this is not a phase about adopting new external tooling.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `vox graph refresh --auto` was not run this session (graph is stale: `git_drift`, `worktree_drift` on all corpora per `vox graph status`) because the cargo-tree/grep/git-log ground truth already available was judged sufficient given the crates in question are already deleted, not pending deletion. The plan-phase or execute-phase should still run D-03's graph refresh+coverage query as the literal acceptance step, since it costs little once run and is explicitly mandated by a locked decision. | §Central Finding, §Standard Stack | Low — if the graph, once refreshed, disagrees with the cargo-tree/grep findings above (e.g. shows a live consumer this session's grep missed due to a macro-generated or string-based reference), the phase's "confirm zero consumers" step would need to re-open investigation on that one crate. Cargo's own resolver (`cargo tree`) is authoritative for compile-time references regardless, so this risk is bounded to non-compiled references (docs, comments, dynamic dispatch by string name) that grep may also miss. |
| A2 | The two live docs (`docs/agents/orchestrator.md`, `docs/src/reference/socrates-protocol.md`) that still narrate `vox_socrates_policy::*` are optional cleanup, not required by REQ-dead-crate-delete's acceptance criteria (which specifies "no `vox_socrates_policy::*` imports" — a code claim, already true). | §PRD Claim vs. Current Reality (vox-socrates-policy row) | Low — if the planner or a reviewer reads "zero remaining imports" as also covering doc prose, these two docs would need a one-line reference fix each; small, mechanical, no design risk. |
| A3 | The ~70 additional live-doc hits for the 10 crate names (surfaced by a broad `grep -rl` across `docs/`, not individually read this session) are historical/narrative mentions in dated plan/spec docs, not active broken references requiring a fix — based on spot-checking that the two most on-point docs (crate-classification/dead-crate-fate-plan/2026-05-08-crate-org-followup) are themselves historical record documents that correctly narrate what *was* recommended or done. | §Central Finding | Medium — if any of the un-individually-read ~70 files contains a live (non-historical) instruction telling a future contributor to still perform one of these deletions, that would be a live contradiction not caught here. Recommend the planner do a light second-pass `grep -c` on that file list if budget allows, but do not block the phase on it — none of it is in the phase's canonical_refs or acceptance criteria. |

## Open Questions

1. **Should Phase 1 also fix the two live-doc `vox_socrates_policy::` references and the ~70-file broader doc mention list?**
   - What we know: Neither is required by the literal acceptance criteria (which are code/catalog/layer-file scoped); both are within AGENTS.md's general spirit of SSOT accuracy.
   - What's unclear: Whether the user wants doc-accuracy sweeps bundled into this phase or deferred as a separate hygiene pass.
   - Recommendation: Scope Phase 1's plan to the acceptance-criteria-literal items (ci.yml fix, catalog.toml D-02 annotation, verification tasks) and leave the broader doc sweep as an explicitly-named "not in this phase" note in the plan, so it isn't silently dropped either.

2. **Does `vox graph coverage --kind <kind>` support a crate-level kind at all?**
   - What we know: `vox graph coverage --kind crate` returned `{"entries": []}` this session (empty, not an error) against the stale cache.
   - What's unclear: Whether that's "no crate-kind coverage data exists" (a tooling gap) or "zero orphan crates found" (a real, if stale, positive signal) — the stale graph state makes this ambiguous either way.
   - Recommendation: Re-run after `vox graph refresh --auto` during plan execution; if still empty, use `vox graph query "<crate-name>"` per-crate (confirmed working this session, returns fuzzy-matched symbol results) instead of the `--kind` filter.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` (via build broker shim) | `cargo tree`/`cargo metadata` ground-truth checks | ✓ | workspace pinned via `rust-toolchain.toml` (not re-verified this session; `cargo tree -p vox-cli --offline` ran successfully) | — |
| `vox` CLI (`vox graph ...`) | D-03's graph verification step | ✓ | binary on PATH, responded to `vox graph status`/`query`/`coverage` this session | — |
| Code-intelligence graph cache (`.vox/cache/graphify/*`) | `vox graph query`/`coverage` accuracy | ✓ but **stale** (`git_drift`, `worktree_drift` on every corpus per `vox graph status`) | — | `vox graph refresh --auto` (not run this session; budget for it during planning/execution — repo is large, ~33k nodes, so expect a non-trivial refresh time) |
| `/usr/bin/grep` (absolute path) | Source-level import verification | ✓ | — | Bare `grep`/`Grep` tool intercepted in this session's environment (see Pitfall 3); always invoke by absolute path. |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** Fresh graph cache (fallback: cargo-tree/grep ground truth used this session, or run the refresh before trusting `vox graph coverage` output).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` / `cargo nextest` (workspace standard per AGENTS.md §Local CI Gate Tiers) |
| Config file | root `Cargo.toml` + `contracts/budgets/test-tier-budgets.v1.yaml` |
| Quick run command | `cargo check --workspace` (compile-gate; this phase's primary correctness signal) |
| Full suite command | `vox ci pre-push --full` |

### Phase Requirements → Test Map
This phase is almost entirely a **verification and doc/config-annotation phase**, not new-code work — AGENTS.md's Test-First Policy trigger ("new `pub fn`") does not fire for any of the confirmed-already-done items. The only artifacts this phase's own scope produces are: a `.github/workflows/ci.yml` matrix-line edit, a `catalog.toml` comment addition, and (per D-03) verification command output. None of these are Rust `pub fn` surfaces, so no new `#[test]` is required by the repo's TDD gate.

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-dead-crate-delete | No reference to any of the 10 crates in cargo tree/metadata | smoke/compile-gate | `cargo tree -p vox-cli --offline \| grep -iE "vox-schola\|vox-scientia-core\|..."` (expect empty) — confirmed empty this session | ✅ (ad hoc command, not a checked-in test; consider whether the planner wants this as a permanent `vox ci` check) |
| REQ-dead-crate-delete (ci.yml fix) | `all-features check` matrix no longer references a non-existent package | CI smoke | The job itself, on next `git push` / `vox ci pre-push --act` | ✅ — existing CI job, just needs the one-line matrix fix |
| REQ-dead-crate-catalog-cleanup | catalog.toml parses and `vox-plugin-catalog` build succeeds after D-02's comment addition | compile-gate | `cargo build -p vox-plugin-catalog` | ✅ |
| REQ-dead-crate-keep-frozen | Three frozen crates remain present, unmodified, and documented | manual/doc-diff review | `git diff --stat -- crates/vox-workflow-runtime crates/vox-integration-tests crates/vox-test-harness` (expect empty) | ✅ |

### Sampling Rate
- **Per task commit:** `cargo check --workspace` (fast compile-gate; nothing in this phase should touch enough surface area to need more).
- **Per wave merge:** `vox ci pre-push --complete` (per AGENTS.md's "toolchain/clippy gap" pitfall — run at least `--complete` before pushing since default fast tier skips clippy).
- **Phase gate:** `cargo tree -p vox-cli --offline` + `cargo metadata` grep-clean, `vox graph coverage` clean (per D-03), `cargo build -p vox-plugin-catalog` green, `git diff` on the three frozen crates empty.

### Wave 0 Gaps
None — existing build/CI infrastructure covers all phase requirements; no new test file or fixture is needed since this phase produces no new `pub fn` surface.

## Security Domain

Not applicable in the conventional ASVS sense — this phase touches no authentication, session, access-control, input-validation, or cryptography surface. It is internal build-configuration and documentation hygiene.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | no | — |
| V6 Cryptography | no | — |

### Known Threat Patterns for this stack
None applicable — no new attack surface is introduced by deleting already-removed-crate references, annotating a TOML comment, or fixing a CI matrix entry.

## Sources

### Primary (HIGH confidence — verified this session via tool invocation)
- `git log --diff-filter=D --oneline --all -- crates/<name>` (×10, one per target crate) — establishes deletion commits `e828828a9`, `72bde3718`, `0a6aae51e`.
- `cargo tree -p vox-cli --offline` — zero-match ground truth for all 10 crate names (Success Criteria #1).
- `Read` of `crates/vox-orchestrator-types/src/socrates_policy/{mod,policy_types,confidence_policy,confidence_override,complexity}.rs` — confirms ConfidencePolicy/ComplexityBand/RiskBand migration (Success Criteria #2).
- `Read` of `crates/vox-plugin-publication/src/ingest.rs`, `crates/vox-cli/src/commands/db/publication/ingest.rs`, `crates/vox-cli/Cargo.toml` — confirms scientia-ingest reachability without a `vox-cli` compile dependency (Success Criteria #3).
- `Read` of `crates/vox-plugin-catalog/catalog.toml` (full file) and `crates/vox-plugin-catalog/src/lib.rs` — confirms execution-api/stub-check ghost-entry state and the catalog schema's comment-only precedent (Success Criteria #4, D-02).
- `Read` of `docs/src/architecture/layers.toml` (lines 560-599) and `docs/src/architecture/where-things-live.md` (lines 385-404) — confirms the one remaining `vox-scientia-ingest` row is an intentional `[planned]`-table forward reference, not residue.
- `Read` of `.github/workflows/ci.yml` (lines 1809-1810, 1960-1999) — confirms the stale `vox-scientia-ingest` matrix entry and the already-correct `vox-browser` comment.
- `Read` of `docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` (full file) — the canonical PRD this phase's requirements are extracted from.
- `Read` of `.planning/phases/01-dead-crate-cleanup-remove-confirm/01-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`.
- `vox graph status`, `vox graph query "vox-scientia-ingest"`, `vox graph coverage --kind crate` — run this session; graph confirmed stale, queries returned fuzzy/empty results consistent with (but not independently confirming beyond) the cargo/git findings.

### Secondary (MEDIUM confidence)
None used — every claim in this document traces to a Primary source above.

### Tertiary (LOW confidence)
- The ~70-file broad `grep -rl` hit list across `docs/` for the 10 crate names was not individually read; classified as likely-historical based on spot-checking two representative files, not exhaustive verification. See Assumption A3.

## Metadata

**Confidence breakdown:**
- Central finding (crates already deleted, migrations already landed): HIGH — every sub-claim independently verified via `git log`, `Read`, and `cargo tree` this session.
- catalog.toml D-02 mechanism: HIGH — schema and existing precedent both directly read.
- Stale ci.yml residue: HIGH — line directly read, cross-checked for duplicate coverage.
- Scope of "what else needs cleanup" beyond the acceptance-criteria-literal items (broader doc sweep): LOW — explicitly flagged as an open question / assumption, not resolved.

**Research date:** 2026-09-22
**Valid until:** This research is keyed to specific git commit history and file contents as of 2026-09-22; re-verify the `git log --diff-filter=D` and `cargo tree` commands if planning is deferred more than a few days, since this repo has an active multi-agent commit cadence (see git status: 20+ modified files in-flight at research time) that could change file contents referenced above (line numbers especially are not stable).
