# Changelog

All notable changes to the Vox project are documented here.

## [Unreleased]

### Changed

- **`ServerState::new_test`**: uses **`OrchestratorConfig::for_testing()`** ( **`toestub_gate: false`**, tight limits) so MCP tests do not inherit production post-task behavior that can run nested **`cargo check --workspace`** for Rust manifests. The **`conflict_diff_success_payload_has_expected_keys`** contract test no longer calls **`complete_task`**; it builds an in-memory snapshot + **`record_conflict`** only (avoids multi-minute / LNK1104-prone paths). Production **`ServerState::new`** is unchanged.
- **Eval benchmark matrix**: `contracts/eval/benchmark-matrix.schema.json` now uses a fixed **`enum`** for each `benchmark_classes` entry (no arbitrary strings). `vox-cli` `eval_matrix` centralizes crate / feature / test-filter literals, runs **`ci command-compliance`** checks **in-process** (`command_compliance::run`) so `eval-matrix run` does not spawn `cargo run` (avoids Windows `vox.exe` replacement locks), and has **drift tests** (matrix JSON ∪ milestones ↔ Rust SSOT ↔ schema enum). GitLab **`vox-ci-guards`** runs **`ci line-endings`** and **`ci command-compliance`** to match the GitHub guard slice.
- **`vox fmt` / `vox install`**: now **exit with an error** (honest stub) with pointers to `docs/src/ref-cli.md` until `vox-fmt` / `vox-pm` are wired.
- **Examples**: `examples/actor.vox` and `examples/mcp_tool.vox` moved to **`examples/archive/legacy_syntax/`** (non-parseable on current grammar); added **`examples/STYLE.md`**, **`FEATURE_INDEX.md`**, **`PARSE_STATUS.md`**, archive READMEs.
- **`vox-parser` `parity_test`**: optional **`VOX_EXAMPLES_STRICT_PARSE=1`** requires every `examples/**/*.vox` to parse (default CI remains golden-only).
- **`vox-cli`**: default features are **`mens-base` only** (no **`gpu`**). Native Mens train / probe / merge / eval-local require **`cargo build -p vox-cli --features gpu`**. **`vox-mens`** binary (prepends `mens` subcommand). **`vox-codex`** removed as a **`vox-cli`** dependency (`codex` / `stub-check` use **`vox-db`**); OS keyring helpers live in **`vox_db::secrets`** (**`vox-codex`** still re-exports for other crates). **`vox-corpus`** and **`vox-runtime`** are **always** linked so grammar / training JSONL paths work even with **`--no-default-features`** (`mens-base` remains the **command-surface** gate).
- **`vox-ludus`**: depends on **`vox-db`** instead of **`vox-codex`**.

### Added

