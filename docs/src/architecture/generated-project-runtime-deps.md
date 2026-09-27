---
title: "Generated project runtime deps"
description: "How a vox build-generated Cargo.toml locates vox-owned runtime crates outside a vox checkout, and which ones it lists."
category: "Architecture SSOTs"
status: "current"
---

## Problem

`vox build src/main.vox` (default `AxumLocalServer` shell) and the Tauri
shell both emit a `Cargo.toml` for the generated project with hardcoded
relative path dependencies on vox's own runtime crates, e.g.:

```toml
vox-db = { path = "../../crates/vox-db" }
vox-actor-runtime = { path = "../../crates/vox-actor-runtime" }
```

These paths assume the generated project sits at a fixed depth inside a vox
source checkout (`<repo>/<generated_pkg>/Cargo.toml`, two levels above
`<repo>/crates/`). Scaffold a project anywhere else — the documented normal
case, since `vox init`/`vox build` are meant to work on user code outside
this repo — and `cargo metadata`/`cargo check` fails immediately:

```text
error: failed to load manifest for workspace member `.../my-app/crates/vox-db`
Caused by:
  failed to read `.../my-app/src/crates/vox-actor-runtime/Cargo.toml`
```

Confirmed 2026-09-21 against `emit_cargo_toml` and `emit_cargo_toml_tauri_app`
in `crates/vox-codegen/src/codegen_rust/emit/mod.rs`.

## Why not crates.io

Vox has no crates.io publishing story (see `AGENTS.md`), so `vox-db = "0.1"`
is not an option. The generated project's only way to a working copy of
these crates is a filesystem path or a git dependency.

## Approach: resolve the repo root, prefer absolute paths

Three-tier resolution, each tier sanity-checked by confirming
`<candidate>/crates/vox-db/Cargo.toml` exists before trusting it:

1. **`VOX_REPO_ROOT` env var** — explicit override, for CI, unusual
   checkout layouts, or testing. Highest priority.
2. **The path `vox` itself was compiled from**, embedded at `vox-codegen`'s
   own build time by its `build.rs` (reads
   `CARGO_MANIFEST_DIR`, walks up to the workspace root, emits
   `cargo:rustc-env=VOX_COMPILER_REPO_ROOT=<path>`; `vox-codegen` reads it
   back with `env!()`). Correct whenever `vox` was built locally from a
   checkout with `cargo build -p vox-cli` — the workflow this repo
   documents (`AGENTS.md` "Build Broker", no crates.io/binary-release
   story described anywhere else at the time of writing).
