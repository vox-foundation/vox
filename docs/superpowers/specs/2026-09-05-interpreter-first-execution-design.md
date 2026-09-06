---
title: "Interpreter-first execution: one toolchain-free tier for scripts and the mesh"
description: "Revision 3 design for making the HIR interpreter the default and isolation tier for VoxScripts, locally and over the mesh, retiring cargo as an end-user dependency for pure Vox."
category: "Architecture SSOTs"
status: "approved"
training_eligible: false
---

# Interpreter-first execution — design

**Date:** 2026-09-06 · **Revision 3**, after two critique rounds (fifteen tracks) against
the source. Round 1 produced revision 2. Round 2 audited revision 2 and produced
[`2026-09-05-interpreter-first-critique-ledger.md`](2026-09-05-interpreter-first-critique-ledger.md)
and this document. Three of revision 2's own corrections had over-corrected; the worst
(`fs_resolve_allowed` denying `fs.write("out.txt")`) would have broken every ordinary
local `vox run` on the tier this program makes the default.
**Status:** approved by the maintainer; implementation plan follows, split into six
revertable PRs.
**Supersedes** revision 2 of this file, the container-tier direction discussed 2026-09-05,
and the "no sandbox exists" gap in
[`2026-09-04-populi-mesh-iroh-transport-design.md`](2026-09-04-populi-mesh-iroh-transport-design.md).
**Evidence base:** [`voxscript-portability-substrate-research-2026.md`](../../src/architecture/voxscript-portability-substrate-research-2026.md).

## 1. Decision in one paragraph

Pure VoxScripts run under the HIR interpreter by default, locally and when received over the
mesh. The interpreter is the **isolation** boundary for Vox code: capabilities are imposed by
the receiver, denial is fatal, every side-effecting path — including `import` and the
`@versioned` snapshot — is gated, and CPU, heap, recursion depth, output, disk bytes, and
file counts are bounded. It is **not** a resource-containment boundary against a
trusted-but-malicious peer on dimensions this design does not count (regex compile time is
capped; executor slots are per-peer). Native compilation (`cargo`) becomes an explicit
opt-in used only when a script pulls in Rust crates or needs peak throughput. The mesh ships
VoxScript source and declarative ML jobs; it never ships native code. One builtin registry
feeds both tiers, and a differential gate proves they agree on every golden that declares an
expected output — a corpus this design also grows to cover arithmetic, display, ordering,
argv, and `crypto.*`. `cargo`, `rustc`, and the Vox source tree stop being end-user
requirements for running Vox.

## 2. What is true today (measured 2026-09-05 / 2026-09-06, this machine, debug build)

| Tier | `hello.vox` cold | warm | End-user needs |
|---|---|---|---|
| Interpreter | **10 ms** | 10 ms | the `vox` binary |
| Native | **275 s** (764 crates) | 40–70 ms | cargo + rustc + **a Vox checkout** (`vox-actor-runtime = { path = … }`) |
| WASI | ≥ native + `rustup target add` | — | cargo + rustc + wasm target |

- `vox run` in `auto` mode routes every non-`@page` file to the **native** lane
  (`run.rs`, `is_script_file_by_page_heuristic` is literally `!head.contains("@page")`).
- The two tiers already disagree, verified by execution: `crypto.hash_fast` → interp
  `UndefinedVariable("crypto")`, native works; `time.now()` → interp works, native has only
  `now_ms`. Native **integers wrap silently** (`script-dev` sets `overflow-checks = false`)
  while the interpreter halts; `str([1, 2])` is `[1, 2]` under interp and `[1,2]` natively;
  `print(Some(3))` prints Rust `Debug` under interp and does not compile natively;
  `env.args()` returns the `vox` command line under interp and the script's argv natively;
  `fs.glob` is unsorted and error-swallowing under interp, sorted and propagating natively;
  `.sorted()` has no codegen arm; script faults exit 1 vs 101.
- **Object iteration is already insertion-ordered in the workspace.** `serde_json`'s
  `preserve_order` is on via `workspace-hack`, so the interpreter and every workspace crate
  iterate in insertion order. Only the **standalone generated script crate**
  (`[workspace]` in its own manifest, `serde_json = "1"`, default features) is key-sorted.
  That is the only divergence, and it is one line in the generated manifest.
- **No test runs one program through two tiers and compares output.** Of 79 goldens, 11
  declare `// EXPECT:` and **4** of those expect a literal `ok`. Two more assert a computed
  value. None exercises overflow, div-by-zero, composite display, float formatting, object
  order, directory order, argv, or `crypto.*`.
- `// vox:caps` is fail-open: engages only if the script declares it; denial prints, returns
  `Null`, continues; `net`/`http` ungated; `eval/repo.rs` bypasses it.
- Interpreter bounds steps (10 M, `StepLimitExceeded` in 0.75 s) but not heap (an allocating
  loop ran until killed), not recursion depth (Vox recursion is Rust recursion; a
  200 000-deep nesting SIGSEGVs at *parse* time), and not wall time.
- **`import "…"` reads and executes arbitrary host `.vox` files with no capability check**
  (`eval/mod.rs`, before `main` runs). `process.exec` replaces the interpreter's process image.
  `process.register_exit_command` pushes into a process-global static whose signal handler
  runs those commands and hard-exits the host — surviving the interpreter that armed it.
  **`@versioned` auto-snapshot** (`eval/expr.rs`) calls `interp.repo.snapshot` directly,
  bypassing method dispatch.
- **Six production embedders** construct `Interpreter` via `Interpreter::new` and never set
  `caps`: `vox-cli` `run` / `repl` / `play`, `vox-langtool` `run`,
  `vox-terminal-core::eval_line`, and `vox-orchestrator-mcp` workspace dispatch (two call
  sites). After `caps` becomes non-optional, `Interpreter::new` defaulting to
  `developer_default()` makes the MCP path that runs `.vox` tools for an LLM **more
  permissive** than today (today `caps: None` at least printed on fs/process/env/secrets).
- The mesh's `ProbeOnlyExecutor` refuses `Run`. Separately, the *HTTP* A2A path in
  `remote_worker.rs::run_dispatched_bundle` executes precompiled **native binaries directly**
  when `VoxMeshExecPolicy` is `permissive` — **which is the default**. That is failure mode
  F2, live today.
