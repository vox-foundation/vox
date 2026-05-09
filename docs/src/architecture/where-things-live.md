---
title: "Where Things Live"
description: "Flat lookup table — concept to crate. Consult before adding code. Referenced by AGENTS.md and CLAUDE.md."
category: "architecture"
status: "current"
training_eligible: true
training_rationale: "Canonical concept-to-crate lookup; high-value for LLM navigation of the Vox workspace."
---

# Where Things Live

Flat lookup table for "I need to add/find X — where does it go?". Optimized
for LLM tool-call efficiency: skim by left column, jump to the right column.

If your concept doesn't appear here, **add the row in the same PR** — that
keeps the table accurate and prevents the next assistant from having to
guess. The full crate roster with layer assignments lives in
[`layers.toml`](./layers.toml).

## Quick reference: subsystem → crate (by layer)

### L0 — pure types

| Crate | One-line scope |
|---|---|
| [`vox-arch-check`](../../../crates/vox-arch-check/) | CI guard binary; enforces layers.toml. |
| [`vox-build-meta`](../../../crates/vox-build-meta/) | Build-time helper emitting `VOX_BUILD_NUMBER` / `VOX_GIT_HASH`; use as `[build-dependencies]` only. |
| [`vox-db-types`](../../../crates/vox-db-types/) | Pure-data L0 leaf for vox-db: row types, IDs, schema descriptors. |
| [`vox-mesh-types`](../../../crates/vox-mesh-types/) | Pure-data mesh transport types. |
| [`vox-orchestrator-types`](../../../crates/vox-orchestrator-types/) | Pure-data L0 leaf for vox-orchestrator: agent/task IDs, file affinity, switch actions, provider catalogs, VCS capability tokens (WorkingTreeWrite, BranchCreate, etc.). |
| [`vox-primitives`](../../../crates/vox-primitives/) | Dependency-neutral id and backoff helpers shared across workspace crates. |
| [`vox-protocol`](../../../crates/vox-protocol/) | Daemon wire-protocol pure-data types. |
| [`workspace-hack`](../../../crates/workspace-hack/) | Cargo-hakari unification crate; do not edit by hand. |

### L1 — primitives & utilities

| Crate | One-line scope |
|---|---|
| [`vox-bounded-fs`](../../../crates/vox-bounded-fs/) | UTF-8 file reads capped by vox-scaling-policy max_file_bytes_hint. |
| [`vox-checksum-manifest`](../../../crates/vox-checksum-manifest/) | SHA-256 release asset verification against checksums.txt manifests. |
| [`vox-crypto`](../../../crates/vox-crypto/) | Pure-Rust crypto primitives (chacha20poly1305 AEAD, ed25519, x25519); sole crypto SSOT per AGENTS.md §Cryptography Policy. |
| [`vox-exec-grammar`](../../../crates/vox-exec-grammar/) | AST parser and risk classifier for shell/Vox command invocations; backs exec-policy.v1.yaml enforcement. |
| [`vox-grammar-export`](../../../crates/vox-grammar-export/) | Exports the Vox grammar artifact for downstream tooling. |
| [`vox-identity`](../../../crates/vox-identity/) | Identity primitives: signing keys, trust ledger entries. |
| [`vox-jsonschema-util`](../../../crates/vox-jsonschema-util/) | Shared JSON Schema compile + validate helpers for CLI, contracts, and tooling. |
| [`vox-openai-sse`](../../../crates/vox-openai-sse/) | OpenAI-compat SSE line reassembly and chat completion delta extraction. |
| [`vox-openai-wire`](../../../crates/vox-openai-wire/) | OpenAI-compatible chat completions JSON types (non-streaming wire) shared across MCP and runtime. |
| [`vox-package-types`](../../../crates/vox-package-types/) | Pure-data L1 leaf for vox-package: manifest, lockfile, package_kind, resolver types. |
| [`vox-plugin-api`](../../../crates/vox-plugin-api/) | Shared API surface for Vox plugins: ABI version, traits, manifest types, error types. |
| [`vox-plugin-types`](../../../crates/vox-plugin-types/) | Pure-types surface for the vox plugin system: manifests, skill types, state-backend trait. |
| [`vox-telemetry`](../../../crates/vox-telemetry/) | L1 telemetry facade: `METRIC_TYPE_*` constants, `TelemetryRecorder` trait, `record_event!` macro. Zero domain dependencies. |
| [`vox-reqwest-defaults`](../../../crates/vox-reqwest-defaults/) | Shared reqwest ClientBuilder presets (user-agent, timeouts) for CLI, runtime, and AI transports. |
| [`vox-scaling-policy`](../../../crates/vox-scaling-policy/) | Compile-time and runtime accessors for scaling SSOT (contracts/scaling/policy.yaml). |
| [`vox-secrets`](../../../crates/vox-secrets/) | Central secret resolution and compatibility adapters for Vox. |

