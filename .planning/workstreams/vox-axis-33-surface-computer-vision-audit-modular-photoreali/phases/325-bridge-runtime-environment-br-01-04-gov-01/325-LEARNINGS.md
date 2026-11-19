---
phase: 325
phase_name: "bridge-runtime-environment-br-01-04-gov-01"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 17
  lessons: 6
  patterns: 10
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 325 Learnings: bridge-runtime-environment-br-01-04-gov-01

## Decisions

### Only edge added
Only edge added: vox-python-bridge -> vox-foundation. No vox-crypto edge; this plan does not hash.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-01-SUMMARY.md

---

### orphan_exempt until plan 325-02 adds vox-compiler and vox-cli consumers.
orphan_exempt until plan 325-02 adds vox-compiler and vox-cli consumers.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-01-SUMMARY.md

---

### vox-foundation max_dependents stays 18; live fan-in becomes 15.
vox-foundation max_dependents stays 18; live fan-in becomes 15.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-01-SUMMARY.md

---

### Added the L2 vox-python-bridge library, workspace dependency, where-thin...
Added the L2 `vox-python-bridge` library, workspace dependency, where-things-live row, and one downward edge to `vox-foundation`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-01-SUMMARY.md

---

### A fake worker and python3 -m vox_bridge both complete a hello round-trip
A fake worker and `python3 -m vox_bridge` both complete a hello round-trip. `debug.print` lands on stderr and the host still parses the reply.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-01-SUMMARY.md

---

### debug.sleep past the deadline and debug.crash bump the generation
`debug.sleep` past the deadline and `debug.crash` bump the generation. A handle from the old generation returns `py: stale handle (worker restarted)`. An oversized declared body returns `py: fetch exceeds cap` before the body is read.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-01-SUMMARY.md

---

### The only allow-file edge this plan inserts is ["vox-python-bridge", "vox...
The only allow-file edge this plan inserts is `["vox-python-bridge", "vox-foundation"]`. `vox-compiler` and `vox-cli` edges wait until those crates depend on the bridge. No `vox-crypto` edge: this plan does not hash.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-01-SUMMARY.md

---

### vox-foundation max_dependents stays 18
`vox-foundation` `max_dependents` stays **18**. The new crate makes the live count 15, which does not exceed the cap.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-01-SUMMARY.md

---

### No vox-package-types or vox-python-bridge dependency on vox-crypto
No vox-package-types or vox-python-bridge dependency on vox-crypto. The CLI, which already depends on vox-crypto, writes the lock line.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-02-SUMMARY.md

---

### No vox-cli dependency on vox-python-bridge
No vox-cli dependency on vox-python-bridge. vox run reaches the bridge through vox-compiler. The only new crate edge is vox-compiler → vox-python-bridge.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-02-SUMMARY.md

---

### Doctor prints a static unsandboxed warning
Doctor prints a static unsandboxed warning. This phase does not take a vox-skill-runtime dependency.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-02-SUMMARY.md

---

### requires-python is emitted as >=<version> when the manifest version star...
requires-python is emitted as >=<version> when the manifest version starts with a digit, because uv rejects a bare 3.12.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 325-02-SUMMARY.md

---

### VoxManifest.python is appended after db
`VoxManifest.python` is appended after `db`. `vox.lock` stores `python_uv_lock_hash`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-02-SUMMARY.md

---

### vox py sync writes .vox/python/pyproject.toml, runs uv lock and uv sync ...
`vox py sync` writes `.vox/python/pyproject.toml`, runs `uv lock` and `uv sync --frozen`, and records the lock hash. `vox py doctor` prints uv, Python, packages, lock status, `sandbox: unsandboxed`, and the deferral warning.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-02-SUMMARY.md

---

### python.version() is a PythonModule method returning Result[str, str]
`python.version()` is a `PythonModule` method returning `Result[str, str]`. The effect checker maps the `python` namespace to `HirCapability::Python`. The interpreter calls the bridge only after `py` is allowed and `[python]` is present.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-02-SUMMARY.md

---

### examples/golden/py_bridge_hello.vox runs through vox run in a temp proje...
`examples/golden/py_bridge_hello.vox` runs through `vox run` in a temp project and prints the worker version (`3.12.10` on this machine).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-02-SUMMARY.md

---

### AGENTS.md exempts crates/vox-python-bridge/worker/ only
AGENTS.md exempts `crates/vox-python-bridge/worker/**` only. `syntax_k.rs` is cited under `vox-codegen`. `HirType` is cited at `stmt_expr.rs:111`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 325-02-SUMMARY.md

