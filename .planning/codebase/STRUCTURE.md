---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
# Codebase Structure

**Analysis Date:** 2026-09-22

## Directory Layout

```
vox/
├── crates/                # Rust workspace — 138 crates, layered L0-L5 (see ARCHITECTURE.md)
│   ├── vox-cli/            # `vox` binary — CLI entry point
│   ├── vox-gui/             # Tauri desktop shell (Rust) + ui/ (React frontend)
│   ├── vox-compiler/        # Lexer/parser/AST/HIR/typeck/interpreter
│   ├── vox-orchestrator*/   # Multi-agent router, queue, MCP tools, daemon (split crates)
│   ├── vox-db*/             # DB facade + pure-data row types
│   ├── vox-plugin-*/        # L4 cdylib plugins (ML backends, sandboxes, browser, skills)
│   └── workspace-hack/      # cargo-hakari unification crate — never hand-edit
├── docs/                  # Narrative documentation SSOT (Markdown, doctested)
│   └── src/
│       ├── architecture/    # SSOT docs: where-things-live.md, layers.toml, ADR-adjacent research
│       ├── adr/              # Architecture Decision Records
│       ├── contributors/     # Contributor guides, governance, agent-instruction docs
│       ├── reference/        # Auto-generated + hand-written reference docs
│       ├── ci/                # CI/runner contract docs
│       └── archive/           # Tombstoned — never ingest (see AGENTS.md §Archival Protocol)
├── docs-astro/             # Astro-built docs site (sidebar generated from docs/ frontmatter)
├── contracts/              # YAML/JSON Schema SSOT bundles consumed by generators + CI gates
├── apps/                  # GUI surfaces outside vox-gui proper (editor, interop, experimental, marquee)
├── clients/                # React Native / web client SDKs generated/consumed outside crates/
├── scripts/                # `.vox` automation scripts, run via `vox run` (VoxScript-first policy)
├── examples/               # golden/, forbidden/, aspirational/, mens/, cli-tour/ .vox fixtures
├── tests/                  # Cross-cutting fixtures and top-level integration test files
├── mens/                   # MENS model training artifacts: config, data, runs, schemas, logs
├── server/telemetry/       # Standalone ClickHouse telemetry ingest server (own Cargo workspace)
├── tools/                  # Small standalone scripts (e.g. hardcoded-values-audit.mjs)
├── tree-sitter-vox/         # Tree-sitter grammar for `.vox` syntax highlighting/parsing
├── infra/, docker/          # Deploy/compose configs (Fly, Coolify, k8s, Docker Compose)
├── .github/workflows/       # CI workflow definitions
├── AGENTS.md / CLAUDE.md    # Cross-tool agent policy (source of truth for conventions)
└── Cargo.toml               # Workspace manifest — `[workspace.dependencies]`, lints, profiles
```

## Directory Purposes

**`crates/`:**

- Purpose: all first-party Rust code; the Cargo workspace root (`members = ["crates/*", "crates/workspace-hack"]`).
- Contains: 138 crates, each assigned a layer (L0-L5) in `docs/src/architecture/layers.toml`.
- Key files: `docs/src/architecture/where-things-live.md` (concept→crate lookup, canonical navigation table).

**`crates/vox-cli/src/commands/`:**

- Purpose: one file/module per CLI subcommand group.
- Contains: `commands/<group>.rs` files (e.g. `ci/`, `db_cli.rs`), registered in `commands/mod.rs`.
- Key files: `crates/vox-cli/src/main.rs` (dispatch table + doc comment listing every subcommand → module mapping).

**`crates/vox-orchestrator/src/`:**

- Purpose: multi-agent routing policy engine.
- Contains: one file per policy concern (`tier_cascade.rs`, `circuit_breaker.rs`, `budget_gate.rs`, `risk_matrix.rs`, `calibration.rs`, ...), plus subsystem dirs (`a2a/`, `agentos/`, `context/`, `memory/`, `drain_oplog/`).
- Key files: `orchestrator_policy.rs` (façade), `bootstrap.rs` (repo-scoped construction).

