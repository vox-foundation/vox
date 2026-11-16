---
phase: "84"
slug: "mermaid-emitter-strict-importer"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-16"
---

# Phase 84 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: cargo-nextest + insta 1.48 + proptest (existing dev-deps; no new crate). Node: built-in `node --test` + `node:assert/strict` in `docs-astro`, oracle = exact-pinned `mermaid` 11.17.2 under a `jsdom` 30.0.1 shim (devDependencies added in 84-01) |
| **Config file** | `.config/nextest.toml` (no per-crate config); Node tests discovered by CI glob `tests/unit/*.test.mjs` (run from `docs-astro/`) |
| **Quick run command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(<test file stem>) \| test(/^mermaid::/)'` and `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` |
| **Full suite command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` and `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs` (the full `tests/unit/*.test.mjs` glob runs in hosted docs-quality after `pnpm build`) |
| **Estimated runtime** | ~90 s Rust after first build; ~10 s Node |

Filter rules: integration-test files are selected with `binary(<file stem>)` (a bare positional filter matches test names, not files); library unit tests in the new module are selected with `test(/^mermaid::/)` (their names start `mermaid::escape::tests::…`, `mermaid::export::tests::…`, `mermaid::import::…`). The freezer is `#[ignore]` and runs only with `--run-ignored only -E 'binary(freeze_corpus)'`. Goldens are blessed with `UPDATE_GOLDEN=1` and must pass without it.

---

## Sampling Rate

- **After every task commit:** `cargo check -p vox-diagram --all-targets` + the task's own `binary(...)` / `test(...)` filter; Node tasks also run the single oracle test file.
- **After every plan wave:** `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'test(/^mermaid::/) | binary(/^mermaid_/) | binary(fmt_golden) | binary(determinism_guard)'` + `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs`.
- **Before `/gsd-verify-work`:** full crate suite, `cargo clippy -p vox-diagram --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps`, `vox ci crate-api-sync`, `vox ci crate-edges`; hosted CI (`tests` shards + `docs-quality`) is authoritative.
- **Max feedback latency:** 120 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 84-01-01 | 01 | 1 | REND-06, MIG-01, MIG-02 | T-84-01, T-84-02, T-84-03, T-84-SC | labels encoded, ids safe, size-gated import, jsdom without scripts/network, pins frozen | tracer (Rust golden + Node oracle) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(mermaid_roundtrip) \| test(/^mermaid::/) \| test(print::tests) \| binary(fmt_golden)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ W0 | ⬜ pending |
| 84-01-02 | 01 | 1 | MIG-02, REND-06 | T-84-05 | oracle canonicalises state/sequence independently; CI path filter guarded | Node unit | `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs` | ❌ W0 | ⬜ pending |
| 84-01-03 | 01 | 1 | MIG-02 | T-84-04 | corpus frozen with provenance; only the ignored freezer reads docs/ | unit | `cargo nextest run -p vox-diagram -E 'binary(mermaid_corpus)'` ; `cargo nextest run -p vox-diagram --run-ignored only -E 'binary(freeze_corpus)'` | ❌ W0 | ⬜ pending |
| 84-02-01 | 02 | 2 | REND-06 | T-84-07 | nested subgraphs keep membership; global unique ids | tracer (golden + Node) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(mermaid_export) \| binary(mermaid_roundtrip) \| test(/^mermaid::export::/)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ W0 | ⬜ pending |
| 84-02-02 | 02 | 2 | REND-06 | T-84-06, T-84-08, T-84-09 | no statement injection; entity smuggling encoded; total on degenerate IRs | proptest + golden + Node + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_export) \| binary(mermaid_export_props) \| test(/^mermaid::/)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ W0 | ⬜ pending |
| 84-02-03 | 02 | 2 | REND-06 | T-84-10 | sequence `;`/wrap/`#` rules; opaque actor ids | proptest + golden + Node + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_export_seq) \| binary(mermaid_export) \| test(/^mermaid::/)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ W0 | ⬜ pending |
| 84-03-01 | 03 | 2 | MIG-01 | T-84-14 | cross-scope rewrite to dotted paths (83-07 gap) | tracer (integration) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(mermaid_import) \| binary(mermaid_roundtrip) \| test(/^mermaid::import::/)'` | ❌ W0 | ⬜ pending |
| 84-03-02 | 03 | 2 | MIG-01 | T-84-13, T-84-14 | leniencies only; corpus outcomes exact | integration | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(mermaid_import) \| binary(mermaid_corpus) \| binary(mermaid_roundtrip) \| test(/^mermaid::import::/)'` | ❌ W0 | ⬜ pending |
| 84-03-03 | 03 | 2 | MIG-01 | T-84-11, T-84-12 | fail closed by line; bounded; never panics; drop_styling opt-in and listed | catalogue + adversarial + proptest + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_unsupported) \| binary(mermaid_import_props) \| binary(mermaid_import) \| binary(mermaid_corpus) \| binary(mermaid_roundtrip) \| test(/^mermaid::import::/)'` | ❌ W0 | ⬜ pending |
| 84-04-01 | 04 | 3 | MIG-01 | T-84-15 | state terminals mapped; state block converts | tracer (integration) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(mermaid_import_state) \| binary(mermaid_corpus) \| binary(mermaid_import) \| binary(mermaid_roundtrip) \| test(/^mermaid::import::/)'` | ❌ W0 | ⬜ pending |
| 84-04-02 | 04 | 3 | MIG-01 | T-84-15, T-84-16, T-84-17 | sequence subset strict; catalogues; property extended | catalogue + proptest + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_import_seq) \| binary(mermaid_import_state) \| binary(mermaid_import_props) \| binary(mermaid_corpus) \| binary(mermaid_unsupported) \| binary(mermaid_import) \| binary(mermaid_roundtrip) \| test(/^mermaid::import::/)'` | ❌ W0 | ⬜ pending |
| 84-05-01 | 05 | 4 | MIG-02 | T-84-18, T-84-19 | three-way equality over the corpus; independence proven by mutation | tracer (golden + Node + mutation) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(mermaid_roundtrip) \| binary(mermaid_corpus)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ W0 | ⬜ pending |
| 84-05-02 | 05 | 4 | MIG-02, MIG-01 | T-84-20 | coverage beyond the corpus; import∘export fixpoint | proptest + golden + Node | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_fixpoint) \| binary(mermaid_roundtrip) \| test(/^mermaid::/)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ❌ W0 | ⬜ pending |
| 84-05-03 | 05 | 4 | REND-06, MIG-01, MIG-02 | T-84-21, T-84-SC | governance clean; no new edges/manifests | gates | `target/debug/vox ci crate-api-sync` ; `cargo clippy -p vox-diagram --all-targets -- -D warnings` ; `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | ✅ (existing gates) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 is folded into 84-01 (its tracer creates the harness every later task uses):

- [ ] `docs-astro/package.json` + `pnpm-lock.yaml`: `mermaid` 11.17.2 and `jsdom` 30.0.1 exact devDependencies (user-approved 2026-10-16; written with pnpm 11)
- [ ] `docs-astro/tests/lib/mermaid-oracle.mjs`, `docs-astro/tests/unit/mermaid-oracle.test.mjs`
- [ ] `.github/workflows/docs-quality.yml`: `crates/vox-diagram/tests/fixtures/**` on both triggers, guarded in `workflow-paths.test.mjs`
- [ ] `crates/vox-diagram/tests/support/roundtrip.rs` (`canon_graph`, `assert_golden` with `UPDATE_GOLDEN=1`; `canon_sequence` in 84-02)
- [ ] `crates/vox-diagram/tests/mermaid_roundtrip.rs` + `tests/fixtures/import-cases/00-minimal.mmd`
- [ ] `crates/vox-diagram/tests/{freeze_corpus,mermaid_corpus}.rs` + `tests/fixtures/corpus/` (33 `.mmd` + `MANIFEST.json`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proofs (CI path filter; `"` encoding; colon guard; sequence wrap guard; importer `classDef` push; sequence `loop` push; importer membership) | REND-06, MIG-01, MIG-02 | Deliberate break-then-restore of guards (repo rule) | Break the guard with a before/after `grep -c` presence check, run the named Rust test and (where listed) bless under the mutation and run the Node fixture, see it fail, restore source and fixtures (`git checkout -- <fixture dir>`), confirm `git diff --quiet`; record in the plan SUMMARY |
| Hosted CI run (`tests` shards, `docs-quality` incl. `pnpm install --frozen-lockfile` on pnpm 11 and Node 24) | REND-06, MIG-02 | Nothing is pushed during the phase; docs-quality is path-filtered, not a required check | Push the milestone branch as a draft PR; confirm `docs-quality` ran (fixture paths changed) and `node --test "tests/unit/*.test.mjs"` passed with the oracle suite |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
