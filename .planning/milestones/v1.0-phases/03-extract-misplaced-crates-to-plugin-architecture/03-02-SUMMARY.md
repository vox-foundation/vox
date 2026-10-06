---
phase: 03-extract-misplaced-crates-to-plugin-architecture
plan: 02
status: complete
subsystem: crate features / CORE Candle policy / planning docs
tags: [candle, optional-dependency, D-06, D-07, D-09, SC-4]
requires: [03-01]
provides:
  - "vox-quantize with candle-core optional behind a new `engine` feature (default off); cuda/metal imply engine"
  - "vox-populi (mens-candle-qlora) and vox-ml-cli (quantize/gpu) opting into vox-quantize/engine"
  - "CORE Candle manifest scan (cargo metadata x crate-layers.v1.json), shown load-bearing by mutation"
  - "ROADMAP SC#4 wording citing D-06/D-08/D-12/D-09"
affects: [vox-quantize, vox-populi, vox-ml-cli, .planning/ROADMAP.md]
tech-stack:
  added: []
  patterns: ["optional heavy dependency behind a named feature, consumers opt in explicitly"]
key-files:
  created:
    - .planning/phases/03-extract-misplaced-crates-to-plugin-architecture/deferred-items.md
  modified:
    - crates/vox-quantize/Cargo.toml
    - crates/vox-quantize/src/lib.rs
    - crates/vox-quantize/src/read.rs
    - crates/vox-populi/Cargo.toml
    - crates/vox-ml-cli/Cargo.toml
    - .planning/ROADMAP.md
decisions:
  - "D-07 honoured: vox-quantize stays L0; no relayer, no crate-edge exception, no edges/baseline edit."
  - "Mutation proof used the smallest VALID mutant (non-optional candle-core plus `engine = []`), because the plan's literal mutant is rejected by cargo as an invalid manifest."
metrics:
  duration: "~10 min wall clock (start 2026-09-26T08:24Z, end 08:35Z)"
  completed: 2026-09-26
plan_head_before: 03cf0e62e684060a91c9e0850012475c0eba6305
actuals:
  tokens: 2250
  tasks: 2
  commits: 3
---

# Phase 3 Plan 02: vox-quantize's Candle behind an `engine` feature (SC#4 CORE half) Summary

candle-core is now optional in vox-quantize, enabled only by a new `engine` feature (`default = []`; `cuda`/`metal` imply `engine`). The whole module tree, the re-exports and the semcov tests are gated on it, so a default vox-quantize build compiles no Candle. vox-populi and vox-ml-cli request `features = ["engine"]` on their already-optional dependency. By `contracts/ci/crate-layers.v1.json`, no L0-L3 crate now declares an unconditional Candle dependency. vox-quantize stays L0, and no edge, exception or lockfile line changed.

## Commits

| Task | Commit | Message | Paths |
|---|---|---|---|
| 1 | `104ad8472` | refactor(03-02): put vox-quantize's Candle dependency behind an engine feature | `crates/vox-ml-cli/Cargo.toml` (1/1), `crates/vox-populi/Cargo.toml` (2/1), `crates/vox-quantize/Cargo.toml` (7/3), `crates/vox-quantize/src/lib.rs` (16/1) |
| 2 (lint fix) | `d2d61984b` | fix(03-02): collapse vox-quantize read.rs nested ifs for clippy | `crates/vox-quantize/src/read.rs` |
| 2 | `11a0cd489` | docs(03-02): state SC#4's CORE-Candle interpretation (D-06/D-09) | `.planning/ROADMAP.md` (1/1) |

## Verification verdicts

- **Precondition:** the index was empty and `crates/vox-quantize`, the vox-populi manifest, the vox-ml-cli manifest and ROADMAP were all clean. Met.
- **Step 1 RED** (`target/phase03-02-scan-red.txt`), with the gate count non-zero (`scan_rc=1`):
  ```text
  vox-plugin-mens-candle-core L4
  vox-plugin-mens-candle-cuda L4
  vox-plugin-mens-candle-metal L4
  vox-quantize L0
  ```