### L2 — domain libraries

| Crate | One-line scope |
|---|---|
| [`vox-capability-registry`](../../../crates/vox-capability-registry/) | Transport-independent capability registry (YAML SSOT) + Mens chat tool descriptors. |
| [`vox-config`](../../../crates/vox-config/) | Centralized configuration and env/default resolution for Vox tooling. |
| [`vox-constrained-gen`](../../../crates/vox-constrained-gen/) | Grammar-constrained inference engine — Earley/PDA backends, deadlock watchdog, stream-of-revision. |
| [`vox-doc-inventory`](../../../crates/vox-doc-inventory/) | Generate and verify docs/agents/doc-inventory.json (schema v3) without Python. |
| [`vox-eval`](../../../crates/vox-eval/) | Vox expression evaluator (interpreter for vox run --interp). |
| [`vox-install-policy`](../../../crates/vox-install-policy/) | SSOT constants for Vox install/update surfaces (source path, release targets, default GitHub coordinates). |
| [`vox-mcp-registry`](../../../crates/vox-mcp-registry/) | Compile-time MCP tool name/description registry from contracts YAML (SSOT). |
| [`vox-project-scaffold`](../../../crates/vox-project-scaffold/) | Shared Vox.toml + src/main.vox + skill scaffolding for vox init and MCP. |
| [`vox-repository`](../../../crates/vox-repository/) | Repository discovery, stable identity, layout probes, and agent scope helpers for external and internal Vox workspaces. |
| [`vox-share`](../../../crates/vox-share/) | Public-URL tunneling for Vox apps: Cloudflare Quick Tunnels (default), localhost.run (fallback), Tailscale Funnel (explicit). |
| [`vox-skill-runtime`](../../../crates/vox-skill-runtime/) | Abstract sandbox runtime trait for skill execution. Implementations ship as plugins (wasm, container). |

### L3 — heavy runtimes

