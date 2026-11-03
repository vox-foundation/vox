# Interpreter-First Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Revision 3 (2026-09-06)** — rewritten from the critique ledger after two review rounds
(fifteen tracks). Revision 2's own corrections introduced the worst defect in the
program: `fs_resolve_allowed` would have denied `fs.write("out.txt")` under
`developer_default()`. Four tests could not fail. The differential gate would have
rebuilt 764 crates per golden and never run on CI. Split into six revertable PRs.

**Goal:** Make the HIR interpreter the default and isolation tier for VoxScripts — locally
and over the mesh — so `cargo`, `rustc`, and a Vox checkout stop being end-user
requirements for running pure Vox.

**Architecture:** Capabilities are receiver-imposed, non-optional and fatal; heap,
recursion, output, disk bytes and file counts join steps as hard bounds; every
side-effecting entry point (including `import` and `@versioned`) is gated. The
interpreter is an isolation boundary, not a resource-containment one beyond those
counted bounds. The mesh executes `VoxScript` by spawning `vox run --mode interp` as a
bounded child from `InterpExecutor` in `vox-mesh-transport` (no new crate edge there)
and never ships native code. A differential gate over one generated crate proves the
tiers agree. The wasi script lane, the HTTP native-bundle lane, `secret_gate`,
`ProbeOnlyExecutor`, and the parsed-then-rejected isolation tiers are deleted and
retired.

**Tech Stack:** Rust 1.96 (edition 2024, let-chains), `vox-compiler::eval`,
`vox-mesh-transport` on `iroh 1.1` (postcard frames — positional), `tokio::process`,
clap 4.5, existing `win32job`.

**Spec:** [`docs/superpowers/specs/2026-09-05-interpreter-first-execution-design.md`](../specs/2026-09-05-interpreter-first-execution-design.md)
(revision 3). Ledger: [`docs/superpowers/specs/2026-09-05-interpreter-first-critique-ledger.md`](../specs/2026-09-05-interpreter-first-critique-ledger.md).

## Global Constraints

Copied from spec §3.6 / §5. Every task implicitly includes this section.

- **Test-first.** Every new `pub fn` gets a test in the same file. Write the failing
  test, watch it fail, then implement. `interp_executor.rs` and `caps_spec.rs` are not
  exempt (`skeleton/untested-pub-api`).
- **Mutation-verify every guard** (spec §4) with a **minimal** mutation (one namespace
  or one method — never delete the whole `_Denied` return). Break it once, confirm the
  test fails, restore, `grep -c` the restoration, record it in the commit body.
- **Formatting:** `cargo fmt -p <crate>`. **Never `cargo fmt --all`.**
- **Clippy before every commit:** `cargo clippy -p <touched-crate> --all-targets -- -D warnings`.
  Use let-chains. Nested `unsafe {}` inside `unsafe fn` is required under edition 2024.
- **Never `presets::N0`, `N0DisableRelay`, or `into_0rtt()`.**
- **Crate edges.** This program **takes** `vox-compiler → vox-crypto` (user-authorized
  2026-09-06). At PR 2, propose the `exceptions` ledger text in the PR description and
  **stop** — do not write the exceptions entry or regenerate
  `crate-edges.allow.v1.json`. Never add a `vox-cli` dev-dependency to
  `vox-mesh-transport`. Removing `script-wasi` removes `vox-cli → vox-wasm-engine`;
  tighten with `cargo run -q -p vox-cli -- ci crate-edges --tighten`.
- **`--features populi` goes on `-p vox-ml-cli`, never `-p vox-cli`.**
- **Do not regenerate `docs/agents/doc-inventory.json`.**
- **Contract regeneration** is part of the task that causes it, in this order when a
  command is deleted: `contracts/operations/catalog.v1.yaml` →
  `contracts/cli/command-registry.yaml` →
  `contracts/capability/{capability-registry.yaml,model-manifest.generated.json}` →
  `docs/src/reference/cli-command-surface.generated.md` →
  `contracts/reports/gui-surface-{registry,coverage}.v1.json` →
  `UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli command_catalog`.
  `vox ci command-sync` alone is not sufficient. When a SecretId is deleted:
  `vox ci secrets-contracts` **before** `vox ci secrets-parity`.
- **Doc frontmatter** on every new `.md` under `docs/src/`.
  `isolation.md` → `category: Language Reference`.
  ADR-048 → `category: Architecture Decisions (ADRs)`, shape of ADR-047.
- **Docs land in the PR that creates the surface.** `where-things-live.md` rows land
  in PR 3 with the surfaces they name.
- **Terminology.** One term each: **script-shaped** (vs service-shaped);
  **denial marker** (`vox: capability denied:`).
- **One asymmetry rule.** No in-process `KNOWN_TIER_ASYMMETRIES`. A golden that would
  disagree is fixed before commit, or it carries `// EXPECT-TIER-ASYMMETRY: <reason>`
  and the gate **fails when the tiers start agreeing**. After §5 decisions the residual
  set is empty on day one. `log.*` is not authorised.
- **`--caps` is repeatable** (`ArgAction::Append`). Do not comma-join tokens on the CLI.
- **Never cargo under `sudo`.** It root-owns `target/`. No password dialog is required
  (application firewall `State = 0`).
- **Accept-loop permit + post-handshake deadline are out of this plan.** Separate
  commit against merged Phase 3. Close code **4003** is free (`REFUSED_PROTO`).
- **`std::sync::Mutex` on the running-job map is load-bearing.** Do not "upgrade" it
  to tokio's.
- **Commit messages:** imperative subject < 72 chars, body explains why, ending with
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. Do not push unless asked.
- **Fresh worktree gotcha:** build `crates/vox-gui/ui/dist` once or workspace clippy fails.
- **`PROTO` 1 → 2 in Task 8 is incompatible by design.** Rebuild both ends before Task 16.
- **Line numbers** were verified on `mesh-phase3-plan` @ `6f85bff02`; re-`rg` before editing.

## PR map

| PR | Tasks | Revert story |
|---|---|---|
| 1 | 0, 1, 1b | No behaviour change. Ships the gate and its corpus. |
| 2 | 2 | Both tiers' display, overflow, argv, glob, `list.push`, crypto SSOT, `preserve_order`. Stop-the-line on crate-edges. |
| 3 | 3, 4, 5, 6 | Additive: no `--caps` ⇒ `developer_default()` ⇒ today's local behaviour. |
| 4 | **7 alone** | One file, one predicate. **The git point of no return.** |
| 5 | 8, 9, 10, 11 | **The fleet point of no return.** `git revert` restores code, not upgraded peers. |
| 6 | 12, 13, 14, 15 | Mechanical, contract-heavy. |
| — | 16 | Verification pass, not a PR. Local Network Privacy at the keyboard. |

PR 3 and PR 5 can proceed in parallel after PR 2. PR 4 requires PR 3. PR 6 requires PR 4 and PR 5.

## File Structure

**Created**

| Path | Responsibility | PR |
|---|---|---|
| `crates/vox-compiler/src/eval/caps.rs` | `CapabilitySet`: grammar, legacy shim, typed `from_roots`, `allows_*`. Roots canonicalised at parse. | 3 |
| `crates/vox-cli/src/mem_limit.rs` | Counting `#[global_allocator]`, armed at runtime, `_exit`/`TerminateProcess`. Declared in `lib.rs`. | 3 |
| `crates/vox-mesh-transport/src/interp_executor.rs` | Bounded child; Unix process group / Windows Job Object; per-peer + global slots; `(peer, job_id)` cancel. | 5 |
| `crates/vox-mesh-transport/src/caps_spec.rs` | Defactored `from_roots` + argv tokens. Same-file tests. | 5 |
| `crates/vox-mesh-transport/tests/common/mod.rs` | Shared live-endpoint helpers. | 5 |
| `crates/vox-mesh-transport/tests/interp_executor.rs` | Executor + process-boundary tests. | 5 |
| `crates/vox-integration-tests/tests/golden_differential_gate.rs` | Both-tier stdout diff; one generated crate; nightly. | 1 |
| `examples/golden/{display_composites,float_formatting,object_field_order,glob_and_listdir_order,int_overflow_boundary,division_by_zero,argv_shape,crypto_hash_parity}.vox` | Gate corpus. | 1 |
| `docs/src/reference/isolation.md` | The page `script.rs` already tells users to read. | 3 |
| `docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md` | Decision record. | 6 |
| `docs/src/architecture/script-tier-timings-2026-09.md` | Task 0 measurement. | 1 |

**Modified (major)** — `eval/{mod,builtins,expr,value,env}.rs`; `vox-crypto` (hex helpers);
`vox-cli` run/dispatch/mem; `vox-langtool` run; `vox-terminal-core`; `vox-orchestrator-mcp`;
`vox-mesh-transport` protocol/endpoint/directory; `vox-orchestrator` a2a; contracts and docs
per task.

**Deleted** — `vox-cli` wasm/wasi/isolation; `vox-skill-runtime` microvm; `secret_gate.rs`;
`ProbeOnlyExecutor`.

---

# PR 1 — Gate and corpus (no behaviour change)

## Task 0: Measure — by executing, not only checking

**Files:** Create `scripts/bench-script-tiers.vox`, `docs/src/architecture/script-tier-timings-2026-09.md`.

**Already measured 2026-09-06 (do not treat as green):**
`scripts/install-hooks.vox` and `scripts/setup.vox` fail under `--mode interp` with
`AssertionFailed("called Option.unwrap() on a None value")`. Both pass `vox check`.
`setup.vox` is what `.github/workflows/setup-e2e.yml` runs. `scripts/fmt.vox` runs.
`scripts/arch-check.vox` runs (its own exit 1 is pre-existing). These two FAILs **block
Task 7** and must be fixed in PR 2.

`-- --help` short-circuits collection loops. This table cannot see the `list.push`
O(n²) class change. That fix is a Task 7 prerequisite regardless of this table.

Re-run this table **after Task 5** (Task 5b). Revision 2's resolver would have denied
every relative create and broken four in-repo scripts that this first run cannot see.

- [ ] **Step 1: Write the script**

```vox
// Time every scripts/**/*.vox under `vox check` AND execute under `--mode interp`.
// Native timings are taken by hand (Step 3): a cold native compile is ~275 s each.
pub fn main() {
  let files = match fs.glob("scripts/**/*.vox") { Ok(fs) => fs.sorted(), Error(e) => [] }
  let vox = match process.which("vox") { Some(p) => p, None => "target/debug/vox" }
  let mut rows = []
  for f in files {
    let t0 = time.now_ms()
    let check = match process.run_capture(vox, ["check", f]) {
      Ok(r) => if r.exit is 0 { "ok" } else { "FAIL" }
      Error(e) => "SPAWN-FAIL"
    }
    let dt = time.now_ms() - t0
    let t1 = time.now_ms()
    let run = match process.run_capture(vox, ["run", "--mode", "interp", f, "--", "--help"]) {
      Ok(r) => if r.exit is 0 { "ok" } else { "FAIL(" + str(r.exit) + ")" }
      Error(e) => "SPAWN-FAIL"
    }
    let dr = time.now_ms() - t1
    rows = rows.push("| " + f + " | " + str(dt) + " ms | " + check + " | " + str(dr) + " ms | " + run + " |")
  }
  print("| script | `vox check` | status | `run --mode interp` | status |")
  print("|---|---|---|---|---|")
  for r in rows { print(r) }
}
```

- [ ] **Step 2: Run it** — `cargo build -q -p vox-cli --bin vox && cargo run -q -p vox-cli -- run --mode interp scripts/bench-script-tiers.vox > /tmp/tiers.md; grep -c FAIL /tmp/tiers.md`. List every `FAIL(n)` with the first stderr line. Confirm `install-hooks.vox` and `setup.vox` are among them.

- [ ] **Step 3: Native timings by hand** for `fmt.vox`, `install-hooks.vox`, `setup.vox`, `arch-check.vox`, cold (`rm -rf ~/.vox/script-cache`) and warm, with `/usr/bin/time -p cargo run -q -p vox-cli -- run --mode script <f> -- --help 2>&1 | grep real`.

- [ ] **Step 4: Write the doc** with frontmatter (`title: "Script tier timings (2026-09)"`, `description: "Executed check and interp-run timings for every scripts/**/*.vox, plus the four automation entry points."`, `category: "Architecture SSOTs"`, `status: "current"`), the machine line from `system_profiler SPHardwareDataType | grep Chip`, the native table, the pasted `/tmp/tiers.md`, the FAIL list, and the sentence that `-- --help` cannot see `list.push` complexity. **No placeholder cells.**

- [ ] **Step 5: Lint and commit** — `cargo run -q -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/script-tier-timings-2026-09.md`. `git commit -m "chore(scripts): measure script tiers by execution before flipping the default"`.

---

## Task 1: The differential gate

**Files:** Create `crates/vox-integration-tests/tests/golden_differential_gate.rs`;
modify `crates/vox-integration-tests/Cargo.toml` (`which = { workspace = true }`);
modify `crates/vox-cli/src/commands/ci/pre_push.rs` nextest filter **and**
`.github/workflows/ci.yml` at the hand-maintained duplicate (~1124–1131).
Do **not** put this in `--include-slow`. Add a nightly workflow job (or a row in the
existing nightly lane) that runs this test with `--run-ignored`.

