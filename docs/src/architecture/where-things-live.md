# Where Things Live

Flat lookup table for "I need to add/find X — where does it go?". Optimized
for LLM tool-call efficiency: skim by left column, jump to the right column.

If your concept doesn't appear here, **add the row in the same PR** — that
keeps the table accurate and prevents the next assistant from having to
guess. The full crate roster with layer assignments lives in
[`layers.toml`](./layers.toml).

## Quick reference: subsystem → crate

| Subsystem | Crate |
|---|---|
| Compiler (lex/parse/HIR/typecheck) | [`vox-compiler`](../../../crates/vox-compiler/) |
| Codegen (lowering, IR, target emission) | [`vox-codegen`](../../../crates/vox-codegen/) |
| Database access (turso) | [`vox-db`](../../../crates/vox-db/) |
| Database pure-data types | [`vox-db-types`](../../../crates/vox-db-types/) |
| Orchestrator core | [`vox-orchestrator`](../../../crates/vox-orchestrator/) |
| Orchestrator MCP tool layer | [`vox-orchestrator-mcp`](../../../crates/vox-orchestrator-mcp/) |
| Orchestrator queue / locks / oplog | [`vox-orchestrator-queue`](../../../crates/vox-orchestrator-queue/) |
| Orchestrator pure-data types | [`vox-orchestrator-types`](../../../crates/vox-orchestrator-types/) |
| Orchestrator daemon binary | [`vox-orchestrator-d`](../../../crates/vox-orchestrator-d/) |
| Code-quality / stub detection | [`vox-code-audit`](../../../crates/vox-code-audit/) |
| Plugin host (loader + registry) | [`vox-plugin-host`](../../../crates/vox-plugin-host/) |
| Plugin types (manifest + traits) | [`vox-plugin-types`](../../../crates/vox-plugin-types/) |
| Plugin ABI (abi_stable surface) | [`vox-plugin-api`](../../../crates/vox-plugin-api/) |
| Skill registry / marketplace | [`vox-skills`](../../../crates/vox-skills/) |
| Skill execution trait | [`vox-skill-runtime`](../../../crates/vox-skill-runtime/) |
| Actor / process primitives | [`vox-actor-runtime`](../../../crates/vox-actor-runtime/) |
| OpenClaw skill executor | [`vox-openclaw-runtime`](../../../crates/vox-openclaw-runtime/) |
| Workflow MVP (interpreted) | [`vox-workflow-runtime`](../../../crates/vox-workflow-runtime/) |
| Mesh transport | [`vox-populi`](../../../crates/vox-populi/) |
| Mesh pure-data types | [`vox-mesh-types`](../../../crates/vox-mesh-types/) |
| ML / training / inference CLI | [`vox-ml-cli`](../../../crates/vox-ml-cli/) |
| Speech-to-text (Whisper) | [`vox-oratio`](../../../crates/vox-oratio/) |
| Package metadata + artifact registry | [`vox-package`](../../../crates/vox-package/) |
| Gamification (quests, companions) | [`vox-gamify`](../../../crates/vox-gamify/) |
| Secret resolution | [`vox-secrets`](../../../crates/vox-secrets/) |
| Identity (signing, trust ledger) | [`vox-identity`](../../../crates/vox-identity/) |
| Configuration | [`vox-config`](../../../crates/vox-config/) |
| Daemon wire protocol types | [`vox-protocol`](../../../crates/vox-protocol/) |
| Bounded filesystem | [`vox-bounded-fs`](../../../crates/vox-bounded-fs/) |
| Crypto primitives | [`vox-crypto`](../../../crates/vox-crypto/) |
| LSP server | [`vox-lsp`](../../../crates/vox-lsp/) |
| Dashboard server | [`vox-dashboard`](../../../crates/vox-dashboard/) |
| Static site generator | [`vox-ssg`](../../../crates/vox-ssg/) |
| Documentation pipeline | [`vox-doc-pipeline`](../../../crates/vox-doc-pipeline/) |
| Build-time version metadata injection | [`vox-build-meta`](../../../crates/vox-build-meta/) — use as `[build-dependencies]` only |

## Common tasks → exact path

| I want to... | The right place |
|---|---|
| Add an MCP tool | `crates/vox-orchestrator-mcp/src/<group>_tools.rs` (e.g. `git_tools.rs`); register dispatch in [`mcp/dispatch.rs`](../../../crates/vox-orchestrator-mcp/src/dispatch.rs) |
| Add an HTTP route (orchestrator) | `crates/vox-orchestrator-mcp/src/services/routes/` |
| Add a CLI subcommand | `crates/vox-cli/src/commands/<group>.rs` + register in [`commands/mod.rs`](../../../crates/vox-cli/src/commands/mod.rs) |
| Add a CI subcommand under `vox ci` | `crates/vox-cli/src/commands/ci/` |
| Add a DB store operation | `crates/vox-db/src/<concept>.rs` (impl block on `VoxDb`) |
| Add a pure-data DB row type | `crates/vox-db-types/src/store_types/` (NOT `vox-db`) |
| Add a pure-data DB type | `crates/vox-db-types/src/` |
| Add an orchestrator type (Agent/Task/etc.) | `crates/vox-orchestrator-types/src/agent_types/` |
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

## Plugins (delivered as cdylib — never compile-time deps for L0..L3)

If you're writing a plugin (concrete sandbox, ML backend, GPU probe, etc.),
it goes in a new `crates/vox-plugin-<name>/` and depends on `vox-plugin-api`.
Don't depend on `vox-orchestrator` or `vox-cli` from a plugin.

Existing plugins for reference:
- ML/training: [`vox-plugin-mens-candle-cuda`](../../../crates/vox-plugin-mens-candle-cuda/)
- Speech (mic): [`vox-plugin-oratio-mic`](../../../crates/vox-plugin-oratio-mic/)
- Sandbox runtime (WASM): [`vox-plugin-runtime-wasm`](../../../crates/vox-plugin-runtime-wasm/)
- Sandbox runtime (container): [`vox-plugin-runtime-container`](../../../crates/vox-plugin-runtime-container/)

## When to NOT add a new crate

The default answer to "should this be a new crate?" is **no**. Add to an
existing crate unless one of these is true:

- The new code has zero callers in any existing crate (likely a plugin)
- The new code is **pure types** (no async, no DB) AND will have ≥3 consumers (consider an L0 or L1 crate)
- A subsystem in an existing crate has grown past its `max_loc` budget and is asking to be split (see Phase 4–5 of the [reorg outcome](./2026-05-08-workspace-reorg-outcome.md))

`vox-arch-check`'s orphan detector flags new crates with no consumers. If you
add one, expect that warning to land on your PR until you wire it up — that's
working as intended.
