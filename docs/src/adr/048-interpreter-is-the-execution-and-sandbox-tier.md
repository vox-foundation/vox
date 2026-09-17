---
title: "ADR 048: Interpreter is the execution and sandbox tier"
description: "Makes the HIR interpreter the default and isolation tier for VoxScripts locally and over the mesh, retiring wasm/container/microvm isolation lanes."
category: "Architecture Decisions (ADRs)"
status: "current"
training_eligible: true

schema_type: "TechArticle"
---

# ADR 048: Interpreter is the execution and sandbox tier

## Status

**Accepted (2026-09-08).**

- **Supersedes** the container-tier direction discussed 2026-09-05 and the
  "no sandbox exists" gap in the [populi mesh iroh design](../../superpowers/specs/2026-09-04-populi-mesh-iroh-transport-design.md).
- **Upholds** [ADR-047](047-iroh-transport.md): mesh-received work stays
  sandboxed by default; pairing grants reachability, never native execution.
  The sandbox *is* the interpreter.
- **Does not fund** inbox drain, lease-over-mesh, or the accept-loop /
  frame-deadline commit (those stay out of band against merged Phase 3).

## Context

Pure VoxScripts were defaulting to the native `cargo` lane. That made `cargo`,
`rustc`, and a Vox checkout end-user requirements, and left the mesh without a
real isolation boundary once the WASI / container / MicroVM lanes were deleted
or never shipped. The HIR interpreter already runs scripts in tens of
milliseconds, imposes capabilities, and can be spawned as a bounded child
(`InterpExecutor`) without a new crate edge from `vox-mesh-transport`.

Design: [interpreter-first execution](../../superpowers/specs/2026-09-05-interpreter-first-execution-design.md)
(revision 3). Caps grammar: [`isolation.md`](../reference/isolation.md).

## Decision

The interpreter is the isolation tier for VoxScripts — locally and when a peer
receives a job. `vox run` in `auto` mode routes **script-shaped** files
(`fn main()`, no service surfaces) to the interpreter. Native compilation is an
explicit hatch (`--mode script`, `Vox.toml [web] run_mode = "script"`,
`VOX_WEB_RUN_MODE=script`). The mesh ships VoxScript source; it never ships
native code.

Capabilities are receiver-imposed, non-optional, and fatal. Repeatable `--caps`
is the public CLI (not comma-joined tokens). Heap, recursion, output, disk
bytes, and file counts join step limits as hard bounds. The interpreter is an
isolation boundary, not a resource-containment boundary beyond those counted
bounds.

Settled calls from the design §5 table:

| Decision | Call |
|---|---|
| Object iteration order | **Insertion order.** One line in the generated manifest; workspace already has `preserve_order`. |
| `vox-compiler → vox-crypto` edge | **Take it** (user-authorized 2026-09-06). `vox-crypto` is SSOT for both tiers' hash/id builtins. |
| Accept-loop permit + missing frame deadline | **Out of band.** Separate commit against merged Phase 3. Close code **4003** is `REFUSED_PROTO`. |
| Permanently-red golden | **No.** Decisions land before the corpus; the residual asymmetry set is empty. |
| Disk/inode DoS | **In-process** `max_disk_bytes` / `max_files` on `JobLimits`. Local `developer_default()` uncapped. |

## Consequences

- **PROTO 2** is incompatible with a PROTO 1 peer by design. The mismatch is
  diagnosable (`Failed` + `vox mesh probe`), not silent.
- **Six embedders** (`vox-cli` run / repl / play, `vox-langtool` run,
  `vox-terminal-core::eval_line`, `vox-orchestrator-mcp` workspace dispatch)
  construct `Interpreter` with caps set; denial is fatal.
- **`@versioned` is gated.** The auto-snapshot is a side-effecting entry point
  and requires an explicit capability; it no longer bypasses method dispatch.
- **`list.push` is in-place** (`Scope::get_mut` + `Rc::make_mut`) before the
  default-lane flip. A cloning receiver would make the interpreter quadratic on
  ordinary scripts.
- `cargo` and `rustc` stop being requirements for running script-shaped Vox.
  `wasm32-wasip1` is not required. `vox doctor` probes these facts instead of
  reporting a permanently-green leftover from a deleted WASI lane.

### Alternatives rejected

- Keep a WASI / container / gVisor / MicroVM isolation flag on `vox run`. Those
  lanes needed cargo + rustc + a rustup target (or a stub that always bailed)
  and their only advantages over the interpreter are now interpreter properties.
- Keep a probe-only mesh executor as the default. A probe-only peer cannot run
  received Vox.
- Ship native bundles over the mesh. Pairing must never grant native execution.

## Retired surfaces

These names are retired. Do not re-introduce them; use the interpreter and
repeatable `--caps` instead.

| Retired | Replacement |
|---|---|
| `--isolation wasm\|container\|gvisor\|microvm` | interpreter (`vox run` / `--interp`); repeatable `--caps` |
| `vox wasm run` | `vox run` (interpreter) |
| `script-wasi` | interpreter (no rustup wasm target) |
| `ProbeOnlyExecutor` | `InterpExecutor` |
| `MicroVmRuntime` / `Tier::MicroVm` | interpreter isolation; planner error-path for the enum variant stays |
| `VoxMeshExecPolicy` (SecretId) | receiver-imposed `--caps` (placement config key survives) |
| `exec_bundle_b64` | VoxScript source on the mesh stream |