| Crate | One-line scope |
|---|---|
| [`vox-actor-runtime`](../../../crates/vox-actor-runtime/) | Process-oriented runtime: actors, mailboxes, supervision, scheduling, LLM/Mens activity primitives. |
| [`vox-cli-ci`](../../../crates/vox-cli-ci/) | vox CLI 'ci' subcommand dispatcher (sync-ignore-files, secret-env-guard, etc.). Extracted from vox-cli to isolate CI-only edits. Implementation files remain in vox-cli/src/commands/ci/ pending bounded_read refactor; this crate is the workspace boundary marker. |
| [`vox-cli-core`](../../../crates/vox-cli-core/) | Shared internals for the vox CLI binary (argv parsing helpers, exit-code policy). |
| [`vox-code-audit`](../../../crates/vox-code-audit/) | AI code quality stub detector — finds stubs, magic values, empty bodies, missing references, and DRY violations. |
| [`vox-drift-check`](../../../crates/vox-drift-check/) | Workspace drift and pattern-repetition linter (multi-language: Rust, TypeScript, Vox). |
| [`vox-codegen`](../../../crates/vox-codegen/) | Codegen + WebIR + vox_ir extracted from vox-compiler. Consumes analysis types from vox-compiler. |
| [`vox-compiler`](../../../crates/vox-compiler/) | Unified Vox compiler: lexer, parser, AST, HIR, typechecker, and codegen. |
| [`vox-container`](../../../crates/vox-container/) | OCI container runtime abstraction — supports Docker and Podman. |
| [`vox-corpus`](../../../crates/vox-corpus/) | Training data contracts, preflight, corpus SSOT, and Mens dataset metadata. |
| [`vox-dashboard`](../../../crates/vox-dashboard/) | Local Axum-served orchestration dashboard (SPA host). |
| [`vox-db`](../../../crates/vox-db/) | Codex / VoxDb facade: schema migrations, store ops, Turso/libSQL access for the Vox workspace. |
| [`vox-deploy-codegen`](../../../crates/vox-deploy-codegen/) | Deployment artifact codegen: Dockerfile, Compose, K8s, Fly, Coolify, systemd. Pure text generation. |
| [`vox-doc-pipeline`](../../../crates/vox-doc-pipeline/) | Doc generator: regenerates SUMMARY.md, architecture-index.md, feed.xml from frontmatter. |
| [`vox-package`](../../../crates/vox-package/) | Vox package manager runtime: content-addressed artifact cache, registry HTTP client, workspace discovery. |
| [`vox-forge`](../../../crates/vox-forge/) | Platform-agnostic Git forge API — GitHub, GitLab, Gitea, Forgejo. |
| [`vox-gamify`](../../../crates/vox-gamify/) | Gamification layer — companions, quests, battles, and free AI integration. |
| [`vox-git`](../../../crates/vox-git/) | Pure-Rust Git bridge using gix (no C, no libgit2). |
| [`vox-lsp`](../../../crates/vox-lsp/) | Vox Language Server (stdio JSON-RPC). |
| [`vox-ml-cli`](../../../crates/vox-ml-cli/) | Vox ML, AI, and Telemetry command-line interface (binary tool). |
| [`vox-openclaw-runtime`](../../../crates/vox-openclaw-runtime/) | OpenClaw client + ARS runtime adapter, executor, context bundles, hooks. |
| [`vox-orchestrator`](../../../crates/vox-orchestrator/) | Glue crate for the multi-agent file-affinity router: dei_shim, planning, services, runtime glue. Core router lives in vox-orchestrator-core, queue/lock/oplog in vox-orchestrator-queue, MCP in vox-orchestrator-mcp. |
| [`vox-orchestrator-core`](../../../crates/vox-orchestrator-core/) | Workspace boundary marker for the core router/dispatcher of vox-orchestrator (the `orchestrator/` subdir, ~11.5K LoC). Full extraction blocked by 30+ `crate::` cross-cuts into sibling modules; code remains in vox-orchestrator until a broader L3 split lands. |
| [`vox-orchestrator-mcp`](../../../crates/vox-orchestrator-mcp/) | MCP (Model Context Protocol) tool layer for vox-orchestrator. Extracted in 2026-05-08 reorg Phase 4. |
| [`vox-orchestrator-queue`](../../../crates/vox-orchestrator-queue/) | Locks, oplog, and affinity tracking for vox-orchestrator. Extracted in 2026-05-08 reorg Phase 5. |
| [`vox-orchestrator-test-helpers`](../../../crates/vox-orchestrator-test-helpers/) | Test-only fixtures and mocks for vox-orchestrator: MockBulletinBoard, load_golden_fixture. |
| [`vox-oratio`](../../../crates/vox-oratio/) | Speech-to-text (Oratio) — Candle Whisper (Rust) STT and transcript refinement. |
| [`vox-plugin-catalog`](../../../crates/vox-plugin-catalog/) | SSOT catalog of all first-party Vox plugins and distribution bundles. |
| [`vox-plugin-host`](../../../crates/vox-plugin-host/) | Host-side plugin discovery, loading, and registry. |
| [`vox-populi`](../../../crates/vox-populi/) | Vox Populi: multi-node worker registry, HTTP control plane, and Mens native ML (Burn / Candle QLoRA). |
| [`vox-publisher`](../../../crates/vox-publisher/) | Unified news syndication and publishing for Vox. |
| [`vox-scientia-ingest`](../../../crates/vox-scientia-ingest/) | Scientia corpus ingestion pipeline. |
| [`vox-search`](../../../crates/vox-search/) | Local-first retrieval execution: memory hybrid, repo inventory, Codex chunks, policy, and optional lexical/vector backends. |
| [`vox-skills`](../../../crates/vox-skills/) | Skill marketplace and plugin architecture for the Vox agent system. |
| [`vox-ssg`](../../../crates/vox-ssg/) | Static site generator for the Vox docs surface. |
| [`vox-tensor`](../../../crates/vox-tensor/) | Pure-CPU JSONL data loaders / training-pair types (Burn extracted 2026-05-08). |
| [`vox-test-harness`](../../../crates/vox-test-harness/) | Shared test fixtures and harness primitives. |
| [`vox-wasm-engine`](../../../crates/vox-wasm-engine/) | Single-source-of-truth Wasmtime engine + WASI execution for Vox programs and skill plugins. |
| [`vox-webhook`](../../../crates/vox-webhook/) | HTTP webhook gateway for the Vox agent system. |
| [`vox-workflow-runtime`](../../../crates/vox-workflow-runtime/) | Interpreted workflow execution MVP (local + mens activity hooks). |

### L5 — surfaces

| Crate | One-line scope |
|---|---|
| [`vox-cli`](../../../crates/vox-cli/) | Vox command-line interface: compile, run, bundle, and workspace diagnostics. |
| [`vox-integration-tests`](../../../crates/vox-integration-tests/) | Cross-crate integration test harness (test-only L5). |
| [`vox-orchestrator-d`](../../../crates/vox-orchestrator-d/) | Vox orchestrator daemon binary. Extracted from vox-orchestrator in 2026-05-08 reorg Phase 4. |

