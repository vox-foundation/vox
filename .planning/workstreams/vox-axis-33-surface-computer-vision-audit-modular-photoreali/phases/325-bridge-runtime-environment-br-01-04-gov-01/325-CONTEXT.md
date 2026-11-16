# Phase 325: Bridge Runtime & Environment - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning
**Mode:** Smart discuss (decisions locked by design SSOT §3, §7, §11)

<domain>
## Phase Boundary
`vox-python-bridge` (new L2 crate) spawns a real Python worker, speaks the framed protocol, survives timeouts/crashes with handle generations, caps fetch size; `[python]` env via uv; `vox py sync|doctor`; one minimal Vox-visible materializer proving end-to-end wiring; governance rows. No std.array/frame (Phase 326+), no Arrow decode yet (Phase 326 adds `arrow-ipc`/`arrow-array` when the first array crosses).
</domain>

<decisions>
## Implementation Decisions
- **D-01 Crate:** `crates/vox-python-bridge` (L2, `kind = "library"`), deps: `vox-foundation` (reuse `DispatchRequest` / `DispatchResponse` from `protocol.rs:192,220` for the JSON header), serde, serde_json, tokio (process, io), thiserror. No trait; tests use a fake worker = a tiny Rust test binary (`[[bin]]` under `tests/` or `src/bin/fake_worker.rs` gated `#[cfg(test)]`-equivalent via `required-features`) selected through a `BridgeConfig.worker_cmd` override.
- **D-02 Framing:** each frame = `u32 LE` header length + JSON header + `u64 LE` body length (0 = none) + body bytes. Body is opaque bytes in this phase (Arrow IPC in 326). Max frame body = `max_fetch_mb`; exceeding → `py: fetch exceeds cap` (checked from the header-declared length before reading).
- **D-03 Worker:** `crates/vox-python-bridge/worker/vox_bridge/` Python package (stdlib-only in this phase). First action in `__main__`: `rpc_out = os.fdopen(os.dup(1), "wb")`; `os.dup2(2, 1)`; `sys.stdout = sys.stderr`. Methods now: `hello` (returns python version, platform, pid), `release` (handle ids), `shutdown`, and test-only `debug.print` / `debug.sleep` / `debug.crash` enabled only when env `VOX_BRIDGE_DEBUG_OPS=1`.
- **D-04 Lifecycle:** one worker per host process, lazily spawned; `generation: u32` bumps on crash/timeout/kill; requests serialized (tokio Mutex). Timeout per request (`timeout_s`, default 300): SIGINT (unix) / CTRL_BREAK (windows), 5 s grace, then kill + generation bump. `HandleRef{generation,id,kind,dtype}`; stale generation → `py: stale handle (worker restarted)`. Error prefixes `py:` / `py-plan:` / `py-env:`.
- **D-05 Env:** `[python]` section in `VoxManifest` (`version`, `packages`, `extras`, `max_fetch_mb`=256, `timeout_s`=300). `vox py sync` writes `.vox/python/pyproject.toml`, runs `uv lock` + `uv sync --frozen` (uv discovered on PATH via process spawn, never via a shell), records `uv.lock` blake3 hash (via `vox-crypto`, per crypto policy) in `vox.lock`. `vox py doctor` reports uv/python/packages/lock-hash match + sandbox availability. No `[python]` → every bridge entry point returns `py-env: python not configured (add [python] to Vox.toml)` without spawning.
- **D-06 Sandbox (design correction):** `vox-skill-runtime::SkillRuntime` is a one-shot `run(RunOpts)->RunOutcome` API, not a long-lived stdio process host, so the worker cannot run under it without extending that trait. This phase: doctor reports sandbox tier availability via `vox_skill_runtime` detection and the worker runs **unsandboxed with an explicit warning**. Sandboxed long-lived worker is deferred (record in design doc §6/§12 and REQUIREMENTS BR-04 note).
- **D-07 Vox surface:** one Tier 1 materializer `python.version() to Result[str]` (namespace `python`, effect `python`, caps `py`) — typeck signature + interpreter builtin calling the bridge `hello`. `stdlib_module_capability("python") => Python`. Golden `examples/golden/py_bridge_hello.vox` runs under `vox run` with `[python]` env present.
- **D-08 Governance:** layers.toml row, where-things-live row, crate-edges pairs (`vox-compiler→vox-python-bridge`, `vox-cli→vox-python-bridge`, bridge→its deps), CLI catalog rows for `py sync`/`py doctor` in `contracts/operations/catalog.v1.yaml` (targeted row insert, then the repo generator), AGENTS.md §VoxLang-First scoped exemption sentence, fix stale citations (`docs/src/reference/syntax-k-complexity.md:65` → `crates/vox-codegen/src/syntax_k.rs`; phase5 subspec `stmt_expr.rs:90-103` → `:111`).
### the agent's Discretion
Module layout inside the crate; exact clap shapes (follow existing `vox db` subcommand patterns).
</decisions>

<code_context>
- `DispatchRequest`/`DispatchResponse`: `crates/vox-foundation/src/protocol.rs:192,220`; JSON-line client precedent `crates/vox-orchestrator-mcp/src/dei_ipc.rs`.
- Interpreter entry: `crates/vox-cli/src/commands/run.rs:41-110` (`run_interp`, `interpreter.caps`); builtins `crates/vox-compiler/src/eval/builtins.rs`, namespaces `eval/mod.rs:41-180`; typeck sigs `typeck/builtins.rs`.
- Manifest: `crates/vox-package-types/src/manifest.rs:10` (Phase 330 adds `[db]` concurrently — coordinate: append, don't reorder); lockfile `lockfile.rs:12`.
- Governance: `docs/src/architecture/layers.toml` `[crates]`, `docs/src/architecture/where-things-live.md`, `contracts/ci/crate-edges.allow.v1.json`.
- Arch guard: no shell spawn (`layers.toml:488-499`).
</code_context>

<specifics>Python 3.12.10 and uv 0.13.0 present locally (aarch64-darwin).</specifics>

<deferred>Sandboxed long-lived worker; Arrow decode (326); worker pool; mmap.</deferred>
