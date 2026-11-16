# Phase 343 Wave 3 (343-03) Summary: Automated Drift Control for CLI Claims in Docs & Agent Files

## Executive Summary
Wave 3 of Phase 343 implemented the `docs_cli_claims` verification subsystem (`crates/vox-cli/src/commands/ci/docs_cli_claims.rs`), registered it in the `vox ci ssot-drift` stage retry chain, enforced clean agent-file claims (commands, flags, stale "planned" claims), measured reader-facing documentation findings in report-only mode (11 findings total), and recorded the pre-authorized decision `option-a` for Wave 4.

## Implementation Details

### 1. `docs_cli_claims` Module (`crates/vox-cli/src/commands/ci/docs_cli_claims.rs`)
- **CLI Surface Inspection**: Reads the in-process compiled clap catalog via `crate::command_catalog::build_catalog()`, including all subcommands and clap aliases (`entry.aliases`). Reads root flags from `VoxCliRoot::command()`, and joins `contracts/cli/command-registry.yaml` for registry-only (gated) and retired commands.
- **Normalizer & Validator**:
  - Handles `vox ...` and `cargo run -p vox-cli [--] ...`.
  - Strips leading environment variables (e.g. `CI=1 vox ...`).
  - Stops at pipeline/shell operator tokens (`|`, `>`, `&&`, `;`, etc.).
  - Resolves longest matching command paths, supporting clap aliases (e.g. `vox secrets doctor` -> `vox secrets status`).
  - Checks `--flag` and `--flag=value` against compiled node flags, ancestor flags, and root flags (`--help` and `--version` accepted; short flags ignored).
  - Skips placeholder tokens (`<...>`, `[...]`, `{...}`, `$X`, `...`, UPPERCASE, paths/dots).
  - Skips checks for registry-only commands (flags not verified) and descriptive contexts (under `# Retired Surfaces` headings or role `Mentions`).
- **Stale Planned Claims Detection**:
  - `stale_planned_claims()` scans for inline code spans immediately followed by `(planned` or `planned per`. If the claimed command is already shipped and functional (`check_command == Ok`), flags it as a stale planned claim.
- **Stage Registration**:
  - Registered `pub(crate) mod docs_cli_claims;` in `crates/vox-cli/src/commands/ci/mod.rs`.
  - Added stage `docs_cli_claims` to `run_ssot_drift` in `crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs` directly preceding `ssot_regen_parity`.

### 2. Agent File Adjustments
- `.cursor/rules/data-storage-policy.mdc:22`: Replaced nonexistent `vox schema generate` mention with prose description (`- Regenerate schemas from the updated contract.`).
- `AGENTS.md:436`: Corrected hallucinated subcommand mention `vox ci dev-loop-guard` to `the dev-loop guard (`vox ci pre-push`, fast tier, local and CI) fails if a hook runs...`.

### 3. Verification & Mutation Proofs
- **Unit & Integration Tests**: 13 tests in `docs_cli_claims` passing (including `real_tree` and `real_tree_reader_facing_measure`).
- **Task 1 Mutation Proof**:
  - Injected `(planned per [` into `AGENTS.md` line 217 for `vox audit effort`.
  - Verified `cargo test -p vox-cli --lib docs_cli_claims::tests::real_tree` failed reporting that `vox audit effort` exists in the CLI but is described as planned.
  - Restored `AGENTS.md` byte-identically (`cmp -s`).
- **Task 2 Mutation Proof**:
  - Injected `--nosuch-flag` into `AGENTS.md:436` for `vox ci pre-push`.
  - Verified `real_tree` failed reporting `UnknownFlag { path: "ci pre-push", flag: "--nosuch-flag" }`.
  - Restored `AGENTS.md` byte-identically (`cmp -s`).

### 4. Task 2 Reader-Facing Measurement
The scan over all reader-facing pages (`status: current`, non-archive in `tutorials/`, `how-to/`, `reference/`, `explanation/`, `concepts/`) produced **11 total findings**:
- **Per-directory breakdown**:
  - `how-to`: 9
  - `reference`: 2
- **All 11 findings**:
  1. `docs/src/how-to/grand-network-quickstart.md:58`: `UnknownFlag { path: "populi", flag: "--dry-run" } in vox populi attest publish --dry-run --task-kinds text_infer,image_gen`
  2. `docs/src/how-to/how-to-deploy.md:136`: `UnknownCommand("package") in vox package --kind docker`
  3. `docs/src/how-to/how-to-deploy.md:155`: `UnknownCommand("package") in vox package`
  4. `docs/src/how-to/how-to-mcp-vox-validate.md:151`: `UnknownCommand("mcp-server") in vox mcp-server`
  5. `docs/src/how-to/troubleshooting-faq.md:33`: `UnknownCommand("agent") in vox agent doctor`
  6. `docs/src/how-to/troubleshooting-faq.md:56`: `UnknownCommand("dashboard") in vox dashboard`
  7. `docs/src/how-to/troubleshooting-faq.md:92`: `UnknownCommand("agent") in vox agent status`
  8. `docs/src/how-to/troubleshooting-faq.md:106`: `UnknownCommand("agent") in vox agent sync`
  9. `docs/src/how-to/troubleshooting-faq.md:113`: `UnknownCommand("orchestrator") in vox orchestrator spawn`
  10. `docs/src/reference/cli-vox-deploy.md:196`: `UnknownFlag { path: "new web", flag: "--dry-run" } in vox new web → vox deploy --dry-run → vox doctor --project`
  11. `docs/src/reference/tavily-integration-ssot.md:242`: `UnknownSubcommand { parent: "search", token: "\"test" } in vox search "test query" --tavily`

### 5. Task 3 Decision
- **Selected Option**: `option-a` ("fix every finding at its source in 343-04 (retired command -> replacement, wrong flag -> real flag), then enforce the reader-facing scope with no baseline").
- **Authorization**: Pre-authorized by user instruction for Phase 343 Wave 3/4 execution.
- **Next Wave**: Wave 4 (`343-04-PLAN.md`) will fix each finding at its source or adjust normalizer handling for arrow chains/quoted args, remove `REPORT_ONLY`, and enforce zero drift across both agent files and reader-facing docs.