- **Docs / scripts:** [`docs/src/how-to/examples-corpus.md`](docs/src/how-to/examples-corpus.md), [`docs/src/how-to/first-full-stack-app.md`](docs/src/how-to/first-full-stack-app.md); [`scripts/examples_strict_parse.sh`](scripts/examples_strict_parse.sh), [`scripts/examples_strict_parse.ps1`](scripts/examples_strict_parse.ps1).
- **Tests:** `budgets_json_loads_and_defines_all_timing_lanes`, `tool_registry_slice_tolerates_bracket_in_description` (LLM-audit hardening).
- **`vox ci build-timings`** — wall-clock **`cargo check`** for default `vox-cli`, GPU+stub lane, optional CUDA lane (`--json` supported). **`--crates`** adds `vox-cli --no-default-features`, `vox-db`, `vox-oratio`, `vox-mens --features train`, and **`vox-cli --features oratio`** lanes. Optional soft budgets: `docs/ci/build-timings/budgets.json` with **`VOX_BUILD_TIMINGS_BUDGET_WARN=1`** / **`VOX_BUILD_TIMINGS_BUDGET_FAIL=1`**. GitHub CI runs **`build-timings --crates`** in place of the standalone GPU/stub check step.
- **`oratio`** feature — Oratio / `vox-oratio` is no longer pulled by default **`mens-base`**; enable **`--features oratio`** for **`vox oratio`** (canonical speech CLI; alias **`vox speech`**). CI feature matrix includes an **`oratio`** compile lane.
- **Docs**: `docs/src/architecture/vox-cli-build-feature-inventory.md` (feature / compile-impact map); `docs/src/architecture/crate-topology-buckets.md` (workspace crate buckets); `docs/ci/build-timings/` (`budgets.json`, `snapshot-metadata.json`, optional `latest.jsonl`); migration matrix + deviation notes in `crate-build-lanes-migration.md`.
- **CLI / Mens**: `vox mens pipeline` — dogfood corpus → eval → optional native train (replaces PS1 orchestration); `scripts/run_mens_pipeline.ps1` is a thin delegate.
- **Scripts / builtins**: `std.process.run_capture` (stdout/stderr/exit record; non-zero exit still `Ok`); `std.fs.glob` (sorted paths); `vox-runtime` helpers `vox_process_run_capture`, `vox_fs_glob`.
- **Compilerd**: JSON `run` params accept optional **`mode`** (`auto` \| `app` \| `script`) aligned with `vox run --mode`.
- **CI**: `vox ci check-docs-ssot` scans `docs/src` and `.github/workflows` for retired Python inventory / `populi_release_gate.sh` references; `FEATURE_SETS` includes `script-execution` matrix rows; docs [command surface duals](docs/src/ci/command-surface-duals.md).
- **GitLab**: `vox-ci-guards` aligned with GitHub (`toestub-scoped`, `cuda-features`, `mens-gate --profile ci_full`); `NATIVE_TRAIN=false` rejected on `ml-train`.
- **CLI / CI**: `vox ci` guard commands (manifest, SSOT checks, doc-inventory, workflow allowlist, Mens `gates.yaml` runner, TOESTUB scoped, CUDA probes); crate `vox-doc-inventory`.
- **Scripts**: `ci_full` Mens profile; GitHub `ci.yml` cut over to `cargo run -p vox-cli -- ci …`; thin shell/ps1 delegates; `docs/agents/script-registry.json` + baseline metrics.
- **Run**: `vox run --mode {auto,app,script}`; `vox script` (feature `script-execution`); frontend bundle path uses **pnpm**; script builtins: `print`, `std.env.get`, `std.process.run` / `exit`, `std.fs.list_dir`, `std.args`.
- **Parser**: Trailing comma support in function parameter lists (A-072/A-100)
- **Parser**: Duplicate parameter name detection with clear error message (A-074/A-101)
- **Parser**: Error recovery test coverage (A-099)
- **Parser**: `filter_fields` support in `VectorIndexDecl` parsing
- **Typeck**: Lambda parameter type checking test (A-092)
- **Typeck**: Lambda outer scope capture test (A-093)
- **Typeck**: Match arm variable binding test (A-094)
- **Typeck**: Match exhaustiveness error test (A-095)
- **Store**: `CodeStore::dry_run_migration()` — report pending migrations without applying (B-059)
- **Store**: `CodeStore::health_check()` — `PRAGMA integrity_check` wrapper (B-060)
- **Store**: `CodeStore::batch_insert()` for bulk artifact insertion (B-062)
- **Store**: Pagination support (`LIMIT`/`OFFSET`) in `list_components` (B-063)
- **Store**: Relevance threshold filtering in `recall_memory` (B-064)
- **VoxDb**: `DbConfig::from_env()` for environment-based configuration (B-065)
- **VoxDb**: Retry logic (3× with backoff) in `VoxDb::connect` (B-066)
- **VoxDb**: `VoxDb::transaction()` wrapper for atomic operations (B-067)
- **VoxDb**: Integration test for in-memory connection (B-068)
- **AGENTS.md**: Phase 5 VoxPM roadmap merged from `PLAN.md` (B-076)
- **Docs**: `vox-runtime/README.md` — actor model architecture (B-112)
- **Docs**: `vox-pm/README.md` — CAS store architecture (B-113)
- **Docs**: mdBook search enabled with full-text indexing (A-136)
- **Docs**: Automated API reference pipeline `vox doc` (A-142)
- **Docs**: Decorator and Keyword manifests in JSON format (B-121/B-122)
- **Docs**: OpenGraph/SEO metadata and social sharing support (B-125)
- **Docs**: RSS/Atom feed generation for release notes (B-124)
- **CI**: Documentation build check and Rustdoc integration (B-117/B-118)
- **CI**: Dashboard API `dead_code` warnings suppressed (future integration)
- **OpenCode CLI**: `vox opencode` subcommand tree (install, setup, doctor, status, dashboard, spawn, review, config, sync, logs, share)
- **OpenCode CLI**: `vox opencode install` — downloads OpenCode AI and scaffolds config
- **OpenCode CLI**: `vox opencode doctor` — preflight check (binary, MCP, LSP, config, version)
- **OpenCode CLI**: `vox opencode dashboard` — launches embedded real-time agent dashboard
- **OpenCode CLI**: `vox completions <shell>` — generate shell completion scripts
- **OpenCode CLI**: `vox mcp-docs` — auto-generate MCP tool reference markdown table
- **OpenCode Integration**: `opencode.json` with version pinning (`opencode_version: >=0.2.0`)
- **OpenCode Integration**: Plugin API compatibility shim for OpenCode < 0.2.0
- **OpenCode Integration**: GitHub Actions workflow for `vox opencode doctor` in CI
- **MCP Server**: Protocol version negotiation (server echoes client's `protocolVersion`)
- **MCP Server**: 34 new tools (102 total): A2A messaging, VCS snapshots, JJ-inspired oplog/conflicts/workspaces, OpenCode bridge tools
- **MCP Tools**: `vox_map_agent_session`, `vox_record_cost`, `vox_heartbeat`, `vox_cost_history` (older docs/plugins may reference `vox_map_opencode_session` — use the canonical name)
- **MCP Tools**: `vox_a2a_send`, `vox_a2a_inbox`, `vox_a2a_ack`, `vox_a2a_broadcast`, `vox_a2a_history`
- **MCP Tools**: `vox_snapshot_*`, `vox_oplog`, `vox_undo`, `vox_redo`
- **MCP Tools**: `vox_workspace_*`, `vox_conflict_*`, `vox_change_*`, `vox_vcs_status`
- **Dashboard**: Redesigned with dark theme, glassmorphism, D3.js topology, SSE event log
- **Dashboard**: VCS panel, gamification panel, cost charts
- **Docs**: `docs/opencode-integration.md` — user-facing setup guide
- **Docs**: `docs/architecture/opencode-bridge.md` — technical deep-dive
- **Docs**: `docs/mcp-tool-reference.md` — auto-generated from 102 MCP tool schemas
- **Docs**: `docs/troubleshooting-faq.md` — common issues: port conflicts, MCP timeouts, LSP crashes
- **AGENTS.md**: Updated with 102 MCP tool list, OpenCode bridge section, new documentation links
- **CLI UX**: Colored output with actionable error suggestions in `vox opencode` commands


### Fixed
- **Docs SSOT:** `orphan-surface-inventory.md` workspace crate list includes **`vox-dei`** (excluded crate under `crates/vox-dei`) so `vox ci check-docs-ssot` matches filesystem inventory.
- **`vox ci build-timings`:** Soft budgets load only from **`docs/ci/build-timings/budgets.json`** (no duplicate Rust const); `VOX_BUILD_TIMINGS_BUDGET_FAIL=1` works without `BUDGET_WARN`; `--json` serialization errors surface with context; `nvcc` probe uses **`CUDA_PATH`** / **`CUDA_HOME`** when `PATH` is stripped.
- **`vox ci command-compliance`:** `TOOL_REGISTRY` slice uses stable **anchor** before `pub fn tool_registry` so `]` inside description strings cannot break parsing.
- **`vox ci command-compliance`**: MCP wiring validates `TOOL_REGISTRY` vs canonical-only `handle_tool_call` arms and checks `crates/vox-mcp/src/tools/tool_aliases.rs` (`TOOL_WIRE_ALIASES` → canonical names).
- **`vox-cli`**: `benchmark_telemetry::record_opt_blocking` no longer mixes `std::io::Error` with Codex `StoreError` in a single `Result` chain.
- **Store**: Replaced `.unwrap()` on embedding `try_into()` with proper error handling (B-056)
- **Normalize**: All `AstNode` variants now have explicit cases (no wildcard fallthrough) (B-058)
- **LSP**: Removed unused imports in `main.rs`

### Removed
- `PLAN.md` — content merged into `AGENTS.md` §3 (B-076)
