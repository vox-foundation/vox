# Rust Build Loop and the `vox-orchestrator-models` Crate — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI** for Part B, one task per
> headless run (`/drive-task docs/superpowers/plans/2026-09-29-rust-build-loop-and-models-crate.md <N>`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> **Part A is Claude's own work** (it needs measurement and judgment, not code). The agent never stages or commits;
> Claude re-runs every check, reviews the diff and commits. Code blocks are transcribed exactly.

**Goal:** Shorten the edit, retest and commit loop for the orchestrator crates without dropping any check, and decide by
measurement whether extracting `models/` into its own crate is worth doing.

**Architecture:** Part A measures the loop in CPU-seconds and tries the cheap levers (one feature set per verification run,
a test-profile debug level). Part B is a gated, behaviour-preserving extraction: lower the shared value enums into
`vox-orchestrator-types`, replace `AgentTask` in `models/`'s signatures with a narrow `RoutingTask`, then move `models/` and the
leaf modules it uses into `crates/vox-orchestrator-models`, with `pub use vox_orchestrator_models as models;` in
`vox-orchestrator` so no other crate changes an import.

**Tech Stack:** Rust 2024 workspace, cargo, lefthook, `vox ci crate-edges` and `vox-arch-check`.

**Spec:** [`docs/src/architecture/rust-build-loop-ssot-2026.md`](../../src/architecture/rust-build-loop-ssot-2026.md) (measured
sinks, the coupling table, and the five rules this plan follows). Already done and not repeated here: the pre-commit
`fmt-fix` no longer rebuilds `vox-cli` (`ff25d0aef`).

**Where this runs in the program.** Part A can run at any quiet moment. **Part B starts only after the model-routing plan's
Task 6 is committed**, because that plan's Tasks 4 to 6 rewrite `registry.rs`, `select.rs`, `mode_select.rs` and add
`ranking.rs`, `health.rs`; a move would conflict with every one of them. It also waits on the chat-trace plan's Task 3 (it edits
`models/mod.rs` and `models/provenance.rs`). Parts of the routing plan's GUI tasks (7 to 12) and the notices plan do not touch
`models/` and are unaffected.

## Global Constraints

- **No behaviour change from a move.** Each Part B task ends with the same tests passing under the same names, and every old
  public path (`vox_orchestrator::models::…`, `vox_orchestrator::config::CostPreference`, `vox_orchestrator::types::FreeRoutingProfile`) still resolving.
- **CPU-seconds, not wall-clock:** `bash`'s `time -p` (user + sys) while the machine is loaded.
- **A profile or flag change ships only if** it cuts the measured loop by at least 20% and loses no test, no panic file:line
  and no lint.
- **Toolchain is pinned stable** (`rust-toolchain.toml`): no `-Z` flags, no Cranelift, no parallel front end.
- **No new external crate.** Every dependency of the new crate is one the moved code already uses, declared through
  `[workspace.dependencies]`.
- **New crate edges are user-authorized** (`AGENTS.md`, Dependency Discipline): never edit
  `contracts/ci/crate-edges.allow.v1.json` or the fan-in snapshot to admit an edge; Task B0 stops and asks.
- Test-first: write the tests, run them, save the failing output to `target/bl-t<N>-red.txt` before implementing (Part B).
- Foreground, `timeout`-prefixed commands; `rustfmt --edition 2024 <file>`, never `cargo fmt`.
- **Speed rules for agents:** run only the filtered tests a task names. Claude runs the crate suites and clippy once per batch.
- Existing tests are not edited except where a task names the edit. Any other existing test that breaks is a STOP.

## File Structure