The native lane (`native.rs:20-23`) discards `shared_target` and uses a per-cache-entry
target dir. One crate per golden × 19 × ~275 s ≈ 87 min, killed by nextest
`slow-timeout` (540 s ci) after two goldens. **Emit every EXPECT golden as a `[[bin]]`
of one generated crate** (one 764-crate build + N leaf links, ~6–8 min).

- [ ] **Step 1: Write the test**

```rust
//! Differential gate (spec §3.3): a golden that declares `// EXPECT:` prints the same
//! bytes under interp and a native `[[bin]]` from one generated crate. `// EXPECT-EXIT:
//! nonzero-both` exits non-zero on both and EXPECT lines are a prefix of each stdout.
//! `// EXPECT-TIER-ASYMMETRY: <reason>` fails when the tiers start agreeing.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn vox_binary() -> PathBuf {
    if let Ok(p) = std::env::var("VOX_BIN") {
        return PathBuf::from(p);
    }
    let root = repo_root();
    let exe = root.join("target/debug").join(if cfg!(windows) { "vox.exe" } else { "vox" });
    if !exe.exists() {
        let st = Command::new(env!("CARGO"))
            .current_dir(&root)
            .args(["build", "-q", "-p", "vox-cli", "--bin", "vox"])
            .status()
            .expect("spawn cargo");
        assert!(st.success(), "could not build the vox binary the gate spawns");
    }
    exe
}

fn collect_vox_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_vox_recursive(&p, out);
            } else if p.extension().is_some_and(|e| e == "vox") {
                out.push(p);
            }
        }
    }
}

fn directive_lines(src: &str, key: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| l.trim_start().strip_prefix(key).map(|r| r.strip_prefix(' ').unwrap_or(r).to_string()))
        .collect()
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end().to_string()
}

struct Run { code: Option<i32>, stdout: String, stderr: String }

fn run_interp(vox: &Path, file: &Path, cwd: &Path) -> Run {
    let out = Command::new(vox)
        .current_dir(cwd)
        .args(["run", "--mode", "interp"])
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("spawn `{}` failed: {e}", vox.display()));
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// One generated crate, N [[bin]] targets, `serde_json` with `preserve_order`.
/// Implementation: write a temp crate whose bins are the native-compiled goldens
/// produced by invoking the existing native codegen once per golden *into the same
/// `CARGO_TARGET_DIR`*, then `cargo build --bins` once. Do **not** call
/// `vox run --mode script` per golden — that path discards the shared target dir.
fn build_native_bins(vox: &Path, files: &[PathBuf], bundle: &Path) -> PathBuf {
    let _ = vox;
    let _ = files;
    let _ = bundle;
    panic!("implement the one-crate native bundle in this task");
}

