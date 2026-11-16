# Phase 331: Tolerant CLI Ergonomics & Precomputed Semantic Routing — Context

**Gathered:** 2026-11-01
**Status:** Ready for planning
**Design SSOT:** [`docs/src/architecture/cli-semantic-tolerance-research-2026.md`](../../../docs/src/architecture/cli-semantic-tolerance-research-2026.md)

<domain>
## Phase Boundary

Deliver a zero-runtime-LLM, `<1.5 ms` tolerant CLI execution layer for `vox` (`crates/vox-cli`, `crates/vox-cli-core`, `crates/vox-capability-registry`) plus a CI/contract-time semantic synonym compiler (`vox ci cli-synonyms --write` → `contracts/cli/semantic-aliases.v1.json`) gated by `vox ci command-compliance`.

The CLI must tolerate human memory gaps—unambiguous prefix completions (`vox sec doc`), global flag hoisting (`vox secrets list --json`), verb↔noun token permutations (`vox list secrets`), unique leaf elision (`vox command-compliance`), wrong-subcommand flag redirection (`vox build --release`), and compositional synonyms (`vox rm secret FOO`)—while preserving 100% determinism and structured 1-turn self-repair diagnostics for LLM agents and non-TTY scripts.
</domain>

<decisions>
## Implementation Decisions

### Zero-Runtime-LLM Resolution Architecture
- **D-01:** Never invoke an LLM or network embedding call on the synchronous CLI parse hot path. Valid commands must take the `0 ms` happy path (`VoxCliRoot::try_parse_from`), and parse errors must resolve in `<1.5 ms` entirely offline using in-memory `CommandCatalog`, `CapabilityIndex`, and `contracts/cli/semantic-aliases.v1.json`.
- **D-02:** Enable `#[command(infer_subcommands = true, infer_long_args = true)]` on `VoxCliRoot` (`crates/vox-cli/src/lib.rs`) and ensure all `GlobalOpts` flags (`--json`, `--quiet`, `--verbose`, `--color`) carry `global = true` in `crates/vox-cli-core` so unambiguous prefixes and trailing/interleaved global flags resolve natively in `clap`.
- **D-03:** Implement `crates/vox-cli/src/argv_normalize.rs` invoked only when `VoxCliRoot::try_parse_from` fails:
  1. Order-agnostic verb↔noun token permutation over `CommandCatalogEntry::path` (`vox list secrets` → `vox secrets list`).
  2. Globally unique leaf subcommand elision (`vox command-compliance` → `vox ci command-compliance`).
  3. Wrong-subcommand flag redirection (`vox build --release` → suggests `vox compile --release`).
  4. Compositional `verb_aliases` × `noun_aliases` and `foreign_idioms` lookup from `contracts/cli/semantic-aliases.v1.json`.
  5. Fallback shortlist via `vox_capability_registry::resolve(bundled_index(), ...)`.

### Dual-Persona & Side-Effect Safety Gate
- **D-04:** Gate auto-execution strictly by `side_effect_class` (`read_only`, `local_mutation`, `remote_mutation` from `contracts/operations/catalog.v1.yaml`) and caller persona (`stdin.is_terminal() && stdout.is_terminal() && !is_agent_or_ci_env()`):
  - Interactive TTY + `read_only` permutation/synonym: auto-execute and print a dim `stderr` canonical notice (`→ vox secrets list`).
  - Interactive TTY + `local_mutation` / `remote_mutation` permutation/synonym: prompt `[Y/n]` before executing (or fail closed with canonical hint when declined).
  - Non-TTY / `--json` / `CI=true` / `VOX_MCP=1` (LLM Agent & script persona): never prompt interactively and never silently auto-execute fuzzy/permuted mutations; exit with code `2` and emit a structured `CliErrorRecovery` / `aci.error_recovery` payload containing `error_code`, `attempted`, `canonical_suggestion`, `side_effect_class`, and `auto_executed: false`.

### Precomputed Semantic Lockfile & Drift Firewall
- **D-05:** Extend `CliProjection` in `crates/vox-cli/src/commands/ci/operations_catalog.rs` (`contracts/operations/catalog.v1.yaml`) with optional `verb_aliases`, `noun_aliases`, and `foreign_idioms` fields, and build `vox ci cli-synonyms` (`--write` / verify mode) emitting `contracts/cli/semantic-aliases.v1.json`.
- **D-06:** Enforce an explicit Antonym Banlist (`add`/`remove`, `start`/`stop`, `lock`/`unlock`, `set`/`get`, `import`/`export`, `enable`/`disable`) and zero alias-to-canonical or alias-to-alias collisions in `vox ci command-compliance`, plus cosine similarity margin verification ($\tau_{\text{min}} \ge 0.82$, $\Delta_{\text{margin}} \ge 0.08$) using `vox_actor_runtime::llm` (`gemini-3.5-flash-lite` structured output for candidate generation when operation hashes change, and `gemini-embedding-2` / `local_embed_text` for offline deterministic verification).
</decisions>

<specifics>
## Specific References

- Research & Design SSOT: `docs/src/architecture/cli-semantic-tolerance-research-2026.md`
- CLI root & dispatch: `crates/vox-cli/src/lib.rs`, `crates/vox-cli/src/main.rs`, `crates/vox-cli/src/command_catalog.rs`
- Global options: `crates/vox-cli-core/src/lib.rs`
- Capability index resolution: `crates/vox-capability-registry/src/explain.rs`, `crates/vox-capability-registry/src/index.rs`
- Operations catalog & compliance: `crates/vox-cli/src/commands/ci/operations_catalog.rs`, `crates/vox-cli/src/commands/ci/command_compliance/mod.rs`
</specifics>