- `db.rs` and `repo.rs` are **pure in-memory stores** with no I/O.
- **`list.push` is O(n²) under the interpreter** (`eval/builtins.rs` does `v.to_vec()`; 
  `VoxValue::Str` holds a `String`) and O(1) amortised natively. `xs = xs.push(v)` is the
  idiomatic collection builder across `scripts/**`. The flip changes its complexity class
  silently; a byte-comparing gate cannot see it.
- **`scripts/install-hooks.vox` and `scripts/setup.vox` fail under the interpreter today**
  with `AssertionFailed("called Option.unwrap() on a None value")`. Both pass `vox check`.
  `setup.vox` is what `.github/workflows/setup-e2e.yml` runs.
- **`sandbox.rs` has exactly two call sites, both in the native lane.** The interpreter
  child gets no Landlock and no seccomp. On Linux the interpreter boundary is the whole
  story for `InterpExecutor`.
- Native compilation **discards** a shared `CARGO_TARGET_DIR` (`native.rs`:
  `let _ = shared_target;`) and uses a per-cache-entry target dir, so every golden rebuilds
  all 764 crates unless the gate emits them as `[[bin]]` targets of one generated crate.

## 3. Design

### 3.1 Tiers and defaults

| Mode | Behaviour after this change |
|---|---|
| `vox run x.vox`, script-shaped (see below) | **interpreter** |
| `vox run --mode script x.vox` | native lane, unchanged (cargo, cached) — the opt-in |
| `vox run --mode interp x.vox` | interpreter, explicit |
| `vox run x.vox`, app- or service-shaped | unchanged: native/app lane, with a one-line message naming `--mode script` |
| `vox run --isolation wasm` / `vox wasm run` | **retired** (see §3.5; plugins keep `vox-wasm-engine`) |

**Script-shaped** means: the file declares `fn main()` and none of `@page`, `routes`,
`server`/`query`/`mutation`, `table`, `actor`, `workflow`/`activity`. Those surfaces are booted
by the native lane (DB init, HTTP listener) and cannot run under the interpreter; the old
`!contains("@page")` predicate would have routed them there. `WebRunMode` gains no new variant.

**Three existing escape hatches revert the flip.** Do not invent a new key.

1. `vox run --mode script x.vox` — one invocation.
2. `Vox.toml [web] run_mode = "script"` — per project.
3. `VOX_WEB_RUN_MODE=script` — process-wide. The name says "web"; document it as the
   global revert anyway.

**Escalation is explicit, never automatic.** No shipped language auto-switches tiers; every one
that offers both makes it a user command (`deno compile`, `go build`). The interpreter's exit-78
message names `--mode script`.

**`list.push` is a flip prerequisite.** Before Task 7 lands, `list.push` must be in-place via
`Scope::get_mut` + `Rc::make_mut` (the machinery at `eval/env.rs` already names `list.push`
as its intended user) and `VoxValue::Str` must be `Rc<str>`. Without that, every
`scripts/**` collection builder changes complexity class on the flip.

**Why not a JIT / bytecode VM now.** CPython's JIT was slower than its interpreter for two
releases with Microsoft funding; `cranelift-jit` self-describes as "extremely experimental";
the largest measured interpreter lever is inline caching, not dispatch. The interpreter's first
optimisation (name→slot resolution — every `Ident` is a `String` scope lookup, every `Object`
field access an O(n) scan) is worth more than a JIT and cannot miscompile. A bytecode lowering
is *permitted* later behind the differential gate; it is not part of this design.

### 3.2 The interpreter is the sandbox — what that claim requires

**Does executing Vox over the mesh require a sandbox? Yes, and the interpreter is it — once
the following is true.** Revision 1 said "every side effect passes through
`call_builtin_method`". Four paths do not: `import` resolution, the `@versioned` snapshot,
`process.exec`, and the exit-command queue. The claim is therefore a *target*, and the list
below is what makes it true.

**The claim, split.** It is an **isolation** property (confidentiality and integrity): the
interpreter has no FFI and no `unsafe` execution path, so gating every side-effecting entry
point is sufficient against Vox code reading outside the job dir, networking, running a host
binary, or fingerprinting the host. Forging `__namespace__`, observing `_Denied` as a value,
and probing existence vs absence were each checked and found closed. It is **not** a
resource-containment property: it counts steps and heap; it does not, until this revision,
count disk bytes, file counts, or executor slots per peer. Regex compile size is capped
(§3.2.11). Availability against a trusted-but-malicious peer depends on those counted
bounds, not on a privileged mount — size-capped tmpfs needs root, which this design will
not require.

It is **not** a reproducibility property (§3.8).

1. **Receiver-imposed capabilities.** `vox run --mode interp --caps <token> --caps <token>`
   **replaces** any `// vox:caps` directive; the script's own declaration is documentation,
   never trusted. `--caps` is **repeatable** (`clap::ArgAction::Append`). Each token is one
   grant; there is no comma-joining of tokens on the CLI, so a Windows `TEMP` of
   `C:\Users\Smith, John\...` is expressible. The `// vox:caps` comment form remains
   comma-separated (documentation only).

   | token | grants |
   |---|---|
   | `fs:ro=<dir>` | reads under `<dir>` (canonicalised at parse; must exist) |
   | `fs:rw=<dir>` | reads and writes under `<dir>` |
   | `net:none` / `net:allow` (`http:` accepted as alias) | `std.http.*` |
   | `process:none` / `process:allow` | `process.*` — see the warning below |
   | `env:none` / `env:ro` / `env:rw` | `env.get`/`env.args` need `ro`; `env.set` needs `rw` |
   | `secrets:none` / `secrets:allow` | `secrets.*` |
   | `time:real` / `time:frozen=<ms>` (non-negative) | `time.now_ms()` wall-clock or fixed |
   | `random:seed=<u64>` / `random:deny` | every randomness-consuming builtin (`crypto.uuid` when it lands) |
   | `agentos:none` / `agentos:allow` | `agentos.*` |
   | `deterministic` | shorthand: `time:frozen=0` + `random:seed=0` + `process:none` + `net:none` + `env:none` |

   Unmentioned namespaces are **denied**. `io:` is rejected as a token — `io.open`/`io.save`
   are fs reads/writes and take their authority from `fs:`. The mesh executor builds its
   `CapabilitySet` through a typed constructor (`from_roots`) and passes tokens via
   repeatable `--caps`; it does **not** refuse a job directory that contains `,`, `=`, or
   `|`.
   **Pure namespaces**, never gated: `path` (except `path.resolve`, which touches disk and is
   an fs read), `json`, `csv`, `toml`, `yaml`, `regex`, `log`. **`db` and `repo` are
   in-memory stores** and stay PURE for ordinary method dispatch; the `@versioned` snapshot
   is the one `repo` write that bypasses dispatch and **is gated** (item 4).