## Common tasks → exact path

| I want to... | The right place |
|---|---|
| Add an MCP tool | `crates/vox-orchestrator-mcp/src/<group>_tools.rs` (e.g. `git_tools.rs`); register dispatch in [`mcp/dispatch.rs`](../../../crates/vox-orchestrator-mcp/src/dispatch.rs) |
| Add an HTTP route (orchestrator) | `crates/vox-orchestrator-mcp/src/services/routes/` |
| Add a CLI subcommand | `crates/vox-cli/src/commands/<group>.rs` + register in [`commands/mod.rs`](../../../crates/vox-cli/src/commands/mod.rs) |
| Add a CI subcommand under `vox ci` | `crates/vox-cli/src/commands/ci/` |
| Add a new CI/db guard | `crates/vox-cli/src/commands/ci/<name>.rs` + register in `cmd_enums.rs` and `run_body.rs`. Mirror `db_schema_coverage.rs`. |
| Local `act` configuration (catalog image pin, platform map) | `.actrc` (repo root) |
| Self-hosted CI runner image | `Dockerfile.ci-runner` (repo root); published via `.github/workflows/publish-ci-runner.yml` to GHCR |
| Extend `vox ci pre-push` modes | `crates/vox-cli/src/commands/ci/pre_push.rs` — add `Step` to `build_steps` or extend `PrePushOpts` |
| Add a `Db<Entity>Id` newtype | `crates/vox-db-types/src/ids.rs` (use the `string_id!` macro). |
| Add a DB store operation | `crates/vox-db/src/<concept>.rs` (impl block on `VoxDb`) |
| Add a pure-data DB row type | `crates/vox-db-types/src/store_types/` (NOT `vox-db`) |
| Add a pure-data DB type | `crates/vox-db-types/src/` |
| Add an orchestrator type (Agent/Task/etc.) | `crates/vox-orchestrator-types/src/agent_types/` |
| Add an orchestrator policy module (D1–D10) | `crates/vox-orchestrator/src/<module>.rs` + register in `lib.rs` + add row to this table |
| Add a research-pipeline stage (claims/gate/planner/provider/types/verifier) | `crates/vox-orchestrator/src/dei_shim/research/<module>.rs`. Phase 0a stubs; Phase 1 replaces claim/verifier with `vox-claim-extractor` calls. |
| Orchestrator policy façade (all D1–D10) | `crates/vox-orchestrator/src/orchestrator_policy.rs` |
| Circuit breaker — doom-loop detection (D6) | `crates/vox-orchestrator/src/circuit_breaker.rs` |
| Confidence fusion — Socrates trigger (D3) | `crates/vox-orchestrator/src/confidence_fusion.rs` |
| Tier cascade — model routing (D1) | `crates/vox-orchestrator/src/tier_cascade.rs` |
| Plan-mode trigger — React vs. plan (D2) | `crates/vox-orchestrator/src/planning/plan_mode_trigger.rs` |
| Risk matrix — HITL escalation (D5+D9) | `crates/vox-orchestrator/src/risk_matrix.rs` |
| Privacy classifier — sensitivity detection (D8) | `crates/vox-orchestrator/src/privacy_classifier.rs` |
| Cache predictor — prefix cache routing (D7) | `crates/vox-orchestrator/src/cache_predictor.rs` |
| Budget gate — token/cost limits (D7) | `crates/vox-orchestrator/src/budget_gate.rs` |
| Compaction trigger — strategy selection (D7) | `crates/vox-orchestrator/src/compaction_trigger.rs` |
| Calibration — drift detection + bandit (D10) | `crates/vox-orchestrator/src/calibration.rs` |
| Sub-agent dispatch — spawn vs. inline (D4) | `crates/vox-orchestrator/src/subagent_dispatch.rs` |
| Orchestrator policy metric_type constants | `crates/vox-telemetry/src/types.rs` — `METRIC_TYPE_*` constants |
| Orchestrator feature flags | `contracts/orchestration/feature-flags.v1.yaml` |
| Add a code-audit detection rule | `crates/vox-code-audit/src/detectors/<rule>.rs` |
| Add a skill manifest field | `crates/vox-plugin-types/src/skill_manifest.rs` |
| Add a plugin manifest field | `crates/vox-plugin-types/src/plugin_manifest.rs` |
| Add a queue / lock / oplog method | `crates/vox-orchestrator-queue/src/{locks,oplog,affinity}/` |
| Add an LLM provider adapter | `crates/vox-orchestrator-mcp/src/llm_bridge/providers/<name>.rs` |
| Add a code generator (Rust target) | `crates/vox-codegen/src/codegen_rust/` |
| Add a code generator (TypeScript target) | `crates/vox-codegen/src/codegen_ts/` |
| Add a layer rule / arch-check rule | `crates/vox-arch-check/src/main.rs` + extend `layers.toml` schema |
| Add an architectural exception (allowed inversion) | Append `[[known_inversions]]` block in [`layers.toml`](./layers.toml) with a `reason` |
| Add a new workspace crate | Update [`Cargo.toml`](../../../Cargo.toml) `[workspace.dependencies]` AND add a row to [`layers.toml`](./layers.toml) — `vox-arch-check` will fail otherwise |