| File | Status | Task | Responsibility |
|---|---|---|---|
| `docs/src/architecture/rust-build-loop-ssot-2026.md` | modify | A1 to A4, B6 | results tables and the go/no-go record |
| `Cargo.toml` (`[profile.test]`) | modify, only if A3 passes | A3 | test-build debug level |
| `crates/vox-orchestrator-types/src/routing_values.rs` (+ `lib.rs`) | create | B1 | `CostPreference`, `RoutingProfile` |
| `crates/vox-orchestrator/src/config/enums.rs`, `types/routing_profile.rs` | modify | B1 | re-export the lowered enums |
| `crates/vox-orchestrator/src/models/routing_task.rs` (+ `mod.rs`) | create | B2 | `RoutingTask`, the narrow view of a task |
| `crates/vox-orchestrator/src/types/tasks.rs` | modify | B2 | `From<&AgentTask> for RoutingTask` |
| `crates/vox-orchestrator/src/models/{registry,mode_select}.rs` | modify | B2 | take `&RoutingTask`, not `&AgentTask` |
| `crates/vox-orchestrator-models/**` | create (moves) | B3, B4 | the extracted crate |
| `crates/vox-orchestrator/src/lib.rs`, `Cargo.toml` | modify | B3, B4 | dependency and the `models` re-export |
| `docs/src/architecture/layers.toml`, `where-things-live.md` | modify | B5 | layer and row for the new crate |

---

## Part A — Measure and try the cheap levers (Claude)

### Task A1: Baseline the edit loop in CPU-seconds

**Files:** Modify `docs/src/architecture/rust-build-loop-ssot-2026.md` (a "Baseline" table).

**Interfaces:** Produces the numbers Task A4 decides on: `T_models` (touch a `models/` file), `T_other` (touch a file outside
`models/`), and the share of each spent in the `vox-orchestrator` test unit.

- [ ] **Step 1: Warm the artifacts.** `timeout 3000s cargo test -p vox-orchestrator --lib --no-run 2>&1 | tail -3`. Expected: `Finished`.
- [ ] **Step 2: Measure one touch of a `models/` file, three times.** Each run appends a comment to
  `crates/vox-orchestrator/src/models/tiering.rs`, times the rebuild, and restores the file from `HEAD`:

```bash
for i in 1 2 3; do
  printf '\n// bench-touch %s\n' "$i" >> crates/vox-orchestrator/src/models/tiering.rs
  { time -p timeout 3000s cargo test -p vox-orchestrator --lib --no-run --timings 2>&1 | tail -1; } 2>> target/bl-a1-models.txt
  git show HEAD:crates/vox-orchestrator/src/models/tiering.rs > crates/vox-orchestrator/src/models/tiering.rs
done
cat target/bl-a1-models.txt
```

  Record `user + sys` for each run and their median as `T_models`. Do not run this while an agent has uncommitted edits in `models/`.
- [ ] **Step 3: Measure one touch of a file outside `models/`** with the same loop on `crates/vox-orchestrator/src/circuit_breaker.rs`,
  output to `target/bl-a1-other.txt`; median as `T_other`.
- [ ] **Step 4: Attribute the time.** The last `--timings` report is `target/cargo-timings/cargo-timing.html`. Print the unit durations:

```bash
python3 - <<'EOF'
import re, json, pathlib
t = pathlib.Path('target/cargo-timings/cargo-timing.html').read_text()
m = re.search(r'const UNIT_DATA = (\[.*?\]);', t, re.S)
units = json.loads(m.group(1))
for u in sorted(units, key=lambda u: -u['duration'])[:8]:
    print(f"{u['duration']:7.1f}s  {u['name']} {u.get('mode','')}")
EOF
```

  Record the top units and `share = duration(vox-orchestrator test unit) / T_models`.
- [ ] **Step 5: Write the table** (`T_models`, `T_other`, `share`, date, load average from `uptime`) into the SSOT under "Baseline",
  then commit that file: message "docs(build): baseline the orchestrator edit loop in CPU-seconds".

### Task A2: One feature set per verification run

**Files:** Modify the SSOT ("Feature unification" table). No repository code changes.

**Interfaces:** Produces `U_sep` and `U_one`: total CPU-seconds of the gates after one touch of `models/tiering.rs`.