#[test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: one native crate for every EXPECT golden; nightly lane"]
fn golden_expect_blocks_match_on_both_tiers() {
    let root = repo_root();
    let vox = vox_binary();
    assert!(which::which("cargo").is_ok(), "the native half needs cargo on PATH; this gate must not pass silently");

    let cache = dirs::home_dir().expect("home").join(".vox/script-cache");
    let _ = std::fs::remove_dir_all(&cache);

    let mut files = Vec::new();
    collect_vox_recursive(&root.join("examples/golden"), &mut files);
    files.sort();

    let tmp = tempfile::tempdir().unwrap();
    let bundle = tmp.path().join("golden-bundle");
    let expect_files: Vec<PathBuf> = files
        .iter()
        .filter(|f| {
            let src = std::fs::read_to_string(f).unwrap();
            !directive_lines(&src, "// EXPECT:").is_empty()
                || !directive_lines(&src, "// EXPECT-EXIT:").is_empty()
                || !directive_lines(&src, "// EXPECT-TIER-ASYMMETRY:").is_empty()
        })
        .cloned()
        .collect();
    let native_dir = build_native_bins(&vox, &expect_files, &bundle);

    let mut checked = 0usize;
    let mut failures = Vec::new();
    for f in &expect_files {
        let src = std::fs::read_to_string(f).unwrap();
        let expect = directive_lines(&src, "// EXPECT:");
        let expect_exit = directive_lines(&src, "// EXPECT-EXIT:");
        let asymmetry = directive_lines(&src, "// EXPECT-TIER-ASYMMETRY:");
        checked += 1;
        let expected = normalize(&expect.join("\n"));
        let i = run_interp(&vox, f, &root);
        let bin = native_dir.join(f.file_stem().unwrap());
        let n_out = Command::new(&bin).current_dir(&root).output().expect("native bin");
        let n = Run {
            code: n_out.status.code(),
            stdout: String::from_utf8_lossy(&n_out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&n_out.stderr).into_owned(),
        };
        let (io, no) = (normalize(&i.stdout), normalize(&n.stdout));
        if !asymmetry.is_empty() {
            if io == no && i.code == n.code {
                failures.push(format!(
                    "{} declared EXPECT-TIER-ASYMMETRY ({}) but the tiers now agree — remove the directive",
                    f.display(),
                    asymmetry.join("; ")
                ));
            }
            continue;
        }
        let fault_expected = expect_exit.iter().any(|d| d == "nonzero-both");
        let ok = if fault_expected {
            i.code != Some(0) && n.code != Some(0) && io.starts_with(&expected) && no.starts_with(&expected)
        } else {
            i.code == Some(0) && n.code == Some(0) && io == expected && no == expected
        };
        if !ok {
            failures.push(format!(
                "{}\n  expected: {expected:?} (fault_expected={fault_expected})\n  interp:   code={:?} {io:?}\n    stderr: {}\n  native:   code={:?} {no:?}\n    stderr: {}",
                f.display(), i.code, i.stderr.trim(), n.code, n.stderr.trim()
            ));
        }
    }
    assert!(checked > 0, "no golden declares EXPECT / EXPECT-EXIT — the gate has no corpus");
    assert!(failures.is_empty(), "tiers disagree on {} golden(s):\n\n{}", failures.len(), failures.join("\n\n"));
}
```

Implement `build_native_bins` by generating one `Cargo.toml` with `[workspace]` and
`serde_json = { version = "1", features = ["preserve_order"] }`, plus one `[[bin]]`
per golden whose `main.rs` is the output of the existing native codegen. Share one
`CARGO_TARGET_DIR` for that crate. Do not call `vox run --mode script` in a loop.

- [ ] **Step 2: Run it once.** `cargo test -q -p vox-integration-tests --test golden_differential_gate -- --ignored --nocapture 2>&1 | tail -40`. Record the report. A green run on today's 11 EXPECT goldens (4 of which are literal `ok`) is evidence the corpus is inadequate, not that the tiers agree.

- [ ] **Step 3: Register it in both filters, nightly, not `--include-slow`.** In `pre_push.rs` the `-E` filter is a `concat!` — do **not** append this test there (that is the `--include-slow` set). Add a nightly job (or extend the existing nightly workflow) that runs:

```
cargo test -p vox-integration-tests --test golden_differential_gate -- --ignored
```

and add the same invocation to `.github/workflows/ci.yml` next to the hand-maintained
nextest filter (~1124–1131) as a **commented pointer plus a real nightly `runs-on`**
that already has a GitHub-hosted exception, or a self-hosted nightly label. The point:
the gate must run on a CI job. Verify the ignore string:
`cargo run -q -p vox-cli -- ci ignored-test-age --mode enforce`.

- [ ] **Step 4: Commit** — `git commit -m "test(gate): differential gate — one generated crate, nightly, both filters"`.

---

## Task 1b: Give the gate a corpus — eight goldens

**Files:** Create the eight `examples/golden/*.vox` files below.

79 goldens, 11 with EXPECT, 4 literal `ok`. Decisions in spec §5 are taken: insertion
order, crypto SSOT. Commit the goldens **with the gate expected red**; PR 2 turns them
green. There is no permanently-red golden.

Windows path literals in later tests use `vox_lit()` (Task 9). These goldens use `/`.

- [ ] **Step 1: `display_composites.vox`**

```vox
// ---
// title: "Display of composite values"
// description: "print/str of list, object, tuple, Option and Result produce the same text under both tiers."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print, str, list, object, tuple, Option, Result]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: [1, 2, 3]
// EXPECT: {a: 1, b: two}
// EXPECT: (1, two)
// EXPECT: Some(3)
// EXPECT: Ok(1)
pub fn main() {
  print(str([1, 2, 3]))
  print(str({a: 1, b: "two"}))
  print(str((1, "two")))
  print(str(Some(3)))
  print(str(Ok(1)))
}
```

- [ ] **Step 2: `float_formatting.vox`**

```vox
// ---
// title: "IEEE-exact float formatting"
// description: "abs/floor/ceil/round/sqrt print the same text on both tiers."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print, abs, floor, ceil, round, sqrt]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: 2
// EXPECT: 1
// EXPECT: 2
// EXPECT: 2
// EXPECT: 2
pub fn main() {
  print(str(abs(-2.0)))
  print(str(floor(1.9)))
  print(str(ceil(1.1)))
  print(str(round(1.5)))
  print(str(sqrt(4.0)))
}
```

- [ ] **Step 3: `object_field_order.vox`** — insertion order (spec §5).

```vox
// ---
// title: "Object field iteration order"
// description: "Literal object fields iterate in insertion order on both tiers."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print, object]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: zeta
// EXPECT: middle
// EXPECT: alpha
pub fn main() {
  let o = {zeta: 1, middle: 2, alpha: 3}
  for k in o.keys() { print(k) }
}
```

- [ ] **Step 4: `glob_and_listdir_order.vox`** — hermetic, asserts non-emptiness.
The gate `current_dir`s the repo root, but this golden still must not depend on
`examples/golden/*.vox` resolving from an integration-test CWD.

```vox
// ---
// title: "Directory enumeration is sorted"
// description: "fs.glob and fs.list_dir return sorted paths and propagate errors on both tiers."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print, fs.glob, fs.list_dir, sorted]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: 2
// EXPECT: true
pub fn main() {
  let xs = match fs.glob("examples/golden/glob_and_listdir_order.vox") {
    Ok(fs) => fs,
    Error(e) => []
  }
  print(str(len(xs)))
  print(str(xs is xs.sorted()))
}
```

- [ ] **Step 5: `int_overflow_boundary.vox`**

```vox
// ---
// title: "Integer overflow is a fault"
// description: "i64 overflow exits non-zero on both tiers after overflow-checks = true."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print]
// training_eligible: true
// difficulty: intermediate
// ---
// EXPECT: before
// EXPECT-EXIT: nonzero-both
pub fn main() {
  print("before")
  let x = 9223372036854775807
  print(str(x + 1))
}
```

- [ ] **Step 6: `division_by_zero.vox`**

```vox
// ---
// title: "Division by zero is a fault"
// description: "Integer division by zero exits non-zero on both tiers."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: before
// EXPECT-EXIT: nonzero-both
pub fn main() {
  print("before")
  print(str(1 / 0))
}
```

- [ ] **Step 7: `argv_shape.vox`** — asserts length and that argv[0] ends with the
script name. Paths differ across tiers until Task 6 threads `script_args`; this
golden turns green in PR 2 once `env.args` is `[source_path] ++ script_args` and
the native bin is invoked with no extra args (length 1).

```vox
// ---
// title: "argv is the script's own"
// description: "env.args()[0] is the script path on both tiers."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print, env.args]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: 1
pub fn main() {
  let a = env.args()
  print(str(len(a)))
}
```

- [ ] **Step 8: `crypto_hash_parity.vox`** — ships now; turns green in PR 2 with the
edge. Empty `KNOWN` list: if this is red after PR 2, that is a bug, not an asymmetry.

```vox
// ---
// title: "crypto.hash_fast is the same bytes on both tiers"
// description: "Both tiers call vox-crypto; hex of hash_fast(\"abc\") matches."
// syntax_version: "0.5.0"
// status: golden
// category: example
// constructs: [fn, print, crypto.hash_fast]
// training_eligible: true
// difficulty: beginner
// ---
// EXPECT: 32
pub fn main() {
  let h = crypto.hash_fast("abc")
  print(str(len(h)))
}
```

- [ ] **Step 9: Commit** — `git commit -m "test(golden): eight goldens that give the differential gate a corpus"`.

PR 1 merge criterion: the gate exists, is registered on a CI/nightly job, and the
corpus is committed. It is allowed to be red until PR 2.

---

# PR 2 — Parity, list.push, crypto SSOT

**Stop-the-line:** after adding `vox-crypto` to `crates/vox-compiler/Cargo.toml`,
write the `crate-edges` exceptions proposal in the PR description and **stop**. Do
not write the exceptions ledger. Do not regenerate `crate-edges.allow.v1.json`.
Wait for the maintainer.

## Task 2: Close the measured drift

**Files:** `crates/vox-cli/src/commands/runtime/run/backend/native.rs` (profile +
generated `Cargo.toml` `serde_json` features); `crates/vox-compiler/src/eval/{builtins,env,value}.rs`;
`crates/vox-crypto/src/` (hex helpers); `crates/vox-actor-runtime/src/builtins/mod.rs`
(route hash/id through `vox-crypto`; stop hashing directly);
`crates/vox-compiler/src/builtin_registry.rs`;
`crates/vox-codegen/src/codegen_rust/{pipeline.rs,emit/*}`;
`crates/vox-compiler/tests/eval_typeck_parity_test.rs`;
`scripts/install-hooks.vox`, `scripts/setup.vox` (the unwraps Task 0 found).

- [ ] **Step 1: Write the failing tests** — append to `eval_typeck_parity_test.rs`.
Do **not** add `every_known_asymmetry_has_a_reason` (that asserted a string literal
in the same file). Do **not** glob `examples/golden/*.vox` from the package CWD.

```rust
#[test]
fn registry_emits_every_eval_only_method() {
    use vox_compiler::builtin_registry::std_namespace_runtime_call;
    for (ns, m, args) in [
        ("time", "now", vec![]),
        ("json", "encode", vec!["x".to_string()]),
        ("json", "stringify", vec!["x".to_string()]),
        ("process", "cwd", vec![]),
        ("secrets", "resolve", vec!["\"K\"".to_string()]),
        ("crypto", "hash_fast", vec!["\"abc\"".to_string()]),
    ] {
        assert!(std_namespace_runtime_call(ns, m, &args).is_some(), "codegen has no emit for {ns}.{m}");
    }
}

#[test]
fn display_of_composites_matches_the_surface_form() {
    let v = run_probe(r#"pub fn main() { return str([1, 2]) + "|" + str({a: 1}) + "|" + str(Some(3)) + "|" + str(Ok(1)) }"#).unwrap();
    assert!(matches!(v, VoxValue::Str(ref s) if s == "[1, 2]|{a: 1}|Some(3)|Ok(1)"), "{v:?}");
}

#[test]
fn glob_is_sorted_and_propagates_errors() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("b.txt"), "b").unwrap();
    std::fs::write(d.path().join("a.txt"), "a").unwrap();
    let pat = format!("{}/*", d.path().display());
    let src = format!(
        r#"pub fn main() {{ return match fs.glob("{pat}") {{ Ok(xs) => xs is xs.sorted() && len(xs) is 2, Error(e) => false }} }}"#
    );
    let v = run_probe(&src).unwrap();
    assert!(matches!(v, VoxValue::Bool(true)), "{v:?}");
}

#[test]
fn list_push_is_amortised_constant_not_quadratic() {
    // 5_000 pushes must finish well inside the 10 M step budget and well under a second.
    let t0 = std::time::Instant::now();
    let v = run_probe(
        r#"pub fn main() { let mut xs = []; let mut i = 0; while i < 5000 { xs = xs.push(i); i = i + 1 }; return len(xs) }"#,
    )
    .unwrap();
    assert!(matches!(v, VoxValue::Int(5000)), "{v:?}");
    assert!(t0.elapsed() < std::time::Duration::from_millis(500), "list.push is still cloning the receiver: {:?}", t0.elapsed());
}

#[test]
fn hash_fast_matches_vox_crypto() {
    let v = run_probe(r#"pub fn main() { return crypto.hash_fast("abc") }"#).unwrap();
    let expected = vox_crypto::hash_fast_hex(b"abc");
    assert!(matches!(v, VoxValue::Str(ref s) if s == &expected), "{v:?} != {expected}");
}
```

- [ ] **Step 2: Run to verify they fail** — `cargo test -q -p vox-compiler --test eval_typeck_parity_test 2>&1 | tail -30`. Expected: `registry_emits` fails on `time.now` and `crypto.hash_fast`; `display_of_composites` fails on `Some(3)`; `list_push_is_amortised…` either times out or exceeds 500 ms; `hash_fast_matches…` fails to resolve `crypto`.

- [ ] **Step 3: Native profile + generated manifest.** `native.rs` script-dev and
`VOX_SCRIPT_RELEASE`: `overflow-checks = true`. In the generated crate's
`Cargo.toml`, set `serde_json = { version = "1", features = ["preserve_order"] }`.
Do not touch workspace features.

- [ ] **Step 4: `vox-crypto` hex helpers** (new `pub fn`s, same-file tests first):

```rust
/// Hex of the fast hash over `bytes`. SSOT for interp `crypto.hash_fast` and
/// native `vox_hash_fast`.
pub fn hash_fast_hex(bytes: &[u8]) -> String {
    hex_encode(&crate::fast_hash(bytes))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
```

Route `vox-actor-runtime`'s `vox_hash_fast` through `vox_crypto::hash_fast_hex`.
Add `vox-crypto` to `vox-compiler` (the authorized edge). Interp `crypto.hash_fast`
calls the same helper. Propose the exceptions text and stop if `crate-edges` fails.

- [ ] **Step 5: One display for both tiers.** In `eval/builtins.rs::vox_value_display`
add arms: `Option(Some(v)) → "Some(" + display(v) + ")"`, `Option(None) → "None"`,
`Result(Ok(v)) → "Ok(…)"`, `Result(Err(e)) → "Err(…)"`,
`Tagged { name, fields } → name + "(" + fields joined ", " + ")"`; replace the
`_ => format!("{v:?}")` catch-all with an explicit list. In
`vox-actor-runtime/src/builtins/mod.rs` add `pub fn vox_display(v: &serde_json::Value) -> String`
reproducing that spacing, with a unit test per shape; route codegen `("str", 1)` and
`("print", n)` through it. `print` accepts n args joined by a space.

- [ ] **Step 6: Registry arms** in `std_namespace_runtime_call`:

```rust
        ("time", "now") => Some("vox_actor_runtime::builtins::vox_now_ms()".to_string()),
        ("json", "encode" | "stringify") if args.len() == 1 => Some(format!(
            "vox_actor_runtime::builtins::vox_json_render(&({})).unwrap_or_default()", args[0]
        )),
        ("process", "cwd") => Some("vox_actor_runtime::builtins::vox_process_cwd()".to_string()),
        ("secrets", "resolve") if args.len() == 1 => Some(format!(
            "vox_actor_runtime::builtins::vox_secrets_resolve(({}).as_str())", args[0]
        )),
        ("crypto", "hash_fast") if args.len() == 1 => Some(format!(
            "vox_crypto::hash_fast_hex(({}).as_bytes())", args[0]
        )),
```

```rust
pub fn vox_process_cwd() -> String {
    std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()
}

pub fn vox_secrets_resolve(key: &str) -> Option<String> {
    let id: vox_secrets::SecretId = std::str::FromStr::from_str(key).ok()?;
    let resolved = vox_secrets::resolve_secret_with_context(id, "script");
    resolved.value.map(|v| v.expose_secret().to_string())
}
```

(Check `vox-actor-runtime` already depends on `vox-secrets`. If not, defer this arm
with a reason — do not take a second new edge in this PR.)

- [ ] **Step 7: Sorted, propagating enumeration.** `eval/builtins.rs` `glob`: collect,
`sort()`, turn `.ok()` swallow into `Result::Err` like `vox_fs_glob` (`mod.rs:1724`,
not `vox_list_dir` at `:331`). `list_dir` / `list_dir_detailed` in both files:
`sort()`. Add a `.sorted()` arm to `emit/method_emit.rs` beside `sorted_by_key`.

- [ ] **Step 8: `list.push` in-place + `Rc<str>`.** `VoxValue::Str` becomes `Rc<str>`.
`list.push` uses `Scope::get_mut` + `Rc::make_mut` (see `eval/env.rs:55` comment).
Do **not** `v.to_vec()`. Same for `pop` / in-place list mutators that today clone.

- [ ] **Step 9: `env.args` field.** Add `pub script_args: Vec<String>` and
`pub source_path: Option<PathBuf>` to `Interpreter`. `env.args` returns
`[source_path] ++ script_args`. Task 6 sets them; until then tests that construct
`Interpreter` set the fields. Native argv shape is `[bin-or-script] ++ args`
(`backend/native.rs`).

- [ ] **Step 10: Faults exit 1 natively.** `pipeline.rs` wrap generated `main` in
`std::panic::catch_unwind`, print the payload to stderr, `std::process::exit(1)`.
Add typecheck error `"async main cannot return a value"` so both tiers refuse it.

- [ ] **Step 11: Fix `install-hooks.vox` and `setup.vox`.** Reproduce the
`Option.unwrap()` with `vox run --mode interp scripts/install-hooks.vox -- --help`
and the same for `setup.vox`. Fix the Vox (or the builtin that returned `None`)
until both exit 0. Do not paper over with `--mode script`.

- [ ] **Step 12: Run everything** — `cargo test -q -p vox-compiler --test eval_typeck_parity_test && cargo test -q -p vox-crypto && cargo test -q -p vox-actor-runtime builtins && cargo build -q -p vox-cli --bin vox && cargo test -q -p vox-integration-tests --test golden_differential_gate -- --ignored 2>&1 | tail -30`. Expected: all eight new goldens green. Anything red is new drift — fix it here or add `// EXPECT-TIER-ASYMMETRY:` with a reason that will fail when it expires.

- [ ] **Step 13: Commit** — `cargo fmt -p vox-compiler -p vox-crypto -p vox-actor-runtime -p vox-codegen -p vox-cli`. `git commit -m "fix(parity): close measured drift; in-place list.push; vox-crypto SSOT for both tiers"`.

---

# PR 3 — Caps and bounds (additive)

## Task 3: `CapabilitySet` — non-optional, canonical roots, repeatable tokens

**Files:** Create `crates/vox-compiler/src/eval/caps.rs`; modify `eval/mod.rs`,
`eval/builtins.rs`, `eval/expr.rs`; `vox-cli` `run` / `repl` / `play`;
`vox-langtool` `run`; `vox-terminal-core/src/vox_interp.rs`;
`vox-orchestrator-mcp/src/workspace_mcp/dispatch.rs`;
`docs/src/architecture/where-things-live.md` (four rows).

`from_roots` does **not** refuse `,` / `=` / `|`. `--caps` is repeatable; the mesh
passes one token per argv slot.

- [ ] **Step 1: Write the failing tests** (create `caps.rs` with the test module first).
Include every test from revision 2 Task 3 **except** `from_roots_refuses_separator_characters`.
Replace that with:

```rust
    #[test]
    fn from_roots_accepts_a_comma_in_the_directory() {
        let d = tempfile::Builder::new().prefix("a,b-").tempdir().unwrap();
        let c = CapabilitySet::from_roots(vec![], vec![d.path().to_path_buf()], &["time:real"]).unwrap();
        let canon = std::fs::canonicalize(d.path()).unwrap();
        assert!(c.allows_path(&canon.join("x"), true));
        let tokens = c.to_tokens();
        assert!(tokens.iter().any(|t| t.starts_with("fs:rw=")), "{tokens:?}");
        assert!(tokens.iter().any(|t| t == "time:real"), "{tokens:?}");
    }

    #[test]
    fn to_tokens_round_trips_through_parse_of_each_token() {
        let d = tempfile::tempdir().unwrap();
        let c = CapabilitySet::from_roots(vec![], vec![d.path().to_path_buf()], &["time:real", "env:ro"]).unwrap();
        let mut again = CapabilitySet::parse("").unwrap();
        for t in c.to_tokens() {
            let piece = CapabilitySet::parse(&t).unwrap();
            again.merge(piece);
        }
        assert_eq!(c.frozen_time_ms(), again.frozen_time_ms());
    }
```

Keep `unmentioned_namespaces_are_denied`, `pure_namespaces_are_always_allowed`
(`path json csv toml yaml regex log db repo`), `roots_are_canonicalised_at_parse_time`,
`fs_roots_scope_reads_and_writes_separately_and_repeat_the_token`,
`a_missing_root_is_a_receiver_error_not_a_silent_deny`,
`pipe_separator_is_gone_and_io_token_is_rejected` (the *comment* form still rejects
`|` inside one token), `env_has_read_and_write_levels`, `frozen_time_and_seeded_random`,
`deterministic_shorthand_expands`, `net_and_http_are_one_namespace`,
`legacy_directive_maps_words_and_is_unscoped`, `developer_default_allows_everything`,
`bad_specs_name_the_offending_token`, and the Windows drive-letter test. On Windows,
`component_eq` must cover `Component::Prefix`.

- [ ] **Step 2: Run to verify they fail** — `cargo test -q -p vox-compiler --lib eval::caps 2>&1 | tail -5` → compile error.

- [ ] **Step 3: Implement.** Same structure as revision 2 Task 3 (`PURE`, `GATED`,
`parse`, `developer_default`, `allows_namespace`, `allows_path`, `is_under`,
`component_eq`) with these changes:

```rust
    /// Tokens suitable for `vox run --caps <token> --caps <token>`.
    pub fn to_tokens(&self) -> Vec<String> {
        let mut parts = Vec::new();
        for d in self.fs_ro.iter().flatten() { parts.push(format!("fs:ro={}", d.display())); }
        for d in self.fs_rw.iter().flatten() { parts.push(format!("fs:rw={}", d.display())); }
        if self.allowed.contains("http") { parts.push("net:allow".into()); }
        if self.allowed.contains("process") { parts.push("process:allow".into()); }
        if self.allowed.contains("env") { parts.push(if self.env_write { "env:rw".into() } else { "env:ro".into() }); }
        if self.allowed.contains("secrets") { parts.push("secrets:allow".into()); }
        if self.allowed.contains("agentos") { parts.push("agentos:allow".into()); }
        match self.frozen_time_ms { Some(ms) => parts.push(format!("time:frozen={ms}")), None if self.allowed.contains("time") => parts.push("time:real".into()), None => {} }
        if let Some(s) = self.random_seed { parts.push(format!("random:seed={s}")); }
        parts
    }

    pub fn from_roots(ro: Vec<PathBuf>, rw: Vec<PathBuf>, extra: &[&str]) -> Result<Self, CapsParseError> {
        let mut out = Self::restrictive();
        for d in ro {
            let root = std::fs::canonicalize(&d).map_err(|_| CapsParseError { token: d.display().to_string(), why: "fs root does not exist or is unreadable" })?;
            out.fs_ro.get_or_insert_with(Vec::new).push(root);
            out.allowed.insert("fs".into());
            out.allowed.insert("io".into());
        }
        for d in rw {
            let root = std::fs::canonicalize(&d).map_err(|_| CapsParseError { token: d.display().to_string(), why: "fs root does not exist or is unreadable" })?;
            out.fs_rw.get_or_insert_with(Vec::new).push(root);
            out.allowed.insert("fs".into());
            out.allowed.insert("io".into());
        }
        for tok in extra {
            let piece = Self::parse(tok)?;
            out.merge(piece);
        }
        Ok(out)
    }

    pub fn merge(&mut self, other: Self) {
        self.allowed.extend(other.allowed);
        self.env_write |= other.env_write;
        match (&mut self.fs_ro, other.fs_ro) {
            (Some(a), Some(b)) => a.extend(b),
            (None, Some(b)) => self.fs_ro = Some(b),
            _ => {}
        }
        match (&mut self.fs_rw, other.fs_rw) {
            (Some(a), Some(b)) => a.extend(b),
            (None, Some(b)) => self.fs_rw = Some(b),
            _ => {}
        }
        if other.frozen_time_ms.is_some() { self.frozen_time_ms = other.frozen_time_ms; }
        if other.random_seed.is_some() { self.random_seed = other.random_seed; }
    }
```

`parse` still splits on `,` for the *comment* / single-string form. `from_roots` never
goes through that splitter for directory names.

- [ ] **Step 4: Make `caps` non-optional and set the six embedders explicitly.**

```rust
// eval/mod.rs Interpreter::new
caps: caps::CapabilitySet::developer_default(),
```

```rust
// vox-cli run.rs — after first-line parse
interpreter.caps = if let Some(tokens) = cli_caps_tokens {
    CapabilitySet::from_tokens(&tokens)?
} else if has_caps_directive {
    CapabilitySet::from_legacy_directive(&legacy_words)
} else {
    CapabilitySet::developer_default()
};
```

Same for `vox-langtool` `run`. `vox-cli` `repl` and `play`:
`interpreter.caps = CapabilitySet::developer_default();` (explicit, not implicit).

```rust
// vox-terminal-core/src/vox_interp.rs
let mut interp = Interpreter::new(100_000);
interp.caps = CapabilitySet::parse("").unwrap(); // restrictive: PURE only
```

```rust
// vox-orchestrator-mcp workspace dispatch (both call sites)
let mut interp = Interpreter::new(100_000);
interp.caps = CapabilitySet::from_roots(
    vec![workspace_root.to_path_buf()],
    vec![],
    &["env:ro", "time:real"],
).expect("workspace root exists");
```

Grep for `Interpreter::new` under `crates/` excluding `tests/` and `#[cfg(test)]`.
Every production site must assign `caps` on the next line. A test:

```rust
#[test]
fn production_embedders_assign_caps_explicitly() {
    let patterns = [
        ("crates/vox-cli/src/commands/run.rs", "developer_default"),
        ("crates/vox-cli/src/commands/repl.rs", "developer_default"),
        ("crates/vox-cli/src/commands/play.rs", "developer_default"),
        ("crates/vox-langtool/src/commands/run.rs", "developer_default"),
        ("crates/vox-terminal-core/src/vox_interp.rs", "CapabilitySet::parse"),
        ("crates/vox-orchestrator-mcp/src/workspace_mcp/dispatch.rs", "from_roots"),
    ];
    for (path, needle) in patterns {
        let src = std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(src.contains("Interpreter::new") && src.contains(needle), "{path} missing explicit caps ({needle})");
    }
}
```

Place this test in `caps.rs` so the paths resolve via `CARGO_MANIFEST_DIR` of
`vox-compiler` (`crates/vox-compiler` → `../..` is repo root). Adjust the join to
`PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)`.

- [ ] **Step 5: Four `where-things-live.md` rows** in the same PR: `CapabilitySet` →
`crates/vox-compiler/src/eval/caps.rs`; counting allocator →
`crates/vox-cli/src/mem_limit.rs`; `InterpExecutor` →
`crates/vox-mesh-transport/src/interp_executor.rs` (row may say "lands in PR 5");
isolation reference → `docs/src/reference/isolation.md`.

- [ ] **Step 6: Run** — `cargo test -q -p vox-compiler --lib eval::caps && cargo build -q -p vox-cli -p vox-langtool -p vox-terminal-core -p vox-orchestrator-mcp && cargo test -q -p vox-langtool --test integration run_caps_directive 2>&1 | tail -5`.

- [ ] **Step 7: Commit** — `git commit -m "feat(eval): CapabilitySet — non-optional, canonical roots, explicit embedder sets"`.

---

## Task 4: Denial is fatal and every side-effecting entry point is gated

**Files:** `eval/value.rs`, `eval/mod.rs`, `eval/builtins.rs`, `eval/expr.rs`,
`eval/env.rs`; create `crates/vox-compiler/tests/caps_enforcement_test.rs`.

- [ ] **Step 1: Write the failing tests** — take revision 2 Task 4's tests
(`denied_fs_read_is_fatal_and_nothing_after_it_runs`, `http_is_gated_under_std`,
`repo_is_pure_and_allowed_without_caps`, `path_resolve_is_an_fs_read`,
`env_set_needs_rw`, `register_exit_command_is_gated_at_queue_time`,
`import_outside_the_fs_roots_is_denied_before_main_runs`,
`a_default_interpreter_is_developer_default_not_ungated`,
`every_seeded_namespace_is_classified`) **and add**:

```rust
#[test]
fn versioned_snapshot_is_gated_when_repo_write_is_denied_by_policy() {
    // The decorator path calls interp.repo.snapshot directly (eval/expr.rs).
    // Under a restrictive set the auto-snapshot is a no-op / denial, not an
    // ungated write. developer_default() still snapshots (in-memory).
    let ok = run_with(
        CapabilitySet::developer_default(),
        "@versioned fn save() { let x = 1 }\npub fn main() { save(); return len(repo.changes()) }",
    );
    assert!(matches!(ok, Ok(VoxValue::Int(n)) if n >= 1), "developer_default snapshots: {ok:?}");

    let denied = run_with(
        CapabilitySet::parse("").unwrap(),
        "@versioned fn save() { let x = 1 }\npub fn main() { save(); return 0 }",
    );
    assert!(
        matches!(denied, Err(EvalError::CapabilityDenied { ref ns, ref method }) if ns == "repo" && method == "snapshot")
            || matches!(denied, Ok(VoxValue::Int(0))),
        "restrictive embedder must not snapshot via the decorator bypass: {denied:?}"
    );
}
```

Prefer the `CapabilityDenied` arm. If you choose the no-op arm, record that as the
explicit exception in the test name and in `isolation.md`. Spec §3.2 item 4 allows
either; the gate must exist.

- [ ] **Step 2: Run to verify they fail** — `cargo test -q -p vox-compiler --test caps_enforcement_test 2>&1 | tail -20`.

- [ ] **Step 3: Implement.** `_Denied(String)` on `VoxValue`; `EvalError::CapabilityDenied`;
gate at `builtins.rs` returns `_Denied(format!("{ns_str}.{method}"))` with no `println!`;
`expr.rs` converts `_Denied` to `Err`; `path.resolve` via `fs_resolve_allowed` (Task 5
— stub as `None` until Task 5 if needed, and keep this test failing until then);
`register_exit_command` gated at queue time; exit-command `OnceLock` moves onto
`Interpreter.exit_commands`; signal handler installed only by `run_interp` (Task 6).
Import gate after canonicalize in `resolve_local_file_import`:

```rust
    if !self.caps.allows_path(&canonical, false) {
        return Err(EvalError::CapabilityDenied { ns: "fs".into(), method: "import".into() });
    }
```

`@versioned` in `eval/expr.rs` (~463):

```rust
                    if is_versioned {
                        if !interp.caps.allows_namespace("repo") {
                            return Err(EvalError::CapabilityDenied {
                                ns: "repo".into(),
                                method: "snapshot".into(),
                            });
                        }
                        interp.repo.snapshot(Some(&format!("@versioned {fn_name}")));
                    }
```

Wait: `repo` is PURE so `allows_namespace("repo")` is always true. The decorator
bypass needs its **own** flag, not the PURE list:

```rust
                        if !interp.caps.allows_versioned_snapshot() {
                            return Err(EvalError::CapabilityDenied {
                                ns: "repo".into(),
                                method: "snapshot".into(),
                            });
                        }
```

`allows_versioned_snapshot()` is `true` for `developer_default()` and for any set
that granted `fs` or `process` (a local / native run), and `false` for the
restrictive embedder sets (`parse("")`, MCP). Pin that in the test above.

- [ ] **Step 4: Run** — `cargo test -q -p vox-compiler --test caps_enforcement_test && cargo test -q -p vox-compiler 2>&1 | tail -5`.

- [ ] **Step 5: Mutation-verify (minimal).** (a) In the `fs` namespace gate only,
change `!caps.allows_namespace(ns_str)` to `ns_str == "http" && !caps.allows_namespace(ns_str)`
→ `denied_fs_read_is_fatal…` MUST FAIL; restore; `grep -c 'allows_namespace(ns_str)'`.
(b) Comment out the `allows_path` check in `resolve_local_file_import` →
`import_outside…` MUST FAIL; restore. (c) Comment out the `allows_versioned_snapshot`
check → `versioned_snapshot…` MUST FAIL; restore. Record all three.

- [ ] **Step 6: Commit** — `git commit -m "feat(eval): capability denial is fatal; import, exit commands, and @versioned are gated"`.

---

## Task 5: Filesystem scoping, frozen time, seeded random, depth bound

**Files:** `eval/builtins.rs` (fs arm), `eval/mod.rs`, `eval/expr.rs` (`apply_closure`),
the parser descent; append to `caps_enforcement_test.rs`.

- [ ] **Step 1: Write the failing tests** — keep revision 2's `symlink_escape…`,
`degenerate_paths…`, `frozen_time…`, `deep_recursion…`, `deeply_nested_source…`.
Replace `every_fs_method_that_takes_a_path_is_scoped` with a version that has a
**positive control** in the same loop, and add the relative-write + glob-escape tests:

```rust
#[test]
fn every_fs_method_that_takes_a_path_is_scoped() {
    let d = tempfile::tempdir().unwrap();
    let inside = d.path().join("in"); std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(inside.join("ok.txt"), "OK").unwrap();
    let outside = d.path().join("out"); std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("s.txt"), "S").unwrap();
    let caps = CapabilitySet::parse(&format!("fs:rw={}", inside.display())).unwrap();
    let allowed = run_with(caps.clone(), &format!(r#"pub fn main() {{ return fs.read("{}") }}"#, inside.join("ok.txt").display()));
    assert!(matches!(allowed, Ok(VoxValue::Str(ref s)) if s == "OK"), "positive control died: {allowed:?}");
    let f = outside.join("s.txt");
    for (m, arg) in [
        ("read", &f), ("read_file", &f), ("read_to_string", &f), ("read_bytes", &f), ("canonicalize", &f),
        ("exists", &f), ("is_file", &f), ("is_dir", &outside), ("stat", &f),
        ("list_dir", &outside), ("list_dir_detailed", &outside), ("walk", &outside), ("list_recursive", &outside),
        ("remove", &f), ("remove_dir_all", &outside), ("mkdir", &outside.join("new")),
    ] {
        let r = run_with(caps.clone(), &format!(r#"pub fn main() {{ return fs.{m}("{}") }}"#, arg.display()));
        assert!(denied(&r, "fs"), "fs.{m} ungated: {r:?}");
    }
}

#[test]
fn developer_default_relative_write_creates_the_file() {
    let cwd = tempfile::tempdir().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(cwd.path()).unwrap();
    let r = run_with(CapabilitySet::developer_default(), r#"pub fn main() { fs.write("out.txt", "hi"); return fs.read("out.txt") }"#);
    std::env::set_current_dir(prev).unwrap();
    assert!(matches!(r, Ok(VoxValue::Str(ref s)) if s == "hi"), "relative write denied: {r:?}");
    assert!(cwd.path().join("out.txt").exists());
}

#[test]
fn glob_filters_results_not_only_the_prefix() {
    let d = tempfile::tempdir().unwrap();
    let job = d.path().join("job"); std::fs::create_dir_all(&job).unwrap();
    std::fs::write(job.join("a.txt"), "a").unwrap();
    std::fs::write(d.path().join("secret.txt"), "S").unwrap();
    let caps = CapabilitySet::parse(&format!("fs:rw={}", job.display())).unwrap();
    let pat = format!("{}/*/../secret.txt", d.path().display());
    let r = run_with(caps, &format!(r#"pub fn main() {{ return match fs.glob("{pat}") {{ Ok(xs) => len(xs), Error(e) => -1 }} }}"#));
    assert!(matches!(r, Ok(VoxValue::Int(0))) || denied(&r, "fs"), "glob escaped via ..: {r:?}");
}

#[test]
fn unscoped_grant_does_not_canonicalize() {
    // developer_default has no boundary; a missing relative parent must still write.
    let r = run_with(CapabilitySet::developer_default(), r#"pub fn main() { fs.mkdir("a/b/c"); return fs.exists("a/b/c") }"#);
    assert!(matches!(r, Ok(VoxValue::Bool(true))), "{r:?}");
}
```

- [ ] **Step 2: Run to verify they fail.**

- [ ] **Step 3: The resolver**

```rust
fn fs_unscoped(caps: &crate::eval::caps::CapabilitySet) -> bool {
    caps.allows_namespace("fs") && caps.allows_path(std::path::Path::new("/"), true)
        && caps.allows_path(std::path::Path::new("/"), false)
}

fn nearest_existing_ancestor(p: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut cur = if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().ok()?.join(p) };
    loop {
        if cur.as_os_str().is_empty() { return None; }
        if cur.exists() { return std::fs::canonicalize(&cur).ok(); }
        if !cur.pop() { return None; }
    }
}

fn fs_resolve_allowed(caps: &crate::eval::caps::CapabilitySet, raw: &str, write: bool) -> Option<std::path::PathBuf> {
    if raw.is_empty() { return None; }
    let p = std::path::Path::new(raw);
    let name = p.file_name()?;
    if name == "." || name == ".." { return None; }
    if fs_unscoped(caps) {
        return Some(if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().ok()?.join(p) });
    }
    let canon = match std::fs::canonicalize(p) {
        Ok(cp) => cp,
        Err(_) => {
            let parent = p.parent().filter(|par| !par.as_os_str().is_empty())?;
            let anc = nearest_existing_ancestor(parent)?;
            let suffix = p.strip_prefix(parent).ok()?;
            anc.join(suffix)
        }
    };
    caps.allows_path(&canon, write).then_some(canon)
}

