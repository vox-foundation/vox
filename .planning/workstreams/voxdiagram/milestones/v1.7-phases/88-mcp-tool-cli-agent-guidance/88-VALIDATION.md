---
phase: "88"
slug: "mcp-tool-cli-agent-guidance"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-17"
---

# Phase 88 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest over `cargo test` binaries. `vox-diagram`: lib unit tests (`validate::tests`) plus integration binaries `validate_envelope` (insta JSON goldens; insta already has the `json` feature) and `grammar_card`. `vox-orchestrator-mcp`: lib unit tests (`diagram_tools::tests`, existing `dispatch::registry_dispatch_tests`, `input_schemas::tests`, `registry::*`) plus the new integration binary `validate_diagram_tool` (end-to-end through `handle_tool_call`; tiktoken-rs, already a normal dependency, measures the card). `vox-cli`: lib unit tests (`commands::diagram::tests`) plus integration binaries `diagram_cli` (spawns `CARGO_BIN_EXE_vox` over temp files and stdin; `tempfile` dev-dependency) and the existing `command_catalog_paths_baseline`. `vox-mcp-registry` profile tests. No new dependency |
| **Config file** | `.config/nextest.toml` (profiles `default`, `ci`, `ci-gate`; no per-crate config) |
| **Quick run command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 --lib --test validate_envelope --test grammar_card -E 'test(/^validate::/) \| binary(validate_envelope) \| binary(grammar_card)'` |
| **Full suite command** | `cargo nextest run -p vox-diagram --retries 0` ; `cargo nextest run -p vox-orchestrator-mcp --retries 0 --lib --test validate_diagram_tool -E 'binary(validate_diagram_tool) \| test(/^diagram_tools::/) \| test(/registry_dispatch_tests::/) \| test(/^input_schemas::tests::/) \| test(/^registry::/)'` ; `cargo nextest run -p vox-cli --retries 0 --lib --test diagram_cli --test command_catalog_paths_baseline -E 'test(/^commands::diagram::/) \| binary(diagram_cli) \| binary(command_catalog_paths_baseline)'` ; `cargo nextest run -p vox-mcp-registry --retries 0` |
| **Estimated runtime** | vox-diagram filters ~15 s after the first build; the vox-orchestrator-mcp and vox-cli test binaries take minutes to build cold (background them), seconds to run |

Filter rules: nextest exits non-zero when a filter selects no test, which is the failing signal for a mistyped
filter. Integration binaries are selected with `binary(<file stem>)` and the matching `--test <stem>` target flag so
only the needed binaries are built (`--lib` adds the unit tests). Generated artifacts are verified by the `vox ci`
gates below, run through `target/debug/vox`, which must be rebuilt (`cargo build -p vox-cli`, background) after
every commit because its freshness guard compares commit counts; never set `VOX_SKIP_FRESHNESS_CHECK`.

---

## Sampling Rate

- **After every task commit:** `cargo check -p <touched crate> --all-targets` plus the task's own filter (backgrounded; never block the foreground on cargo).
- **After every plan wave:** the quick run command plus that plan's MCP or CLI filter from the full suite.
- **Before `/gsd-verify-work`:** 88-03 Task 3 closing gates: `target/debug/vox ci ssot-drift`, `command-sync`, `command-compliance`, `crate-edges`, `crate-api-sync`, `affected-crates --check`, `profile-parity`, `check-links`, `gui-surface-registry`; `cargo run -p vox-arch-check`; `cargo hakari generate --diff`; `target/debug/vox ci pre-push` (fast tier); `cargo fmt -p vox-diagram -p vox-orchestrator-mcp -p vox-cli --check`; the full suite; `cargo nextest run -p vox-doc-pipeline -E 'test(/^pipeline::diagrams::/)'`; phase-wide `git diff --numstat 82b24d2d4 -- contracts/ci/crate-edges.allow.v1.json` = `7 0`. Hosted CI on the milestone PR is authoritative (clippy on vox-orchestrator-mcp and vox-cli, the `ui` job for the regenerated GUI data files).
- **Max feedback latency:** 60 seconds once the test binaries are built.

---

## Requirement → Automated Command Map

| Requirement | Success criterion | Automated command | Fails when |
|-------------|-------------------|-------------------|------------|
| TOOL-01 | SC1: broken diagram returns every diagnostic, `fixes[]`, `normalized_source`, `stats`; `from_mermaid` conversion or `unsupported`; deterministic, no model calls | `cargo nextest run -p vox-orchestrator-mcp --retries 0 --lib --test validate_diagram_tool -E 'binary(validate_diagram_tool) \| test(/^diagram_tools::/)'` | non-zero exit, "no tests to run", any FAIL |
| TOOL-01 | SC1: envelope JSON shape pinned | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 --lib --test validate_envelope -E 'test(/^validate::/) \| binary(validate_envelope)'` | non-zero exit, any FAIL, a pending `.snap.new` |
| TOOL-01 | SC2: catalog row, dispatch, schema, fixture, profile, generated artifacts, GUI coverage, one edge | `cargo nextest run -p vox-orchestrator-mcp --retries 0 --lib -E 'test(/registry_dispatch_tests::/) \| test(/^input_schemas::tests::/) \| test(/^registry::/)'` ; `cargo nextest run -p vox-mcp-registry --retries 0` ; `target/debug/vox ci ssot-drift && target/debug/vox ci crate-edges && target/debug/vox ci command-sync` | non-zero exit, any FAIL, a drift line, `NEW EDGE` |
| TOOL-02 | SC3: five subcommands on files and stdin, exit codes 0/1/2, `--json` parity, no vox-cli edge | `cargo nextest run -p vox-cli --retries 0 --lib --test diagram_cli -E 'test(/^commands::diagram::/) \| binary(diagram_cli)'` ; `git diff --quiet 82b24d2d4 -- crates/vox-cli/Cargo.toml` | non-zero exit, "no tests to run", any FAIL, a manifest diff |
| TOOL-02 | SC3: catalog rows, handler path, `cli.md`, surface registry, paths baseline | `cargo nextest run -p vox-cli --retries 0 --test command_catalog_paths_baseline` ; `target/debug/vox ci command-compliance && target/debug/vox ci gui-surface-registry` | non-zero exit, any line naming a `diagram` path |
| TOOL-03 | SC4: card at most ~600 tokens by measurement, both examples pass `check`, in the MCP description | `cargo nextest run -p vox-diagram --retries 0 --test grammar_card` ; `cargo nextest run -p vox-orchestrator-mcp --retries 0 --test validate_diagram_tool -E 'test(/grammar_card_fits_600_tokens\|tool_list_advertises_the_card/)'` | non-zero exit, any FAIL (count printed) |
| TOOL-03 | SC4: published page and AGENTS.md directive | `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/vox-diagram.md,reference/cli.md,architecture/where-things-live.md` ; `cargo run -p vox-doc-pipeline -- --lint-only --mermaid-only` ; `target/debug/vox ci check-links` | non-zero exit, any ERROR or `legacy-mermaid-` line, a broken link |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 88-01-01 | 01 | 1 | TOOL-01 | T-88-01 | one manifest line, one 7-line exception; registry only through generators | tracer (dispatch e2e + lib unit) | `cargo nextest run -p vox-orchestrator-mcp --retries 0 --lib --test validate_diagram_tool -E 'binary(validate_diagram_tool) \| test(/^diagram_tools::/) \| test(/registry_dispatch_tests::/) \| test(/^input_schemas::tests::/) \| test(/^registry::profile_backcompat_tests::/)'` ; `cargo nextest run -p vox-diagram --retries 0 --lib -E 'test(/^validate::/)'` ; `target/debug/vox ci crate-edges` | ❌ W0 (`validate.rs`, `diagram_tools.rs`, `tests/validate_diagram_tool.rs` created by the task) | ⬜ pending |
| 88-01-02 | 01 | 1 | TOOL-01 | T-88-02, T-88-03, T-88-04 | unknown fields and bad `emit` refused (mutation); render off the async workers; render failure surfaced | lib unit + insta goldens + dispatch e2e + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 --lib --test validate_envelope -E 'test(/^validate::/) \| binary(validate_envelope)'` ; `cargo nextest run -p vox-orchestrator-mcp --retries 0 --lib --test validate_diagram_tool -E 'binary(validate_diagram_tool) \| test(/^diagram_tools::/) \| test(/^input_schemas::tests::/)'` ; `cargo clippy -p vox-diagram --all-targets -- -D warnings` | ❌ W0 (`tests/validate_envelope.rs` and its snapshots created by the task) | ⬜ pending |
| 88-01-03 | 01 | 1 | TOOL-01, TOOL-03 | T-88-01, T-88-SC | ledger entry proven necessary (mutation); hakari clean | integration + gates + mutation | `cargo nextest run -p vox-orchestrator-mcp --retries 0 --lib --test validate_diagram_tool -E 'binary(validate_diagram_tool) \| test(/^diagram_tools::/) \| test(/^registry::/)'` ; `cargo nextest run -p vox-mcp-registry --retries 0` ; `target/debug/vox ci ssot-drift && target/debug/vox ci crate-edges && target/debug/vox ci crate-api-sync && target/debug/vox ci affected-crates --check` ; `cargo run -p vox-arch-check` | ✅ after 88-01-01 | ⬜ pending |
| 88-02-01 | 02 | 2 | TOOL-02 | T-88-07, T-88-09 | bounded read; no vox-cli manifest change | tracer (binary e2e + lib unit) | `cargo nextest run -p vox-cli --retries 0 --lib --test diagram_cli --test command_catalog_paths_baseline -E 'test(/^commands::diagram::/) \| binary(diagram_cli) \| binary(command_catalog_paths_baseline)'` ; `target/debug/vox ci command-compliance && target/debug/vox ci gui-surface-registry && target/debug/vox ci crate-edges` ; `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/cli.md` | ❌ W0 (`commands/diagram.rs`, `tests/diagram_cli.rs` created by the task) | ⬜ pending |
| 88-02-02 | 02 | 2 | TOOL-02 | T-88-06, T-88-08 | `fmt --write` never writes invalid input, atomic rename; exit-code guard proven by mutation | binary e2e + lib unit + mutation | `cargo nextest run -p vox-cli --retries 0 --lib --test diagram_cli --test command_catalog_paths_baseline -E 'test(/^commands::diagram::/) \| binary(diagram_cli) \| binary(command_catalog_paths_baseline)'` ; `target/debug/vox ci command-compliance && target/debug/vox ci command-sync && target/debug/vox ci crate-api-sync` | ✅ after 88-02-01 | ⬜ pending |
| 88-03-01 | 03 | 3 | TOOL-03 | T-88-10 | card and published copy cannot diverge | tracer (integration + doc lint) | `cargo nextest run -p vox-diagram --retries 0 --test grammar_card` ; `cargo nextest run -p vox-orchestrator-mcp --retries 0 --test validate_diagram_tool -E 'test(/grammar_card_fits_600_tokens\|tool_list_advertises_the_card/)'` ; `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/vox-diagram.md` | ✅ (`grammar_card.rs` exists; the page is created by the task) | ⬜ pending |
| 88-03-02 | 03 | 3 | TOOL-03 | T-88-11 | AGENTS.md pure insertion of at most 10 lines; links resolve; no Mermaid growth | doc lint + link check | `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/vox-diagram.md,reference/cli.md,architecture/where-things-live.md` ; `cargo run -p vox-doc-pipeline -- --lint-only --mermaid-only` ; `target/debug/vox ci check-links` | ✅ after 88-03-01 | ⬜ pending |
| 88-03-03 | 03 | 3 | TOOL-01, TOOL-02, TOOL-03 | T-88-SC | one ledger change phase-wide; manifests and exemptions unchanged | gates + full suite | `target/debug/vox ci ssot-drift && target/debug/vox ci command-sync && target/debug/vox ci command-compliance && target/debug/vox ci crate-edges && target/debug/vox ci crate-api-sync && target/debug/vox ci affected-crates --check && target/debug/vox ci profile-parity && target/debug/vox ci check-links` ; `cargo run -p vox-arch-check && cargo hakari generate --diff` ; the full suite ; `target/debug/vox ci pre-push` | ✅ (existing gates) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 is folded into each plan's tracer task (each creates its module and failing tests first):

- [ ] `crates/vox-diagram/src/validate.rs` with its inline test module and `crates/vox-orchestrator-mcp/src/diagram_tools.rs` with its inline tests; `crates/vox-orchestrator-mcp/tests/validate_diagram_tool.rs` (88-01 Task 1)
- [ ] `crates/vox-diagram/tests/validate_envelope.rs` and its four `validate_envelope__*.snap` goldens, created with `INSTA_UPDATE=always` once, reviewed, then verified with `INSTA_UPDATE=no` (88-01 Task 2)
- [ ] `crates/vox-cli/src/commands/diagram.rs` with its inline tests and `crates/vox-cli/tests/diagram_cli.rs` (88-02 Task 1)
- [ ] `docs/src/reference/vox-diagram.md` and the two new tests in `crates/vox-diagram/tests/grammar_card.rs` (88-03 Task 1)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proofs: ledger entry removal (88-01 T3), `deny_unknown_fields` removal (88-01 T2), exit-code mapping (88-02 T2) | TOOL-01, TOOL-02 | deliberate break-then-restore of guards (repo rule) | copy the file to the scratchpad, `grep -c` the guarded line before and after, run the named test or gate and see it fail, restore from the copy (`cmp` clean), rebuild if needed, rerun green; record file, mutation, failing test and message in the plan SUMMARY |
| Golden snapshot review | TOOL-01 | a first-time golden is accepted by a human-readable review, not by a test | after `INSTA_UPDATE=always`, read each new `.snap`: keys in the documented order, `normalized_source` present, no absolute paths or timestamps; then rerun with `INSTA_UPDATE=no` |
| Generated files outside a plan's list | TOOL-01, TOOL-02 | the regenerator may refresh unrelated stale artifacts (Phase 87 left capability-index stale) | name each such file in the SUMMARY and commit it only if `ssot-drift` needs it |
| GUI typecheck of regenerated data files | TOOL-01, TOOL-02 | `crates/vox-gui/ui/node_modules` is absent in this worktree | run `pnpm --dir crates/vox-gui/ui typecheck` if node_modules exists; otherwise record the deferral to hosted CI's `ui` job |
| Hosted CI on the milestone PR | TOOL-01, TOOL-02, TOOL-03 | nothing is pushed during the phase | push the milestone branch as a draft PR; `Check, Build, and Test (Rust)` must pass (full workspace suite because manifests and `contracts/` changed); clippy for vox-orchestrator-mcp and vox-cli runs there |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