**`crates/vox-orchestrator-mcp/src/`:**

- Purpose: MCP tool layer over `vox-orchestrator`.
- Contains: `<group>_tools.rs` per tool family (git, agent, browser, benchmark, gamify, ...), `dispatch.rs` (central router), `chat_tools/`, `aci/` (AgentOS envelope).
- Key files: `dispatch.rs`, `src/bin/gui-visual-review.rs`.

**`crates/vox-compiler/src/`:**

- Purpose: the `.vox` language frontend + interpreter.
- Contains: `lexer/`, `parser/`, `hir/`, `typeck/`, `eval/` (interpreter), `fmt/` (formatter), `contract_ir/`.
- Key files: `pipeline.rs` (frontend orchestration), `eval/value.rs` (`VoxValue`), `eval/caps.rs` (`CapabilitySet`).

**`crates/vox-gui/`:**

- Purpose: Tauri 2 desktop application.
- Contains: `src/` (Rust: `commands/`, `config/`, `drive/`, `main.rs`), `ui/` (React/TypeScript frontend: `src/components/`, `src/lib/`, `e2e/` Playwright specs).
- Key files: `src/main.rs` (Tauri builder + command registration), `ui/src/App.tsx`, `ui/src/transport.ts` (Tauri IPC bridge).

**`docs/src/architecture/`:**

- Purpose: architecture SSOT documents — the project's own map of itself.
- Contains: `where-things-live.md` (crate lookup table), `layers.toml` (machine-enforced layer rules), dated research/design docs (`*-2026.md`, `*-design.md`).
- Key files: `where-things-live.md`, `layers.toml`.

**`contracts/`:**

- Purpose: YAML/JSON Schema single sources of truth consumed by code generators and CI drift gates.
- Contains: per-domain subdirs (`aci/`, `ci/`, `db/`, `frontend/`, `scientia/`, `policy/`, ...), `index.yaml` (contract registry).
- Key files: `contracts/index.yaml`, `contracts/policy/policy-registry.v1.yaml`.

**`scripts/`:**

- Purpose: all project automation, written in `.vox` and run via `vox run` (never new `.sh`/`.ps1`/`.py` glue — see AGENTS.md §VoxScript-First Glue Code).
- Contains: 71 `.vox` scripts (e.g. `fmt.vox`, `install-hooks.vox`, `clean-build-artifacts.vox`).
- Key files: `scripts/fmt.vox`, `scripts/install-hooks.vox`.

**`examples/`:**

- Purpose: non-production `.vox` sandboxes and compile/parse fixtures.
- Contains: `golden/` (must-compile reference programs, doctested), `forbidden/` (regression cases with `// expect-error:` headers, run through `forbidden_corpus_test.rs`), `aspirational/`, `mens/`, `cli-tour/`, `compile-suite/`, `parser-inventory/`.
- Key files: `examples/PARSE_STATUS.md`, `examples/examples.ssot.v1.yaml`.

**`apps/`:**

- Purpose: GUI/editor surfaces that are not `vox-gui` proper.
- Contains: `apps/editor/vox-vscode` (VS Code extension host), `apps/interop/marquee_app`, `apps/experimental/visualizer`, `apps/build-tools/render-durable-animation/`.
- Key files: `contracts/frontend/surface-ownership.v1.yaml` (canonical registry of which app owns which surface).

**`mens/`:**