fn glob_dir_prefix(pat: &str) -> Option<&str> {
    let cut = pat.find(['*', '?', '[']).unwrap_or(pat.len());
    let prefix = &pat[..cut];
    let dir = prefix.rsplit_once(['/', '\\']).map(|(d, _)| d).unwrap_or(".");
    if dir.split(['/', '\\']).any(|c| c == "..") { return None; }
    Some(dir)
}
```

At the head of the `Some("fs")` arm, keep revision 2's `args` shadowing for
`cwd` / `copy` / path methods. Replace the `glob` arm:

```rust
                "glob" => {
                    let Some(VoxValue::Str(pat)) = args.first() else { return None };
                    let Some(dir) = glob_dir_prefix(pat) else {
                        return Some(VoxValue::_Denied("fs.glob".into()));
                    };
                    if fs_resolve_allowed(caps, dir, false).is_none() {
                        return Some(VoxValue::_Denied("fs.glob".into()));
                    }
                    args
                }
```

After the glob crate returns, filter:

```rust
                    let mut out: Vec<String> = matches.into_iter()
                        .filter_map(|p| fs_resolve_allowed(caps, &p, false).map(|c| c.to_string_lossy().into_owned()))
                        .collect();
                    out.sort();
```

`FS_PATH_METHODS` table as in revision 2 (the verified 19). Unknown methods denied
by default. `io.open` / `io.save` same shape.

- [ ] **Step 4: Frozen time and seeded random.** `time` arm: return
`caps.frozen_time_ms()` before `SystemTime`. `Interpreter.rng: Option<StdRng>` seeded
from `caps.random_seed()`.

- [ ] **Step 5: Depth bound in `apply_closure`, not `eval_expr`.**
`EvalError::RecursionLimitExceeded`. `MAX_EVAL_DEPTH: usize = 1024`. Increment /
decrement only around closure application. Parser nesting limit 4 096 stays in
descent. `--max-depth` is wired in Task 6.

- [ ] **Step 6: Run, then mutate the symlink check** — change `std::fs::canonicalize(p)`
in the scoped branch to `Ok(p.to_path_buf())` → `symlink_escape…` MUST FAIL; restore.

- [ ] **Step 7: Commit** — `git commit -m "feat(eval): parent-walk fs resolver; glob filters results; depth bound in apply_closure"`.

---

## Task 5b: Re-run Task 0's table

- [ ] **Step 1:** `cargo run -q -p vox-cli -- run --mode interp scripts/bench-script-tiers.vox > /tmp/tiers-after-5.md`. `fmt` / `install-hooks` / `setup` / `arch-check` must show `ok` in the run column (arch-check's own exit 1 is allowed if stderr is its usual report, not `CapabilityDenied`). Relative writes in those scripts must not be denials.
- [ ] **Step 2:** Paste the new table under a heading in `script-tier-timings-2026-09.md`. Commit `chore(scripts): re-measure interp execution after fs scoping`.

---

## Task 6: `vox run` flags, exit codes, memory ceiling

**Files:** Create `crates/vox-cli/src/mem_limit.rs`; modify `lib.rs`, `Cargo.toml`
(`windows-sys` features **must include `Win32_System_IO`**), `cli_args.rs`,
`commands/run.rs`, `cli_dispatch/lanes.rs`, `compilerd.rs` (not under `commands/`);
create `crates/vox-cli/tests/run_interp_limits.rs`; create
`docs/src/reference/isolation.md` (`category: Language Reference`).

- [ ] **Step 1: Write the failing integration test**

```rust
use std::process::Command;
fn vox() -> String { env!("CARGO_BIN_EXE_vox").to_string() }
fn write(name: &str, src: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("vox-limits-{name}-{}.vox", std::process::id()));
    std::fs::write(&p, src).unwrap();
    p
}