> **L0/L1 split:** if your consumer only needs row/param TYPES (no async, no
> connection), depend on `vox-db-types` directly — not on `vox-db`. The full
> `vox-db` crate transitively pulls in `turso` and tokio.

## Plugins (L4 — cdylib only; never compile-time deps for L0..L3)

If you're writing a plugin (concrete sandbox, ML backend, GPU probe, etc.),
it goes in a new `crates/vox-plugin-<name>/` and depends on `vox-plugin-api`.
Don't depend on `vox-orchestrator` or `vox-cli` from a plugin.

| Plugin crate | Provides |
|---|---|
| [`vox-plugin-browser`](../../../crates/vox-plugin-browser/) | Browser automation plugin (chromiumoxide CDP). |
| [`vox-plugin-cloud`](../../../crates/vox-plugin-cloud/) | CloudSync plugin stub: HF Hub / S3 model artifact sync. |
| [`vox-plugin-grammar-export`](../../../crates/vox-plugin-grammar-export/) | Export Vox grammar in standard formats (Lark, EBNF, JSON Schema, XGrammar-2, etc.). |
| [`vox-plugin-mens-candle-cuda`](../../../crates/vox-plugin-mens-candle-cuda/) | ML training backend plugin: Candle + CUDA. Implements MlBackend. |
| [`vox-plugin-nvml-probe`](../../../crates/vox-plugin-nvml-probe/) | Hardware probe plugin: NVML for NVIDIA GPU introspection. |
| [`vox-plugin-oratio`](../../../crates/vox-plugin-oratio/) | Speech-to-text plugin: Candle Whisper backend extracted from vox-oratio. |
| [`vox-plugin-oratio-mic`](../../../crates/vox-plugin-oratio-mic/) | AudioCapture plugin stub: Oratio microphone device backend. |
| [`vox-plugin-populi-mesh`](../../../crates/vox-plugin-populi-mesh/) | Populi mesh transport plugin (composite: code + skill). |
| [`vox-plugin-publication`](../../../crates/vox-plugin-publication/) | Publication plugin: RSS/Atom ingest with dedup, Reddit/YouTube publish, scholarly job feeds. |
| [`vox-plugin-runtime-container`](../../../crates/vox-plugin-runtime-container/) | Skill-runtime plugin: Docker + Podman backends for vox-skill-runtime. |
| [`vox-plugin-runtime-wasm`](../../../crates/vox-plugin-runtime-wasm/) | Skill-runtime plugin: wasmtime-based WASI sandbox (default for pure-compute skills). |
| [`vox-plugin-script-execution`](../../../crates/vox-plugin-script-execution/) | ScriptExecutor plugin stub: sandboxed .vox script runner. |
| [`vox-plugin-webhook`](../../../crates/vox-plugin-webhook/) | Webhook plugin: HTTP listener with HMAC signature verification (GitHub, GitLab, generic). |

## When to NOT add a new crate

The default answer to "should this be a new crate?" is **no**. Add to an
existing crate unless one of these is true:

### Binary-only tools

Crates with `kind = "binary"` in `layers.toml` (e.g., `vox-arch-check`, `vox-ml-cli`, `vox-orchestrator-d`) don't need a `[workspace.dependencies]` entry in the root `Cargo.toml` — they're consumed via `cargo run -p <name>`, not as library dependencies. The "Add a new workspace crate" instruction below applies to libraries only.

- The new code has zero callers in any existing crate (likely a plugin)
- The new code is **pure types** (no async, no DB) AND will have ≥3 consumers (consider an L0 or L1 crate)
- A subsystem in an existing crate has grown past its `max_loc` budget and is asking to be split (see Phase 4–5 of the [reorg outcome](./2026-05-08-workspace-reorg-outcome.md))

`vox-arch-check`'s orphan detector flags new crates with no consumers. If you
add one, expect that warning to land on your PR until you wire it up — that's
working as intended.