## Lessons

### RED was confirmed before implementation
RED was confirmed before implementation: `cargo test -p vox-python-bridge` failed 7 lifecycle tests with `Protocol("not implemented")`, then passed 7 lifecycle tests plus 2 unit tests in 2.16s after the worker landed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 325-01-SUMMARY.md

---

### cargo run -p vox-arch-check still exits 1 on 7 pre-existing [raw-git-exe...
`cargo run -p vox-arch-check` still exits 1 on 7 pre-existing `[raw-git-exec]` hits in `crates/vox-cli-ci` (`check_changed.rs`, `ci_changes.rs`, `evidence_asset_ban.rs`, `hit_every_surface.rs`, `suppression_review_gate.rs`). Also pre-existing fan-in warns for `vox-config` (41/40), `vox-db` (28/27), and `vox-secrets` (35/34), plus LoC-delta and `vox-cli` workspace-dep warns. Not fixed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 325-01-SUMMARY.md

---

### VOX_SKIP_FRESHNESS_CHECK=1 ./target/debug/vox ci crate-edges accepts the...
`VOX_SKIP_FRESHNESS_CHECK=1 ./target/debug/vox ci crate-edges` accepts the new edge and still reports a pre-existing violation: `vox-plugin-mens-candle-core -> vox-eval`, plus a stale baseline edge `vox-gui -> vox-db-types`. The installed `vox` is one commit behind HEAD, so the freshness gate was skipped. Not fixed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 325-01-SUMMARY.md

---

### A cached arch-check git index, written before these commits, briefly rep...
A cached arch-check git index, written before these commits, briefly reported the new crate as stale. A cache rebuild does not.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 325-01-SUMMARY.md

---

### N/A — infrastructure phase
N/A — infrastructure phase. Timeout, crash, cap, and hello behaviors were exercised by the lifecycle tests. No user-facing flow is left unverified.

**Context:** Verified in phase acceptance checks.
**Source:** 325-VERIFICATION.md

---

### No gaps. The phase goal holds in the committed bridge, CLI, and compiler...
No gaps. The phase goal holds in the committed bridge, CLI, and compiler wiring. The sandbox host is an accepted deferral, not missing work for this phase.

**Context:** Verified in phase acceptance checks.
**Source:** 325-VERIFICATION.md

## Patterns

### Worker stdout is dup'd onto the RPC fd and sys.stdout points at stderr b...
Worker stdout is dup'd onto the RPC fd and sys.stdout points at stderr before any reply.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-01-SUMMARY.md

---

### A declared body over max_fetch_mb returns py
A declared body over max_fetch_mb returns py: fetch exceeds cap before the body is read.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-01-SUMMARY.md

---

### u32 LE header length, JSON DispatchRequest/DispatchResponse, u64 LE body...
u32 LE header length, JSON DispatchRequest/DispatchResponse, u64 LE body length, opaque body

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-01-SUMMARY.md

---

### one lazy worker, requests serialized, SIGINT then 5s grace then kill
one lazy worker, requests serialized, SIGINT then 5s grace then kill

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-01-SUMMARY.md

---

### fake worker via BridgeConfig.worker_cmd; real worker via python3 -m vox_...
fake worker via BridgeConfig.worker_cmd; real worker via python3 -m vox_bridge

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-01-SUMMARY.md

---

### Bridge entry points return py-env
Bridge entry points return py-env: python not configured (add [python] to Vox.toml) and do not spawn when the section is absent.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-02-SUMMARY.md

---

### python.version() is gated by caps.allows_namespace(py) before the generi...
python.version() is gated by caps.allows_namespace(py) before the generic builtin namespace check.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-02-SUMMARY.md

---

### uv is spawned as argv (uv --directory <dir> lock / sync --frozen), never...
uv is spawned as argv (uv --directory <dir> lock / sync --frozen), never a shell

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-02-SUMMARY.md

---

### The uv.lock BLAKE3 is vox_crypto::secure_hash in vox-cli; vox-package-ty...
The uv.lock BLAKE3 is vox_crypto::secure_hash in vox-cli; vox-package-types only stores the hex

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-02-SUMMARY.md

---

### python.version() runs the bridge on a dedicated current-thread runtime s...
python.version() runs the bridge on a dedicated current-thread runtime so the child is not dropped and block_on is not called inside the caller runtime

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 325-02-SUMMARY.md

## Surprises

