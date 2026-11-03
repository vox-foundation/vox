# Phase 1: Dead Crate Cleanup — Remove & Confirm - Pattern Map

**Mapped:** 2026-09-22
**Files analyzed:** 2 modified (config/CI) + 0 new source files + N/A verification commands (no file produced)
**Analogs found:** 2 / 2

## Scope Note (read first)

Per RESEARCH.md's central finding, this phase is **not** a crate-deletion phase — all 10 originally-scoped crates are already deleted and their type/logic migrations already landed. There is no new Rust source to write, so there are **no controller/service/model/component files** to classify in the usual sense. The phase's entire file-level footprint is:

1. `crates/vox-plugin-catalog/catalog.toml` — add a prose comment block (D-02, KEEP-FROZEN annotation).
2. `.github/workflows/ci.yml` — fix one stale matrix line (`vox-scientia-ingest` → `vox-scientia`).

Everything else in this phase (D-03's `vox graph`/`cargo tree`/`cargo metadata` verification pass) is **command output, not a file diff** — there is no file for the planner to assign an analog to. It is captured here as a "no analog / not applicable" verification task instead of a fabricated file entry.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `crates/vox-plugin-catalog/catalog.toml` (add KEEP-FROZEN comment block) | config (data file, TOML, deserialized by `vox-plugin-catalog`) | transform (static data → parsed Rust struct via `include_str!` + `toml::from_str`) | `crates/vox-plugin-catalog/catalog.toml` lines 73-77 (execution-api/stub-check ghost-entry comment) | exact (same file, same section style, literal precedent for "document a removed/inactive thing as a comment") |
| `.github/workflows/ci.yml` (fix `vox-scientia-ingest` → `vox-scientia` matrix entry, line 1980) | config (CI workflow YAML, matrix list) | batch (per-crate `cargo check --all-features` fan-out) | `.github/workflows/ci.yml` lines 1808-1810 (`vox-browser` → `vox-plugin-browser` rename precedent) | role-match (same file, same "crate renamed/removed, CI reference needs updating" problem, one section up) |

## Pattern Assignments

### `crates/vox-plugin-catalog/catalog.toml` (config, transform)

**Analog:** same file, lines 73-77 (the only existing precedent for this exact kind of edit)

**Exact precedent block to model the new comment on** (verified via Read, lines 73-77):
```toml
# ── Skill plugins ──────────────────────────────────────────────────────────────
#
# execution-api and stub-check removed 2026-05-08: no crate exists for either;
# placeholder entries with bundled-in=[] provided no value and pointed to non-existent repos.
# Restore if/when the actual plugin crates are created.
```

**Surrounding structure for placement context** (lines 1-9, file header — confirms the file's own convention of a top-of-file/top-of-section prose comment, and that `cargo build -p vox-plugin-catalog` is the stated post-edit validation step):
```toml
# vox-plugin-catalog SSOT
#
# Every first-party Vox plugin and distribution bundle is declared here.
# After editing: `cargo build -p vox-plugin-catalog` validates the file.
# Regenerate the human-readable docs with:
#     vox ci generate-plugin-catalog-docs
#
# See: docs/src/architecture/plugin-system-redesign-2026.md
# See: docs/src/reference/plugin-catalog.md
```

**Schema constraint (why this MUST be a comment, not a new `[[table]]`)** — `crates/vox-plugin-catalog/src/lib.rs` lines 15-25:
```rust
#[derive(Deserialize)]
struct CatalogFile {
    #[serde(default, rename = "plugin")]
    plugins: Vec<PluginCatalogEntry>,
    #[serde(default, rename = "bundle")]
    bundles: Vec<BundleEntry>,
    #[serde(default, rename = "component")]
    components: Vec<Component>,
    #[serde(default, rename = "skill-bundle")]
    skill_bundles: Vec<SkillBundleEntry>,
}
```
No `deny_unknown_fields` — a novel `[[frozen]]` table would parse silently and be dropped, never surfaced through `all_plugins()`/`all_bundles()`/etc. (lines 36-40 show the accessor pattern: each known table gets one `pub fn all_*()` reader — there is no generic "any table" accessor to piggyback on). Follow the comment precedent exactly; do not add a new table kind.

**Concrete shape for the new block** (per D-02, naming the three KEEP-FROZEN crates): a new `#`-prefixed section, in the same voice as the execution-api/stub-check block — state crate names, "intentionally inactive" status, and a one-line reason/pointer (e.g. to the dead-crate-fate-plan disposition doc), placed as its own section (not interleaved into `[[plugin]]` tables) anywhere convenient in the file — the existing precedent has no fixed location requirement beyond being a standalone comment block near a section boundary.

**Validation:** `cargo build -p vox-plugin-catalog` (stated in the file's own header, line 4) — must stay green since this is a comment-only change and the file must still parse.

---

### `.github/workflows/ci.yml` (config, batch)

**Analog:** same file, lines 1808-1810 (`vox-browser` → `vox-plugin-browser` rename, already-landed precedent one section above the stale line)

**Precedent excerpt** (verified via Read, lines 1808-1810):
```yaml
  # Chromium/CDP (chromiumoxide): requires Chrome or Chromium on the runner (browser pool).
  # Canonical crate is `vox-plugin-browser` (historical `vox-browser` crate removed).
  vox-browser-cdp-smoke:
```

**Stale site to fix** — `all-features check` job matrix, verified via Read at lines 1962-2000 (target line is **1980**, matches RESEARCH.md's line citation):
```yaml
    strategy:
      fail-fast: false
      matrix:
        crate:
          - vox-compiler
          - vox-codegen
          - vox-cli
          - vox-actor-runtime
          - vox-db
          - vox-db-types
          - vox-secrets
          - vox-orchestrator
          - vox-vcs
          - vox-populi
          - vox-ml-cli
          - vox-gamify
          - vox-oratio
          - vox-skills
          - vox-scientia-ingest          # <- line 1980: fix to vox-scientia
          - vox-doc-pipeline
          - vox-mcp-registry
          - vox-package
          - vox-primitives
          - vox-config
          - vox-crypto
          - vox-code-audit
          - vox-openclaw-runtime
    steps:
      - uses: actions/checkout@v7
      - uses: ./.github/actions/setup-rust
        with:
          cache-key-suffix: -matrix-${{ matrix.crate }}
      - name: Check with all features
        run: cargo check -p ${{ matrix.crate }} --all-features
```

**Fix:** replace the `vox-scientia-ingest` list item with `vox-scientia` (the crate that now owns that code per RESEARCH.md's `crates/vox-scientia/src/ingest/` finding) — a straight substitution, not a deletion, so `all-features check` coverage isn't silently dropped. Optionally add a one-line comment matching the `vox-browser` precedent's style (`# vox-scientia-ingest folded into vox-scientia (facade crate removed 2026-05-12)`), though the existing matrix has no per-entry comments elsewhere, so a bare substitution is also consistent with local style — planner/implementer discretion.

**Validation:** the job itself on next push, or `vox ci pre-push --act` locally (per RESEARCH.md's Test Map); `cargo check -p vox-scientia --all-features` can be spot-run standalone first.

---

## Shared Patterns

### "Document a removed/inactive thing as a comment, not a new schema shape"
**Source:** `crates/vox-plugin-catalog/catalog.toml` lines 73-77
**Apply to:** the D-02 KEEP-FROZEN annotation (only file in this phase where this applies — no other new-schema temptation exists elsewhere in scope).

### "Crate renamed/removed → fix the CI matrix reference in place, don't drop coverage"
**Source:** `.github/workflows/ci.yml` lines 1808-1810 (`vox-browser` precedent, already landed)
**Apply to:** the line-1980 `vox-scientia-ingest` → `vox-scientia` fix.

## No Analog Found

| File / Task | Role | Data Flow | Reason |
|---|---|---|---|
| D-03 verification pass (`vox graph status` / `refresh --auto` / `coverage`, `cargo tree -p vox-cli --offline`, `cargo metadata`) | N/A — produces command output, not a source file | N/A | Not a file-creation task; RESEARCH.md §Standard Stack and §Validation Architecture already specify the exact commands and expected (empty/clean) output. Planner should schedule this as a verification/acceptance step referencing those commands directly, not a pattern-assignable file edit. |
| Optional doc-accuracy cleanup (`docs/agents/orchestrator.md`, `docs/src/reference/socrates-protocol.md` — stale `vox_socrates_policy::*` prose) | doc | N/A | Explicitly flagged in RESEARCH.md as **discretionary, out of literal acceptance-criteria scope** (Open Question 1) — not required, no analog needed unless the planner opts in. If opted in, these are plain prose edits removing/updating a crate-name reference; no special pattern beyond standard doc-accuracy editing. |
| `layers.toml:580` / `where-things-live.md:396` `vox-scientia-ingest` rows | doc (SSOT) | N/A | **Do not touch** — confirmed intentional `[planned]`-table forward references to an unrelated future SCIENTIA-pipeline crate, not residue (RESEARCH.md Pitfall 2). Listed here only so the planner doesn't accidentally generate a task against them. |

## Metadata

**Analog search scope:** `crates/vox-plugin-catalog/` (catalog.toml + src/lib.rs), `.github/workflows/ci.yml` (full-file grep for `vox-browser`/`Canonical crate`, targeted reads at the header, the execution-api/stub-check precedent, the vox-browser-cdp-smoke job, and the all-features-check matrix).
**Files scanned:** 3 (`catalog.toml`, `crates/vox-plugin-catalog/src/lib.rs`, `.github/workflows/ci.yml`); all confirmed git-tracked via `git ls-files` (no gitignored-mirror risk — this phase has no plugin-capability-sync surface).
**Pattern extraction date:** 2026-09-22
**Note on tooling:** this session's `Bash` shell is intercepted by a project-root-scoped MCP shim (`lean-ctx`) that intermittently rejects `sed`/multi-command invocations against this repo with "path escapes project root" (bound to an unrelated sibling project). `Read` and plain single-command `grep` via `Bash` both worked reliably; used those for all line-numbered excerpts above, consistent with RESEARCH.md's own Pitfall 3 observation (recommends absolute-path `/usr/bin/grep`, confirmed unnecessary here since plain `grep` worked for the confirming search).