#[test]
fn capability_denial_exits_77_with_a_marker_on_stderr_and_nothing_on_stdout() {
    let f = write("caps", r#"pub fn main() { let s = fs.read("/etc/hosts"); print("LEAK") }"#);
    let out = Command::new(vox()).args(["run", "--mode", "interp", "--caps", "env:ro"]).arg(&f).output().unwrap();
    assert_eq!(out.status.code(), Some(77), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("LEAK"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("vox: capability denied: fs.read"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--caps"), "error must name the flag that grants it");
}

#[test]
fn caps_is_repeatable() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.txt"), "A").unwrap();
    let f = write("rep", &format!(r#"pub fn main() {{ print(fs.read("{}/a.txt")) }}"#, d.path().display()));
    let out = Command::new(vox())
        .args(["run", "--mode", "interp", "--caps", &format!("fs:ro={}", d.path().display()), "--caps", "time:real"])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains('A'));
}

#[test]
fn caps_flag_overrides_script_directive() {
    let f = write("override", "// vox:caps fs\npub fn main() { let s = fs.read(\"/etc/hosts\"); print(\"LEAK\") }");
    let out = Command::new(vox()).args(["run", "--mode", "interp", "--caps", "env:ro"]).arg(&f).output().unwrap();
    assert_eq!(out.status.code(), Some(77));
}

#[test]
fn step_and_depth_limits_exit_78() {
    let f = write("steps", "pub fn main() { let mut i = 0; while true { i = i + 1 } }");
    let out = Command::new(vox()).args(["run", "--mode", "interp", "--max-steps", "10000"]).arg(&f).output().unwrap();
    assert_eq!(out.status.code(), Some(78), "{}", String::from_utf8_lossy(&out.stderr));
    let g = write("depth", "fn f(n: int) to int { return f(n + 1) } pub fn main() { return f(0) }");
    let out = Command::new(vox()).args(["run", "--mode", "interp", "--max-depth", "32"]).arg(&g).output().unwrap();
    assert_eq!(out.status.code(), Some(78), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn memory_limit_exits_79() {
    // Doubling crosses 64 MiB in 26 iterations. Do not use list.push.
    let f = write("mem", r#"pub fn main() { let mut s = "x"; let mut i = 0; while i < 40 { s = s + s; i = i + 1 } }"#);
    let out = Command::new(vox()).args(["run", "--mode", "interp", "--max-memory", "67108864", "--max-steps", "10000"]).arg(&f).output().unwrap();
    assert_eq!(out.status.code(), Some(79), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("memory limit exceeded"));
}

#[test]
fn argv_is_the_scripts_own() {
    let f = write("argv", r#"pub fn main() { let a = env.args(); print(str(len(a))); print(a[1]) }"#);
    let out = Command::new(vox()).args(["run", "--mode", "interp"]).arg(&f).args(["--", "hello"]).output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("hello"), "{s:?} stderr={}", String::from_utf8_lossy(&out.stderr));
}
```

- [ ] **Step 2: Run to verify they fail.**

- [ ] **Step 3: The allocator** in `mem_limit.rs`, declared from `lib.rs`:

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(C, align(64))]
struct Line(AtomicUsize);

static USED: Line = Line(AtomicUsize::new(0));
static LIMIT: Line = Line(AtomicUsize::new(usize::MAX));

pub fn arm(bytes: usize) { LIMIT.0.store(bytes, Ordering::Relaxed); }

struct Capped;
#[global_allocator]
static ALLOC: Capped = Capped;

unsafe impl GlobalAlloc for Capped {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() { charge(new_size - layout.size()); }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        USED.0.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn charge(n: usize) {
    if LIMIT.0.load(Ordering::Relaxed) == usize::MAX { return; }
    let new = USED.0.fetch_add(n, Ordering::Relaxed) + n;
    if new > LIMIT.0.load(Ordering::Relaxed) {
        let _ = write_limit_line();
        die(79);
    }
}

fn die(code: i32) -> ! {
    #[cfg(unix)]
    unsafe { libc::_exit(code); }
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Threading::TerminateProcess(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            code as u32,
        );
        loop { std::hint::spin_loop(); }
    }
    #[cfg(not(any(unix, windows)))]
    std::process::abort();
}
```

Same-file tests for `charge` skipping when `LIMIT == usize::MAX` and for `die` not
being `std::process::exit`. `write_limit_line` writes a stack-formatted
`vox: memory limit exceeded\n` to stderr without allocating.

- [ ] **Step 4: Flags.** `RunArgs`: `caps: Vec<String>` with `ArgAction::Append`,
`max_steps: Option<usize>`, `max_memory: Option<usize>`, `max_depth: Option<usize>`
(default 1024). `run_interp` sets `script_args`, `source_path`, `caps` from tokens /
legacy / developer_default, arms the allocator, installs the signal handler here
only. Exit map:

```rust
        Err(vox_compiler::eval::EvalError::CapabilityDenied { ns, method }) => {
            eprintln!("vox: capability denied: {ns}.{method} — grant with --caps <token> (see isolation.md)");
            std::process::exit(77);
        }
        Err(vox_compiler::eval::EvalError::StepLimitExceeded) | Err(vox_compiler::eval::EvalError::RecursionLimitExceeded) => {
            eprintln!("vox: execution budget exceeded; pass --max-steps / --max-depth or use --mode script");
            std::process::exit(78);
        }
```

Thread from `cli_dispatch/lanes.rs` and `compilerd.rs` (`None`s).

- [ ] **Step 5: `isolation.md`** (`category: Language Reference`). Grammar table
(repeatable `--caps`, one token per flag; `env:ro|rw`; `random:`; `deterministic`);
limits table including `--max-depth` and the sentence "`--max-steps` bounds evaluated
HIR nodes, not CPU time; a local run has no wall-clock bound"; exit codes
`0 / 1 fault / 77 / 78 / 79 / 101 interpreter bug`; "`vox run` without `--caps`
grants everything; `--caps` is opt-in locally and mandatory on the mesh"; the legacy
directive is unscoped; `process:allow` means "runs any binary on this host as the
daemon user"; what `--max-memory` does not count; the TOCTOU residual; disk/file
caps apply on the mesh only.

- [ ] **Step 6: Run; mutate** — `cargo test -q -p vox-cli --test run_interp_limits && cargo test -q -p vox-cli --lib mem_limit`. Mutation: `if new > LIMIT.0.load(…)` → `if false` in `charge` → `memory_limit_exits_79` MUST FAIL (exits 0 or 78); restore; `grep -c 'new > LIMIT' crates/vox-cli/src/mem_limit.rs` → 1.

- [ ] **Step 7: Commit** — `cargo fmt -p vox-cli`. `git commit -m "feat(run): repeatable --caps, --max-steps/--max-memory/--max-depth, allocator ceiling"`.

---

# PR 4 — The git point of no return

## Task 7: The interpreter becomes the default for script-shaped files

**Files:** `crates/vox-cli/src/commands/runtime/run/run.rs` (predicate),
`commands/run.rs`, `cli_args.rs`, `docs/src/reference/cli.md`,
`.github/workflows/setup-e2e.yml` (pin `--mode script` on the three native-lane
lines), `crates/vox-cli/tests/run_mode_dispatch.rs`.

**Preconditions (do not start without all four):**
1. Task 5b table: `fmt` / `install-hooks` / `setup` / `arch-check` show `ok` (or
   arch-check's documented own exit) in the run column.
2. `list.push` shipped (PR 2 `list_push_is_amortised_constant_not_quadratic` green).
3. `PATH=""` proof prepared (Step 4) — auto-mode no longer needs cargo.
4. Three existing escape hatches documented in `cli.md` and `isolation.md`. Do not
   invent a new key.

- [ ] **Step 1: Confirm preconditions.** Re-read Task 5b's table. Re-run
`cargo test -q -p vox-compiler --test eval_typeck_parity_test list_push`. If either
is not green, stop.

- [ ] **Step 2: Write the failing tests** (append to `run_mode_dispatch.rs`; retarget
the ignored `run_mode_auto_matches_script_for_script_shaped_file`)

```rust
#[test]
fn auto_mode_runs_script_shaped_files_under_the_interpreter() {
    let f = std::env::temp_dir().join(format!("vox-auto-{}.vox", std::process::id()));
    std::fs::write(&f, r#"pub fn main() { print("AUTO_INTERP_OK") }"#).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vox"))
        .env("PATH", "")
        .args(["run"])
        .arg(&f)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains("AUTO_INTERP_OK"));
}

#[test]
fn script_shaped_predicate_keeps_service_surfaces_on_the_native_lane() {
    use vox_cli::commands::runtime::run::run::is_script_shaped;
    for src in [
        "table T { id: Id[T] }\npub fn main() {}",
        "routes { }\npub fn main() {}",
        "server hello() to str { return \"x\" }\npub fn main() {}",
        "workflow w() { }\npub fn main() {}",
        "actor A { }\npub fn main() {}",
        "@page fn home() {}",
    ] {
        assert!(!is_script_shaped(src), "must keep the native/app lane: {src:?}");
    }
    assert!(is_script_shaped("pub fn main() { print(1) }"));
}

#[test]
fn three_escape_hatches_still_name_the_native_lane() {
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_vox")).args(["run", "--help"]).output().unwrap();
    let s = String::from_utf8_lossy(&help.stdout);
    assert!(s.contains("--mode"), "{s}");
}
```

- [ ] **Step 3: Implement.** One predicate, one file:

```rust
/// Script-shaped: declares `fn main()` and none of the surfaces the native lane boots.
/// Scans the first 8 KiB like the `@page` heuristic; false positives route to the native
/// lane, which is the safe direction.
pub fn is_script_shaped(head: &str) -> bool {
    let has_main = head.contains("fn main(");
    let service = ["@page", "\nroutes", "\ntable ", "\nserver ", "\nquery ", "\nmutation ", "\nactor ", "\nworkflow ", "\nactivity "];
    let h = format!("\n{head}");
    has_main && !service.iter().any(|s| h.contains(s))
}
```

In `commands/run.rs::run`: when `mode == Auto` and `web_mode != WebRunMode::Script`
and `is_script_shaped(&head)` → `run_interp(...)`; otherwise the existing lanes.
When the file has `fn main()` but a service surface was found,
`eprintln!("vox: this program declares a service surface; running it on the native lane (use --mode script to silence this)")`.
Update `RunMode::Auto` docs, `cli_args.rs`, `cli.md`. Document the three hatches
(`--mode script`, `Vox.toml [web] run_mode = "script"`, `VOX_WEB_RUN_MODE=script`)
in those words.

- [ ] **Step 4: Pin `setup-e2e.yml`.** The three lines that exist to test
`--features script-execution` get `--mode script` so they keep testing the native
lane after auto flips. `lefthook.yml` uses `cargo run` and does not need a PATH
`vox` version guard.

- [ ] **Step 5: Run and commit** — `cargo test -q -p vox-cli --test run_mode_dispatch && cargo test -q -p vox-cli --test run_interp_limits`. `git commit -m "feat(run): the interpreter is the default for script-shaped files; cargo is opt-in"`.

This commit is the only revert needed to undo the local default flip.

---

# PR 5 — The fleet point of no return

## Task 8: Protocol — job ids, honest version refusal, payload framing

**Files:** `crates/vox-mesh-transport/src/protocol.rs`, `endpoint.rs`,
`tests/security.rs`, `tests/mailbox.rs` (note only).

`REFUSED_PROTO = 4003` (4001 untrusted, 4002 too large, 4004 no mailbox).
`JobLimits` grows `max_disk_bytes` (32 MiB), `max_files` (4 096), `max_depth`,
`max_concurrent`. `QueueStats` reports `pending_count` **and** `max_concurrent`.

- [ ] **Step 1: Write the failing tests** (append to `security.rs`)

```rust
#[tokio::test]
async fn a_payload_of_exactly_the_declared_size_is_accepted_and_reaches_the_executor() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let payload = b"pub fn main() { print(\"hi\") }".to_vec();
    let resp = send_run_on(&server, JobId(1), TaskKind::VoxScript, &payload).await;
    assert!(matches!(resp, JobResponse::Output(_)), "{resp:?}");
    assert_eq!(server.exec.last_payload(), payload);
}

#[tokio::test]
async fn a_payload_shorter_than_its_claim_is_refused_not_truncated() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let resp = send_run_with_claim(&server, JobId(2), TaskKind::VoxScript, b"short", 999).await;
    assert!(matches!(resp, JobResponse::Failed(ref m) if m.contains("claim")), "{resp:?}");
}

#[tokio::test]
async fn a_voxscript_payload_over_four_mib_is_refused_before_transfer() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let resp = send_run_with_claim(&server, JobId(3), TaskKind::VoxScript, b"", 5 * 1024 * 1024).await;
    assert!(matches!(resp, JobResponse::Failed(ref m) if m.contains("exceeds")), "{resp:?}");
    assert_eq!(server.exec.invocations(), 0);
}

#[tokio::test]
async fn a_v1_peer_is_told_which_machine_to_upgrade() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let (resp, close) = send_raw_hello_on(&server, Hello { proto: 1, ..Hello::current() }).await;
    assert!(matches!(resp, Some(JobResponse::Failed(ref m)) if m.contains("v1") && m.contains("v2")), "{resp:?}");
    assert_eq!(close, Some(REFUSED_PROTO));
    assert_eq!(REFUSED_PROTO, 4003);
}

#[test]
fn a_new_trailing_field_is_not_readable_from_an_old_sender() {
    // Same variant index as JobResponse::Probed, plus a dummy at index 0 so a
    // discriminant mismatch cannot masquerade as the trailing-field question.
    #[derive(serde::Serialize)]
    enum OldResp {
        #[allow(dead_code)]
        ProbeAck,
        Probed { host_triple: String, vox: String, task_kinds: Vec<TaskKind> },
    }
    let bytes = postcard::to_allocvec(&OldResp::Probed {
        host_triple: "t".into(),
        vox: "v".into(),
        task_kinds: vec![],
    })
    .unwrap();
    assert!(
        postcard::from_bytes::<JobResponse>(&bytes).is_err(),
        "#[serde(default)] does not buy wire compatibility under postcard; PROTO does"
    );
}

#[test]
fn isolation_default_is_the_interpreter_and_there_is_no_third_tier() {
    assert_eq!(Isolation::DEFAULT_FOR_MESH, Isolation::Interpreter);
    for v in [Isolation::Interpreter, Isolation::Native] {
        match v {
            Isolation::Interpreter | Isolation::Native => {}
        }
    }
}

#[test]
fn proto_is_two_and_limits_carry_every_bound() {
    assert_eq!(vox_mesh_transport::protocol::PROTO, 2);
    let l = JobLimits::default();
    assert_eq!(l.max_payload_for(TaskKind::VoxScript), 4 * 1024 * 1024);
    assert_eq!(l.max_memory_bytes, 512 * 1024 * 1024);
    assert_eq!(l.max_steps, 50_000_000);
    assert_eq!(l.max_disk_bytes, 32 * 1024 * 1024);
    assert_eq!(l.max_files, 4_096);
    assert!(l.max_concurrent >= 2);
}
```

Extend `SpyExecutor` with `last_payload()` / `invocations()`. Write
`send_run_on` / `send_run_with_claim` / `send_raw_hello_on`. Update existing `Run`
senders to send a `job_id` and a payload frame.

- [ ] **Step 2: Run to verify they fail.**

- [ ] **Step 3: Implement `protocol.rs`.**

```rust
pub const PROTO: u16 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

pub enum JobRequest {
    Probe,
    Run { job_id: JobId, kind: TaskKind, payload_bytes: u64 },
    Cancel { job_id: JobId },
    QueueStats,
}

pub enum Isolation { Interpreter, Native }
impl Isolation { pub const DEFAULT_FOR_MESH: Self = Self::Interpreter; }

pub struct JobLimits {
    pub wall_clock: Duration,
    pub max_output_bytes: usize,
    pub max_payload_bytes: u64,   // default 16 MiB
    pub max_memory_bytes: usize,  // default 512 MiB
    pub max_steps: u64,           // default 50_000_000
    pub max_depth: usize,         // default 1_024
    pub max_disk_bytes: u64,      // default 32 MiB
    pub max_files: u32,           // default 4_096
    pub max_concurrent: u32,      // default available_parallelism.max(2)
    pub isolation: Isolation,
}
impl JobLimits {
    pub fn max_payload_for(&self, kind: TaskKind) -> u64 {
        match kind { TaskKind::VoxScript => 4 * 1024 * 1024, _ => self.max_payload_bytes }
    }
}

pub struct QueueStats {
    pub pending_count: u64,
    pub max_concurrent: u64,
}
```

`endpoint.rs`: `pub const REFUSED_PROTO: u32 = 4003;`. On `check_hello` `Err`, write
`JobResponse::Failed`, `finish`, `conn.close(REFUSED_PROTO.into(), b"proto mismatch")`.
Payload path: refuse claim `> cap` before read; `read_frame` max =
`payload_bytes.saturating_add(8)`; refuse if `p.len() as u64 != *payload_bytes`.
`ReceivedJob` gains `payload`. Fix every `Isolation::Wasm`/`Container` the compiler
reports.

- [ ] **Step 4: Run** — `cargo test -q -p vox-mesh-transport && cargo clippy -q -p vox-mesh-transport --all-targets -- -D warnings`.

- [ ] **Step 5: Commit** — `git commit -m "feat(mesh): PROTO 2 — sender job ids, REFUSED_PROTO 4003, payload framing"`.

---

## Task 9: `InterpExecutor` — the mesh runs VoxScript

**Files:** Create `interp_executor.rs` (same-file `mod tests`), `caps_spec.rs`
(same-file `mod tests`), `tests/common/mod.rs`, `tests/interp_executor.rs`;
modify `lib.rs`, `Cargo.toml` (`tokio` features `process, io-util, time, sync, fs, macros`;
`tempfile`; `libc` under `cfg(unix)`; existing `win32job` — no new crate).

- [ ] **Step 1: Move the helpers** to `tests/common/mod.rs`.
`security.rs` declares `mod common;`. Run `cargo test -q -p vox-mesh-transport --test security` → still green.

- [ ] **Step 2: Write the failing tests** in `tests/interp_executor.rs`. Every live
spawn test carries `#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]`.
Add `vox_lit` for Windows path literals in any later string-built `.vox`.

```rust
mod common;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vox_mesh_transport::endpoint::JobExecutor;
use vox_mesh_transport::protocol::{JobId, JobLimits, JobResponse};
use vox_mesh_transport::trust::TrustLevel;
use vox_mesh_transport::{InterpExecutor, MeshTrust};
use vox_mesh_types::TaskKind;

fn vox_lit(p: &Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

fn vox_bin() -> PathBuf {
    if let Ok(p) = std::env::var("VOX_BIN") { return p.into(); }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let exe = root.join("target/debug").join(if cfg!(windows) { "vox.exe" } else { "vox" });
    if !exe.exists() {
        let st = std::process::Command::new(env!("CARGO")).current_dir(&root)
            .args(["build", "-q", "-p", "vox-cli", "--bin", "vox"]).status().expect("spawn cargo");
        assert!(st.success(), "could not build the vox binary the executor spawns");
    }
    exe
}

fn exec(trust: Arc<MeshTrust>) -> Arc<dyn JobExecutor> {
    Arc::new(InterpExecutor::new(trust, vox_bin(), JobLimits::default()))
}

#[test]
fn caps_mapping_never_grants_more_than_the_trust_level() {
    let d = tempfile::tempdir().unwrap();
    let tokens = InterpExecutor::caps_for(TrustLevel::Sandboxed, d.path()).unwrap();
    let s = tokens.join(",");
    assert!(s.contains("fs:rw=") && s.contains("time:real"));
    for forbidden in ["net:allow", "process:allow", "env:", "secrets"] { assert!(!s.contains(forbidden), "{s}"); }
    let n = InterpExecutor::caps_for(TrustLevel::Native, d.path()).unwrap().join(",");
    assert!(n.contains("net:allow") && n.contains("process:allow") && n.contains("env:ro"));
    assert!(!n.contains("secrets"));
}

#[test]
fn a_job_dir_with_a_comma_is_accepted() {
    let d = tempfile::Builder::new().prefix("a,b-").tempdir().unwrap();
    assert!(InterpExecutor::caps_for(TrustLevel::Sandboxed, d.path()).is_ok());
}

#[test]
fn read_capped_does_not_mark_an_exact_max() {
    let exact = vec![b'x'; 64];
    let (buf, truncated) = futures::executor::block_on(InterpExecutor::read_capped_for_test(&exact[..], 64));
    assert_eq!(buf.len(), 64);
    assert!(!truncated, "exact-max must not be marked truncated");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_voxscript_job_runs_and_returns_its_output() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(&server, JobId(1), TaskKind::VoxScript, b"pub fn main() { print(\"MESH_RAN\") }").await;
    assert!(matches!(resp, JobResponse::Output(ref b) if String::from_utf8_lossy(b).contains("MESH_RAN")), "{resp:?}");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_sandboxed_peer_cannot_read_the_host_filesystem() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(&server, JobId(2), TaskKind::VoxScript, b"pub fn main() { let s = fs.read(\"/etc/hosts\"); print(\"LEAK\") }").await;
    assert!(matches!(resp, JobResponse::Failed(ref m) if m.contains("capability denied") && m.contains("fs.read")), "{resp:?}");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_forged_exit_77_under_grant_native_is_not_a_denial() {
    let server = common::start_server_with(exec).await;
    server.trust.trust_with(&common::client_id(), vox_mesh_transport::trust::TrustLevel::Native).unwrap();
    let resp = common::send_run_on(&server, JobId(3), TaskKind::VoxScript, b"pub fn main() { process.exit(77) }").await;
    assert!(matches!(resp, JobResponse::Failed(ref m) if !m.contains("capability denied")), "{resp:?}");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_runaway_allocation_is_killed_and_reported() {
    let mut limits = JobLimits::default();
    limits.max_memory_bytes = 64 * 1024 * 1024;
    limits.max_steps = 10_000;
    let server = common::start_server_with(|t| Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(&server, JobId(4), TaskKind::VoxScript, b"pub fn main() { let mut s = \"x\"; let mut i = 0; while i < 40 { s = s + s; i = i + 1 } }").await;
    assert!(matches!(resp, JobResponse::Failed(ref m) if m.contains("memory limit")), "{resp:?}");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn output_is_capped_and_marked_only_when_over() {
    let mut limits = JobLimits::default();
    limits.max_output_bytes = 64 * 1024;
    let server = common::start_server_with(|t| Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(&server, JobId(5), TaskKind::VoxScript, b"pub fn main() { let mut i = 0; while i < 10000 { print(\"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"); i = i + 1 } }").await;
    match resp {
        JobResponse::Output(b) => {
            assert!(b.len() <= 64 * 1024 + 128);
            assert!(String::from_utf8_lossy(&b).contains("[vox: output truncated"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn one_peer_cannot_cancel_another_peers_job() {
    let mut limits = JobLimits::default();
    limits.max_steps = 50_000_000;
    let server = common::start_server_with(|t| Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>).await;
    let (a, b) = (common::client_endpoint(common::client_sk_a()).await, common::client_endpoint(common::client_sk_b()).await);
    server.trust.trust(&a.id(), None).unwrap();
    server.trust.trust(&b.id(), None).unwrap();
    let slow = b"pub fn main() { let mut i = 0; while i < 200000 { i = i + 1 }; print(\"DONE\") }";
    let a_job = tokio::spawn(common::send_run_from(a.clone(), &server, JobId(9), TaskKind::VoxScript, slow));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let cancel = common::send_cancel_from(b, &server, JobId(9)).await;
    assert!(matches!(cancel, JobResponse::Failed(ref m) if m.contains("no such running job")), "{cancel:?}");
    let done = a_job.await.unwrap();
    assert!(matches!(done, JobResponse::Output(ref o) if String::from_utf8_lossy(o).contains("DONE")), "{done:?}");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_reused_job_id_from_the_same_peer_is_refused() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let slow = b"pub fn main() { let mut i = 0; while i < 200000 { i = i + 1 }; print(\"A\") }";
    let first = tokio::spawn(common::send_run_on_owned(server.clone(), JobId(11), TaskKind::VoxScript, slow));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let second = common::send_run_on(&server, JobId(11), TaskKind::VoxScript, b"pub fn main() { print(\"B\") }").await;
    assert!(matches!(second, JobResponse::Failed(ref m) if m.contains("already running")), "{second:?}");
    let _ = first.await;
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn ml_task_kinds_are_refused_with_a_reason_when_no_engine_is_installed() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(&server, JobId(6), TaskKind::TextInfer, b"{}").await;
    assert!(matches!(resp, JobResponse::Failed(ref m) if m.contains("no engine")), "{resp:?}");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn process_boundary_clears_unlisted_env_and_points_home_at_the_readonly_dir() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(7),
        TaskKind::VoxScript,
        b"pub fn main() { print(env.get(\"VOX_SHOULD_NOT_LEAK\") is None) }",
    )
    .await;
    assert!(matches!(resp, JobResponse::Output(ref b) if String::from_utf8_lossy(b).contains("true")), "{resp:?}");
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn unix_process_group_kill_reaches_a_grandchild() {
    let mut limits = JobLimits::default();
    limits.wall_clock = std::time::Duration::from_millis(400);
    let server = common::start_server_with(|t| Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>).await;
    server.trust.trust_with(&common::client_id(), TrustLevel::Native).unwrap();
    let t0 = std::time::Instant::now();
    let resp = common::send_run_on(&server, JobId(8), TaskKind::VoxScript, b"pub fn main() { process.run(\"sleep\", [\"30\"]) }").await;
    assert!(t0.elapsed() < std::time::Duration::from_secs(2), "grandchild leaked: {:?}", t0.elapsed());
    assert!(matches!(resp, JobResponse::Failed(_)), "{resp:?}");
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn windows_job_object_kill_reaches_a_grandchild() {
    let mut limits = JobLimits::default();
    limits.wall_clock = std::time::Duration::from_millis(400);
    let server = common::start_server_with(|t| Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>).await;
    server.trust.trust_with(&common::client_id(), TrustLevel::Native).unwrap();
    let t0 = std::time::Instant::now();
    let resp = common::send_run_on(&server, JobId(8), TaskKind::VoxScript, b"pub fn main() { process.run(\"timeout\", [\"/t\", \"30\", \"/nobreak\"]) }").await;
    assert!(t0.elapsed() < std::time::Duration::from_secs(2), "grandchild leaked: {:?}", t0.elapsed());
    assert!(matches!(resp, JobResponse::Failed(_)), "{resp:?}");
}
```

(`common` gains `client_sk_a/b`, `send_run_from`, `send_cancel_from`,
`send_run_on_owned`, `trust_with`. Reuse `baseline_passthrough_env()` from
`remote_worker.rs` — defactor the ~13 names, do not invent a shorter list.)

- [ ] **Step 3: Run to verify they fail.**

- [ ] **Step 4: Implement `caps_spec.rs`** (`// vox:defactored-from vox-compiler 2026-09-06`).
`from_roots` + `to_tokens` only. Same-file tests: comma in the directory is accepted;
tokens parse individually via a `vox-compiler` test that consumes `caps_spec`'s
string so the two cannot drift. No `allows_path` here — the child does that.

- [ ] **Step 5: Implement `interp_executor.rs`.** Same-file tests for
`read_capped` (exact-max, over-max) and `caps_for`. Structure:

```rust
    pub async fn read_capped<R: tokio::io::AsyncRead + Unpin>(mut r: R, max: usize) -> (Vec<u8>, bool) {
        let mut buf = Vec::new();
        let mut truncated = false;
        let mut chunk = [0u8; 8192];
        loop {
            match r.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    let would = buf.len().saturating_add(n);
                    if would > max {
                        truncated = true;
                        if buf.len() < max {
                            buf.extend_from_slice(&chunk[..max - buf.len()]);
                        }
                    } else {
                        buf.extend_from_slice(&chunk[..n]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        (buf, truncated)
    }
```

`run_script`:
- `try_acquire` on a **per-`EndpointId`** semaphore (in front of the global one).
  Refuse with a retryable message past either.
- Refuse if `(peer, job_id)` is already in `running`.
- Job dir + second read-only home dir.
- Spawn `vox run --mode interp` with **repeatable** `--caps` tokens, `--max-steps`,
  `--max-memory`, `--max-depth`.
- `env_clear()` then apply `baseline_passthrough_env()`. Unix: `HOME` = home dir,
  `TMPDIR` = job dir, `process_group(0)`. Windows: `USERPROFILE` = home dir,
  `TEMP`/`TMP` = job dir, `SystemRoot` passthrough, assign to a `win32job` Job
  Object.
- `select!` on `child.wait()`, wall clock, `Ok(()) = cancel_rx` (a dropped sender
  is not a cancel). Kill after the select (`Done` enum).
- On cancel/timeout: abort the drain tasks (`out_task.abort(); err_task.abort();`),
  then kill the group / Job Object.
- Thread `JobLimits::max_steps` into the child.
- Exit map as spec §3.4. Windows NTSTATUS arm names `STATUS_STACK_OVERFLOW`;
  do not use the Unix `None` wording there.
- One `tracing::info!(peer, job_id, exit, elapsed)` per job.

`JobExecutor::execute` keeps `std::sync::Mutex` on `running`. Probe returns
`engines: Vec::new()`. QueueStats sets `pending_count` and `max_concurrent`.

- [ ] **Step 6: Register the slow tests** in `pre_push.rs` **and**
`.github/workflows/ci.yml` (~1124–1131). Nightly is acceptable; `--include-slow`
is not required if the nightly lane already runs `--run-ignored` for this crate.

- [ ] **Step 7: Run; mutate twice.** Mutation (a): `Sandboxed` extras become the
`Native` extras → `caps_mapping…` MUST FAIL. Mutation (b): `Cancel` keys on
`job_id` only → `one_peer_cannot_cancel…` MUST FAIL. Restore; grep.

- [ ] **Step 8: Commit** — `cargo fmt -p vox-mesh-transport`. `git commit -m "feat(mesh): InterpExecutor — bounded child, Job Object, per-peer slots, peer-scoped cancel"`.

---

## Task 10: Wire the executor; delete `ProbeOnlyExecutor`, the bundle lane, and `secret_gate`

**Files:** `crates/vox-ml-cli/src/commands/mesh_cli.rs`; `vox-mesh-transport`
`endpoint.rs`, `tests/mailbox.rs`, `tests/security.rs`;
`vox-orchestrator/src/a2a/remote_worker.rs` (+ its test module);
delete `secret_gate.rs` (+ `secret_bag.rs` if orphaned); `envelope.rs`;
`task_submit.rs`; `vox-secrets` spec; HTTP dispatch handlers;
`vox-config` comment on the surviving placement key.

- [ ] **Step 1: Retarget** `a_trusted_peer_gets_a_sandbox_by_default` to
`job.limits.isolation == Isolation::Interpreter`.

- [ ] **Step 2: Swap the executor** at `mesh_cli.rs`. Banner on `vox mesh join`:
the command now executes Vox received over the mesh. Log `vox --version` at
construction; warn on mismatch with `CARGO_PKG_VERSION`.

```rust
                let vox_bin = std::env::current_exe().ok()
                    .and_then(|p| p.parent().map(|d| d.join(if cfg!(windows) { "vox.exe" } else { "vox" })))
                    .filter(|p| p.exists())
                    .unwrap_or_else(|| std::path::PathBuf::from("vox"));
                let exec = std::sync::Arc::new(vox_mesh_transport::InterpExecutor::new(
                    trust.clone(), vox_bin, vox_mesh_transport::protocol::JobLimits::default(),
                ));
```

- [ ] **Step 3: Delete `ProbeOnlyExecutor`.** Fix `tests/mailbox.rs` to
`common::SpyExecutor`. `cargo build -q -p vox-mesh-transport -p vox-ml-cli --features populi`.

- [ ] **Step 4: Delete the bundle lane and `secret_gate`.** Delete
`run_dispatched_bundle`, `BundleKind`, `classify_bundle`, the bundle call site,
and the tests that reference them. At the source-lane call site:
`let secret_env: Vec<(String, String)> = Vec::new();`. Per-dispatch tempdir +
repeatable `--caps`. Remove `exec_bundle_*` fields. Remove
`SecretId::VoxMeshExecPolicy`. HTTP handlers: literal `"source-only"` plus
`// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="mesh-phase6" canonical="vox_mesh_transport::InterpExecutor"`.

- [ ] **Step 5: `vox ci secrets-contracts` then `secrets-parity` then
`secret-env-guard`.** Comment at `vox-config` `config_registry.rs`: the surviving
`VOX_MESH_EXEC_POLICY` is task *placement*.

- [ ] **Step 6: `vox mesh probe`** is the operator surface over `directory()`.
Keep PROTO-mismatch `Failed` — do **not** map `_ => None` in `directory.rs`.

- [ ] **Step 7: Build, test, commit** — `cargo test -q -p vox-mesh-transport -p vox-orchestrator 2>&1 | tail -5 && cargo clippy -q -p vox-orchestrator -p vox-ml-cli -p vox-secrets -p vox-populi -p vox-plugin-populi-mesh --all-targets -- -D warnings`. `git commit -m "feat(mesh): serve with InterpExecutor; delete ProbeOnlyExecutor, the native bundle lane, and secret_gate"`.

---

## Task 11: `PopuliHttpOp::Dispatch` over the mesh

**Files:** `directory.rs` (`PeerEntry.addrs`); `vox-workflow-runtime`
`workflow/populi.rs`, `workflow/types.rs`.

- [ ] **Step 1: `PeerEntry.addrs`.** Add the field; `fan_out` returns it;
`directory()` populates it. Test:
`a_probed_peer_carries_the_addresses_it_was_dialled_on`.

- [ ] **Step 2: Failing tests**

```rust
    #[tokio::test]
    #[ignore = "owner:mesh sunset:2026-12-31 slow: spawns the vox binary via InterpExecutor"]
    async fn dispatch_runs_real_source_on_a_loopback_peer() {
        // arrange: loopback server with InterpExecutor, trust this process
        // act: execute_populi_step with Dispatch + source `pub fn main() { print("WF_RAN") }`
        // assert: envelope["control"] == "dispatch_ok"
        //         envelope["result_output"] contains "WF_RAN"
        //         envelope["peer"] is the server id
        //         envelope["candidates"] == 1
        //         no "control_url" key
        // If PopuliActivity has no source field, assert the named error instead
        // ("activity `X` has no dispatchable source") and delete the shim synthesis.
        unimplemented!("fill from PopuliActivity's real fields — do not leave this as comments");
    }

    #[test]
    fn wait_is_inline_and_keeps_the_result_keys() {
        let env = wait_envelope(&sample_activity());
        assert_eq!(env["control"], "completed_inline");
        assert_eq!(env["success"], true);
        assert!(env.get("result_output").is_some() && env.get("exit_code").is_some());
    }
```

Read `PopuliActivity`. If it carries source, wrap with `pub fn main()` as needed
and send it. If it does not, return
`Err("activity `X` has no dispatchable source; inline source is required for mesh dispatch")`
and **delete** `workflow_durable_shim::execute_activity` synthesis as dead. Either
way the test above is a real assertion.

- [ ] **Step 3: Implement.** `run_on_peer` sends
`JobRequest::Run { job_id: JobId(next_local_id()), kind: VoxScript, payload_bytes }`
and reads with a 16 MiB frame max. `Dispatch` first-fit on `VoxScript` in
`task_kinds` — comment: `// first-fit, no queue-depth weighting. Phase 4 Task 4.1
replaces this with a PlacementRecord.` Emit `"peer"` and `"candidates": n` through
`mesh_envelope` without changing its signature. `Wait` →
`completed_inline` with `success` / `result_output` / `exit_code`. Delete HTTP
`Dispatch`/`Wait` and `VOX_MESH_CONTROL_ADDR` text.

- [ ] **Step 4: Test and commit** — `cargo test -q -p vox-workflow-runtime --features mens -- --include-ignored 2>&1 | tail -5 && cargo clippy -q -p vox-workflow-runtime --all-targets --features mens -- -D warnings`. `git commit -m "feat(workflow): Dispatch runs real source on a mesh peer; Wait is inline"`.

Update the mesh plan Status in this PR (the surface now exists): Task 3.4 `[x]`;
Known-gaps sandbox row → ADR-048.

---

# PR 6 — Mechanical retirements

## Task 12: Delete the wasi script lane and rejected isolation tiers

**Files (compile):** delete `vox-cli/src/commands/wasm.rs`,
`commands/runtime/run/backend/wasi.rs`, `src/isolation.rs`; modify `Cargo.toml`,
`lib.rs`, `cli_dispatch/mod.rs`, `commands/mod.rs`, `cli_args.rs`, `script.rs`,
`backend/{mod,tests}.rs`, `diagnostics/` doctor sources (not a bare `commands/`
path), `voxup` `provision_wasm_sysroots`, `vox-codegen` `WasiBinary`,
`vox-populi` and `vox-plugin-populi-mesh` HTTP dispatch (those spawn
`vox wasm run` / `--isolation wasm`).

**Contracts, in Global Constraints order:** catalog → command-registry →
capability-registry + model-manifest → cli-command-surface.generated.md →
gui-surface reports → `UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli command_catalog`
→ `crate-edges --tighten` (only `["vox-cli","vox-wasm-engine"]` removed).

- [ ] **Step 1: Failing test** `isolation_and_wasm_surfaces_are_gone` (clap 4.5:
`unrecognized subcommand` / `unexpected argument '--isolation' found`).

- [ ] **Step 2: Delete and fix every compile site.** Doctor row:
`Check::pass("WASI target (optional)", "not required: scripts run under the interpreter; `--mode script` targets the host")`.

- [ ] **Step 3: Regenerate the contract chain in order.** Then
`cargo run -q -p vox-cli -- ci ssot-drift` must be green.

- [ ] **Step 4: `cargo tree -p vox-cli -e features -i wasmtime` → `did not match any packages`.**
`cargo run -q -p vox-cli -- ci check-links`.

- [ ] **Step 5: Commit** — `git commit -m "chore(cli): delete the wasi script lane, vox wasm, and rejected isolation tiers"`.

---

## Task 13: Delete the MicroVM stub; rename `vox_ir` → `hir_export`

**Files:** delete `vox-skill-runtime/src/microvm.rs`, `tests/microvm_tier.rs`
(**move** `Tier` ordering and `plan_for_min_tier` error-path assertions into
`src/runtime.rs`); rename `vox-codegen/src/vox_ir/` → `hir_export/` with **scoped**
edits only (no blind `sed`); update both schema mirrors together; rewrite
`vox-ir-specification.md`. Keep `--emit-ir` and `"2.0.0"`.

- [ ] **Steps:** failing tests (`tier_ordering_and_min_tier_error_path` moved
verbatim; `hir_export_is_a_json_envelope_and_says_so`) → `git rm` / `git mv` →
enumerated edits →
`cargo test -q -p vox-skill-runtime -p vox-codegen -p vox-compiler --test ir_emission_test && cargo run -q -p vox-cli -- ci check-links && cargo run -q -p vox-cli -- ci ssot-drift`
→ commit `chore: delete the MicroVM stub; rename vox_ir to hir_export`.

---

## Task 14: `sandbox.rs` and `native.rs` stop presenting `VOX_SANDBOX=1` as isolation

Honesty about the *native* lane. `sandbox.rs` is not defence-in-depth for the
interpreter child.

- [ ] **Steps:** failing test `macos_does_not_pretend_an_env_var_is_a_sandbox`
(also assert `native.rs`'s command builder sets no `VOX_SANDBOX`) → replace the
"Other" branch with `tracing::warn!` naming `isolation.md` and delete both `env`
calls → `cargo test -q -p vox-cli sandbox` → commit
`fix(sandbox): stop presenting VOX_SANDBOX=1 as isolation`. Banner the
contradicting sentence at
`docs/src/architecture/vox-language-rules-phase4-runtime-monitors-2026.md`.

---

## Task 15: ADR, retirement rows, doctor, bookkeeping

**Files:** create
`docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md`
(`category: Architecture Decisions (ADRs)`, shape of ADR-047); modify
`docs/src/adr/{index,README}.md`, `AGENTS.md` (tier table **and** §Retired
Surfaces), both retirement YAMLs, mesh plan remaining rows, `research-index.md`.

**Doctor: five real rows.** Delete the permanently-green tombstone of a deleted
feature. Rows: (1) interpreter is the default for script-shaped files; (2) cargo
is optional for scripts; (3) rustc is optional for scripts; (4) wasm32-wasip1 is
not required; (5) **executor binary accepts `--caps`** (`vox run --help` contains
`--caps`). Sources live under `diagnostics/`.

- [ ] **Step 1: ADR-048.** Decision: the interpreter is the isolation tier for
VoxScripts. Record the §5 table (insertion order; crypto edge; accept-loop out of
band; no permanently-red golden; in-process disk caps). Consequence: PROTO 2;
six embedders; `@versioned` gated; `list.push` in-place before the flip.

- [ ] **Step 2: Retirement rows** for `--isolation wasm|container|gvisor|microvm`,
`vox wasm run`, `script-wasi`, `ProbeOnlyExecutor`, `MicroVmRuntime`/`Tier::MicroVm`,
`VoxMeshExecPolicy` (SecretId), `exec_bundle_b64`. Same in AGENTS.md §Retired
Surfaces. `cargo run -q -p vox-cli -- ci retired-symbol-check`.

- [ ] **Step 3: AGENTS.md tier table** and residual references:
`GEMINI.md`, `.cursor/rules/voxscript-first-automation.mdc`,
`docs/src/explanation/expl-architecture.md`, `docs/src/reference/mobile-edge-ai.md`,
`docs/src/reference/cli.md`.

- [ ] **Step 4: Mesh plan + indexes** — Status "Tasks 3.1–3.4 done and merged";
Task 3.1 note "inbox drain: not funded by ADR-048"; Task 6.1 "bundle lane deleted
by ADR-048 Task 10". ADR-048 into `adr/index.md`, `README.md`, `research-index.md`.

- [ ] **Step 5: Five doctor rows**, no tombstone.

- [ ] **Step 6: Lint, fast gate, commit** —
`cargo run -q -p vox-doc-pipeline -- --lint-only --paths docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md && cargo run -q -p vox-cli -- ci pre-push`.
`git commit -m "docs: ADR-048, retirement rows, five doctor checks, plan bookkeeping"`.

---

# Task 16 — Verification (not a PR)

- [ ] **Step 1:** `ls crates/vox-gui/ui/dist || (cd crates/vox-gui/ui && pnpm install && pnpm build)`.
- [ ] **Step 2:** `cargo run -q -p vox-cli -- ci pre-push --complete 2>&1 | tail -30`. If it
halts at `doc-inventory verify`, leave the file uncommitted, re-run, then
`git checkout -- docs/agents/doc-inventory.json`.
- [ ] **Step 3:** Nightly gate + executor ignored tests. Expected PASS. No
`KNOWN_TIER_ASYMMETRIES` list.
- [ ] **Step 4: Local Network Privacy, at the keyboard.** The first time `vox`
touches the local network macOS may show a TCC dialog. Trigger it while a human
is present: `vox mesh probe` or the join smoke below. The application firewall is
off (`State = 0`); there is no password dialog. Do not `sudo cargo`.
- [ ] **Step 5: Cross-machine smoke after rebuilding both ends at PROTO 2.**
`vox mesh id` on both, `vox mesh join <ticket>`, dispatch
`pub fn main() { print("cross-machine") }`. Record the round-trip. If unreachable
or not rebuilt, say so; do not fake the number.
- [ ] **Step 6: Report** — what shipped per PR, every mutation and its result,
every residual `EXPECT-TIER-ASYMMETRY` (expected: none), the Task 0 and Task 5b
tables, and the pre-push output verbatim. Nothing is pushed.

---

## Self-review against the spec (revision 3)

| Spec section | PR / Task |
|---|---|
| §3.1 script-shaped routing, three hatches, `list.push` prerequisite | PR 2 Task 2, PR 4 Task 7 |
| §3.2 isolation claim, six embedders, `@versioned`, parent-walk, glob filter, disk/file caps, regex size, depth in `apply_closure` | PR 3 |
| §3.2 threat model (process:allow = shell; forged 77 under `grant_native`) | PR 5 Task 9 |
| §3.3 one-crate gate, nightly, both CI filters, cache wipe, empty residual asymmetries, crypto SSOT, insertion order | PR 1, PR 2 |
| §3.4 executor, Job Object, per-peer slots, `(peer, JobId)` + refuse duplicate, PROTO/4003, disk caps, banner, probe | PR 5 |
| §3.4 accept-loop DoS | **out of this plan** |
| §3.5 deletions and retirements | PR 5 Task 10, PR 6 |
| §3.6 execute-then-re-run, contract chain, doctor, where-things-live same PR | PR 1 Task 0, PR 3 Task 5b, PR 6 |
| §4 mutation-verified guards, process-boundary tests, `PATH=""` flip proof | PR 3, PR 4, PR 5 |
| §5 settled decisions | this header + PR 2 stop-the-line |

**Still narrowed, stated:** `eval/builtins.rs` is not made table-driven (the fs arm
has a table; the rest does not). The inbox drain, lease-over-mesh, and the
accept-loop / frame-deadline commit are out of scope by decision.