2. **`caps` is not optional.** `Interpreter.caps: CapabilitySet`, defaulting to
   `developer_default()` (everything allowed, including unscoped fs) in `Interpreter::new`.
   A `None` that meant "no gate" becomes unrepresentable. **Each of the six production
   embedders receives an explicit set** at the construction site — relying on the default
   is a defect:

   | Site | Set |
   |---|---|
   | `vox-cli` `run` | `developer_default()`, or `--caps` / `// vox:caps` |
   | `vox-cli` `repl` | `developer_default()` |
   | `vox-cli` `play` | `developer_default()` |
   | `vox-langtool` `run` | `developer_default()`, or `--caps` / `// vox:caps` |
   | `vox-terminal-core::eval_line` | **restrictive**: `path`/`json`/`log` only — a one-liner in the terminal is not a filesystem grant |
   | `vox-orchestrator-mcp` workspace dispatch | **restrictive**: `fs:ro=<workspace root>,env:ro,time:real` — an LLM running `.vox` tools must not inherit `developer_default()` |

3. **Denial is fatal.** `EvalError::CapabilityDenied { ns, method }` terminates the run with
   exit 77, one line on stderr (`vox: capability denied: <ns>.<method> — grant with --caps <token>`),
   nothing on stdout. Deferred exit commands do not run — a script denied a capability must
   not get cleanup it queued. `--max-depth` is a real flag (default 1 024) or it is not
   mentioned in any error string.
4. **Every side-effecting entry point is gated**, not only `call_builtin_method`. Four
   bypass paths, all gated:

   - the `fs` arm, with the guard at the **head of the arm keyed by a method table**
     (`read`/`read_file`/`read_to_string`, `read_bytes`, `canonicalize`, `write`/`write_file`/
     `write_to_file`, `cwd`, `copy`, `remove`, `walk`/`list_recursive`, `exists`, `is_file`,
     `is_dir`, `remove_dir_all`, `list_dir` (`vox_list_dir` at `vox-actor-runtime` `mod.rs:331`),
     `glob` (`vox_fs_glob` at `mod.rs:1724`), `list_dir_detailed`, `stat`, `mkdir`, and
     `io.open`/`io.save`), each marked read or write, with unknown methods **denied by
     default** so a new method cannot ship ungated;
   - `path.resolve`, as an fs read;
   - **`import` resolution** in `eval/mod.rs`, as an fs read of the canonicalised target;
   - `process.register_exit_command`, at queue time, under `process`;
   - **`@versioned` auto-snapshot** in `eval/expr.rs`, as a `repo` write. `repo` stays PURE
     for explicit `repo.*` method calls (in-memory, no I/O). The decorator path is gated
     because it bypasses dispatch; under `Sandboxed` mesh trust it is a no-op (the in-memory
     store dies with the child) and under `developer_default()` it proceeds. The gate exists
     so a future backing store cannot ship ungated, and so the four-path list is closed.
   - a test derives the seeded namespace list at runtime from the interpreter's scope and
     asserts every one is classified pure or gated.
