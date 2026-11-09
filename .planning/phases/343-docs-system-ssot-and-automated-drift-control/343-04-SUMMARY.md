# Phase 343 Wave 4 (343-04) Summary: Reader-Facing CLI Claims Enforcement & SSOT Alignment

## Executive Summary
Wave 4 of Phase 343 resolved all reader-facing documentation findings at their source, eliminated `REPORT_ONLY` mode, updated the documentation SSOT and coverage ledger, and enforced zero CLI claim drift across both agent files and reader-facing documentation. The `vox ci ssot-drift` gate now runs `docs_cli_claims` as a blocking verification stage.

## Source Changes

### 1. Reader-Facing Documentation Fixes
| File | Before | After | Reason |
|---|---|---|---|
| `docs/src/how-to/how-to-deploy.md:136` | `vox package --kind docker` | `vox container build --tag my-vox-app` | `package` is not a CLI verb; container builder is `vox container build` |
| `docs/src/how-to/how-to-deploy.md:155` | `All `vox package` and `vox deploy` options.` | `All `vox container` and `vox deploy` options.` | Canonical command reference alignment |
| `docs/src/how-to/how-to-mcp-vox-validate.md:151` | `vox mcp-server` | `vox mcp` | Real MCP stdio server command is `vox mcp` |
| `docs/src/how-to/troubleshooting-faq.md:33` | `vox agent doctor` | `vox doctor` | Diagnostic doctor command is `vox doctor` |
| `docs/src/how-to/troubleshooting-faq.md:56,68` | `vox dashboard` | `vox gui` / `VOX_GUI_PORT` | Axum dashboard retired in ADR-037; Tauri GUI is `vox gui` |
| `docs/src/how-to/troubleshooting-faq.md:92` | `vox agent status` | `vox mcp` | Agent mapping inspection is handled via orchestrator / `vox mcp` |
| `docs/src/how-to/troubleshooting-faq.md:106` | `vox agent sync` | Prose instruction | Nonexistent CLI verb replaced with agent file edit instructions |
| `docs/src/how-to/troubleshooting-faq.md:113` | `vox orchestrator spawn` | Prose instruction | Nonexistent CLI verb replaced with AI session task submission guidance |
| `docs/src/reference/tavily-integration-ssot.md:242` | `vox search "test query" --tavily` | `Tavily integration test (`cargo test -p vox-search --test tavily_dispatcher_test`)` | Tavily integration is tested via `vox-search` test suite, not a standalone CLI flag |

### 2. Normalizer & Checker Improvements (`crates/vox-cli/src/commands/ci/docs_cli_claims.rs`)
- **Delegated Commands**: Added delegation awareness for `populi`, `mens`, `speech`, and `oratio`, which forward trailing arguments and flags directly to the underlying `vox-ml-cli` binary.
- **Workflow / Chain Arrows**: Added recognition of unicode arrow `→` and `->` as sequence separators so command workflows in code spans (e.g. in `docs/src/reference/cli-vox-deploy.md`) evaluate cleanly.

### 3. Enforcing Mode (`crates/vox-cli/src/commands/ci/docs_cli_claims.rs`)
- Deleted the `REPORT_ONLY` constant and warning branch.
- `docs_cli_claims::run()` now fails immediately on any finding in either `Scope::AgentFiles` or `Scope::ReaderFacing`.
- Replaced temporary measurement tests with:
  - `real_tree_reader_facing_clean`: Enforces 0 reader-facing findings across all `status: current` pages in `docs/src/`.
  - `run_errors_on_reader_facing_unknown_command`: Verifies that a synthetic reader-facing page containing an unknown command fails `run()`.

### 4. SSOT Map & Detector Ledger Updates
- `docs/src/architecture/docs-system-ssot.md`:
  - Added gate row in §4: `CLI-claims check (`docs_cli_claims` ssot-drift stage)` (detects prescriptive `vox` commands/flags and stale planned claims; runs in pre-push fast and PR CI).
  - Cleaned up §6 by removing the closed gap bullets for `Stale "planned" claims` and `Prose vs CLI surface`.
- `docs/src/contributors/detector-coverage-ledger.md`:
  - Added detector row: Bug class `Docs name a nonexistent CLI command/flag or call a shipped command planned`, detector id `docs_cli_claims`, kind `Rust stage, pulldown-cmark mentions + clap tree`, severity `blocking`, enforcement `pre-push fast + PR CI linux via ssot-drift`.

## Verification & Mutation Proofs

### Automated Test Results
- `cargo test -p vox-cli --lib docs_cli_claims`: 14 passed; 0 failed.
- `cargo test -p vox-cli --lib ssot_drift`: 12 passed; 0 failed.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths ...`: Passed on all touched documentation files with no hard errors.
- `CI=1 ./target/debug/vox ci ssot-drift`: Passed all stages, including `overlay_parity` (12ms), `docs_cli_claims` (201ms), and `ssot_regen_parity` (1ms).

### Mutation Proof (Task 2)
- Injected `Run `vox nosuch-command --now`.` into reader-facing page `docs/src/reference/cli-vox-deploy.md`.
- Executed `cargo test -p vox-cli --lib docs_cli_claims::tests::real_tree_reader_facing_clean`.
- Verified the test failed naming `cli-vox-deploy.md` and `nosuch-command`.
- Restored `cli-vox-deploy.md` byte-identically (`cmp -s`).

### Baseline Invariant
- Verified with `git status --porcelain -- contracts`: No baseline files were created, grown, or modified. Zero baseline needed.
