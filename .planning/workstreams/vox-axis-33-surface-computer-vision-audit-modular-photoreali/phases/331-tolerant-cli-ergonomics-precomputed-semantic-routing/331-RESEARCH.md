# Phase 331: Tolerant CLI Ergonomics & Precomputed Semantic Routing — Research

**Researched:** 2026-11-01
**Status:** Complete
**Canonical Architecture SSOT:** [`docs/src/architecture/cli-semantic-tolerance-research-2026.md`](../../../docs/src/architecture/cli-semantic-tolerance-research-2026.md)

## 1. Technical Findings & Codebase Grounding

Full research is persisted in [`docs/src/architecture/cli-semantic-tolerance-research-2026.md`](../../../docs/src/architecture/cli-semantic-tolerance-research-2026.md). Key implementation bindings verified against the codebase:

1. **Clap Prefix Unfolding & Global Flag Hoisting (`CLI-TOL-01`):**
   - [`crates/vox-cli/src/lib.rs`](../../../crates/vox-cli/src/lib.rs) defines `VoxCliRoot` (`#[derive(Parser)]`). Adding `infer_subcommands = true, infer_long_args = true` to `#[command(...)]` enables zero-overhead unambiguous prefix unfolding across the subcommand tree.
   - [`crates/vox-cli-core/src/lib.rs`](../../../crates/vox-cli-core/src/lib.rs) defines `GlobalOpts` (`color`, `json`, `verbose`, `quiet`). Currently verify whether `global = true` is set on every field so flags placed after subcommands (`vox secrets list --json`) or between subcommands parse cleanly.
   - [`crates/vox-cli/src/main.rs`](../../../crates/vox-cli/src/main.rs) calls `vox_cli::VoxCliRoot::parse()` on line 166. Replacing `parse()` with `try_parse_from` (or `parse_with_tolerance`) preserves the `0 ms` happy path and routes `Err(clap_err)` into `argv_normalize`.

2. **Order-Agnostic Token & Flag Normalizer (`CLI-TOL-02`):**
   - [`crates/vox-cli/src/command_catalog.rs`](../../../crates/vox-cli/src/command_catalog.rs) exposes `build_catalog() -> CommandCatalog` containing every `CommandCatalogEntry { path, command, aliases, capability_id, arguments, ... }`.
   - [`crates/vox-capability-registry/src/index.rs`](../../../crates/vox-capability-registry/src/index.rs) embeds `contracts/capabilities/capability-index.v1.json` (`bundled_index()`) and [`crates/vox-capability-registry/src/explain.rs`](../../../crates/vox-capability-registry/src/explain.rs) provides `resolve(index, query)`.
   - A new module `crates/vox-cli/src/argv_normalize.rs` can inspect failed `argv` tokens in `<1.5 ms` and classify:
     - **Verb↔Noun Token Permutation:** e.g., `["vox", "list", "secrets"]` → matches `["secrets", "list"]` in `CommandCatalog`.
     - **Unique Leaf Elision:** e.g., `["vox", "command-compliance"]` → matches unique leaf `["ci", "command-compliance"]`.
     - **Wrong-Subcommand Flag Redirection:** e.g., `["vox", "build", "--release", "app.vox"]` → detects `--release` is absent on `build` but present on sibling `compile` in the same `fabrica`/`app` lane.

3. **Side-Effect & Dual-Persona Safety Gate (`CLI-TOL-03`):**
   - Every operation in [`contracts/operations/catalog.v1.yaml`](../../../contracts/operations/catalog.v1.yaml) (and `IndexRow` in `capability-index.v1.json`) carries `side_effect_class` (`read_only`, `local_mutation`, `remote_mutation`).
   - Caller persona is determined by `std::io::IsTerminal` (`stdin().is_terminal() && stdout().is_terminal()`) combined with environment probes (`!global.json`, `VOX_MCP`, `CI`, `VOX_NONINTERACTIVE`).
   - On non-TTY / `--json` / agent invocations, any non-exact recovery exits with code `2` and emits `CliErrorRecovery` JSON (compatible with `aci.error_recovery`).

4. **Precomputed Semantic Synonym Lockfile & Drift Firewall (`CLI-TOL-04`):**
   - [`crates/vox-cli/src/commands/ci/operations_catalog.rs`](../../../crates/vox-cli/src/commands/ci/operations_catalog.rs) defines `CliProjection`. Extending it with `verb_aliases: Vec<String>`, `noun_aliases: Vec<String>`, and `foreign_idioms: Vec<String>` allows declarative and LLM-mined synonyms in `contracts/operations/catalog.v1.yaml`.
   - `vox ci cli-synonyms` (`--write` / check) compiles `contracts/cli/semantic-aliases.v1.json`, verifying cosine margins ($\ge 0.08$) via `vox_actor_runtime::llm::local_embed_text` (offline deterministic) or `gemini-embedding-2` / `gemini-3.5-flash-lite` structured output when opted in, and enforcing the antonym banlist in `vox ci command-compliance`.

## Validation Architecture

- **Framework:** `cargo test` (scoped per crate: `-p vox-cli-core`, `-p vox-cli`, `-p vox-capability-registry`).
- **Quick run command:** `timeout 900s cargo test -p vox-cli --lib argv_normalize`
- **Full suite command:** `timeout 900s cargo test -p vox-cli-core && timeout 900s cargo test -p vox-cli --test cli_tolerance_tests`
- **Max feedback latency:** 60 seconds.