- **Task 1 verify 1:** `rc=101` (literal block RED; see Deviation 1). Per component:
  - The default tree (`cargo tree -p vox-quantize -e normal --depth 1`) has no candle line. The engine tree has `candle-core v0.10.2`.
  - `cargo test -p vox-quantize --features engine`: `test result: ok. 62 passed`.
  - `cargo check -p vox-quantize` is clean, and so is `cargo check -p vox-populi --features mens-candle-qlora`.
  - `cargo check -p vox-ml-cli --features quantize` FAILS in vox-populi, not vox-ml-cli: E0063 at `crates/vox-populi/src/mens/hub.rs:201`, missing `chat_template` and `tokenizer_config`. This is a pre-existing HEAD error; see Deviation 1.
  - Substitute evidence: `cargo check -p vox-ml-cli --no-default-features --features quantize` rc=0. That compiles `commands/quantize.rs`, which holds vox-ml-cli's `vox_quantize::{DevicePref, QuantMixture, quantize, plan_quantize, QuantizeRequest}` uses.
- **Task 1 verify 2 (GREEN scan)**, `verify2=0`: `target/phase03-02-scan.txt` lists only the three L4 `vox-plugin-mens-candle-*` crates, and is non-empty.
- **Task 1 verify 3:** `diff` of the vox-quantize Cargo.lock block before and after is empty. `affected-crates --check` gives `rc=0`, `verify3=0`. `git diff --stat -- Cargo.lock` still shows only the other sessions' 19 inserted lines.
- **Extra check, crate-edges:** `rc=1` with the same 3 pre-existing violations as 03-01: `vox-gui -> vox-db-types`, `vox-gui -> vox-research-shim` and `vox-research-shim -> vox-compiler`. Its output also has the known stale-baseline warning for `vox-populi -> vox-grammar-export`. No NEW or UPWARD line names vox-quantize, vox-populi or vox-ml-cli. No `--tighten`, and no edit to `edges`, exceptions or fan-in-snapshot.
- **Task 1 staged-set check:** `rc=0`. Commit `104ad8472` holds exactly the four planned paths. The hooks passed: fmt-fix (`fmt: formatted 48 file(s).`, the other sessions' dirty .rs files, not staged by me) and tdd-guard ("No issues found").
- **Task 2 mutation proof:**
  - (a) Before the mutation, `git diff --quiet HEAD -- crates/vox-quantize/Cargo.toml` passed.
  - (c) The presence check `grep -c -x 'candle-core = { workspace = true }'` printed `1`.
  - The literal mutant alone makes `cargo metadata` fail (`feature \`engine\` includes \`dep:candle-core\`, but \`candle-core\` is not an optional dependency`), so the scan printed nothing. That output is saved as `target/phase03-02-scan-mutant-literal-invalid.txt`.
  - With the valid mutant (`engine = []` also set; presence check `1`), `cargo metadata` rc=0 and `target/phase03-02-scan-mutant.txt` contains `vox-quantize L0`. The gate count went non-zero (`gate_on_mutant=1`).
  - (e) Both lines were restored with normal edits.
- **Task 2 verify 1:** `t2verify1=0`. The mutant scan has `vox-quantize L0`, the manifest is byte-identical to HEAD, and the commit file list is exactly the four paths.
- **Task 2 verify 2 (clippy):** the first run printed `default=0 engine=101`, and the block still passed. That pass was false: see Deviation 3. After the read.rs fix, the rerun with `CARGO_TERM_COLOR=never` gave `default=0 engine=0`, `t2verify2=0`, and engine tests `62 passed`.
- **Task 2 verify 3:** `t2verify3=0`. Phase 3 contains `D-06`, `D-09` and `D-08/D-12`, and the docs commit numstat is `1 1`.
- **Plan verification 2:** `git status --porcelain -- crates/vox-quantize` is empty.

## Deviations from Plan

1. **[Pre-existing, out of scope] vox-ml-cli `--features quantize` check red on HEAD.**
   - Cause: `crates/vox-populi/src/mens/hub.rs:201`, which is untouched and clean in the working tree, omits two struct fields added on 2026-09-17 (E0063). vox-ml-cli's default `mens-base` reaches this code through `vox-populi/mens-train` -> `mens-hf-hub`, whether or not `quantize` is set.
   - Proof it is independent of this plan: `cargo check -p vox-populi --features mens-hf-hub` fails with the identical E0063, and `cargo tree -p vox-populi --features mens-hf-hub -i vox-quantize` reports vox-quantize is not in that graph.
   - The plan's fails_when does not cover this failure: it expects an unresolved `vox_quantize::` path, and none occurred. The scope boundary puts it outside this plan, so I did not fix it and recorded it in `deferred-items.md`.
   - Substitute evidence: the `--no-default-features --features quantize` check above. vox-ml-cli's other `vox_quantize::` site (`schola/merge_qlora.rs`, under `gpu`) cannot compile until hub.rs is fixed. Its API is unchanged when `engine` is on, and `gpu` implies `quantize`, which requests `engine`.
2. **[Plan defect] Mutation proof needed a valid mutant.** The plan says to drop only `, optional = true`, but that leaves `engine = ["dep:candle-core"]` pointing at a non-optional dependency, and cargo rejects the manifest. Run as written, the plan's check would be vacuous: an empty scan, where the grep fails for the wrong reason. I also set `engine = []` to get the smallest valid regression. The scan then reported `vox-quantize L0`. Both lines were restored and the manifest is byte-identical to HEAD.
3. **[Plan defect + Rule 1] The clippy verify block is vacuous under colored output, and it hid 8 real errors.** Cargo's log puts ANSI codes between `--> ` and the path, so `grep -c -- '--> crates/vox-quantize/'` counted 0 even though clippy `--features engine` failed with 8 `collapsible_if` errors in `crates/vox-quantize/src/read.rs:181-201` (`SafeTensorsSource::resolve_base_dir`). Those errors predate the gate: read.rs was last touched 2026-09-17. As the plan's fails_when directs, I fixed them in a follow-up commit limited to vox-quantize (`d2d61984b`, let-chains, same lookup order and early returns). The rerun used `CARGO_TERM_COLOR=never`, so the grep sees real paths. Any durable version of this check should strip ANSI codes or force `--color never`.
4. **vox-populi line number.** The plan said line 167; the optional vox-quantize line was 166. The content matched, so this changed nothing.

No `--no-verify`, stash, checkout, restore or reset. No `VOX_SKIP_FRESHNESS_CHECK`. Cargo.lock was not staged or changed by this plan. STATE.md and ROADMAP progress rows were not updated.

## Follow-ups

- **D-09 layer disagreement (not fixed):** `vox-plugin-mens-candle-core` is L3 in `docs/src/architecture/layers.toml:215` but L4 in `contracts/ci/crate-layers.v1.json`. It has unconditional `candle-core`/`candle-nn` dependencies. Under D-09, crate-layers.v1.json defines CORE, which puts this crate outside SC#4's scope. Reconciling the two files, and feature-gating the crate if it is ever ruled CORE, is deferred per CONTEXT.md.
- **vox-populi hub.rs E0063:** see `deferred-items.md`. It currently breaks vox-ml-cli's default build.
- **Scan is a verification command, not a CI gate** (a deliberate exclusion in the plan). If it becomes a gate, it must fail when `cargo metadata` fails (the mutant showed an invalid manifest yields empty output). Any clippy location grep also needs uncolored output.

## Known Stubs

None. With `engine` off, vox-quantize is intentionally empty (every module needs Candle); this is by design (D-07), not a stub.

## Self-Check: PASSED

- `crates/vox-quantize/Cargo.toml` has `candle-core = { workspace = true, optional = true }`, `engine = ["dep:candle-core"]`, and cuda/metal list `"engine"`. `lib.rs` has 13 `#[cfg(feature = "engine")]` and `#[cfg(all(test, feature = "engine"))]`.
- Commits `104ad8472`, `d2d61984b`, `11a0cd489` are on `main`. `git rev-list --count 03cf0e62e..HEAD` = 3 before this SUMMARY commit.