- [ ] **Step 1: Separate invocations (today's shape).** After a touch, run and time the three commands in order:
  `cargo test -p vox-orchestrator --lib --no-run`, `cargo test -p vox-orchestrator-mcp --lib --no-run`,
  `cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp --all-targets`. Sum `user + sys` as `U_sep`. Restore the file from `HEAD`.
- [ ] **Step 2: One invocation.** Repeat the touch, then run and time
  `cargo test -p vox-orchestrator -p vox-orchestrator-mcp --lib --no-run` followed by the same clippy command. Sum as `U_one`.
- [ ] **Step 3: Decide.** If `U_one <= 0.8 * U_sep`, adopt the single invocation in the agy driver note and in
  `docs/src/contributors/antigravity-driven-execution.md` ("verify with one invocation over both crates"); otherwise record why not
  (for example: feature unification made one crate compile a heavier feature set) and stop. Record both numbers and the decision in the SSOT.
- [ ] **Step 4: Commit** the SSOT (and the driver doc if changed): "docs(build): one feature set per verification run".

### Task A3: Test-profile debug level

**Files:** Modify `Cargo.toml` (only on a pass) and the SSOT ("Profile experiments" table).

**Interfaces:** Produces `T_models_d0` (Task A1's loop under `debug = 0`) and a yes/no on panic locations.

- [ ] **Step 1: Baseline is Task A1's `T_models`** (`[profile.dev] debug = 1`).
- [ ] **Step 2: Measure with the override.** `CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0` set for the three-touch loop from A1 Step 2
  (the first run also pays the one-time first-party rebuild; drop it and use the median of the next three touches). Record `T_models_d0`.
- [ ] **Step 3: Check that panic locations survive.** Apply a one-line mutant to `tiering.rs` (`>=` to `>` in `derive_tier_with`) so an existing test fails, then run
  `CARGO_PROFILE_TEST_DEBUG=0 cargo test -p vox-orchestrator --lib -- models::tiering 2>&1 | grep -E "panicked at .*tiering.rs:[0-9]+"`.
  Expected: a `file:line` is printed. Restore the file from `HEAD`.
- [ ] **Step 4: Decide.** Adopt only if `T_models_d0 <= 0.8 * T_models` and Step 3 printed a location. To adopt, add to the root `Cargo.toml`, directly
  after `[profile.dev.package."*"]`'s block:

```toml
[profile.test]
# Test builds skip DWARF for first-party crates (measured: docs/src/architecture/rust-build-loop-ssot-2026.md).
# Panic file:line still prints; only debugger stepping and backtrace frames are lost.
debug = 0
```

  Then run `timeout 3000s cargo test -p vox-orchestrator --lib -- models::tiering` once to confirm it builds, record the numbers, and commit
  "build(profile): first-party test builds skip DWARF". If it fails the rule, record the numbers and change nothing.

### Task A4: Go / no-go for the extraction

**Files:** Modify the SSOT ("Decision" section).

- [ ] **Step 1: Apply the rule.** **GO** only if both hold, using Task A1 and A3's numbers:
  1. `T_models >= 150` CPU-seconds (after A3's adoption, if any): the loop is long enough to be worth a refactor;
  2. `share >= 0.6`: the `vox-orchestrator` test unit is at least 60% of that loop, so a crate about a sixth of its size
     (`models/` plus the leaf modules is about 16k of 94k lines) can remove most of it.
- [ ] **Step 2: Record** GO or NO-GO with the two values and the date under "Decision". On NO-GO, stop: Part B is not started, and the
  routing plan's Execution Order note is updated to say so.
- [ ] **Step 3: Commit** "docs(build): extraction decision (GO|NO-GO) with measurements".

---

## Part B — Extract `vox-orchestrator-models` (runs only on GO, after routing Task 6)

### Task B0: Authorization (User) and the proposal (Claude)

- [ ] **Step 1 (Claude):** Draft, without applying, the three registration changes and put them in the plan's PR description or in chat:
  a `layers.toml` row `vox-orchestrator-models = { layer = 3 }`; the new dependency edges (`vox-orchestrator → vox-orchestrator-models`,
  `vox-orchestrator-models → vox-orchestrator-types, vox-config, vox-db, vox-secrets, vox-actor-runtime, vox-telemetry, vox-mesh-transport, vox-mesh-policy, vox-mesh-types, vox-bounded-fs, vox-repository`
  — the exact list comes from Task B4's `cargo check`); and the `where-things-live.md` row.
- [ ] **Step 2 (User):** Approve or refuse the edge-set and fan-in baseline changes. **STOP until approved.** Never regenerate a baseline to admit an edge.

### Task B1: Lower `CostPreference` and `RoutingProfile` into `vox-orchestrator-types`

**Files:** Create `crates/vox-orchestrator-types/src/routing_values.rs`. Modify `crates/vox-orchestrator-types/src/lib.rs`,
`crates/vox-orchestrator/src/config/enums.rs`, `crates/vox-orchestrator/src/types/routing_profile.rs`.

**Interfaces:** Produces `vox_orchestrator_types::{CostPreference, RoutingProfile}` with identical derives, serde names (`snake_case`) and
defaults (`CostPreference::Economy`, `RoutingProfile::Free`), and `RoutingProfile::as_str`. Consumed by Task B4.

- [ ] **Step 1: Write the failing test.** Create `routing_values.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_preference_wire_names_and_default_are_unchanged() {
        assert_eq!(serde_json::to_string(&CostPreference::Economy).unwrap(), "\"economy\"");
        assert_eq!(serde_json::to_string(&CostPreference::Performance).unwrap(), "\"performance\"");
        assert_eq!(CostPreference::default(), CostPreference::Economy);
    }

    #[test]
    fn routing_profile_wire_names_keys_and_default_are_unchanged() {
        for (p, key) in [
            (RoutingProfile::Free, "free"),
            (RoutingProfile::Mixed, "mixed"),
            (RoutingProfile::Performance, "performance"),
            (RoutingProfile::Local, "local"),
        ] {
            assert_eq!(serde_json::to_string(&p).unwrap(), format!("\"{key}\""));
            assert_eq!(p.as_str(), key);
        }
        assert_eq!(RoutingProfile::default(), RoutingProfile::Free);
    }
}
```

  Add `pub mod routing_values;` and `pub use routing_values::{CostPreference, RoutingProfile};` to `lib.rs` so the file compiles.
- [ ] **Step 2: Run to verify failure.** `timeout 1500s cargo test -p vox-orchestrator-types --lib routing_values > target/bl-t1-red.txt 2>&1; tail -12 target/bl-t1-red.txt`.
  Expected: `error[E0412]`/`E0433` (`CostPreference`, `RoutingProfile` missing), not `0 passed`.
- [ ] **Step 3: Move the definitions.** Cut `pub enum CostPreference { … }` (with its doc comments and derives) from `config/enums.rs`, and
  `pub enum RoutingProfile { … }` plus `impl RoutingProfile { as_str … }` (only those, not `config_to_routing_profile`) from
  `types/routing_profile.rs`; paste them above the test module in `routing_values.rs` with `use serde::{Deserialize, Serialize};`. Replace each cut with
  `pub use vox_orchestrator_types::CostPreference;` and `pub use vox_orchestrator_types::RoutingProfile;`. Do not touch the other enums in `enums.rs`.
- [ ] **Step 4: Run to verify pass.** `timeout 1500s cargo test -p vox-orchestrator-types --lib routing_values 2>&1 | tail -6 && timeout 1500s cargo test -p vox-orchestrator --lib -- config::enums types::routing_profile 2>&1 | tail -6`.
- [ ] **Step 5 (Claude): proofs.** Mutants: change `#[default] Economy` to `#[default] Performance` (fails the first test); change `"mixed"` in `as_str` (fails the second).
  Then run `timeout 3000s cargo test -p vox-orchestrator --lib 2>&1 | tail -4` once and `cargo check -p vox-research-shim -p vox-orchestrator-mcp`.
- [ ] **Step 6: Commit** the five files: "refactor(types): lower CostPreference and RoutingProfile into vox-orchestrator-types".

### Task B2: `models/` takes a `RoutingTask`, not an `AgentTask`

**Files:** Create `crates/vox-orchestrator/src/models/routing_task.rs`. Modify `models/mod.rs`, `models/registry.rs`, `models/mode_select.rs`,
`crates/vox-orchestrator/src/types/tasks.rs` (one `impl From`), and the call sites `cargo check` names.

**Interfaces:** Produces `models::RoutingTask` with the fields the ranking code reads (Step 1 lists them by reading the four signatures
`best_for_task`, `best_for_task_with_filter`, `best_for_task_in_mode`, and the `Option<&AgentTask>` parameter), and
`impl From<&AgentTask> for RoutingTask` in `types/tasks.rs`. Consumed by Task B4 (this is what lets `models/` stop depending on `AgentTask`).

- [ ] **Step 1: Inventory.** Run `git ls-files crates/vox-orchestrator/src/models | xargs rg -n "AgentTask|task\.(task_category|estimated_complexity|[a-z_]+)"` and list, in a scratch note, every `AgentTask` field
  the non-test `models/` code reads. If it reads a field that is not a plain `Copy`/`Clone` value type, or reads more than six fields, STOP and report the list.
- [ ] **Step 2: Write the failing tests** in `routing_task.rs` (only a test module): one test that builds an `AgentTask` with a chosen `task_category` and
  `estimated_complexity` and asserts `RoutingTask::from(&task)` carries both, and one that asserts the registry's `best_for_task` returns the same model for a `&AgentTask`
  converted with `.into()` as it did before (use the existing `hard_task()` fixture pattern in `mode_select.rs` tests). Save the red run to `target/bl-t2-red.txt`.
- [ ] **Step 2b: Sanctioned test edit.** The three places in `models/` that build an `AgentTask` (`AgentTask::new` or a literal; find them with
  `rg -n "AgentTask::new|AgentTask \{" crates/vox-orchestrator/src/models`) build a `RoutingTask` directly instead, keeping each test's name and assertions.
- [ ] **Step 3: Implement.** `RoutingTask { pub category: TaskCategory, pub complexity: u8, … the Step 1 fields }` (derive `Debug, Clone, PartialEq`), the `From` impl, and change the four
  signatures to `&RoutingTask` / `Option<&RoutingTask>`. Every caller outside `models/` passes `&RoutingTask::from(&task)`; the compiler lists them.
- [ ] **Step 4: Run to verify pass.** `timeout 1500s cargo test -p vox-orchestrator --lib -- models::routing_task models::mode_select models::tests 2>&1 | tail -8`.
- [ ] **Step 5 (Claude):** the crate suite once; mutants: drop `complexity` in the `From` impl (fails the first test); make `best_for_task` ignore `category` (fails an existing registry test).
- [ ] **Step 6: Commit:** "refactor(models): route by a narrow RoutingTask, not AgentTask".

### Task B3: Create the crate and move the leaf modules that do not use `models`

**Files:** Create `crates/vox-orchestrator-models/Cargo.toml`, `src/lib.rs`. Move (with `git mv`) `crates/vox-orchestrator/src/{usage.rs,usage_policy.rs,calibration.rs}` into it.
Modify `crates/vox-orchestrator/Cargo.toml`, `src/lib.rs`, root `Cargo.toml` (`[workspace.dependencies]` and members, if the workspace lists members explicitly).

**Interfaces:** Produces the crate with `pub mod usage; pub mod usage_policy; pub mod calibration;` and, in `vox-orchestrator`, `pub use vox_orchestrator_models::{usage, usage_policy, calibration};`
so `crate::usage::RemainingBudget` etc. keep resolving. Consumes Task B0's approval (STOP if it is not recorded).

- [ ] **Step 0:** `rg -n "B0.*approved" docs/src/architecture/rust-build-loop-ssot-2026.md` must print the authorization line. Otherwise STOP.
- [ ] **Step 1: Write the failing test** in `crates/vox-orchestrator-models/src/lib.rs`: `#[cfg(test)] mod tests { #[test] fn reexport_paths_resolve() { let _: Option<crate::usage::RemainingBudget> = None; } }`
  (`RemainingBudget` has no constructor, so the test names the type), and an identical assertion in `vox-orchestrator` through the old path. Save the red run.
- [ ] **Step 2: Implement.** `Cargo.toml` (`name = "vox-orchestrator-models"`, `version.workspace = true`, `edition.workspace = true`, `[dependencies]`: only `serde`, `serde_json`, `vox-secrets` for these three modules — copy the exact
  `workspace = true` lines from `vox-orchestrator/Cargo.toml`), `git mv` the three files, fix the one `crate::usage_policy` path, add the dependency and the `pub use` to `vox-orchestrator`.
- [ ] **Step 3: Verify.** `timeout 1500s cargo test -p vox-orchestrator-models 2>&1 | tail -6 && timeout 3000s cargo test -p vox-orchestrator --lib -- usage calibration 2>&1 | tail -6`; the moved tests keep their names.
- [ ] **Step 4 (Claude):** `vox ci crate-edges` reports only the edges B0 approved; the crate suite once; clippy on both crates.
- [ ] **Step 5: Commit:** "refactor(orchestrator): usage, usage_policy and calibration move to vox-orchestrator-models".

### Task B4: Move `models/` and the modules that use it

**Files:** Move `crates/vox-orchestrator/src/models/**`, `route_policy.rs`, `catalog.rs`, `catalog_classifier.rs` into `crates/vox-orchestrator-models/src/` (`models/*` to the crate root modules as `pub mod`s, or keep a `models` module — see Step 1).
Modify `crates/vox-orchestrator-models/Cargo.toml`, `crates/vox-orchestrator/src/lib.rs`.

**Interfaces:** Produces `vox_orchestrator_models::{…}` and, in `vox-orchestrator`, `pub use vox_orchestrator_models::models;` (plus `route_policy`, `catalog`, `catalog_classifier` re-exports), so `vox_orchestrator::models::ModelRegistry` and every path in the Spec's table keep resolving.
Consumes B1 (`CostPreference` from `vox-orchestrator-types`) and B2 (`RoutingTask`).

- [ ] **Step 1: Decide the module shape** (record it in the commit body): keep `models` as one module (`pub mod models;` inside the new crate) so file paths and `super::` references inside `models/` are untouched.
- [ ] **Step 2: Write the failing test** in the new crate: `#[cfg(test)] mod tests { #[test] fn the_registry_is_reachable_through_the_old_path() { let _ = crate::models::ModelRegistry::default(); } }`, and in `vox-orchestrator` a test that names `vox_orchestrator::models::ModelRegistry` and `vox_orchestrator::route_policy::is_local_http_provider`. Save the red run.
- [ ] **Step 3: Move and compile-fix, in this order,** running `timeout 1500s cargo check -p vox-orchestrator-models 2>&1 | tail -30` after each: (1) `git mv` the files; (2) add dependencies exactly as the errors name them
  (the crate list in Task B0 Step 1), each as `workspace = true`; (3) mirror the three features `models/` uses — `populi-transport`, `runtime`, `test-support` — in the new crate's `[features]` and forward them from `vox-orchestrator`'s features
  (`populi-transport = ["vox-orchestrator-models/populi-transport", …existing]`); (4) replace `crate::config::CostPreference`/`crate::types::FreeRoutingProfile` with the `vox_orchestrator_types` paths and `crate::mode::ClutchProfile` per the Step 4 rule.
- [ ] **Step 4: `ClutchProfile`.** `mode_select.rs` reads only `excludes_elite(clutch)`. Replace its `clutch: ClutchProfile` parameter with `exclude_elite: bool`, and have the (single) orchestrator caller compute
  `matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)`. If the post-Task-6 file reads more from the clutch than that, STOP and report what.
- [ ] **Step 5: Anything else that still needs the old crate.** If `cargo check` names a `crate::` item outside the Spec's coupling table, STOP and report it; do not lower or copy it.
- [ ] **Step 6: Verify.** `timeout 3000s cargo test -p vox-orchestrator-models 2>&1 | tail -6`; the old test count for the moved tests must be unchanged (compare with `git grep -c "#\[test\]"` before and after).
- [ ] **Step 7 (Claude):** the full suites for `vox-orchestrator-models`, `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-research-shim`; `cargo check -p vox-cli -p vox-gui` (if `vox-gui` fails inside `tauri-build`, that is the known fresh-worktree issue in `AGENTS.md`: note it and continue); clippy on all four; `vox ci crate-edges`; `cargo run -q -p vox-arch-check`.
  Mutants: delete one re-export (fails the old-path test); flip `exclude_elite` (fails `efficiency_on_a_hard_task_never_picks_elite`).
- [ ] **Step 8: Commit:** "refactor(models): extract vox-orchestrator-models; vox-orchestrator re-exports it".

### Task B5: Register the crate

**Files:** Modify `docs/src/architecture/layers.toml`, `docs/src/architecture/where-things-live.md`.

- [ ] **Step 1:** Add `vox-orchestrator-models = { layer = 3 }` next to `vox-orchestrator`, and a `where-things-live.md` row "Model registry, selection, scoring, provenance | `crates/vox-orchestrator-models/`".
  Run `cargo run -q -p vox-arch-check` and `cargo run -q -p vox-cli -- ci wtl-parity` (or the parity command named in `AGENTS.md`); both must pass.
- [ ] **Step 2 (User applies):** the edge-set and fan-in baseline changes Task B0 was approved for; Claude never writes them.
- [ ] **Step 3: Commit** the two docs.

### Task B6: Re-measure and keep or revert (Claude)

- [ ] **Step 1:** Repeat Task A1 Steps 2 and 3 with the touch target `crates/vox-orchestrator-models/src/models/tiering.rs` and with `crates/vox-orchestrator/src/circuit_breaker.rs`. Record `T_models_after`, `T_other_after`.
- [ ] **Step 2: Rule.** Keep only if `T_models_after <= 0.5 * T_models` and `T_other_after <= 1.2 * T_other`. Otherwise revert B4 to B1 (`git revert`, one commit per task, newest first) and record why. Write the result and the date into the SSOT.
- [ ] **Step 3: Commit** "docs(build): extraction result (kept|reverted)".

---

## Decisions (resolved 2026-09-29; open decisions delegated to Claude)

1. **Measure first.** No extraction without the Task A4 numbers.
2. **Move `models/` whole, with the leaf modules it uses.** Inherent `impl ModelRegistry` blocks force `registry.rs`, `select.rs`, `mode_select.rs`, `family.rs` and `ranking.rs` into the same crate; `route_policy`, `catalog`, `catalog_classifier`, `usage`, `usage_policy` and `calibration` are small and only need `models` types, so they move with it instead of being lowered further.
3. **`AgentTask` never moves** (its fields reference six orchestrator subsystems); `models/` gets a narrow `RoutingTask`.
4. **Re-export shim, no downstream edits.** About 60 files in eight crates import `vox_orchestrator::models::…`; none changes.
5. **`ClutchProfile` stays where it is;** `models/` takes a `bool`, because it reads one predicate of it.
6. **Revert is one `git revert` per task,** so a failed B6 costs nothing but the time.

## Deferred

- Splitting `vox-orchestrator-mcp` (158 s, the slowest first-party crate).
- Moving `models/tests.rs` into `tests/`: the Test-First Policy requires in-file tests for new public functions.
- Untangling `AgentTask` from `reconstruction`, `socrates`, `attachment_manifest`, `observer`, `contract` and `planning`.

## Execution Order

**Status (2026-10-04): B1 to B3 committed on `work/part-b`; B4 stopped at its gate (two unapproved edges, `TaskCategory` lowering, `AgentTask` fixtures) — see the SSOT's "Part B status".** Earlier status (2026-10-03): Part A measured, decision NO-GO. T_models 37.1 CPU-s against the 150 the rule needs; neither the one-invocation form (1.04x) nor `debug = 0` (1.6%) met its 20% rule, so the repository is unchanged. Part B is not started. Results and method: `docs/src/architecture/rust-build-loop-ssot-2026.md`.

1. **Part A, any quiet moment** (Claude): A1 → A2 → A3 → A4. A2 and A3 change nothing in the repository unless their rule is met.
2. **Part B after the model-routing plan's Task 6 and the chat-trace plan's Task 3 are committed, and only on GO:** B0 (stop for approval) → B1 → B2 → B3 → B4 → B5 → B6.
3. **Shared files with in-flight plans:** `models/registry.rs`, `select.rs`, `mode_select.rs`, `mod.rs` (routing Tasks 4 to 6, trace Tasks 2b and 3) and `types/tasks.rs` (nothing else edits it). Part B waits for all of them.
4. **Agents:** B1 to B4 run through agy, one task per run, each followed by Claude's suite, clippy and mutation proofs. B4 is the only large one; if it STOPs, Claude splits it by module and re-drives.