3. **Historical relative path** (`../../crates/<name>` for the Axum shell,
   `../../../crates/<name>` for Tauri's `src-tauri/`) — unchanged fallback,
   so generating inside this repo (tests, scratch builds under the
   checkout) keeps working even if tiers 1–2 both fail.

If none resolve to a real checkout, tier 3 is used as a last resort and the
generated `Cargo.toml` gets a comment pointing at `VOX_REPO_ROOT` as the
escape hatch.

### Why not a git dependency

The alternative this doc's authoring task also asked us to weigh: emit
`vox-db = { git = "https://github.com/vox-foundation/vox", rev = "<hash>" }`
using the git hash `vox-build-meta` already embeds (`VOX_GIT_HASH`). This is
the right answer for a `vox` binary *distributed* to a machine that never
had the source checkout (nightly builds via `voxup`, per
`docs/src/architecture/nightly-builds-ssot.md`) — tier 2 above embeds the
CI runner's path in that case, which is useless on the end user's machine.

It is **not implemented in this change**: it needs the resolved commit to
actually be pushed and public (true today — `vox-foundation/vox` is a public
GitHub repo — but not a property codegen can verify at generation time), and
a `cargo check` of the generated project would then compile the full
`vox-db`/`vox-actor-runtime`/etc. dependency graph from a git checkout on
every fresh clone, which is a materially heavier and slower ask than the
one bug this change fixes. Flagged as follow-up: add a tier between 2 and 3
that falls back to a pinned `git`/`rev` dependency when neither the env
override nor the embedded local path resolves.

## Trimming unconditional heavy deps

Separately, `emit_cargo_toml` unconditionally listed `vox-orchestrator` and
`vox-speech` regardless of whether the generated code actually references
them:

- `vox-orchestrator` is only referenced (per
  `crates/vox-codegen/src/codegen_rust/emit/ai_fixture/llm.rs`) by an
  `@ai(subagent(...))` fixture (`vox_orchestrator::subagent_dispatch`,
  `a2a::bus`) or an `@ai(search(corpus: "memory"))` fixture
  (`vox_orchestrator::memory::manager::MemoryManager`). Gated on
  `module_needs_vox_orchestrator` (scans `HirAiFixture` on every function,
  test, MCP tool/resource, and `forall`, mirroring the existing
  `module_has_distributed_subagent` / `module_needs_vox_search_docs` scans
  in the same file).
- `vox-speech` is only referenced by the `Speech.transcribe(...)` builtin
  (`crates/vox-codegen/src/codegen_rust/emit/method_emit.rs`). Gated on
  `module_needs_vox_speech`, which serializes the `HirModule` to JSON (the
  same serialization `emit_hir_embed_helper` already performs for the
  embedded-HIR const) and substring-searches for `"Speech"` and
  `"transcribe"`, rather than writing a bespoke recursive `HirExpr`/`HirStmt`
  walker that has to track every enum variant — a walker that misses a
  variant silently under-detects and breaks the generated build; a
  substring false positive on the serialized tree just keeps a dep that
  wasn't strictly needed, which is always safe.

### `vox-compiler` and `vox-workflow-runtime` stay unconditional

The bug report that prompted this doc also named `vox-compiler` as an
example of an always-pulled-in heavy crate. It is **not** trimmed here:
`emit_main`'s durable-boot prelude (`main_boot.rs::emit_durable_boot_prelude`
+ `emit_durable_boot_helpers`) unconditionally emits
`::vox_workflow_runtime::workflow::set_current_hir_module(load_hir_module_from_embedded())`,
and `load_hir_module_from_embedded`'s return type is
`::vox_compiler::hir::HirModule` — for *every* generated Axum/Tauri project,
not only ones with `workflow`/`activity`/`actor`/`@scheduled` declarations.
This is deliberate, locked-in behavior, not an oversight:
`crates/vox-codegen/tests/emit_main_includes_durable_boot.rs::emit_main_includes_durable_boot_even_without_scheduled`
and `crates/vox-codegen/tests/main_boot_hir_roundtrip.rs` both assert HIR
registration happens for modules with no scheduled/workflow functions at
all ("HIR registration is still useful for workflows (and harmless
otherwise)" — see that test's own comment), and ADR-041 §6(b) is the
underlying contract.

Making this conditional is a legitimate follow-up but is a larger, separate
change: it means auditing every durability kind's codegen for a hidden
dependency on `current_hir_module()`, rewriting the three tests above to
assert the new conditional contract, and updating ADR-041. Out of scope for
this fix.

### A latent bug the trim exposed: `vox-db` needs `host-integration`

Trimming `vox-orchestrator` for the non-AI case surfaced a second,
independent bug. The durable-boot prelude's `vox_db::DbConfig::resolve_canonical()`
call (`main_boot.rs`, unconditional — see above) and the `has_tables` health
probe's `vox_db::resolve_app_db_url` / `resolve_codex_db_url`
(`tables/codegen.rs`) are all `#[cfg(feature = "host-integration")]` in
`crates/vox-db/Cargo.toml`, which is **not** in `vox-db`'s own default
feature set (`default = ["local"]`). The generated `Cargo.toml`'s
`vox-db = { path = "..." }` line never requested it either.

This compiled anyway, for every generated project, purely by accident:
`vox-orchestrator`'s default features pull in `vox-lsp` (via its
`toestub-gate` default feature), and `vox-lsp` depends on
`vox-db = { features = ["host-integration"], optional = true }` — Cargo's
per-resolution feature unification then silently turned `host-integration`
on for the generated crate's *entire* `vox-db` dependency, including the
codegen's own unconditional use of it. Trimming `vox-orchestrator` out of
the "trivial notes app" case (the exact scenario named in the bug report)
removed that accidental enabler and broke the build the trim was supposed
to fix.

Fixed by requesting the feature explicitly:
`vox-db = { path = "...", features = ["host-integration"] }` in both
`emit_cargo_toml` and `emit_cargo_toml_tauri_app`, matching how `vox-cli`,
`vox-gamify`, and the `vox-plugin-mens-candle-*` crates already depend on
`vox-db` in this workspace. This does not add net weight versus the
pre-trim baseline — `host-integration` (and the `keyring` / `vox-config` /
`vox-repository` / `vox-secrets` it pulls in) was already being compiled
into every generated project before this change, just via the accidental
path instead of a declared one.