- Purpose: MENS (the project's fine-tuned model) training artifacts.
- Contains: `config/`, `data/`, `runs/`, `schemas/`, `logs/`, `baseline-evals/`, `staging/`.
- Generated: mostly yes (training run outputs); `B8-TRAINING-GATES.md` is hand-maintained.

## Key File Locations

**Entry Points:**

- `crates/vox-cli/src/main.rs`: `vox` CLI binary — argv parsing, subcommand dispatch table.
- `crates/vox-gui/src/main.rs`: Tauri desktop shell binary.
- `crates/vox-orchestrator-d/src/bin/`: standalone orchestrator daemon binary.
- `crates/vox-orchestrator-mcp/src/dispatch.rs`: MCP tool router (not a `main.rs`, but the effective request entry point).
- `crates/vox-lsp/src/`: language server stdio entry point.

**Configuration:**

- `Cargo.toml` (root): workspace members, `[workspace.dependencies]`, lints, build profiles.
- `docs/src/architecture/layers.toml`: architectural layer/dependency rules, enforced by `vox-arch-check`.
- `contracts/index.yaml`: registry of all SSOT contract bundles.
- `.env` / `.env.example`: environment configuration (never commit real secrets — see AGENTS.md).
- `AGENTS.md` (root, symlinked as `CLAUDE.md`): cross-tool agent policy SSOT.

**Core Logic:**

- `crates/vox-compiler/src/pipeline.rs`: compiler frontend orchestration.
- `crates/vox-codegen/src/projection_bundle.rs`: HIR → downstream-emitter bundle assembly.
- `crates/vox-orchestrator/src/orchestrator_policy.rs`: agent routing policy façade.
- `crates/vox-db/src/`: `VoxDb` store operations (one file per concept, e.g. `crates/vox-db/src/<concept>.rs`).

**Testing:**

- Per-crate `crates/<name>/tests/` — integration tests colocated with each crate.
- `crates/vox-cli-tests/`: end-to-end `vox build`/`vox new` CLI harness (subprocess-based, test-only L5 crate).
- `crates/vox-integration-tests/`: cross-crate integration harness (test-only L5).
- `crates/vox-gui/ui/e2e/`: Playwright visual/interaction specs (`installTauriMock`/`installTauriMockRich` harnesses); screenshots land in `crates/vox-gui/ui/review-bundle/latest/`.
- `examples/forbidden/`: negative-case regression corpus (`// expect-error:` header convention).

## Naming Conventions

**Files:**

- Rust: `snake_case.rs`, one concern per file where possible (god-object rule caps at 500 non-blank lines, enforced by `arch/god_object`).
- Vox source: `.vox` extension; golden examples live under `examples/golden/**/*.vox`.
- Generated files: carry a `@generated-hash <hex>` header comment — never hand-edit (`vox/codegen/generated-file-drift` lint); regenerate via the owning generator.
- Docs: `*-research-2026.md`, `*-findings-2026.md` for research; `*-ssot.md` or descriptive names for architecture SSOT docs; all under `docs/src/`.

**Directories:**

- Crate naming: `vox-<domain>` for libraries, `vox-<domain>-types` / `vox-<domain>-core` for narrow pure-data extraction crates used to avoid heavy dependency edges (e.g. `vox-db-types`, `vox-orchestrator-types`, `vox-container-types`).
- Plugin crates: `vox-plugin-<name>` (L4, cdylib only).
- CLI-extraction crates: `vox-cli-<concern>` (e.g. `vox-cli-ci`, `vox-cli-research`, `vox-cli-review`, `vox-cli-share`) — hold code pulled out of `vox-cli` purely to respect layer/LoC budgets; consumed exclusively by `vox-cli`.
- Test-only crates: `kind = "test-only"` in `layers.toml`, typically named `vox-<domain>-test-helpers` or `vox-<domain>-tests`.

## Where to Add New Code

**New CLI subcommand:**

- Primary code: `crates/vox-cli/src/commands/<group>.rs`, registered in `crates/vox-cli/src/commands/mod.rs`.
- New CI subcommand specifically: `crates/vox-cli/src/commands/ci/<name>.rs`, registered in `cmd_enums.rs` and `run_body.rs`.

**New MCP tool:**

- Implementation: `crates/vox-orchestrator-mcp/src/<group>_tools.rs`.
- Registration: dispatch entry in `crates/vox-orchestrator-mcp/src/dispatch.rs`.

**New orchestrator policy module (D1-D10 style):**

- Implementation: `crates/vox-orchestrator/src/<module>.rs`, registered in `lib.rs`.
- Also add a row to `docs/src/architecture/where-things-live.md`'s "Common tasks → exact path" table.

**New HTTP route (orchestrator):**

- `crates/vox-orchestrator-mcp/src/services/routes/`.

**New DB store operation / row type:**

- Store op: `crates/vox-db/src/<concept>.rs` (impl block on `VoxDb`).
- Pure-data row type: `crates/vox-db-types/src/store_types/` (never in `vox-db` — keep L0/L1 split so type-only consumers don't pull in Turso/tokio).
- New ID newtype: `crates/vox-db-types/src/ids.rs` (use the `string_id!` macro).

**New workspace crate:**

- Update `Cargo.toml` `[workspace.dependencies]` AND add a row to `docs/src/architecture/layers.toml` — `vox-arch-check` fails otherwise.
- Also add a row to `docs/src/architecture/where-things-live.md` in the same PR.

**New GUI surface/panel:**

- Rust commands: `crates/vox-gui/src/commands/<surface>.rs`, registered via `generate_handler!` in `main.rs`.
- React component: `crates/vox-gui/ui/src/components/surfaces/<Surface>/`.
- Register in `contracts/gui/surface-registry.v1.yaml` and `contracts/frontend/surface-ownership.v1.yaml`.
- Required: a Playwright visual stepper spec under `crates/vox-gui/ui/e2e/` before merge (see AGENTS.md §GUI Visual Verification Invariant).

**New Vox language enforcement rule / lint:**

- Detector: `crates/vox-code-audit/src/detectors/<rule>.rs`.

**New skill (agent-facing):**

- Directory under `.vox/skills/`, `.agents/skills/`, or `.claude/skills/` (workspace or `~`), spec-compliant `SKILL.md`. Parser SSOT: `crates/vox-plugin-host/src/skill_parser.rs`.

**Utilities:**

- Cross-crate shared helpers that would otherwise create a heavy edge: consider a narrow new `-types`/`-core` L0/L1 crate, or duplicate a <50-line helper with a `// vox:defactored-from <crate> <date>` comment (defactor policy) rather than adding a workspace dependency edge.

## Special Directories

**`docs/src/archive/`:**

- Purpose: tombstoned historical research/plans.
- Generated: No (hand-authored, then frozen).
- Committed: Yes — but never read/ingested by agents when planning new work (AGENTS.md §Archival Protocol).

**`crates/workspace-hack/`:**

- Purpose: `cargo-hakari` feature-unification crate keeping build times sane across 138 crates.
- Generated: Yes (by `cargo hakari generate`).
- Committed: Yes — never hand-edit.

**`target/`, `graphify-out/`, `.sccache/`, `.worktrees/`:**

- Purpose: build artifacts, code-intelligence graph cache, sccache cache, git worktree checkouts.
- Generated: Yes.
- Committed: No (gitignored).

**`.vox/`:**

- Purpose: project-local Vox runtime state (policy status overlay, cache, skill discovery roots).
- Generated: Partially (`policy-status/<branch>.json`, `cache/`).
- Committed: No (gitignored per repo convention; distinct from the tracked `crates/vox-*` source).

**`server/telemetry/`:**

- Purpose: standalone ClickHouse-backed telemetry ingest server.
- Generated: No.
- Committed: Yes — but deliberately its **own** Cargo workspace, excluded from the root `[workspace] exclude` list so ClickHouse deps never enter the main cargo graph or `vox-arch-check`.

**`docs-astro/dist/`, `docs/src/SUMMARY.md`, `docs/src/feed.xml`:**

- Purpose: Astro-built docs site output and generated sidebar/RSS.
- Generated: Yes, at Astro build time from `docs/src/` frontmatter.
- Committed: No (gitignored) — do not hand-create or edit.

---

*Structure analysis: 2026-09-22*