5. **Filesystem scoping.** Roots are canonicalised **at parse time** (on macOS `/tmp` is
   `/private/tmp`; on Windows `canonicalize` yields `\\?\C:\…`; comparing a canonical path
   against a raw root denies everything). The check returns the resolved path and the
   syscall uses *that* path, so check and operation name the same file.

   **Parent-walk, not `parent().unwrap_or(".")`.** `Path::new("out.txt").parent()` is
   `Some("")`, not `None`. `canonicalize("")` is `ENOENT`. The resolver walks to the
   nearest existing ancestor and joins the missing suffix; `""`, `.`, and `..` as the
   *final* component are denied, not degraded. A `developer_default()` relative write
   (`fs.write("out.txt", …)`, `fs.mkdir("a/b/c")`) must succeed.

   **Skip resolution when the grant is the whole filesystem.** `developer_default()` has
   `fs_ro = None` / `fs_rw = None` (unscoped). There is no boundary to enforce; calling
   `canonicalize` twice per fs builtin (measured 47.5 µs on a repo path here) is cost
   with no check. Do **not** cache resolved paths — that converts the documented
   final-component TOCTOU into a whole-run one and defeats the symlink test.

   **`fs.glob` filters results, not only the prefix.** Split the pattern on `/` and `\`;
   reject any `..` component; glob the pattern; drop every result that fails `allows_path`.
   A prefix-only check plus `rsplit_once('/')` is a sandbox escape on Windows
   (`C:\Windows\System32\config\*` has no `/`, so `dir` falls back to `"."` — the job
   dir, an rw root — and the raw pattern is globbed) and on Unix
   (`/tmp/job/*/../../etc/*`).

   Windows compares components case-insensitively; `component_eq` covers `Prefix` for the
   UNC-casing gap. Verbatim / UNC / SUBST / 8.3 paths are normalised by `canonicalize` on
   both sides. **Residual, stated:** this is not `openat`-based like WASI preopens;
   directory components are resolved twice, and a script that also holds `process:allow`
   can race the final component. Relative paths resolve against the process CWD; over the
   mesh that is the job directory, always an rw root.
6. **Memory bound.** A counting `#[global_allocator]` in the `vox-cli` **library** (not the
   binary — `run()` is library code, and two module copies would give the allocator and
   `arm()` different statics). Exceeding the ceiling writes a stack-formatted line and calls
   `libc::_exit(79)` / `TerminateProcess` — never `std::process::exit`, which runs atexit
   handlers that allocate and re-enter the allocator. After `TerminateProcess`,
   `loop { spin_loop() }` — the call is documented asynchronous and may return;
   `unreachable_unchecked()` after it is UB. `realloc` and `alloc_zeroed` are overridden so
   the compiler's own `Vec` growth keeps `realloc(3)`'s in-place path. The Windows crate
   features include `Win32_System_IO` (required by `WriteFile`'s `OVERLAPPED`).
   **Hot path:** `if LIMIT == usize::MAX { return; }` so two atomic RMWs become one shared
   load for every consumer that never sets a ceiling. `USED` and `LIMIT` live on separate
   cache lines. **What it counts:** Rust heap allocations in this process, armed after the
   CLI parse. Not child processes, not `mmap` by C dependencies, not thread stacks, not
   bytes written to disk.
7. **Step bound** (`--max-steps`, exit 78) **bounds evaluated HIR nodes, not work.** A single
   builtin call is one step whatever it does (`str.repeat(1e9)`, `fs.glob("/**")`, a regex
   compile, a 30 s HTTP timeout). Work is bounded by the memory ceiling, the disk/file
   caps, the regex size limit, and, over the mesh, by the receiver's wall-clock kill, which
   the job cannot influence. **A locally-run script has no wall-clock bound; that is a
   deliberate difference between the local and mesh tiers.**
8. **Depth bound.** Vox recursion maps onto Rust stack frames; without a limit
   `fn f() { f() }` SIGSEGVs the process, defeating every other bound and surfacing as
   `exit None`. The counter lives in `apply_closure`, **not** `eval_expr` — a guard struct
   in `eval_expr` increases native stack per Vox recursion level and shrinks the headroom
   the bound exists to protect. Expression nesting is already bounded statically by the
   parser. Default 1 024; `--max-depth` is the flag. Exit 78.
9. **Output bound** is enforced by the parent (`InterpExecutor`) reading a capped pipe; a
   truncated result carries a marker line so the peer can tell. The truncation flag is
   computed **before** the buffer extend: a job producing exactly `max` bytes is not
   marked, and must not be pushed over the cap by a spurious marker.
10. **Determinism knobs.** `time:frozen`, `random:seed`, and sorted directory enumeration
    (`fs.list_dir`, `fs.glob`) on **both** tiers. The float surface is IEEE-exact
    (`abs floor ceil round sqrt`); transcendentals, when added, go through the Vox-owned libm.
11. **Regex compile size.** Shared `RegexBuilder` with `size_limit(1 << 20)` (1 MiB compiled
    program). The crate default is 10 MiB and a compile is one step, so the step budget
    does not bound it. Unbounded locally, where there is no wall clock.
12. **Disk and file caps (mesh only; in-process).** `JobLimits` grows `max_disk_bytes`
    (default 32 MiB) and `max_files` (default 4 096). The fs write / mkdir / copy arms
    account both and deny (`CapabilityDenied { ns: "fs", method: "quota" }`, exit 77) at
    the cap. Local `developer_default()` is uncapped — a developer machine is not a mesh
    job. Size-capped tmpfs is **not** used: it needs a privileged mount.

**Threat model, stated.** This defends against **malicious or buggy Vox code**. It does not
defend against a bug in the interpreter or its pure-Rust dependencies (`regex` is
linear-time once compiled; `serde_json` has a depth limit); the process boundary in §3.4
is the mitigation.

**Verified closed** (do not re-investigate): forging `{__namespace__: "fs"}` reaches
dispatch but the gate keys on the same string and `allows_path` still scopes it; `_Denied`
/ `_Panic` are converted at the single `call_builtin_method` return site before any
`match` / `Option` wrapping; an existing-but-outside path and a nonexistent path yield the
same `CapabilityDenied`, and denial is fatal, so a script cannot probe-then-measure.

**`process:allow` is shell access:** `process.exec` replaces the interpreter's process image
and `process.spawn_background` outlives it; no interpreter property survives either. Over the
mesh, `process` is denied for `Sandboxed` trust and `grant_native` — never pairing — is the
only way to grant it, and the documentation says "runs any binary on this host as the daemon
user" in those words.

**Denial marker.** A script can call `process.exit(77)` to forge a denial; the executor
therefore requires the stderr marker as well as the exit code. `log.error` is PURE and can
write the marker string to stderr, but reaching exit 77 needs `process.exit`, which
`Sandboxed` denies. The two-factor check holds — only an already-`Native` peer could forge
a verdict. **Design smell, not a hole:** prefer a channel the script cannot write (fd 3 or
a status file) when convenient; not a revision-3 blocker.

### 3.3 One builtin surface, and proof the tiers agree

- `vox-compiler::builtin_registry` is the intended SSOT for every builtin. This program does
  not yet make `eval/builtins.rs` table-driven (2,867 LoC); it closes the measured drift and
  makes new drift a differential-gate failure.
- **Asymmetry rule (one rule).** The differential gate has no exclusion hatch. A golden
  that would disagree is either fixed before it is committed, or it carries
  `// EXPECT-TIER-ASYMMETRY: <reason>` and the gate **fails when the tiers start agreeing**
  (the reason expired). There is no in-process `KNOWN_TIER_ASYMMETRIES` list. `log.*` is
  not an authorised asymmetry. After the decisions in §5, the authorised residual set is
  empty on day one.
- **Parity fixes in scope:** `overflow-checks = true` in the native script profiles (one
  line; makes both tiers halt, by construction); a shared `vox_display` so `str()`/`print()`
  of lists, objects, tuples, `Option`, `Result`, and tagged values print the same text on
  both tiers (today neither is acceptable); `print` of n arguments; `env.args()` returns
  `[<script path>] ++ args` on both tiers and `run_interp` actually threads the args;
  sorted `fs.glob`/`fs.list_dir` with propagated errors on both; a `.sorted()` codegen arm;
  the native `main` catches panics and exits 1 so script faults agree (0 and 77–79 are
  contractual, 1 is the fault code, 101 means "the interpreter/binary has a bug");
  **`list.push` in-place + `Rc<str>`**; **object iteration is insertion-ordered on both
  tiers** (`features = ["preserve_order"]` on `serde_json` in the generated crate
  manifest — zero workspace blast radius); **`crypto.*` and id builtins share
  `vox-crypto` as SSOT** (hex helpers live there; both tiers call them).
- **Differential gate.** Every golden with `// EXPECT:` runs under both tiers and stdout is
  diffed; `// EXPECT-EXIT: nonzero-both` covers faults whose stdout is a prefix.
  **Eight new goldens** cover overflow, division by zero, composite display, float
  formatting, object order, directory order, argv shape, and crypto parity. The gate
  `current_dir`s the **repo root** (Cargo sets an integration test's CWD to the package
  root; a glob of `examples/golden/*.vox` from `crates/vox-integration-tests/` matches
  nothing and `[] is [].sorted()` is true on both tiers). It wipes `~/.vox/script-cache`
  per run — the cache is keyed on the source hash, not the build profile, so a machine that
  ran overflow goldens before `overflow-checks` flipped keeps a wrapping binary forever.
  Native goldens are emitted as `[[bin]]` targets of **one** generated crate (one
  764-crate dependency build plus N leaf links, ~6–8 min), not one crate per golden
  (~87 min, killed by nextest `slow-timeout` after two). The gate lives in a **nightly
  lane**, not `--include-slow`. It is registered in both `pre_push.rs`'s nextest filter
  **and** the hand-maintained duplicate at `.github/workflows/ci.yml` (~1124–1131). CI's
  main nextest step does not pass `--run-ignored`; a local-only registration never runs
  on CI.
- `@test` blocks already execute under the interpreter; `vox test` (cargo) is the opt-in for
  native-lane test runs and says so.

### 3.4 Mesh execution

**The mesh ships two things, and only two: VoxScript source, and declarative ML jobs.** It
never ships native code.

| `TaskKind` | Executor on the peer | Isolation |
|---|---|---|
| `VoxScript` | `InterpExecutor` → child `vox run --mode interp --caps … --caps … --max-steps … --max-memory … --max-depth …` | the interpreter (§3.2) + process boundary. **Not** `sandbox.rs` — that file has two call sites, both native; the interpreter child gets no Landlock / seccomp |
| `TextInfer`, `Embed`, `ImageGen`, `SpeechTranscribe`, `TrainQLoRA` | declarative `{engine, model, input}` → a **locally installed engine** the peer already trusts | none needed — no foreign code executes; admission refuses kinds with no engine |

- `InterpExecutor` lives in `vox-mesh-transport` (L2): it spawns a child and needs no
  compiler dependency, so **no new crate edge** there. Every new `pub fn` in
  `interp_executor.rs` and `caps_spec.rs` has a same-file test
  (`skeleton/untested-pub-api`). It reuses the curated passthrough from
  `remote_worker.rs::baseline_passthrough_env()` (defactored, ~13 names) — a raw
  `env_clear()` drops `LD_LIBRARY_PATH` and breaks a `vox` built in a Nix shell.
- **Child environment.** Unix: the curated passthrough including `PATH`, `HOME`,
  `TMPDIR`, `LD_LIBRARY_PATH`. Windows: `Path`, `USERPROFILE`, `TEMP`, `TMP`,
  `SystemRoot` (without it Winsock and DLL loading break). `HOME`/`USERPROFILE` point at
  a **second, read-only tempdir**, not the writable job directory, so a script cannot
  author a `~/.vox/config.toml` that a later config load reads. On Windows
  `dirs::home_dir()` uses `SHGetKnownFolderPath` and ignores `USERPROFILE` — stated
  honestly; the real boundary is that neither profile is under an `fs` root. CWD is the
  job directory. Unix: spawned in its own **process group** so the wall-clock kill and
  `Cancel` reach grandchildren. Windows: a **Job Object** via the existing `win32job`
  workspace dependency (no new crate edge). Pipe drains are bounded and **aborted on
  cancel** — dropping a `JoinHandle` does not abort the task; up to 2 detached tasks and
  2 open pipe fds otherwise leak per cancel, which an attacker controls.
- **Capabilities** come from the **receiver's** trust row, through a typed constructor,
  never a formatted string: `Sandboxed` → `fs:rw=<job dir>` + `time:real`; `Native` →
  those plus `net:allow`, `process:allow`, `env:ro`. `secrets` is never granted by trust
  level. Tokens are passed as repeatable `--caps`.
- **`JobLimits` is the one place node policy lives**: `wall_clock`, `max_output_bytes`,
  `max_payload_bytes`, `max_memory_bytes`, `max_steps`, `max_depth`, `max_disk_bytes`,
  `max_files`, and a per-kind payload cap — **4 MiB for `VoxScript`** (source is text;
  1 GiB was sized for the bundle lane this design deletes). `max_payload_bytes` default
  drops to 16 MiB. The receiver checks the claim, reads the frame with the varint
  allowance, and refuses a payload whose length differs from the claim. `max_steps` is
  **threaded into the child** (`--max-steps`); a test that loops 3 M times over the
  CLI's 10 M default exits 78 before printing `DONE`.
- **Concurrency is bounded per node** by a semaphore sized to available parallelism,
  **and per `EndpointId`** in front of that semaphore. One trusted peer must not take
  every slot. Past either cap the executor refuses with a retryable message rather than
  queueing behind a wall clock. `QueueStats` reports `pending_count` (running) **and**
  `max_concurrent` so Phase 4 placement does not read a metric whose range is `0..=N`
  regardless of load. Adding `max_concurrent` is free while PROTO is already bumping.
- **Job identity.** `JobId` is a newtype. `JobRequest::Run` carries a **sender-assigned
  `job_id`**. The running map is keyed by `(EndpointId, JobId)`. A `Run` whose
  `(peer, JobId)` is already live is **refused** — `HashMap::insert` over an occupied
  key drops the first job's `Sender`, whose `cancel_rx` then resolves `Err(RecvError)`,
  and `_ = cancel_rx` cannot tell that from a real cancel. `Cancel` looks up only the
  caller's own entries — one message for "not yours" and "not there". The cancel arm is
  the refutable pattern `Ok(()) = cancel_rx`. A payload-hash id was derivable by any
  peer holding the same script and collided on identical concurrent jobs.
- **Protocol.** `Isolation { Interpreter, Native }` replaces `{ Wasm, Container, Native }`;
  `DEFAULT_FOR_MESH = Interpreter`; `PROTO` bumps to 2. **Every frame is version-locked by
  `PROTO`.** postcard is positional: `#[serde(default)]` on a trailing field does *not* let
  an old sender's frame decode (the reader's field count bounds the sequence, so a short
  buffer is `DeserializeUnexpectedEnd`); the existing `task_kinds` default is ornamental
  for the same reason, and a test pins that fact with the **same variant index** plus a
  positive control (a dummy variant at index 0 proves a discriminant mismatch, not the
  trailing-field question). `Hello` is frozen so a mismatch is *diagnosable*: the
  receiver answers a `Failed` frame naming both versions and closes with
  `REFUSED_PROTO = 4003` (4001 untrusted, 4002 too large, 4004 no mailbox; **4003 is
  free**). `directory.rs` must **keep** that `Failed` — mapping `_ => None` makes a v1
  peer indistinguishable from a switched-off machine. `vox mesh probe` is the operator
  surface over `directory()`. `Probed` gains `engines`.
- **Exit-code mapping in the executor:** 0 → output; 77 → denied **only with** the stderr
  marker; 78 → limit; 79 → memory; 101 → "interpreter bug", stderr **not** forwarded
  (backtraces leak host paths); Unix `None` → "killed by signal (stack overflow or OOM)";
  Windows NTSTATUS faults render with names (`STATUS_STACK_OVERFLOW`), not
  `-1073741571` — Windows has no signals, so the `None` arm never fires there.
- The HTTP A2A bundle lane (`run_dispatched_bundle`, `BundleKind`, the `VoxMeshExecPolicy`
  SecretId and its `no-exec`/`source-only`/`permissive` ladder) is **deleted now**, not in
  Phase 6. `secret_gate.rs` goes with it. The HTTP source lane gains `--caps` scoped to a
  **per-dispatch** tempdir (not the shared `/tmp`).
- **`PopuliHttpOp::Dispatch`** runs on a mesh peer — *once it has a payload that can
  execute*. The current synthesised source, `workflow_durable_shim::execute_activity(…)`,
  names a symbol that exists nowhere in the repository. `Dispatch` is supported for
  activities that carry dispatchable source and errors by name otherwise; the shim
  synthesis is deleted as dead. `Wait` becomes inline and keeps
  `success`/`result_output`/`exit_code` keys so existing readers degrade instead of
  failing.
- **`vox mesh join` states what it now permits.** Task 10 swaps the executor; the banner
  must say the command now executes Vox received over the mesh, not just that a node
  joined. One `tracing::info!` per job (peer, job id, exit, elapsed) — Phase 4
  `PlacementRecord` wants data this executor otherwise computes and discards.
- **Inbox drain is not in this program.** `Inbox::messages` has zero production consumers
  today; draining it needs the agent-id→`EndpointId` mapping that also caps `mesh_relay`
  at one peer.

**Accept-loop permit and post-handshake deadline are out of this program.** On
`mesh-phase3-plan` today, `endpoint.rs` moves the accept permit into the spawned task and
holds it through `handle()` — up to 300 s of authenticated execution — for a gate that
exists to bound "~100 µs of *unauthenticated* work". On any host with ≥64 hardware
threads the executor's own semaphore is ≥64 = `MAX_INFLIGHT_HANDSHAKES`, so 64
legitimate slow jobs wedge the accept loop: no `Probe`, no `QueueStats`, no `Cancel`.
There is no read deadline after the handshake; `HANDSHAKE_TIMEOUT` covers only
`incoming`. One trusted peer opens 64 connections, sends zero bytes, and the node stops
accepting from anyone, indefinitely, at zero cost. The peer-scoped cancel map is
unreachable exactly when it matters. **Fix, separate commit against merged Phase 3:**
`drop(permit)` after the trust check, and a `REQUEST_TIMEOUT` on every post-handshake
frame. Not buried in this plan.

**Consequence for the outstanding mesh work.**

| Item | Effect of this design |
|---|---|
| "No sandbox exists" gap | closed as an *isolation* boundary by §3.2 + §3.4; not a resource-containment boundary |
| F2 (unsandboxed executor) | eliminated by construction: nothing native is ever received |
| Accept-loop DoS on merged Phase 3 | **not funded here**; separate commit, recorded above |
| Task 3.1 ceiling (agent-id → `EndpointId`) | unchanged; blocks both `mesh_relay` and the inbox drain |
| Task 3.1 inbox drain | **not funded here**; dependency stated above |
| `task_submit.rs` / lease-over-mesh | unchanged; PROTO 2 was the cheap moment to add `Lease` and does not — that is a deliberate scope cut, and PROTO 3 will be needed |
| Q4 mDNS, Windows DPAPI | unchanged |
| Phase 4 placement | `engines` and `max_concurrent` on the wire; first-fit peer choice records the chosen peer and candidate count as the seam |
| Phase 5 Axis | data exists; no GUI task here |
| Phase 6 deletion | bundle lane and `VoxMeshExecPolicy` leave now; `vox-populi/src/transport/handlers/dispatch.rs` survives with a deprecation marker |

### 3.5 Deletions and retirements

Deleted outright:

- `remote_worker.rs::run_dispatched_bundle`, `BundleKind`, `classify_bundle`, the
  `VoxMeshExecPolicy` SecretId and its ladder, `secret_gate.rs` (+ `secret_bag.rs` if it has
  no other consumer), `envelope.rs` `exec_bundle_*` fields.
- `vox-cli` feature `script-wasi`, `WasiBackend`, `vox wasm run`, `--isolation` for
  `vox run` / `vox script`, `isolation.rs` (`IsolationPolicy`, `IsolationCapabilities`).
  Nothing in the repository enables the feature; it needs cargo + rustc + a rustup target;
  its only advantages over the interpreter are now interpreter properties.
  **`vox-wasm-engine` and `vox-plugin-runtime-wasm` stay** — plugins are a different
  surface. The orphaned `codegen_rust/pipeline.rs` `WasiBinary` target and
  `vox-script-wasi` path dependency go too.
- `vox-skill-runtime::microvm` (`MicroVmRuntime`, every method `bail!`s), `Tier::MicroVm`,
  and its test file — with the file's two real assertions (tier ordering,
  `plan_for_min_tier` error path) moved, not lost.
- `vox-mesh-transport::ProbeOnlyExecutor`, `Isolation::{Wasm, Container}`.
- `sandbox.rs`'s "Other: warning + `VOX_SANDBOX=1` hint" branch **and** the second
  `VOX_SANDBOX=1` at `backend/native.rs`. macOS has no OS-level sandbox in Vox; the file
  says so instead of setting an informational variable. This is honesty about the native
  lane, not defence-in-depth for the interpreter.
- `voxup::provision_wasm_sysroots` (creates an empty directory nothing reads).

**Retirement, not just deletion.** `contracts/retirement/retired-surfaces.v1.yaml` and
`contracts/documentation/retired-symbols.v1.yaml` exist so an agent cannot re-introduce
`--isolation wasm`, `vox wasm run`, `ProbeOnlyExecutor`, `MicroVmRuntime`, or
`VoxMeshExecPolicy` from stale training. Rows are added for each, and to AGENTS.md
§Retired Surfaces.

Retained, but corrected:

- `vox_ir` (87 LoC) is a JSON export behind `vox check --emit-ir`, not an IR. Renamed
  `hir_export` with a **scoped** symbol rename (not a blind `sed` over `crates/`), and the
  public `vox-ir-specification.md` page plus both schema mirrors updated together.
- **`VOX_MESH_EXEC_POLICY` exists twice.** The SecretId (deleted) and a **config key** in
  `vox-config`/`vox-gui` meaning task *placement* (`local_only`/`prefer_remote`/`remote_only`).
  The config key survives. Removing the SecretId drops the name from the managed-secret
  regex, so `secret-env-guard` stops policing direct `env::var` reads of the survivor —
  noted in the registry so the collision is not rediscovered.

### 3.6 Repository-wide consequences

- **Automation gets faster — after the scripts actually run.** lefthook and CI invoke
  `vox run scripts/*.vox` with no mode and move from the native lane to the interpreter.
  **`scripts/install-hooks.vox` and `scripts/setup.vox` fail under the interpreter
  today.** Both pass `vox check`. Task 0 executes, not typechecks; those two are Task 7
  blockers and must be green before the flip. The measurement table is re-run **after**
  filesystem scoping lands, because `fs_resolve_allowed` as specified in revision 2
  denies every relative create. `setup-e2e.yml` pins `--mode script` on the three lines
  that exist to test the native lane, so `--features script-execution` does not become
  decorative.
- **Contract chain.** Deleting `vox wasm` reaches `contracts/operations/catalog.v1.yaml`,
  `contracts/cli/command-registry.yaml`, the capability registry and model manifest, both
  `gui-surface-*.v1.json` reports, the command-catalog test baseline, and `cli.md`'s
  command table. `vox ci command-sync` alone regenerates only the markdown from an input
  it does not update; the chain is regenerated in order or `ssot-drift` fails on four
  sub-gates. Removing the SecretId requires `vox ci secrets-contracts` **before**
  `secrets-parity`.
- **The two HTTP dispatch handlers** (`vox-populi`, `vox-plugin-populi-mesh`) spawn
  `vox wasm run` and `vox run --isolation wasm` — commands this design deletes. They fail
  at runtime, not compile time, and are fixed in the same change.
- **`vox doctor`** reports five real rows for the execution tier, including "executor
  binary accepts `--caps`". The permanently-green tombstone of a deleted feature is
  deleted, not rewritten. Doctor sources live under `diagnostics/` (not a bare
  `commands/` path); `compilerd.rs` is not under `commands/`.
- **Docs.** `docs/src/reference/isolation.md` (`category: Language Reference` — the page
  `script.rs` already tells users to read, which does not exist); ADR-048
  (`category: Architecture Decisions (ADRs)`, shape of ADR-047); `AGENTS.md` tier table;
  `GEMINI.md` and `.cursor/rules/voxscript-first-automation.mdc` (both name
  `--isolation wasm`); **four `where-things-live.md` rows in the same PR that creates
  the surfaces** (AGENTS.md same-PR rule); the mesh plan's Status; two markdown links to
  deleted files that `vox ci check-links` would fail on.
- **Contracts.** This program **takes** the `vox-compiler → vox-crypto` edge (L2→L0,
  downward, layer-legal, user-authorized 2026-09-06). Hex helpers for both tiers' hash/id
  builtins live in `vox-crypto`, which also closes the existing `vox-actor-runtime`
  direct-hash violation of AGENTS.md §Cryptography. The implementer **proposes** the
  `crate-edges` exceptions text in the PR description and stops; they do not write the
  exceptions ledger or regenerate `crate-edges.allow.v1.json`. Never add a `vox-cli`
  dev-dependency to `vox-mesh-transport` (upward edge). Removing `script-wasi` removes
  `vox-cli → vox-wasm-engine`; tighten with `vox ci crate-edges --tighten`.
- **MENS corpus.** The differential gate's pass/fail is a clean reward signal.
- **Rollout is six PRs**, by revertability. Task 7 (the default flip) is alone in PR 4 —
  one file, one predicate, one `git revert`. PR 5 is the fleet point of no return:
  `git revert` restores code, not the machines already upgraded. Docs move into the PR
  that creates each surface. Task 16 is a verification pass, not a PR. macOS will not
  prompt for a password: the application firewall is disabled (`State = 0`) and cargo
  under `sudo` is forbidden (it root-owns `target/`). The one genuine interrupt is the
  Local Network Privacy TCC dialog, fired the first time `vox` touches the local
  network — Task 16, at the keyboard.

### 3.7 Use without `.vox` scripts — dependency map

| You run… | Needs |
|---|---|
| a pure VoxScript | `vox` |
| a VoxScript over the mesh | `vox` on both ends |
| a VoxScript that imports a Rust crate | `vox` + cargo + rustc (`--mode script`) |
| a Vox web/desktop app, or a `table`/`routes`/`server`/`workflow` program | `vox` + cargo (+ node/pnpm for web), as today |
| an ML job over the mesh | `vox` + the named engine installed on the executing peer |
| a Vox plugin in wasm | `vox` (embedded `vox-wasm-engine`, unchanged) |

### 3.8 What this design does not claim

- **Bit-identical output across operating systems for jobs granted `fs` or `process`** —
  path separators, directory order (sorted by the runtime, but the *set* is the host's),
  and subprocess output are the host's. Deterministic execution is the `deterministic`
  capability profile, not a property of the tier.
- **GPU numerics identical across vendors** — not achievable by anyone (MLX diverges
  across its own two backends). Results are asserted within tolerance.
- **Peak compute throughput under the interpreter** — expect 3–10× off native. After
  `list.push` is in-place, collection builders stay the same complexity class; before
  that fix they do not.
- **Defence against interpreter bugs** — mitigated by the process boundary and OS limits.
- **Identical exit codes across tiers for script faults** until the native `main` catches
  panics; after that, 1 on both. Only 0 and 77–79 are contractual.
- **A bytecode VM or JIT** — explicitly deferred.
- **JSON object key order preserved from `json.parse`** — `json.parse` is key-sorted on
  both tiers today; that is deterministic, not insertion-ordered. **Literal object
  iteration** is insertion-ordered on both tiers after the generated-crate
  `preserve_order` line.
- **Resource containment of disk/inodes/slots without the caps in §3.2.12 and §3.4.**
  Those caps are in this design; a privileged tmpfs mount is not.
- **That `sandbox.rs` isolates the interpreter child.** It does not.
- **That an old `vox` on `PATH` is a hook risk.** `lefthook.yml` uses `cargo run`, not a
  PATH `vox`. Do not build a version guard for that.

## 4. Testing

- **Mutation-verified guards** (AGENTS.md rule), each with a **minimal** mutation (one
  namespace or one method — not "delete the whole `_Denied` return"): capability denial,
  the fs symlink check, `fs.glob` result filtering, the import gate, the `@versioned`
  gate, the memory ceiling, the trust-row→caps mapping, the `Cancel` ownership check,
  duplicate-`JobId` refusal.
- **Differential gate** (§3.3) with the eight new goldens is the acceptance test for
  tiers agreeing. No `KNOWN_TIER_ASYMMETRIES` list. Goldens that glob the repo assert
  non-emptiness. The gate `current_dir`s the repo root and wipes the script cache.
- **Registry symmetry test** for builtin coverage.
- **Process boundary** (the non-interpreter half of isolation) has tests: `env_clear`
  drops unlisted vars and keeps the curated list; `HOME`/`USERPROFILE` point at the
  read-only tempdir; Unix process-group kill reaches a grandchild; Windows Job Object
  kill reaches a grandchild; wall-clock kill fires.
- **Mesh, on loopback:** a `VoxScript` job runs and returns output; a path outside the
  job dir is refused fatally with the denial on stderr; a memory blow-up uses
  `s = s + s` doubling (26 iterations to 64 MiB, not the push loop) and is reported as
  79; output of exactly `max` bytes is not marked truncated; output over the cap is;
  `Cancel` kills the caller's own child and not another peer's; a reused `JobId` from
  the same peer is refused; a payload of exactly the declared size is accepted and a
  short one refused; a v1 `Hello` gets a `Failed` naming both versions **and**
  `vox mesh probe` surfaces it; ML kinds are refused with "no engine"; a peer with no
  trust row cannot `Run`; a forged exit 77 **under `grant_native`** (the only threat
  model where forgery matters — `process.exit` is itself gated) is not reported as a
  denial. `dispatch_runs_real_source_on_a_loopback_peer` is a real assertion, not three
  comments.
- **Embedder sets.** A test greps or type-checks that the six production sites pass an
  explicit `CapabilitySet`, and that the MCP / `eval_line` sets are restrictive.
- **Repository scripts:** every `scripts/**/*.vox` is *executed* (not only `vox check`)
  under the interpreter **twice** — once before any code change (Task 0) and once after
  filesystem scoping (after Task 5) — before the default flips. The ones that
  legitimately cannot are listed with a reason and run with `--mode script`.
  `fmt` / `install-hooks` / `setup` / `arch-check` must show `ok` in the run column on
  the second run.
- **Task 7 proof** is `PATH=""` (no cargo), not a 5 s stopwatch against a warm script
  cache.

## 5. Risks and decisions recorded

| Decision | Basis | Call |
|---|---|---|
| Object iteration order | Generated crate is standalone (`[workspace]`, `serde_json = "1"`); workspace already has `preserve_order` via `workspace-hack` | **Insertion order.** One line in the generated manifest. Zero workspace blast radius |
| `vox-compiler → vox-crypto` edge | L2→L0, downward; `vox-actor-runtime` hashing directly already violates AGENTS.md §Cryptography | **Take it**, user-authorized 2026-09-06. `vox-crypto` is SSOT for both tiers' hash/id builtins. Implementer proposes the `crate-edges` exceptions text and stops |
| Accept-loop permit + missing frame deadline | Live on merged Phase 3 today; not introduced here | **Separate commit** against merged Phase 3. Close code **4003** is `REFUSED_PROTO` |
| Permanently-red golden | A red gate teaches contributors to ignore it | **No.** Decisions forced before the corpus lands. `crypto_hash_parity.vox` ships with the PR that takes the edge |
| Disk/inode DoS | Size-capped tmpfs needs root; this machine will not elevate; cargo under `sudo` root-owns `target/` | **In-process** `max_disk_bytes` / `max_files` on `JobLimits`. Local `developer_default()` uncapped |
| macOS elevation | Application firewall `State = 0`; no `vox` entry | **No password dialog.** Local Network Privacy TCC fires at Task 16, at the keyboard |

- **PROTO 2 is incompatible with a PROTO 1 peer by design.** BLAPTOP04 must be rebuilt
  before the cross-machine smoke. The mismatch is diagnosable (`Failed` + `vox mesh
  probe`), not silent.
- **`--caps` grammar stability.** Public CLI surface once shipped; keep it small, version
  it in `isolation.md`. Repeatable flags, not comma-joined tokens.
- **Global allocator in the `vox-cli` library** applies to every binary linking it
  (`vox-gui`, `vox-ml-cli`, test crates): one shared load per allocation when disarmed
  (`LIMIT == usize::MAX`), and no dependent crate may declare its own
  `#[global_allocator]` (none does today).
- **Interpreter performance on real repo scripts** is measured by *execution* (Task 0)
  before the flip, and again after Task 5. `-- --help` short-circuits collection loops;
  the `list.push` fix is a prerequisite of the flip regardless of what Task 0's help
  short-circuit reports.
- **macOS Seatbelt** stays optional defence-in-depth for the *native* lane; this design
  does not depend on it, and `sandbox.rs` does not wrap the interpreter child.
- **`std::sync::Mutex` on the running-job map is load-bearing.** `execute` returns
  `Pin<Box<dyn Future + Send>>`; a `tokio::sync::Mutex` guard across that await is a
  compile error today and must not be "upgraded".
