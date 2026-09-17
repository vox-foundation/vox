# True Workflow Durability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Supersedes:** `docs/superpowers/plans/2026-09-05-true-workflow-durability.md` — do not implement that file. Several of its tasks are unimplementable or would ship a green suite that does not prove durability.
>
> **Spec:** `docs/src/architecture/true-workflow-durability-design-2026.md` (audit-corrected 2026-09-06). Findings G1–G27 in that spec are binding.

**Goal:** Make Vox workflow replay real — execute the workflow body in the interpreter, intercept activity and host-builtin calls, persist real results, and survive crashes, timers, and signals.

**Architecture:** The workflow body runs in the tree-walking interpreter on a dedicated OS thread (`Interpreter` is `!Send` because `VoxValue` and `Scope` hold `Rc`). Durability primitives cross a JSON channel as `HostRequest` / `HostDecision`. The async runner owns journal I/O via `WorkflowTracker`. Parking abandons the thread; resume replays from the top. The compile-time linearizer in `workflow/plan.rs` is deleted after the new runner is green, not extended. Do not Arc-ify `VoxValue` (`eval_cow_semantics_test.rs` tripwire).

**Tech Stack:** Rust 2024, `vox-compiler` (HIR + `eval::Interpreter`), `vox-workflow-runtime`, `vox-db` (libsql), `vox-codegen`, tokio, serde_json, blake3.

## Global Constraints

- **Format with `vox run scripts/fmt.vox`**, never `cargo fmt --all` (Windows `CreateProcess` limit). Check-only: `VOX_FMT_CHECK=1 vox run scripts/fmt.vox`.
- **Before every push:** `vox ci pre-push --complete` (the default fast tier does not run clippy or tests). Never use GitHub Actions as the feedback loop.
- **No new workspace crate edges.** `vox-workflow-runtime` already depends on `vox-compiler`, `vox-db`, `vox-populi`. Adding an edge requires a USER-AUTHORIZED ledger entry in `contracts/ci/crate-edges.allow.v1.json` — propose, do not write.
- **Layer budgets:** `vox-compiler` is already ~52k LoC vs its 45k budget — stay in-crate, do not add a crate. `vox-workflow-runtime` is **L4** in `contracts/ci/crate-layers.v1.json` (the 2026-09-05 plan incorrectly called it L3).
- **Test-first is binding.** Every new `pub fn` in `crates/*/src/**` needs a test in the same file before the commit lands (`skeleton/untested-pub-api`). The failing test is step 1 of every task below.
- **Any new `.md` under `docs/src/` needs frontmatter** (`title`, `description`, `category`) written at file-creation time.
- **Verify guards by mutation.** Break the guard, confirm the test fails, restore. A test that passes against both the fixed and unfixed code is worthless.
- **Journal contract:** events carry `journal_version = 1`. Adding event *names* is backward-compatible. Admit a name in the same PR as the first emitter — do not pre-admit Phase 3–4 names in Phase 0.
- **New tables / columns:** edit `crates/vox-db/src/schema/domains/execution.rs`, bump `BASELINE_VERSION` in `crates/vox-db/src/schema/manifest.rs`, keep `contracts/db/baseline-version-policy.yaml` in sync, add a retention row if a new table, run `vox ci data-storage-guard`.
- **New `VOX_*` env vars:** add a row to `contracts/config/env-vars.v1.yaml` in the same PR.
- **`workflow_signal_log` already exists.** Do not recreate it.
- **Do not add `Co-Authored-By: Claude Opus 5`.** Use a normal conventional commit with no false trailer.

---

## File Structure

**Created**

| Path | Responsibility |
|---|---|
| `crates/vox-compiler/src/eval/activity_hook.rs` | `HostCall` / `HostDecision` and interpreter-side dispatch. Keeps `expr.rs` from growing. |
| `crates/vox-workflow-runtime/src/workflow/host.rs` | `!Send` bridge: OS thread, JSON channels, `Drop` join/abort. |
| `crates/vox-workflow-runtime/src/workflow/waker.rs` | Parked-run waker (timers + signals), wall-clock remaining-time clamp. |
| `crates/vox-workflow-runtime/src/workflow/counting_tracker.rs` | Test tracker that faults after N tracker calls. |
| `crates/vox-workflow-runtime/tests/journal_schema_conformance.rs` | Real-journal schema validation. |
| `crates/vox-workflow-runtime/tests/real_activity_execution.rs` | Activity bodies actually run; `?` sees `Result::Err`. |
| `crates/vox-workflow-runtime/tests/crash_windows.rs` | Kill-after-completed / kill-after-started with expired lease. |
| `crates/vox-workflow-runtime/tests/runtime_branching.rs` | `if charge.ok`, `match`, `while` over a journalled value. |
| `crates/vox-workflow-runtime/tests/durable_timer.rs` | Park-and-wake via backdated `wake_at_ms`. |
| `crates/vox-workflow-runtime/tests/durable_signal.rs` | Park-on-missing-signal; late signal after park. |
| `crates/vox-codegen/tests/adr021_parity.rs` | Emit → compile/run generated workflow fn vs interp on the same tracker. |
| `examples/golden/durable_workflow_branching.vox` | Golden covering runtime branch + loop + `?`. |

**Modified**

| Path | Change |
|---|---|
| `crates/vox-compiler/src/eval/value.rs` | Strict `to_journal_json` / `from_journal_json` (`Result`, not `Null` on unknown). |
| `crates/vox-compiler/src/eval/mod.rs` | Hook fields; `EvalError::WorkflowParked`; `with_options` stack. |
| `crates/vox-compiler/src/eval/expr.rs` | Intercept after callee eval; evaluate `With` and `WorkflowVersion`. |
| `crates/vox-compiler/tests/eval_cow_semantics_test.rs` | Add `assert_not_impl_any!(Interpreter: Send, Sync)`. |
| `crates/vox-workflow-runtime/src/workflow/run.rs` | Host request loop; `args` param; persist `args_json`; call `handle_workflow_patch`. |
| `crates/vox-workflow-runtime/src/workflow/return_extract.rs` | Decode `__vox` envelope before `from_value`. |
| `crates/vox-workflow-runtime/src/workflow/plan.rs` | Linearizer deleted in Task 1.5; keep parse helpers. |
| `crates/vox-workflow-runtime/src/db_tracker.rs` | Park clears lease; steal expired lease; consume-or-park; persist `args_json`. |
| `crates/vox-db/src/schema/domains/execution.rs` | `args_json`, later `wake_at_ms` + `parked_reason`; `workflow_patch_log` in Phase 5. |
| `crates/vox-codegen/src/codegen_rust/emit/durability_lower.rs` | `VoxDbTracker` + `features = ["sql"]`; drop `journal::execute`; emit `__vox_run_workflow`. |
| `contracts/workflow/workflow-journal.v1.schema.json` | Admit only names the runner emits in that PR. |
| `contracts/config/env-vars.v1.yaml` | `VOX_WORKFLOW_RUN_ID` in Phase 6. |
| ADRs 019 / 021 / 041 | Interp `if`/`match`/`for`/`while` in-scope; codegen stays Preview until Phase 6. |

---

## Phase 0 — Honesty and contracts

### Task 0.1: Journal schema admits only the events actually emitted

`run.rs` emits `WorkflowPatch` and `ActivityCacheHit`. Neither is in the v1 schema's `event` enum. Do **not** pre-admit `ActivityParked`, `TimerScheduled`, `SignalAwaited`, or `WorkflowParked` — those names land in the same PR as their first emitter (Phases 3–4).

The conformance test must call **today's 3-argument** `interpret_workflow_durable(hir, name, tracker)`. The `args` parameter arrives in Task 1.4; do not invent it here.

**Files:**
- Modify: `contracts/workflow/workflow-journal.v1.schema.json`
- Test: `crates/vox-workflow-runtime/tests/journal_schema_conformance.rs` (create)

**Interfaces:**
- Consumes: today's `pub async fn interpret_workflow_durable(hir: &HirModule, workflow_name: &str, tracker: &mut impl WorkflowTracker) -> anyhow::Result<Vec<Value>>`.
- Produces: schema enum including `WorkflowPatch` and `ActivityCacheHit` only (plus the existing names).

- [ ] **Step 1: Write the failing test**

Create `crates/vox-workflow-runtime/tests/journal_schema_conformance.rs`:

```rust
#![allow(missing_docs)]
//! Every event the runner ACTUALLY emits must validate against the v1 schema.

use jsonschema::validator_for;
use serde_json::Value;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;

const SRC: &str = r#"
activity charge_card(amount: int) to Result[str] {
    return Ok("tx")
}

workflow checkout(amount: int) to Result[str] {
    workflow.version("add-audit-v2", 1, 2)
    let tx = charge_card(amount)?
    return Ok(tx)
}
"#;

#[tokio::test]
async fn every_emitted_event_validates_against_v1_schema() {
    let module = parse(lex(SRC)).expect("parses");
    let hir = lower_module(&module);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut tracker = VoxDbTracker::new(db, "schema-conformance-1");

    let journal = interpret_workflow_durable(&hir, "checkout", &mut tracker)
        .await
        .expect("workflow runs");

    let schema: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/workflow/workflow-journal.v1.schema.json"
    )))
    .expect("schema parses");
    let validator = validator_for(&schema).expect("schema compiles");

    for entry in &journal {
        if let Err(err) = validator.validate(entry) {
            panic!("runner emitted an event the v1 schema rejects: {err}\nentry={entry}");
        }
    }
    assert!(
        journal
            .iter()
            .any(|e| e["event"].as_str() == Some("WorkflowPatch")),
        "test must actually exercise the WorkflowPatch path; got {journal:#?}"
    );
}
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p vox-workflow-runtime --test journal_schema_conformance
```

Expected: FAIL — `runner emitted an event the v1 schema rejects` naming `WorkflowPatch`.

- [ ] **Step 3: Add only the two missing names**

In `contracts/workflow/workflow-journal.v1.schema.json`, extend `properties.event.enum` after `"MeshActivitySkipped"`:

```json
        "WorkflowPatch",
        "ActivityCacheHit"
```

Do not add `ActivityParked`, `TimerScheduled`, `SignalAwaited`, or `WorkflowParked`.

- [ ] **Step 4: Run it and watch it pass**

```bash
cargo test -p vox-workflow-runtime --test journal_schema_conformance
```

Expected: PASS.

- [ ] **Step 5: Mutation-verify the guard**

Temporarily remove `"WorkflowPatch"` from the enum, re-run, confirm FAIL, restore, confirm PASS.

- [ ] **Step 6: Commit**

```bash
cargo run -q -p vox-cli -- ci contracts-index
vox run scripts/fmt.vox
git add contracts/workflow/workflow-journal.v1.schema.json crates/vox-workflow-runtime/tests/journal_schema_conformance.rs contracts/index.yaml
git commit -m "$(cat <<'EOF'
fix(workflow): admit WorkflowPatch and ActivityCacheHit into the v1 journal schema

The runner has emitted both since P2-T2/P2-T5 but the schema enum rejected
them. Admit only names the runner emits today; later park/timer events land
with their first emitter.
EOF
)"
```

---

### Task 0.2: Documentation and ADRs stop over-claiming

Until Phase 1 lands, every doc that implies activity bodies run is wrong. ADR-019/021/041 still list runtime `match` / unbounded loops as non-goals of the linearizer-era subset; this plan moves interp control flow in-scope.

**Files:**
- Modify: `docs/src/explanation/expl-durable-execution.md`
- Modify: `docs/src/tutorials/tut-workflow-durability.md`
- Modify: `docs/src/architecture/durability-runtime-audit-2026.md`
- Modify: `docs/src/architecture/research-index.md` (if the parse-only row is still present)
- Modify: `docs/src/adr/019-durable-workflow-journal-contract-v1.md`
- Modify: `docs/src/adr/021-generated-workflow-durability-parity.md`
- Modify: `docs/src/adr/041-durable-functions-completion-2026.md`

**Interfaces:**
- Consumes: `docs/src/architecture/true-workflow-durability-design-2026.md`.
- Produces: nothing code-facing.

- [ ] **Step 1: Banner the explanation**

Insert immediately after the H1 of `docs/src/explanation/expl-durable-execution.md`:

```markdown
> [!WARNING]
> **Known limits as of 2026-09-06.** The interpreted runner does not yet execute
> local activity bodies — a non-mesh activity records a fixed
> `{"event":"LocalActivity","status":"executed"}` payload instead of your
> function's return value. `workflow_wait` sleeps in-process rather than
> scheduling a durable wake. `workflow_wait_signal` fails the run when the
> signal is absent instead of parking it. `HirExpr::With` and
> `HirExpr::WorkflowVersion` error in the interpreter (`PARITY_UNIMPLEMENTED`).
> Workflows built with `vox build` use an in-memory tracker, so a crash
> replays from zero. Tracked in
> [True workflow durability](../architecture/true-workflow-durability-design-2026.md);
> remove this banner as each phase lands.
```

- [ ] **Step 2: Correct the Recovery-via-Replay list**

Replace the step-4 line so it says the replayed payload is the runtime's own step record, not the activity's return value (see the banner).

- [ ] **Step 3: Banner the tutorial**

Insert the identical `> [!WARNING]` block after the H1 of `docs/src/tutorials/tut-workflow-durability.md`, adjusting the relative link to `../architecture/true-workflow-durability-design-2026.md`.

- [ ] **Step 4: Mark the 2026-05-01 audit superseded**

Insert after the H1 of `docs/src/architecture/durability-runtime-audit-2026.md`:

```markdown
> [!NOTE]
> **Superseded (2026-09-06).** This audit describes the tree as of 2026-05-01 and
> states there is no journal. There is one, and ADR-019 froze it. Local activity
> bodies are still stubs. Current design:
> [True workflow durability](true-workflow-durability-design-2026.md).
```

- [ ] **Step 5: Amend ADR-019 §6 non-goals**

Replace the first non-goal bullet:

```markdown
- no unrestricted branch/loop decision replay in the **generated-Rust** subset
  (`match`, unbounded loops). Interpreted-runtime `if` / `match` / `for` /
  `while` are in scope under the existing determinism lint (no `time.now` /
  `random` / `uuid` in workflow bodies). See
  [true-workflow-durability-design-2026.md](../architecture/true-workflow-durability-design-2026.md).
```

- [ ] **Step 6: Amend ADR-021 supported subset**

After the "Supported subset for initial parity" list, add:

```markdown
**Amended 2026-09-06.** Interpreted-runtime control flow (`if` / `match` /
`for` / `while` over journalled values) is in scope for the body-first
engine. Generated-Rust *emission* may stay linear until the Phase 6 compile
gate in `docs/superpowers/plans/2026-09-06-true-workflow-durability.md` lands.
The compatibility gate remains: emit → compile/run the generated workflow fn
against the same tracker/`run_id` as the interpreter; do not call
`interpret_workflow_durable` twice and call that parity.
```

- [ ] **Step 7: Amend ADR-041 Decision §2–3**

Replace Decision items 2 and 3 with:

```markdown
2. The durable **interpreted** runtime is **Preview** until
   [true-workflow-durability-design-2026.md](../architecture/true-workflow-durability-design-2026.md)
   Phase 1 lands (activity bodies actually run). After Phase 1 it is Stable
   for the body-first subset. Generated-Rust durability remains **Preview**
   until Phase 6 emits `VoxDbTracker`, `__vox_run_workflow`, and the real
   ADR-021 compile gate. Existing crash-replay tests prove tracker skip of
   canned rows, not user-body execution.
3. Out-of-subset for **generated** Rust may stay linear. Interpreted
   `match` / loops are in-scope under the determinism lint. Mesh /
   `PopuliActivity` is out of the 2026-09-06 body-first plan (loud
   unsupported error, not a silent drop).
```

- [ ] **Step 8: Confirm research-index**

In `docs/src/architecture/research-index.md`, the durability-audit bullet must **not** say parse-only / ADR-028. If it still does, replace it with the wording in that file's 2026-09-06 row for `true-workflow-durability-design-2026.md`.

- [ ] **Step 9: Lint and commit**

```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/explanation/expl-durable-execution.md docs/src/tutorials/tut-workflow-durability.md docs/src/architecture/durability-runtime-audit-2026.md docs/src/architecture/true-workflow-durability-design-2026.md docs/src/adr/019-durable-workflow-journal-contract-v1.md docs/src/adr/021-generated-workflow-durability-parity.md docs/src/adr/041-durable-functions-completion-2026.md
```

Expected: PASS.

```bash
git add docs/
git commit -m "$(cat <<'EOF'
docs(durability): state the actual limits and amend ADR-019/021/041

Local activity bodies do not run, timers sleep in-process, and generated
workflows use an in-memory tracker. Interp control flow is now in-scope;
codegen durability stays Preview until the body-first engine ships.
EOF
)"
```

---

## Phase 1 — Body executes (load-bearing)

Phases 2–6 are meaningless before this: they would park and replay a canned string.

### Task 1.1: Lossless `VoxValue` ↔ JSON with strict decode

`eval/builtins.rs` already has `vox_to_json` / `json_to_vox`, but they are private and **lossy**. Do not change those functions — `json.encode` depends on their plain mapping. Add a separate tagged encoding. **Unknown `__vox` envelopes must error**, not decode to `Null` (G20).

**Files:**
- Modify: `crates/vox-compiler/src/eval/value.rs`
- Modify: `crates/vox-compiler/tests/eval_cow_semantics_test.rs`
- Modify: `crates/vox-workflow-runtime/src/workflow/return_extract.rs`
- Test: `crates/vox-compiler/src/eval/value.rs` (`#[cfg(test)] mod journal_json_tests`)
- Test: `crates/vox-workflow-runtime/src/workflow/return_extract.rs` (existing test module or new)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `VoxValue::to_journal_json(&self) -> serde_json::Value`
  - `VoxValue::from_journal_json(v: &serde_json::Value) -> Result<VoxValue, JournalDecodeError>`
  - `pub struct JournalDecodeError { pub message: String }` with `Display` + `Error`
  - `extract_terminal_return` decodes a `__vox` envelope into serde-facing JSON before `from_value`

- [ ] **Step 1: Write the failing round-trip tests**

Append to `crates/vox-compiler/src/eval/value.rs`:

```rust
#[cfg(test)]
mod journal_json_tests {
    use super::*;

    fn roundtrip(v: VoxValue) -> VoxValue {
        VoxValue::from_journal_json(&v.to_journal_json()).expect("roundtrip")
    }

    #[test]
    fn result_ok_survives_roundtrip() {
        let v = VoxValue::Result(Ok(Box::new(VoxValue::Str("tx_5".into()))));
        match roundtrip(v) {
            VoxValue::Result(Ok(inner)) => assert!(matches!(*inner, VoxValue::Str(ref s) if s == "tx_5")),
            other => panic!("expected Result(Ok(Str)), got {other:?}"),
        }
    }

    #[test]
    fn result_err_survives_roundtrip() {
        let v = VoxValue::Result(Err(Box::new(VoxValue::Str("declined".into()))));
        match roundtrip(v) {
            VoxValue::Result(Err(inner)) => {
                assert!(matches!(*inner, VoxValue::Str(ref s) if s == "declined"))
            }
            other => panic!("expected Result(Err(Str)), got {other:?}"),
        }
    }

    #[test]
    fn option_none_is_distinguishable_from_null() {
        assert!(matches!(roundtrip(VoxValue::Option(None)), VoxValue::Option(None)));
        assert!(matches!(roundtrip(VoxValue::Null), VoxValue::Null));
    }

    #[test]
    fn tagged_adt_survives_roundtrip() {
        let v = VoxValue::Tagged {
            name: "Declined".into(),
            fields: vec![VoxValue::Int(402)],
        };
        match roundtrip(v) {
            VoxValue::Tagged { name, fields } => {
                assert_eq!(name, "Declined");
                assert!(matches!(fields.as_slice(), [VoxValue::Int(402)]));
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
    }

    #[test]
    fn user_object_with_vox_key_is_escaped() {
        let v = VoxValue::object(vec![("__vox".into(), VoxValue::Str("Ok".into()))]);
        match roundtrip(v) {
            VoxValue::Object(fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].0, "__vox");
            }
            other => panic!("expected Object, got {other:?}"),
        }
    }

    #[test]
    fn plain_scalars_and_lists_roundtrip() {
        assert!(matches!(roundtrip(VoxValue::Int(7)), VoxValue::Int(7)));
        assert!(matches!(roundtrip(VoxValue::Bool(true)), VoxValue::Bool(true)));
        match roundtrip(VoxValue::list(vec![VoxValue::Int(1), VoxValue::Int(2)])) {
            VoxValue::List(items) => assert_eq!(items.len(), 2),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn unknown_envelope_is_an_error() {
        let raw = serde_json::json!({"__vox": "FromTheFuture", "value": 1});
        let err = VoxValue::from_journal_json(&raw).expect_err("unknown envelope must fail");
        assert!(
            err.message.contains("FromTheFuture"),
            "error must name the unknown kind; got {err}"
        );
    }

    #[test]
    fn unjournalable_kind_is_an_error_on_decode() {
        let raw = serde_json::json!({"__vox": "Unjournalable", "kind": "Fn"});
        assert!(VoxValue::from_journal_json(&raw).is_err());
    }
}
```

Append to `crates/vox-compiler/tests/eval_cow_semantics_test.rs` immediately after the existing `VoxValue` tripwire:

```rust
static_assertions::assert_not_impl_any!(vox_compiler::eval::Interpreter: Send, Sync);
```

Add this test to `crates/vox-workflow-runtime/src/workflow/return_extract.rs` (create `#[cfg(test)] mod tests` if missing):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_ok_envelope_as_rust_result() {
        let journal = vec![json!({
            "event": "WorkflowCompleted",
            "return_value": {"__vox": "Ok", "value": "tx"}
        })];
        let got: Result<String, String> = extract_terminal_return(&journal).expect("extracts");
        assert_eq!(got, Ok("tx".into()));
    }

    #[test]
    fn extracts_err_envelope_as_rust_result() {
        let journal = vec![json!({
            "event": "WorkflowCompleted",
            "return_value": {"__vox": "Err", "value": "declined"}
        })];
        let got: Result<String, String> = extract_terminal_return(&journal).expect("extracts");
        assert_eq!(got, Err("declined".into()));
    }
}
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cargo test -p vox-compiler --lib eval::value::journal_json_tests
cargo test -p vox-workflow-runtime --lib workflow::return_extract
```

Expected: FAIL — `no method named to_journal_json`; extract type-mismatch on `__vox`.

- [ ] **Step 3: Implement the tagged encoding**

Add next to `VoxValue` in `value.rs`:

```rust
/// Why a journal envelope could not be decoded.
#[derive(Debug, Clone)]
pub struct JournalDecodeError {
    /// Human-readable reason (unknown kind, malformed fields, unjournalable).
    pub message: String,
}

impl std::fmt::Display for JournalDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "journal decode error: {}", self.message)
    }
}

impl std::error::Error for JournalDecodeError {}
```

Append to `impl VoxValue`:

```rust
    /// Encode this value for the durable workflow journal.
    ///
    /// Unlike `json.encode` (`eval::builtins::vox_to_json`, deliberately
    /// lossy), this encoding is round-trippable. Objects that contain a
    /// `"__vox"` key are escaped. Unjournalable kinds (`Fn`, `Regex`,
    /// sentinels) encode as `{"__vox":"Unjournalable",...}` so decode fails
    /// loudly rather than substituting `Null`.
    #[must_use]
    pub fn to_journal_json(&self) -> serde_json::Value {
        use serde_json::{Value as J, json};
        match self {
            VoxValue::Int(n) => json!(n),
            VoxValue::Float(f) => json!(f),
            VoxValue::Str(s) => json!(s),
            VoxValue::Bool(b) => json!(b),
            VoxValue::Null => J::Null,
            VoxValue::Decimal(d) => json!({"__vox": "Decimal", "value": d.to_string()}),
            VoxValue::List(items) => {
                J::Array(items.iter().map(VoxValue::to_journal_json).collect())
            }
            VoxValue::Tuple(items) => json!({
                "__vox": "Tuple",
                "items": items.iter().map(VoxValue::to_journal_json).collect::<Vec<_>>(),
            }),
            VoxValue::Object(fields) => {
                let escaped = fields.iter().any(|(k, _)| k == "__vox");
                let map: serde_json::Map<String, J> = fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_journal_json()))
                    .collect();
                if escaped {
                    json!({"__vox": "Object", "fields": J::Object(map)})
                } else {
                    J::Object(map)
                }
            }
            VoxValue::Option(Some(v)) => json!({"__vox": "Some", "value": v.to_journal_json()}),
            VoxValue::Option(None) => json!({"__vox": "None"}),
            VoxValue::Result(Ok(v)) => json!({"__vox": "Ok", "value": v.to_journal_json()}),
            VoxValue::Result(Err(e)) => json!({"__vox": "Err", "value": e.to_journal_json()}),
            VoxValue::Tagged { name, fields } => json!({
                "__vox": "Tagged",
                "name": name,
                "fields": fields.iter().map(VoxValue::to_journal_json).collect::<Vec<_>>(),
            }),
            other => json!({"__vox": "Unjournalable", "kind": format!("{other:?}")}),
        }
    }

    /// Decode a value produced by [`VoxValue::to_journal_json`].
    ///
    /// Unknown or `Unjournalable` envelopes return [`Err`] — silent `Null`
    /// is replay corruption.
    pub fn from_journal_json(value: &serde_json::Value) -> Result<VoxValue, JournalDecodeError> {
        use serde_json::Value as J;
        let fail = |message: String| Err(JournalDecodeError { message });
        match value {
            J::Null => Ok(VoxValue::Null),
            J::Bool(b) => Ok(VoxValue::Bool(*b)),
            J::Number(n) => n
                .as_i64()
                .map(VoxValue::Int)
                .or_else(|| n.as_f64().map(VoxValue::Float))
                .ok_or_else(|| JournalDecodeError {
                    message: format!("unrepresentable number: {n}"),
                }),
            J::String(s) => Ok(VoxValue::Str(s.clone())),
            J::Array(items) => Ok(VoxValue::list(
                items
                    .iter()
                    .map(VoxValue::from_journal_json)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            J::Object(map) => match map.get("__vox").and_then(J::as_str) {
                Some("Decimal") => map
                    .get("value")
                    .and_then(J::as_str)
                    .and_then(|s| s.parse().ok())
                    .map(VoxValue::Decimal)
                    .ok_or_else(|| JournalDecodeError {
                        message: "malformed Decimal envelope".into(),
                    }),
                Some("Tuple") => {
                    let items = map.get("items").and_then(J::as_array).ok_or_else(|| {
                        JournalDecodeError {
                            message: "malformed Tuple envelope".into(),
                        }
                    })?;
                    Ok(VoxValue::tuple(
                        items
                            .iter()
                            .map(VoxValue::from_journal_json)
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                }
                Some("Object") => match map.get("fields") {
                    Some(J::Object(inner)) => Ok(VoxValue::object(
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.clone(), VoxValue::from_journal_json(v)?)))
                            .collect::<Result<Vec<_>, JournalDecodeError>>()?,
                    )),
                    _ => fail("malformed Object envelope".into()),
                },
                Some("Some") => Ok(VoxValue::Option(Some(Box::new(VoxValue::from_journal_json(
                    map.get("value").unwrap_or(&J::Null),
                )?)))),
                Some("None") => Ok(VoxValue::Option(None)),
                Some("Ok") => Ok(VoxValue::Result(Ok(Box::new(VoxValue::from_journal_json(
                    map.get("value").unwrap_or(&J::Null),
                )?)))),
                Some("Err") => Ok(VoxValue::Result(Err(Box::new(VoxValue::from_journal_json(
                    map.get("value").unwrap_or(&J::Null),
                )?)))),
                Some("Tagged") => Ok(VoxValue::Tagged {
                    name: map
                        .get("name")
                        .and_then(J::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    fields: map
                        .get("fields")
                        .and_then(J::as_array)
                        .ok_or_else(|| JournalDecodeError {
                            message: "malformed Tagged envelope".into(),
                        })?
                        .iter()
                        .map(VoxValue::from_journal_json)
                        .collect::<Result<Vec<_>, _>>()?,
                }),
                Some("Unjournalable") => fail(format!(
                    "unjournalable value: {}",
                    map.get("kind").and_then(J::as_str).unwrap_or("?")
                )),
                Some(other) => fail(format!("unknown journal envelope kind: {other}")),
                None => Ok(VoxValue::object(
                    map.iter()
                        .map(|(k, v)| Ok((k.clone(), VoxValue::from_journal_json(v)?)))
                        .collect::<Result<Vec<_>, JournalDecodeError>>()?,
                )),
            },
        }
    }
```

In `return_extract.rs`, replace the `from_value` line with a decode helper in the same file:

```rust
/// Map a journal-encoded `return_value` into JSON that `T: DeserializeOwned`
/// can read. `{"__vox":"Ok","value":"tx"}` becomes the serde shape of
/// `Ok("tx")` (`{"Ok":"tx"}` for a Rust `Result`).
pub fn journal_value_to_serde(value: &Value) -> Result<Value, ExtractError> {
    match value.get("__vox").and_then(Value::as_str) {
        Some("Ok") => Ok(serde_json::json!({"Ok": journal_value_to_serde(
            value.get("value").unwrap_or(&Value::Null)
        )?})),
        Some("Err") => Ok(serde_json::json!({"Err": journal_value_to_serde(
            value.get("value").unwrap_or(&Value::Null)
        )?})),
        Some("Some") => Ok(journal_value_to_serde(
            value.get("value").unwrap_or(&Value::Null),
        )?),
        Some("None") => Ok(Value::Null),
        Some("Decimal") => Ok(value.get("value").cloned().unwrap_or(Value::Null)),
        Some("Tuple") => Ok(value.get("items").cloned().unwrap_or_else(|| serde_json::json!([]))),
        Some("Object") => Ok(value.get("fields").cloned().unwrap_or_else(|| serde_json::json!({}))),
        Some("Tagged") => Ok(value.clone()),
        Some(other) => Err(ExtractError::TypeMismatch(serde::de::Error::custom(format!(
            "cannot project journal envelope {other} into serde T"
        )))),
        None => Ok(value.clone()),
    }
}

pub fn extract_terminal_return<T: DeserializeOwned>(journal: &[Value]) -> Result<T, ExtractError> {
    let completed = journal
        .iter()
        .rev()
        .find(|e| e.get("event").and_then(Value::as_str) == Some("WorkflowCompleted"))
        .ok_or(ExtractError::NoTerminal)?;
    let return_value = completed
        .get("return_value")
        .ok_or(ExtractError::MissingReturnValue)?;
    let serde_facing = journal_value_to_serde(return_value)?;
    Ok(serde_json::from_value(serde_facing)?)
}
```

`ExtractError::TypeMismatch` currently wraps `serde_json::Error`. If `serde::de::Error::custom` is awkward, add a new variant `UnprojectableEnvelope(String)` instead — do not swallow the envelope as `Null`.

- [ ] **Step 4: Run the tests and watch them pass**

```bash
cargo test -p vox-compiler --lib eval::value::journal_json_tests
cargo test -p vox-compiler --test eval_cow_semantics_test
cargo test -p vox-workflow-runtime --lib workflow::return_extract
```

Expected: PASS (8 journal tests + extract tests). The `Interpreter: Send` assertion is compile-time.

- [ ] **Step 5: Mutation-verify unknown-envelope**

Temporarily map `Some(other)` in `from_journal_json` to `Ok(VoxValue::Null)`. Confirm `unknown_envelope_is_an_error` FAILS. Restore; confirm PASS.

- [ ] **Step 6: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-compiler/src/eval/value.rs crates/vox-compiler/tests/eval_cow_semantics_test.rs crates/vox-workflow-runtime/src/workflow/return_extract.rs
git commit -m "$(cat <<'EOF'
feat(eval): add strict journal encoding for VoxValue

json.encode stays lossy for external consumers. The durable journal needs
Result/Option/Tagged to survive restart, and unknown envelopes must fail
the run rather than decode to Null.
EOF
)"
```

---

### Task 1.2: Intercept surface — activities after callee eval, plus With / Version / waits

An activity-only hook on `Call(Ident)` misses `let f = charge; f(x)`, `workflow.version`, `with { }`, and `workflow_wait` / `workflow_wait_signal` (G14, G23). `HirExpr::With` and `HirExpr::WorkflowVersion` currently **error** (`PARITY_UNIMPLEMENTED`). They must evaluate in this task, before the linearizer is deleted.

There is **no `Retry` decision**. `failed` on `End` means panic / `_Panic` only. `VoxValue::Result(Err(_))` is `failed: false` and flows to `?` (G15).

The hook type does **not** need `+ Send` (it is created on the interpreter thread).

**Files:**
- Create: `crates/vox-compiler/src/eval/activity_hook.rs`
- Modify: `crates/vox-compiler/src/eval/mod.rs`
- Modify: `crates/vox-compiler/src/eval/expr.rs`
- Test: `crates/vox-compiler/src/eval/activity_hook.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `VoxValue::to_journal_json` / `from_journal_json` (Task 1.1).
- Produces:
  - `pub enum HostCall { Begin { name: String, args: Vec<VoxValue>, options: Option<VoxValue> }, End { name: String, result: VoxValue, failed: bool }, Patch { change_id: String, min: u32, max: u32 }, Wait { deadline_ms: i64 }, WaitSignal { key: String } }`
  - `pub enum HostDecision { Replay(VoxValue), Execute, Park(String), Ack }`
  - `pub type ActivityHook = Box<dyn FnMut(HostCall) -> Result<HostDecision, EvalError>>`  *(no `+ Send`)*
  - `Interpreter::activity_names: HashSet<String>`
  - `Interpreter::activity_hook: Option<ActivityHook>`
  - `Interpreter::with_options: Vec<VoxValue>`
  - `EvalError::WorkflowParked(String)`
  - `pub(crate) fn dispatch_activity(...)` and `pub(crate) fn dispatch_host_call(...)`

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-compiler/src/eval/activity_hook.rs` with this test module (types land in step 3):

```rust
//! Durability interception for workflow execution.
//!
//! Three families: activity Begin/End (after callee eval), Patch
//! (`HirExpr::WorkflowVersion`), and host builtins (`workflow_wait` /
//! `workflow_wait_signal`). `with { }` stashes options for the next Begin.

#[cfg(test)]
mod tests {
    use crate::eval::activity_hook::{HostCall, HostDecision};
    use crate::eval::value::VoxValue;
    use crate::eval::Interpreter;
    use crate::hir::lower_module;
    use crate::lexer::cursor::lex;
    use crate::parser::parse;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn interp_for(src: &str) -> (Interpreter, std::collections::HashSet<String>) {
        let hir = lower_module(&parse(lex(src)).expect("parses"));
        let activities = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(crate::hir::nodes::DurabilityKind::Activity))
            .map(|f| f.name.clone())
            .collect();
        let mut interp = Interpreter::new(1_000_000);
        interp.run_module(&hir).expect("module loads");
        (interp, activities)
    }

    #[test]
    fn hook_observes_every_activity_call() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int {
    let a = charge(amount)
    let b = charge(a)
    return b
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let seen_c = seen.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Begin { name, .. } = &call {
                seen_c.borrow_mut().push(name.clone());
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                HostCall::End { .. } => HostDecision::Ack,
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(5)]).expect("runs");
        assert!(matches!(out, VoxValue::Int(20)), "5*2*2 = 20, got {out:?}");
        assert_eq!(*seen.borrow(), vec!["charge".to_string(), "charge".to_string()]);
    }

    #[test]
    fn intercept_after_callee_eval_not_only_ident() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int {
    let f = charge
    return f(amount)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let seen = Rc::new(RefCell::new(0usize));
        let seen_c = seen.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if matches!(call, HostCall::Begin { .. }) {
                *seen_c.borrow_mut() += 1;
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(5)]).expect("runs");
        assert!(matches!(out, VoxValue::Int(10)), "got {out:?}");
        assert_eq!(*seen.borrow(), 1, "aliased callee must still intercept");
    }

    #[test]
    fn replay_decision_skips_the_body() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int { return charge(amount) }
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        interp.activity_hook = Some(Box::new(|call| {
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Replay(VoxValue::Int(100)),
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(5)]).expect("runs");
        assert!(matches!(out, VoxValue::Int(100)), "got {out:?}");
    }

    #[test]
    fn result_err_is_not_failed() {
        const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Error("declined") }
workflow wf(amount: int) to Result[str] { return charge(amount) }
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let failed_flag = Rc::new(RefCell::new(None));
        let failed_c = failed_flag.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::End { failed, .. } = &call {
                *failed_c.borrow_mut() = Some(*failed);
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(1)]).expect("runs");
        assert!(matches!(out, VoxValue::Result(Err(_))), "Err must flow; got {out:?}");
        assert_eq!(*failed_flag.borrow(), Some(false), "Result::Err is not a crash");
    }

    #[test]
    fn with_options_reach_begin() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    return charge(amount) with { activity_id: "explicit-1" }
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let opts = Rc::new(RefCell::new(None));
        let opts_c = opts.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Begin { options, .. } = &call {
                *opts_c.borrow_mut() = options.clone();
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let _ = interp.call("wf", vec![VoxValue::Int(1)]).expect("with must not PARITY_UNIMPLEMENTED");
        let some = opts.borrow();
        let some = some.as_ref().expect("Begin must carry with-options");
        match some {
            VoxValue::Object(fields) => {
                assert!(fields.iter().any(|(k, v)| k == "activity_id"
                    && matches!(v, VoxValue::Str(s) if s == "explicit-1")));
            }
            other => panic!("expected object options, got {other:?}"),
        }
    }

    #[test]
    fn workflow_version_is_a_patch_call() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    workflow.version("add-audit-v2", 1, 2)
    return charge(amount)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let patches = Rc::new(RefCell::new(Vec::<String>::new()));
        let patches_c = patches.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Patch { change_id, .. } = &call {
                patches_c.borrow_mut().push(change_id.clone());
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                HostCall::Patch { .. } => HostDecision::Ack,
                _ => HostDecision::Ack,
            })
        }));
        let _ = interp.call("wf", vec![VoxValue::Int(1)]).expect("version must not PARITY_UNIMPLEMENTED");
        assert_eq!(*patches.borrow(), vec!["add-audit-v2".to_string()]);
    }

    #[test]
    fn workflow_wait_is_a_wait_call() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    workflow_wait(5000)
    return charge(amount)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        interp.activity_hook = Some(Box::new(|call| {
            Ok(match call {
                HostCall::Wait { .. } => HostDecision::Park("timer".into()),
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let err = interp.call("wf", vec![VoxValue::Int(1)]).expect_err("wait must park");
        assert!(matches!(err, crate::eval::EvalError::WorkflowParked(ref r) if r == "timer"));
    }

    #[test]
    fn plain_functions_are_not_intercepted() {
        const SRC: &str = r#"
fn double(n: int) to int { return n * 2 }
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    let d = double(amount)
    return charge(d)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let seen_c = seen.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Begin { name, .. } = &call {
                seen_c.borrow_mut().push(name.clone());
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let _ = interp.call("wf", vec![VoxValue::Int(3)]).expect("runs");
        assert_eq!(*seen.borrow(), vec!["charge".to_string()]);
    }
}
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p vox-compiler --lib eval::activity_hook
```

Expected: FAIL — unresolved `HostCall` / no field `activity_names`.

- [ ] **Step 3: Implement types and dispatch**

Prepend to `activity_hook.rs` (above the test module):

```rust
use crate::eval::value::VoxValue;
use crate::eval::{EvalError, Interpreter};

/// One interception point. Parking is a decision on Wait / WaitSignal,
/// not a fake activity named `__durable_timer_wait`.
#[derive(Debug, Clone)]
pub enum HostCall {
    Begin {
        name: String,
        args: Vec<VoxValue>,
        options: Option<VoxValue>,
    },
    End {
        name: String,
        result: VoxValue,
        /// True only when the body panicked or returned `_Panic`.
        failed: bool,
    },
    Patch {
        change_id: String,
        min: u32,
        max: u32,
    },
    Wait {
        deadline_ms: i64,
    },
    WaitSignal {
        key: String,
    },
}

/// Runner decision. There is no `Retry` — the runner re-sends `Execute`
/// after recording an attempt failure.
#[derive(Debug, Clone)]
pub enum HostDecision {
    Replay(VoxValue),
    Execute,
    Park(String),
    Ack,
}

/// Installed by the durable runner. Created on the interpreter thread;
/// do not add `+ Send`.
pub type ActivityHook = Box<dyn FnMut(HostCall) -> Result<HostDecision, EvalError>>;

fn take_hook(interp: &mut Interpreter) -> Option<ActivityHook> {
    interp.activity_hook.take()
}

fn restore_hook(interp: &mut Interpreter, hook: ActivityHook) {
    interp.activity_hook = Some(hook);
}

/// Intercept one activity call after the callee has evaluated to a named Fn.
pub(crate) fn dispatch_activity(
    interp: &mut Interpreter,
    name: &str,
    args: Vec<VoxValue>,
) -> Result<VoxValue, EvalError> {
    let mut hook = match take_hook(interp) {
        Some(h) => h,
        None => return interp.call(name, args),
    };
    let options = interp.with_options.last().cloned();
    let outcome = (|| -> Result<VoxValue, EvalError> {
        let decision = hook(HostCall::Begin {
            name: name.to_string(),
            args: args.clone(),
            options,
        })?;
        match decision {
            HostDecision::Replay(value) => Ok(value),
            HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
            HostDecision::Execute => {
                restore_hook(interp, hook);
                let result = match interp.call(name, args.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        hook = take_hook(interp).ok_or_else(|| {
                            EvalError::Panic("activity hook lost during body execution".into())
                        })?;
                        let failed = matches!(e, EvalError::Panic(_));
                        let _ = hook(HostCall::End {
                            name: name.to_string(),
                            result: VoxValue::Null,
                            failed,
                        });
                        return Err(e);
                    }
                };
                hook = take_hook(interp).ok_or_else(|| {
                    EvalError::Panic("activity hook lost during body execution".into())
                })?;
                let failed = false;
                match hook(HostCall::End {
                    name: name.to_string(),
                    result: result.clone(),
                    failed,
                })? {
                    HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
                    HostDecision::Replay(value) => Ok(value),
                    HostDecision::Ack | HostDecision::Execute => Ok(result),
                }
            }
            HostDecision::Ack => interp.call(name, args.clone()),
        }
    })();
    restore_hook(interp, hook);
    outcome
}

pub(crate) fn dispatch_patch(
    interp: &mut Interpreter,
    change_id: &str,
    min: u32,
    max: u32,
) -> Result<VoxValue, EvalError> {
    let Some(mut hook) = take_hook(interp) else {
        return Ok(VoxValue::Null);
    };
    let decision = hook(HostCall::Patch {
        change_id: change_id.to_string(),
        min,
        max,
    });
    restore_hook(interp, hook);
    match decision? {
        HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
        _ => Ok(VoxValue::Null),
    }
}

pub(crate) fn dispatch_wait(
    interp: &mut Interpreter,
    deadline_ms: i64,
) -> Result<VoxValue, EvalError> {
    let Some(mut hook) = take_hook(interp) else {
        return Err(EvalError::Panic(
            "workflow_wait requires a durable runner".into(),
        ));
    };
    let decision = hook(HostCall::Wait { deadline_ms });
    restore_hook(interp, hook);
    match decision? {
        HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
        HostDecision::Ack | HostDecision::Execute | HostDecision::Replay(_) => Ok(VoxValue::Null),
    }
}

pub(crate) fn dispatch_wait_signal(
    interp: &mut Interpreter,
    key: String,
) -> Result<VoxValue, EvalError> {
    let Some(mut hook) = take_hook(interp) else {
        return Err(EvalError::Panic(
            "workflow_wait_signal requires a durable runner".into(),
        ));
    };
    let decision = hook(HostCall::WaitSignal { key });
    restore_hook(interp, hook);
    match decision? {
        HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
        HostDecision::Replay(v) => Ok(v),
        HostDecision::Ack | HostDecision::Execute => Ok(VoxValue::Null),
    }
}
```

- [ ] **Step 4: Wire interpreter fields**

In `crates/vox-compiler/src/eval/mod.rs`:

1. `pub mod activity_hook;`
2. Add to `EvalError` after `Panic(String)`:

```rust
    /// A durable workflow step could not proceed. The run is parked, not failed.
    WorkflowParked(String),
```

Update the `Display` impl:

```rust
            EvalError::WorkflowParked(reason) => write!(f, "workflow parked: {reason}"),
```

Fix every remaining exhaustive `match` the compiler names.

3. Add fields to `Interpreter`:

```rust
    pub activity_names: std::collections::HashSet<String>,
    pub activity_hook: Option<crate::eval::activity_hook::ActivityHook>,
    /// Stack of `with { }` option objects; the innermost is attached to Begin.
    pub with_options: Vec<crate::eval::value::VoxValue>,
```

Initialize in `Interpreter::new`:

```rust
            activity_names: std::collections::HashSet::new(),
            activity_hook: None,
            with_options: Vec::new(),
```

- [ ] **Step 5: Intercept after callee eval; evaluate With and WorkflowVersion**

In `crates/vox-compiler/src/eval/expr.rs`, in the `HirExpr::Call` arm, **after** `let c = eval_expr(interp, callee)?;` and **before** the `match c` that applies the Fn, insert:

```rust
            if let HirExpr::Ident(name, _) = callee.as_ref() {
                match name.as_str() {
                    "workflow_wait" => {
                        let deadline = match eval_args.first() {
                            Some(VoxValue::Int(n)) => *n,
                            other => {
                                return Err(EvalError::Panic(format!(
                                    "workflow_wait expects int ms, got {other:?}"
                                )));
                            }
                        };
                        return super::activity_hook::dispatch_wait(interp, deadline);
                    }
                    "workflow_wait_signal" => {
                        let key = match eval_args.first() {
                            Some(VoxValue::Str(s)) => s.clone(),
                            other => {
                                return Err(EvalError::Panic(format!(
                                    "workflow_wait_signal expects str key, got {other:?}"
                                )));
                            }
                        };
                        return super::activity_hook::dispatch_wait_signal(interp, key);
                    }
                    _ => {}
                }
            }
            if let VoxValue::Fn { name: ref fn_name, .. } = c
                && interp.activity_names.contains(fn_name)
            {
                return super::activity_hook::dispatch_activity(interp, fn_name, eval_args);
            }
```

Replace the `HirExpr::With` arm:

```rust
        HirExpr::With(inner, opts, _) => {
            let opts_val = eval_expr(interp, opts)?;
            interp.with_options.push(opts_val);
            let result = eval_expr(interp, inner);
            interp.with_options.pop();
            result
        }
```

Replace the `HirExpr::WorkflowVersion` arm:

```rust
        HirExpr::WorkflowVersion(v) => {
            super::activity_hook::dispatch_patch(interp, &v.change_id, v.min, v.max)
        }
```

If `Block` / `while` / `loop` error paths can leave `with_options` pushed, pop in those unwind paths the same way the existing Call arm restores `interp.scope` (RAII / closure). Do not rely on Park to clean the stack.

Nested `workflow` calls: if `c` is a `Fn` whose name is in a new `interp.workflow_names` set (populate from `DurabilityKind::Workflow` in the host), return `EvalError::Panic("nested workflow calls are not supported".into())`. Add `workflow_names` to `Interpreter` in this task if the host will populate it; otherwise the host (Task 1.3) sets it.

- [ ] **Step 6: Run the tests and watch them pass**

```bash
cargo test -p vox-compiler --lib eval::activity_hook
```

Expected: PASS (8 tests).

- [ ] **Step 7: Mutation-verify intercept-after-eval**

Temporarily restore a `Call(Ident)`-only intercept (check `HirExpr::Ident` before eval, skip the `VoxValue::Fn` name check). Confirm `intercept_after_callee_eval_not_only_ident` FAILS. Restore; confirm PASS.

- [ ] **Step 8: Commit**

```bash
cargo test -p vox-compiler --lib
cargo clippy -p vox-compiler --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-compiler/src/eval/
git commit -m "$(cat <<'EOF'
feat(eval): intercept durability primitives after callee eval

Activities, with-options, workflow.version, and wait builtins must reach
the runner. Result::Err is a value, not a crash. The hook is !Send and
lives on the interpreter thread.
EOF
)"
```

---

### Task 1.3: The `!Send` host bridge with `Drop`

Confine the interpreter: one OS thread, two unbounded channels, JSON values. `blocking_recv` has no timeout — `Drop` must unblock the thread (G19). Persist `args_json` is a **runner** concern (Task 1.4); this task only carries `args` into `spawn`.

**Files:**
- Create: `crates/vox-workflow-runtime/src/workflow/host.rs`
- Modify: `crates/vox-workflow-runtime/src/workflow/mod.rs`
- Test: `crates/vox-workflow-runtime/src/workflow/host.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `HostCall`, `HostDecision` (Task 1.2); `to_journal_json` / `from_journal_json` (Task 1.1).
- Produces:
  - `pub struct WorkflowHost`
  - `WorkflowHost::spawn(hir: HirModule, workflow_name: &str, args: Vec<Value>) -> anyhow::Result<WorkflowHost>`
  - `WorkflowHost::next(&mut self) -> Option<HostRequest>` *(async)*
  - `WorkflowHost::respond(&self, decision: HostDecision) -> anyhow::Result<()>`
  - `WorkflowHost::finish(self) -> anyhow::Result<HostOutcome>` *(async)*
  - `impl Drop for WorkflowHost`
  - `pub enum HostRequest { Begin { name, args, options }, End { name, result, failed }, Patch { change_id, min, max }, Wait { deadline_ms }, WaitSignal { key } }`
  - `pub enum HostDecision { Replay(Value), Execute, Park(String), Ack }`
  - `pub enum HostOutcome { Completed(Value), Parked(String), Failed(String) }`

Note the host crate's `HostDecision` uses `serde_json::Value`; the interpreter's uses `VoxValue`. Convert at the hook.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-workflow-runtime/src/workflow/host.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vox_compiler::hir::lower_module;
    use vox_compiler::lexer::cursor::lex;
    use vox_compiler::parser::parse;

    const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int {
    let a = charge(amount)
    return a
}
"#;

    fn hir_of(src: &str) -> vox_compiler::hir::HirModule {
        lower_module(&parse(lex(src)).expect("parses"))
    }

    #[tokio::test]
    async fn host_executes_the_body_and_returns_the_real_value() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(21)]).expect("spawns");
        let mut begins = 0;
        while let Some(req) = host.next().await {
            match req {
                HostRequest::Begin { name, args, .. } => {
                    begins += 1;
                    assert_eq!(name, "charge");
                    assert_eq!(args, vec![json!(21)]);
                    host.respond(HostDecision::Execute).expect("respond");
                }
                HostRequest::End { result, failed, .. } => {
                    assert_eq!(result, json!(42));
                    assert!(!failed);
                    host.respond(HostDecision::Ack).expect("respond");
                }
                other => panic!("unexpected request {other:?}"),
            }
        }
        assert_eq!(begins, 1);
        match host.finish().await.expect("joins") {
            HostOutcome::Completed(v) => assert_eq!(v, json!(42)),
            other => panic!("expected Completed(42), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn host_replay_does_not_execute_the_body() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(21)]).expect("spawns");
        let mut ends = 0;
        while let Some(req) = host.next().await {
            match req {
                HostRequest::Begin { .. } => {
                    host.respond(HostDecision::Replay(json!(999))).expect("respond");
                }
                HostRequest::End { .. } => {
                    ends += 1;
                    host.respond(HostDecision::Ack).expect("respond");
                }
                other => {
                    host.respond(HostDecision::Ack).expect("respond");
                    panic!("unexpected {other:?}");
                }
            }
        }
        assert_eq!(ends, 0);
        match host.finish().await.expect("joins") {
            HostOutcome::Completed(v) => assert_eq!(v, json!(999)),
            other => panic!("expected Completed(999), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn host_park_surfaces_as_parked_outcome() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(1)]).expect("spawns");
        while let Some(req) = host.next().await {
            if let HostRequest::Begin { .. } = req {
                host.respond(HostDecision::Park("timer".into())).expect("respond");
            } else {
                host.respond(HostDecision::Ack).expect("respond");
            }
        }
        match host.finish().await.expect("joins") {
            HostOutcome::Parked(reason) => assert_eq!(reason, "timer"),
            other => panic!("expected Parked, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn host_reports_a_missing_workflow_as_an_error() {
        let err = WorkflowHost::spawn(hir_of(SRC), "no_such_workflow", vec![]);
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn drop_unblocks_a_waiting_thread() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(1)]).expect("spawns");
        let first = host.next().await;
        assert!(matches!(first, Some(HostRequest::Begin { .. })));
        drop(host);
    }
}
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p vox-workflow-runtime --lib workflow::host
```

Expected: FAIL — `cannot find struct WorkflowHost`.

- [ ] **Step 3: Implement the host**

Prepend to `host.rs`:

```rust
//! The `!Send` bridge between the tree-walking interpreter and the async runner.

use anyhow::Context;
use serde_json::Value;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use vox_compiler::eval::Interpreter;
use vox_compiler::eval::activity_hook::{HostCall, HostDecision as InterpDecision};
use vox_compiler::eval::value::VoxValue;
use vox_compiler::hir::HirModule;
use vox_compiler::hir::nodes::DurabilityKind;

const WORKFLOW_STEP_LIMIT: usize = 10_000_000;

#[derive(Debug, Clone)]
pub enum HostRequest {
    Begin {
        name: String,
        args: Vec<Value>,
        options: Option<Value>,
    },
    End {
        name: String,
        result: Value,
        failed: bool,
    },
    Patch {
        change_id: String,
        min: u32,
        max: u32,
    },
    Wait {
        deadline_ms: i64,
    },
    WaitSignal {
        key: String,
    },
}

#[derive(Debug, Clone)]
pub enum HostDecision {
    Replay(Value),
    Execute,
    Park(String),
    Ack,
}

#[derive(Debug, Clone)]
pub enum HostOutcome {
    Completed(Value),
    Parked(String),
    Failed(String),
}

pub struct WorkflowHost {
    req_rx: UnboundedReceiver<HostRequest>,
    resp_tx: UnboundedSender<HostDecision>,
    handle: Option<std::thread::JoinHandle<HostOutcome>>,
}

impl WorkflowHost {
    pub fn spawn(
        hir: HirModule,
        workflow_name: &str,
        args: Vec<Value>,
    ) -> anyhow::Result<Self> {
        let activity_names: std::collections::HashSet<String> = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(DurabilityKind::Activity))
            .map(|f| f.name.clone())
            .collect();
        let workflow_names: std::collections::HashSet<String> = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(DurabilityKind::Workflow))
            .map(|f| f.name.clone())
            .collect();
        if !workflow_names.contains(workflow_name) {
            anyhow::bail!(
                "workflow `{workflow_name}` not found in HIR; declare it with the `workflow` keyword"
            );
        }

        let (req_tx, req_rx) = unbounded_channel::<HostRequest>();
        let (resp_tx, resp_rx) = unbounded_channel::<HostDecision>();
        let name = workflow_name.to_string();

        let handle = std::thread::Builder::new()
            .name(format!("vox-workflow-{name}"))
            .spawn(move || {
                run_on_thread(hir, name, args, activity_names, workflow_names, req_tx, resp_rx)
            })
            .context("spawning the workflow interpreter thread")?;

        Ok(Self {
            req_rx,
            resp_tx,
            handle: Some(handle),
        })
    }

    pub async fn next(&mut self) -> Option<HostRequest> {
        self.req_rx.recv().await
    }

    pub fn respond(&self, decision: HostDecision) -> anyhow::Result<()> {
        self.resp_tx
            .send(decision)
            .map_err(|_| anyhow::anyhow!("workflow interpreter thread hung up"))
    }

    pub async fn finish(mut self) -> anyhow::Result<HostOutcome> {
        let handle = self
            .handle
            .take()
            .ok_or_else(|| anyhow::anyhow!("workflow host already joined"))?;
        tokio::task::spawn_blocking(move || handle.join())
            .await
            .context("joining the workflow interpreter thread")?
            .map_err(|_| anyhow::anyhow!("workflow interpreter thread panicked"))
    }
}

impl Drop for WorkflowHost {
    fn drop(&mut self) {
        let _ = self.resp_tx.send(HostDecision::Park("host-dropped".into()));
        if let Some(handle) = self.handle.take() {
            let _ = std::thread::Builder::new()
                .name("vox-workflow-reap".into())
                .spawn(move || {
                    let _ = handle.join();
                });
        }
    }
}

fn run_on_thread(
    hir: HirModule,
    workflow_name: String,
    args: Vec<Value>,
    activity_names: std::collections::HashSet<String>,
    workflow_names: std::collections::HashSet<String>,
    req_tx: UnboundedSender<HostRequest>,
    mut resp_rx: UnboundedReceiver<HostDecision>,
) -> HostOutcome {
    let mut interp = Interpreter::new(WORKFLOW_STEP_LIMIT);
    if let Err(e) = interp.run_module(&hir) {
        return HostOutcome::Failed(format!("loading module: {e:?}"));
    }
    interp.activity_names = activity_names;
    interp.workflow_names = workflow_names;
    interp.activity_hook = Some(Box::new(move |call| {
        let request = match &call {
            HostCall::Begin {
                name,
                args,
                options,
            } => HostRequest::Begin {
                name: name.clone(),
                args: args.iter().map(VoxValue::to_journal_json).collect(),
                options: options.as_ref().map(VoxValue::to_journal_json),
            },
            HostCall::End {
                name,
                result,
                failed,
            } => HostRequest::End {
                name: name.clone(),
                result: result.to_journal_json(),
                failed: *failed,
            },
            HostCall::Patch {
                change_id,
                min,
                max,
            } => HostRequest::Patch {
                change_id: change_id.clone(),
                min: *min,
                max: *max,
            },
            HostCall::Wait { deadline_ms } => HostRequest::Wait {
                deadline_ms: *deadline_ms,
            },
            HostCall::WaitSignal { key } => HostRequest::WaitSignal { key: key.clone() },
        };
        if req_tx.send(request).is_err() {
            return Err(vox_compiler::eval::EvalError::Panic(
                "durable workflow runner hung up".into(),
            ));
        }
        let Some(decision) = resp_rx.blocking_recv() else {
            return Err(vox_compiler::eval::EvalError::Panic(
                "durable workflow runner dropped the response channel".into(),
            ));
        };
        Ok(match decision {
            HostDecision::Replay(v) => InterpDecision::Replay(
                VoxValue::from_journal_json(&v).map_err(|e| {
                    vox_compiler::eval::EvalError::Panic(e.message)
                })?,
            ),
            HostDecision::Execute => InterpDecision::Execute,
            HostDecision::Park(r) => InterpDecision::Park(r),
            HostDecision::Ack => InterpDecision::Ack,
        })
    }));

    let vox_args = match args
        .iter()
        .map(VoxValue::from_journal_json)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(e) => return HostOutcome::Failed(e.message),
    };
    match interp.call(&workflow_name, vox_args) {
        Ok(value) => HostOutcome::Completed(value.to_journal_json()),
        Err(vox_compiler::eval::EvalError::WorkflowParked(reason)) => HostOutcome::Parked(reason),
        Err(e) => HostOutcome::Failed(format!("{e:?}")),
    }
}
```

Add `pub workflow_names: HashSet<String>` to `Interpreter` in the same commit if Task 1.2 did not (initialize empty). Nested workflow: in the Call arm, after callee eval, if `Fn.name` is in `workflow_names` and is not the currently executing workflow, return `EvalError::Panic("nested workflow calls are not supported".into())`.

- [ ] **Step 4: Export the module**

In `crates/vox-workflow-runtime/src/workflow/mod.rs`:

```rust
pub mod host;
pub use host::{HostDecision, HostOutcome, HostRequest, WorkflowHost};
```

- [ ] **Step 5: Run the tests and watch them pass**

```bash
cargo test -p vox-workflow-runtime --lib workflow::host
```

Expected: PASS (5 tests). `drop_unblocks_a_waiting_thread` must finish; a missing `Drop` hangs until the test harness times out.

- [ ] **Step 6: Mutation-verify Drop**

Temporarily empty `Drop`. Confirm `drop_unblocks_a_waiting_thread` hangs or the next test run is killed. Restore; confirm PASS. Do not leave this broken.

- [ ] **Step 7: Commit**

```bash
cargo clippy -p vox-workflow-runtime --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-workflow-runtime/src/workflow/host.rs crates/vox-workflow-runtime/src/workflow/mod.rs crates/vox-compiler/src/eval/mod.rs
git commit -m "$(cat <<'EOF'
feat(workflow): add the interpreter-thread host bridge

The interpreter is !Send, so it stays on a dedicated OS thread. Values
cross as journal JSON. Drop unblocks blocking_recv so a forgotten
respond cannot hang a worker forever.
EOF
)"
```

---

### Task 1.4: Rewrite the runner around the host

Replace the plan-walking loop. Call `handle_workflow_patch` from `HostRequest::Patch` (the 2026-09-05 draft orphaned it). `Result::Err` is a value. Mesh / `PopuliActivity` is a **loud error**, not a silent drop (G18). Persist `args_json` on `workflow_run_log` at start (G27). Refuse resume of pre-Phase-1 canned `LocalActivity` rows.

**Files:**
- Modify: `crates/vox-workflow-runtime/src/workflow/run.rs`
- Modify: `crates/vox-workflow-runtime/src/db_tracker.rs` (persist/load `args_json`; refuse canned replay)
- Modify: `crates/vox-db/src/schema/domains/execution.rs` (add `args_json TEXT`)
- Modify: `crates/vox-db/src/schema/manifest.rs` (bump `BASELINE_VERSION`)
- Modify: `crates/vox-db/src/facade/workflow.rs` (write/read `args_json`)
- Modify: `contracts/db/baseline-version-policy.yaml` (keep in sync)
- Modify: `crates/vox-ml-cli/src/commands/ai/workflow.rs`
- Modify: `crates/vox-codegen/src/codegen_rust/emit/durability_lower.rs`
- Modify: `crates/vox-workflow-runtime/tests/journal_schema_conformance.rs`
- Modify: `crates/vox-workflow-runtime/tests/crash_replay.rs`
- Modify: `crates/vox-workflow-runtime/tests/codegen_roundtrip.rs`
- Modify: `crates/vox-workflow-runtime/tests/workflow_patch.rs`
- Modify: `crates/vox-workflow-runtime/tests/workflow_tracker_tests.rs`
- Test: `crates/vox-workflow-runtime/tests/real_activity_execution.rs` (create)

**Interfaces:**
- Consumes: `WorkflowHost`, `HostRequest`, `HostDecision`, `HostOutcome` (Task 1.3); existing `WorkflowTracker`.
- Produces: `pub async fn interpret_workflow_durable(hir: &HirModule, workflow_name: &str, args: Vec<serde_json::Value>, tracker: &mut impl WorkflowTracker) -> anyhow::Result<Vec<serde_json::Value>>`

Add to `WorkflowTracker` (default no-op for `DefaultTracker`):

```rust
async fn persist_run_args(&mut self, args_json: &Value) -> anyhow::Result<()> { let _ = args_json; Ok(()) }
async fn load_run_args(&self) -> anyhow::Result<Option<Value>> { Ok(None) }
```

`VoxDbTracker` implements both against `workflow_run_log.args_json`.

- [ ] **Step 1: Write the failing real-execution tests**

Create `crates/vox-workflow-runtime/tests/real_activity_execution.rs`:

```rust
#![allow(missing_docs)]

use serde_json::json;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_workflow_runtime::workflow::{DefaultTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

#[tokio::test]
async fn activity_body_return_is_journalled() {
    const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Ok("tx_5") }
workflow checkout(amount: int) to Result[str] {
    return charge(amount)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(5)], &mut tracker)
        .await
        .expect("runs");
    let completed = journal
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .expect("completed");
    assert_eq!(
        completed["return_value"],
        json!({"__vox": "Ok", "value": "tx_5"}),
        "canned LocalActivity payload is a failure"
    );
}

#[tokio::test]
async fn question_mark_sees_result_err() {
    const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Error("declined") }
workflow checkout(amount: int) to Result[str] {
    let tx = charge(amount)?
    return Ok(tx)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(1)], &mut tracker)
        .await
        .expect("Err is a value, not a runner failure");
    let completed = journal
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .expect("completed");
    assert_eq!(
        completed["return_value"],
        json!({"__vox": "Err", "value": "declined"})
    );
}

#[tokio::test]
async fn patch_request_is_journalled() {
    const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow checkout(amount: int) to int {
    workflow.version("add-audit-v2", 1, 2)
    return charge(amount)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(1)], &mut tracker)
        .await
        .expect("runs");
    assert!(
        journal.iter().any(|e| e["event"] == "WorkflowPatch"
            && e["change_id"] == "add-audit-v2"),
        "HostRequest::Patch must call handle_workflow_patch; got {journal:#?}"
    );
}

#[tokio::test]
async fn mesh_activity_is_a_loud_error() {
    const SRC: &str = r#"
@place(mens) activity remote_charge(amount: int) to int { return amount }
workflow checkout(amount: int) to int {
    return remote_charge(amount)
}
"#;
    let mut tracker = DefaultTracker;
    let err = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(1)], &mut tracker)
        .await
        .expect_err("mesh is out of this plan");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("mesh") || msg.contains("Populi") || msg.contains("mens"),
        "must name mesh, not silently run local; got {msg}"
    );
}
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p vox-workflow-runtime --test real_activity_execution
```

Expected: FAIL — `interpret_workflow_durable` still has the 3-arg signature, and the journal still records canned `LocalActivity`.

- [ ] **Step 3: Add `args_json` via the data-storage path**

In `crates/vox-db/src/schema/domains/execution.rs`, add to `workflow_run_log`:

```sql
    args_json        TEXT
```

Bump `BASELINE_VERSION` in `crates/vox-db/src/schema/manifest.rs` by 1. Update `repository_baseline_integer` (and digest, if the guard requires it) in `contracts/db/baseline-version-policy.yaml`. Add a facade write/read:

```rust
pub async fn workflow_run_set_args_json(&self, run_id: &str, args_json: &str) -> anyhow::Result<()> {
    // UPDATE workflow_run_log SET args_json = ?2 WHERE run_id = ?1
}
pub async fn workflow_run_args_json(&self, run_id: &str) -> anyhow::Result<Option<String>> {
    // SELECT args_json FROM workflow_run_log WHERE run_id = ?1
}
```

Run:

```bash
vox ci data-storage-guard
```

Expected: PASS after the bump is consistent.

- [ ] **Step 4: Rewrite `interpret_workflow_durable`**

Replace the body of `interpret_workflow_durable` in `run.rs`. Keep `handle_workflow_patch` and `derive_activity_id`. Delete `execute_local_activity_step` and the plan-walking loop. Sketch:

```rust
pub async fn interpret_workflow(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<Value>> {
    let mut tracker = DefaultTracker;
    interpret_workflow_durable(hir, workflow_name, Vec::new(), &mut tracker).await
}

pub async fn interpret_workflow_durable(
    hir: &HirModule,
    workflow_name: &str,
    args: Vec<Value>,
    tracker: &mut impl WorkflowTracker,
) -> anyhow::Result<Vec<Value>> {
    tracker.persist_run_args(&Value::Array(args.clone())).await?;
    tracker.on_workflow_started(workflow_name, 0).await?;
    let mut journal = vec![versioned_event(json!({
        "event": "WorkflowStarted",
        "workflow": workflow_name,
    }))];

    let mut host = WorkflowHost::spawn(hir.clone(), workflow_name, args)?;
    let mut position: usize = 0;
    while let Some(req) = host.next().await {
        match req {
            HostRequest::Patch {
                change_id,
                min,
                max,
            } => {
                handle_workflow_patch(workflow_name, &change_id, min, max, &mut journal, tracker)
                    .await?;
                host.respond(HostDecision::Ack)?;
            }
            HostRequest::Begin {
                name,
                args: act_args,
                options,
            } => {
                if activity_is_mesh(hir, &name) {
                    host.respond(HostDecision::Park("unused".into()))?;
                    let _ = host.finish().await;
                    anyhow::bail!(
                        "mesh / PopuliActivity `{name}` is unsupported in the body-first runner; \
                         use the deferred mens path"
                    );
                }
                if let Some(id) = explicit_activity_id(options.as_ref()) {
                    // Phase 1: honor explicit with.activity_id; Phase 5 errors on duplicates.
                    let _ = id;
                }
                let activity_id = derive_activity_id(workflow_name, &name, position);
                position += 1;
                if let Some(prior) = tracker.load_completed(workflow_name, &activity_id).await? {
                    refuse_canned_local_activity(&prior)?;
                    journal.push(versioned_event(json!({
                        "event": "ActivityReplayed",
                        "workflow": workflow_name,
                        "activity": name,
                        "activity_id": activity_id,
                    })));
                    host.respond(HostDecision::Replay(prior))?;
                } else {
                    tracker
                        .on_activity_started(workflow_name, &activity_id, &name)
                        .await?;
                    journal.push(versioned_event(json!({
                        "event": "ActivityStarted",
                        "workflow": workflow_name,
                        "activity": name,
                        "activity_id": activity_id,
                    })));
                    host.respond(HostDecision::Execute)?;
                }
            }
            HostRequest::End {
                name,
                result,
                failed,
            } => {
                if failed {
                    let activity_id = derive_activity_id(workflow_name, &name, position.saturating_sub(1));
                    tracker
                        .on_activity_attempt_failed(workflow_name, &activity_id, &result)
                        .await?;
                    host.respond(HostDecision::Execute)?;
                    continue;
                }
                let activity_id = derive_activity_id(workflow_name, &name, position.saturating_sub(1));
                tracker
                    .on_activity_completed(workflow_name, &activity_id, &result)
                    .await?;
                journal.push(versioned_event(json!({
                    "event": "ActivityCompleted",
                    "workflow": workflow_name,
                    "activity": name,
                    "activity_id": activity_id,
                    "result": result,
                })));
                host.respond(HostDecision::Ack)?;
            }
            HostRequest::Wait { .. } | HostRequest::WaitSignal { .. } => {
                host.respond(HostDecision::Park(
                    "timers and signals land in Phase 3/4".into(),
                ))?;
            }
        }
    }

    match host.finish().await? {
        HostOutcome::Completed(return_value) => {
            tracker.on_workflow_completed(workflow_name).await?;
            journal.push(versioned_event(json!({
                "event": "WorkflowCompleted",
                "workflow": workflow_name,
                "return_value": return_value,
            })));
            Ok(journal)
        }
        HostOutcome::Parked(reason) => Ok({
            journal.push(versioned_event(json!({
                "event": "WorkflowStarted",
            })));
            let _ = reason;
            journal
        }),
        HostOutcome::Failed(msg) => anyhow::bail!("{msg}"),
    }
}

fn refuse_canned_local_activity(prior: &Value) -> anyhow::Result<()> {
    if prior.get("event").and_then(Value::as_str) == Some("LocalActivity")
        && prior.get("status").and_then(Value::as_str) == Some("executed")
    {
        anyhow::bail!(
            "refusing to replay a pre-Phase-1 canned LocalActivity row; \
             start a new run_id after upgrading"
        );
    }
    Ok(())
}

fn activity_is_mesh(hir: &HirModule, name: &str) -> bool {
    hir.functions.iter().any(|f| {
        f.name == name
            && f.durability == Some(vox_compiler::hir::nodes::DurabilityKind::Activity)
            && f.execution_boundary
                .as_deref()
                .is_some_and(|b| b == "mens" || b == "mesh")
    })
}

fn explicit_activity_id(options: Option<&Value>) -> Option<String> {
    options
        .and_then(|o| o.get("activity_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}
```

Adapt tracker method names to the **actual** `WorkflowTracker` trait in `tracker.rs` (`on_activity_started`, `load_completed`, etc.). Do not invent a second trait. Wire `persist_run_args` on `VoxDbTracker` to the facade from Step 3.

If `HirFn` has no `execution_boundary` field, detect mesh the same way `plan.rs` / `execute_step_once` does today (`mens` / `PopuliActivity`). Match that existing predicate.

Parked outcome in this task: return the journal without `WorkflowCompleted` and without failing. Phase 3 sets status `parked`.

- [ ] **Step 5: Update every 3-arg callsite**

Pass `vec![]` or real args at each site:

| File | Change |
|---|---|
| `crates/vox-workflow-runtime/tests/journal_schema_conformance.rs` | add `vec![]` (or `vec![json!(1)]`) |
| `crates/vox-workflow-runtime/tests/crash_replay.rs` | same |
| `crates/vox-workflow-runtime/tests/codegen_roundtrip.rs` | same |
| `crates/vox-workflow-runtime/tests/workflow_patch.rs` | same |
| `crates/vox-workflow-runtime/tests/workflow_tracker_tests.rs` | same |
| `crates/vox-ml-cli/src/commands/ai/workflow.rs` | pass decoded workflow args, not `vec![]` if the CLI already has them |
| `crates/vox-codegen/src/codegen_rust/emit/durability_lower.rs` | emit the new 4-arg call; args from the generated workflow fn parameters |

Search for remaining 3-arg calls after the compile:

```bash
rg "interpret_workflow_durable\\(" --glob "*.rs"
```

Every hit must compile.

- [ ] **Step 6: Run tests**

```bash
cargo test -p vox-workflow-runtime --test real_activity_execution
cargo test -p vox-workflow-runtime --test journal_schema_conformance
cargo test -p vox-workflow-runtime --test crash_replay
cargo test -p vox-workflow-runtime --test workflow_patch
cargo test -p vox-workflow-runtime --test codegen_roundtrip
cargo test -p vox-workflow-runtime --test workflow_tracker_tests
cargo clippy -p vox-workflow-runtime --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 7: Mutation-verify `handle_workflow_patch`**

Temporarily ignore `HostRequest::Patch` (`host.respond(Ack)` only). Confirm `patch_request_is_journalled` FAILS. Restore; confirm PASS. Do this in **this** task — there is no Task 1.5 crash_replay hook to hide behind.

- [ ] **Step 8: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-workflow-runtime crates/vox-db crates/vox-ml-cli crates/vox-codegen contracts/db
git commit -m "$(cat <<'EOF'
feat(workflow): execute the workflow body through the host bridge

The runner now journals real activity returns, honors workflow.version via
handle_workflow_patch, persists args_json at start, and loud-errors mesh.
Result::Err flows to ? instead of triggering retry.
EOF
)"
```

---

### Task 1.5: Delete the linearizer after 1.4 is green

`plan_workflow_replay_ir` is today's engine. Delete it only after Task 1.4's suite is green. Keep duration / mesh parse helpers. `plan_workflow_activities` is exported and used by `crates/vox-integration-tests/tests/parity_contracts_test.rs` (ignored at runtime, still compiled).

**Files:**
- Modify: `crates/vox-workflow-runtime/src/workflow/plan.rs`
- Modify: `crates/vox-workflow-runtime/src/lib.rs` (export)
- Modify: `crates/vox-integration-tests/tests/parity_contracts_test.rs`
- Modify: `crates/vox-workflow-runtime/src/workflow/mod.rs` (unit test that expected planner errors)
- Create: `examples/golden/durable_workflow_branching.vox`
- Test: `crates/vox-workflow-runtime/tests/runtime_branching.rs`

**Interfaces:**
- Consumes: Task 1.4 runner.
- Produces: `plan_workflow_activities` as a thin exported walker of `DurabilityKind::Activity` names (or the integration test updated to not call the linearizer). `plan_workflow_replay_ir` gone.

- [ ] **Step 1: Write the failing branching test**

Create `crates/vox-workflow-runtime/tests/runtime_branching.rs`:

```rust
#![allow(missing_docs)]

use serde_json::json;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_workflow_runtime::workflow::{DefaultTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

#[tokio::test]
async fn runtime_if_on_activity_result() {
    const SRC: &str = r#"
activity charge(amount: int) to Result[str] {
    if amount > 0 { return Ok("tx") }
    return Error("bad")
}
workflow checkout(amount: int) to str {
    let result = charge(amount)
    if result.ok {
        return "paid"
    } else {
        return "declined"
    }
}
"#;
    let mut tracker = DefaultTracker;
    let paid = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(10)], &mut tracker)
        .await
        .expect("ok branch");
    let paid_ret = paid.iter().rev().find(|e| e["event"] == "WorkflowCompleted").unwrap();
    assert_eq!(paid_ret["return_value"], json!("paid"));

    let mut tracker = DefaultTracker;
    let declined = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(0)], &mut tracker)
        .await
        .expect("err branch");
    let declined_ret = declined.iter().rev().find(|e| e["event"] == "WorkflowCompleted").unwrap();
    assert_eq!(declined_ret["return_value"], json!("declined"));
}
```

Create `examples/golden/durable_workflow_branching.vox` with the same program plus an `@test` block that asserts both branches (Vox golden files require `@test` for new `fn`s). If `result.ok` is not the real field, use the actual Result projection the interpreter already supports (`result is Ok`, or `?` + else). Match existing golden Result tests.

- [ ] **Step 2: Run it — this should already pass if 1.4 is real**

```bash
cargo test -p vox-workflow-runtime --test runtime_branching
```

Expected: PASS on the body-first runner. If it FAILS because the planner still rejected `if`, Task 1.4 is not done.

- [ ] **Step 3: Delete the linearizer**

In `plan.rs`, remove `plan_workflow_replay_ir` and the compile-time `if`/`match`/`while` walk. Keep helpers that parse durations / detect mesh. Replace `plan_workflow_activities` with a name-list walker:

```rust
pub fn plan_workflow_activities(hir: &HirModule, workflow_name: &str) -> anyhow::Result<Vec<String>> {
    if !hir.functions.iter().any(|f| {
        f.durability == Some(vox_compiler::hir::nodes::DurabilityKind::Workflow)
            && f.name == workflow_name
    }) {
        anyhow::bail!("workflow `{workflow_name}` not found");
    }
    Ok(hir
        .functions
        .iter()
        .filter(|f| f.durability == Some(vox_compiler::hir::nodes::DurabilityKind::Activity))
        .map(|f| f.name.clone())
        .collect())
}
```

Update `parity_contracts_test.rs` if it asserted planned-step shapes that no longer exist. Update `workflow/mod.rs` unit tests that expected linearizer errors for `match`.

- [ ] **Step 4: Confirm nothing still calls the deleted IR**

```bash
rg "plan_workflow_replay_ir" --glob "*.rs"
cargo test -p vox-workflow-runtime --lib
cargo test -p vox-integration-tests --test parity_contracts_test -- --ignored
```

Expected: no remaining `plan_workflow_replay_ir` references; lib tests PASS.

- [ ] **Step 5: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-workflow-runtime crates/vox-integration-tests examples/golden/durable_workflow_branching.vox
git commit -m "$(cat <<'EOF'
refactor(workflow): delete the compile-time linearizer

Body-first execution subsumes planned-step walking. Keep a thin
plan_workflow_activities export for ignored integration tests.
EOF
)"
```

---

## Phase 2 — Crash windows that prove the right thing

### Task 2.1: `CountingTracker` and two crash windows

The 2026-09-05 window-1 test ran both activities to completion, then replayed — that does not prove “kill after activity 1 completed; activity 2 still runs.” Window 2 must **expire the lease** (G17). Crash Vox uses `std.fs.write` and `@uses(fs)`.

**Files:**
- Create: `crates/vox-workflow-runtime/src/workflow/counting_tracker.rs`
- Modify: `crates/vox-workflow-runtime/src/workflow/mod.rs`
- Modify: `crates/vox-workflow-runtime/src/db_tracker.rs` (export a test helper to expire a lease, or use the facade)
- Test: `crates/vox-workflow-runtime/tests/crash_windows.rs`

**Interfaces:**
- Consumes: Task 1.4 runner; `VoxDbTracker`.
- Produces: `pub struct CountingTracker<T> { inner: T, fail_after: usize, calls: usize }` that implements `WorkflowTracker` and returns `Err` after `fail_after` successful inner calls.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-workflow-runtime/tests/crash_windows.rs`:

```rust
#![allow(missing_docs)]

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::counting_tracker::CountingTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

const SRC: &str = r#"
@uses(fs)
activity write_one(path: str) to int {
    std.fs.write(path, "one")
    return 1
}

@uses(fs)
activity write_two(path: str) to int {
    std.fs.write(path, "two")
    return 2
}

@uses(fs)
workflow pair(dir: str) to int {
    let _a = write_one(dir + "/one.txt")
    let _b = write_two(dir + "/two.txt")
    return 3
}
"#;

#[tokio::test]
async fn window_a_kill_after_first_completed() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().to_string_lossy().to_string();
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "crash-a");
    // fail_after counts tracker calls; tune so the first on_activity_completed
    // succeeds and the next tracker call (second start) faults.
    let mut tracker = CountingTracker::new(inner, /* fail_after */ 4);
    let first = interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir.clone())], &mut tracker)
        .await;
    assert!(first.is_err(), "must crash after activity 1 completed");
    let one = std::fs::read_to_string(tmp.path().join("one.txt")).expect("one written");
    assert_eq!(one, "one");
    assert!(!tmp.path().join("two.txt").exists(), "activity 2 must not have run");

    let mut resume = VoxDbTracker::new(db, "crash-a");
    resume.expire_lease_for_test().await.expect("expire");
    let journal = interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir)], &mut resume)
        .await
        .expect("resume");
    let one_again = std::fs::read_to_string(tmp.path().join("one.txt")).expect("one still");
    assert_eq!(one_again, "one", "activity 1 must not re-run");
    let two = std::fs::read_to_string(tmp.path().join("two.txt")).expect("two ran");
    assert_eq!(two, "two");
    assert!(journal.iter().any(|e| e["event"] == "WorkflowCompleted"));
}

#[tokio::test]
async fn window_b_started_without_completed_retries() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().to_string_lossy().to_string();
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "crash-b");
    // fail_after tuned so on_activity_started persists and the body/completion faults.
    let mut tracker = CountingTracker::new(inner, /* fail_after */ 3);
    let first = interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir.clone())], &mut tracker)
        .await;
    assert!(first.is_err());

    let mut resume = VoxDbTracker::new(db, "crash-b");
    resume.expire_lease_for_test().await.expect("expire");
    let journal = interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir)], &mut resume)
        .await
        .expect("retry");
    let attempt = journal
        .iter()
        .find(|e| e["activity"].as_str() == Some("write_one") && e.get("resume_attempt").is_some())
        .and_then(|e| e["resume_attempt"].as_u64())
        .unwrap_or(0);
    assert_eq!(attempt, 2, "started-without-completed must retry; got {journal:#?}");
}
```

If `std.fs.write` / `@uses(fs)` is not the live builtin spelling, copy the exact call from an existing `@uses(fs)` golden. Do not invent `fs.write`. Tune `fail_after` by printing tracker call names in `CountingTracker` during the first red run — then pin the number.

Add `tempfile` to `vox-workflow-runtime` `[dev-dependencies]` if missing.

- [ ] **Step 2: Run and watch fail**

```bash
cargo test -p vox-workflow-runtime --test crash_windows
```

Expected: FAIL — `CountingTracker` / `expire_lease_for_test` missing.

- [ ] **Step 3: Implement `CountingTracker` and lease expire**

```rust
// crates/vox-workflow-runtime/src/workflow/counting_tracker.rs
use super::tracker::WorkflowTracker;
use anyhow::anyhow;
use async_trait::async_trait;

pub struct CountingTracker<T> {
    inner: T,
    fail_after: usize,
    calls: usize,
}

impl<T> CountingTracker<T> {
    pub fn new(inner: T, fail_after: usize) -> Self {
        Self {
            inner,
            fail_after,
            calls: 0,
        }
    }

    fn bump(&mut self) -> anyhow::Result<()> {
        self.calls += 1;
        if self.calls > self.fail_after {
            return Err(anyhow!("CountingTracker crash after {} calls", self.fail_after));
        }
        Ok(())
    }
}
```

Forward every `WorkflowTracker` method: `self.bump()?; self.inner.<method>(...).await`.

On `VoxDbTracker`:

```rust
#[cfg(any(test, feature = "test-support"))]
pub async fn expire_lease_for_test(&self) -> anyhow::Result<()> {
    // UPDATE workflow_run_log SET lease_until_ms = 0, lease_owner = NULL WHERE run_id = self.run_id
}
```

Crash-resume already steals when `lease_until_ms < now` (`db_tracker.rs` `next_activity_attempt_start`). Do not mint a second owner and hope the 30s TTL elapses.

- [ ] **Step 4: Run and watch pass; mutation-verify**

```bash
cargo test -p vox-workflow-runtime --test crash_windows
```

Expected: PASS.

Mutation A: skip replay of activity 1 on resume; confirm `one.txt` is rewritten or the “must not re-run” assert fails. Restore.

Mutation B: do not expire the lease; confirm window B fails with a lease error (wrong reason). Restore.

- [ ] **Step 5: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-workflow-runtime
git commit -m "$(cat <<'EOF'
test(workflow): prove crash windows with CountingTracker and expired leases

Window A kills after the first completion so activity 2 still runs on resume.
Window B retries started-without-completed only after the lease is stealable.
EOF
)"
```

---

## Phase 3 — Timers as `HostRequest::Wait`

### Task 3.1: Persist `wake_at_ms`, park, release the lease

Parking is a decision on `HostRequest::Wait`, not a fake `__durable_timer_wait` activity. Shared with Phase 4: status `parked`, clear `lease_owner`. DDL lives on `workflow_run_log` (one table, not a new `workflow_timer_log`):

```sql
    wake_at_ms       INTEGER
    parked_reason    TEXT
```

**Files:**
- Modify: `crates/vox-db/src/schema/domains/execution.rs`
- Modify: `crates/vox-db/src/schema/manifest.rs` + `contracts/db/baseline-version-policy.yaml`
- Modify: `crates/vox-db/src/facade/workflow.rs`
- Modify: `crates/vox-workflow-runtime/src/workflow/run.rs` (`HostRequest::Wait` arm)
- Modify: `crates/vox-workflow-runtime/src/db_tracker.rs` (park clears lease)
- Create: `crates/vox-workflow-runtime/src/workflow/waker.rs`
- Modify: `contracts/workflow/workflow-journal.v1.schema.json` — add `TimerScheduled` and `WorkflowParked` **in this PR**
- Test: `crates/vox-workflow-runtime/tests/durable_timer.rs`

**Interfaces:**
- Consumes: `HostRequest::Wait { deadline_ms }` (relative ms from the Vox call).
- Produces: `tracker.park_run(reason, wake_at_ms)` ; `wake_at_ms = host_now_ms() + deadline_ms`. Waker: `WorkflowWaker::tick(db) -> anyhow::Result<Vec<String>>` of resumed `run_id`s. Resume uses stored `args_json`, never `vec![]`.

Extract a `wall_now_ms()` helper using the **pattern** in `scheduled/runner.rs` (clamp remaining time). Do not copy that runner. `SystemTime` does **not** follow `tokio::time::advance`.

- [ ] **Step 1: Write the failing timer test**

```rust
// crates/vox-workflow-runtime/tests/durable_timer.rs
#![allow(missing_docs)]

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;
use vox_workflow_runtime::workflow::waker::WorkflowWaker;

#[tokio::test]
async fn wait_parks_and_waker_resumes_after_backdate() {
    const SRC: &str = r#"
activity after(n: int) to int { return n + 1 }
workflow delayed(n: int) to int {
    workflow_wait(3_600_000)
    return after(n)
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "timer-1");
    let journal = interpret_workflow_durable(&hir, "delayed", vec![json!(4)], &mut tracker)
        .await
        .expect("park is not a failure");
    assert!(
        journal.iter().any(|e| e["event"] == "WorkflowParked" || e["event"] == "TimerScheduled"),
        "must emit a park/timer event; got {journal:#?}"
    );
    let status = db.workflow_run_status("timer-1").await.expect("status");
    assert_eq!(status, "parked");
    let owner = db.workflow_run_lease_owner("timer-1").await.expect("lease");
    assert!(owner.is_none(), "park must clear lease_owner");

    db.workflow_run_backdate_wake_at_ms("timer-1", 0)
        .await
        .expect("backdate");
    let waker = WorkflowWaker::new(db.clone());
    let resumed = waker.tick().await.expect("tick");
    assert_eq!(resumed, vec!["timer-1".to_string()]);

    let mut resume = VoxDbTracker::new(db, "timer-1");
    let journal = interpret_workflow_durable(&hir, "delayed", vec![], &mut resume)
        .await
        .expect("waker args come from args_json, vec![] is a runner bug if this loses n");
    let done = journal.iter().rev().find(|e| e["event"] == "WorkflowCompleted").unwrap();
    assert_eq!(done["return_value"], json!(5));
}
```

Name facade methods after the ones you add. The resume call may pass `vec![]` **only if** the runner reloads `args_json` when `args` is empty; prefer `tracker.load_run_args()` inside `interpret_workflow_durable` when `args` is empty and a row exists.

Do **not** write a real 5-second sleep. Do **not** claim design gate 4 (crash at 2s of a 5s wait) unless you inject a fake host clock in this same task.

- [ ] **Step 2: Run and watch fail**

```bash
cargo test -p vox-workflow-runtime --test durable_timer
```

Expected: FAIL — Wait arm still parks with a stub reason / no `wake_at_ms`.

- [ ] **Step 3: Implement DDL, Wait arm, waker**

Wait arm:

```rust
HostRequest::Wait { deadline_ms } => {
    let wake_at_ms = wall_now_ms().saturating_add(deadline_ms as u64);
    tracker.park_timer(wake_at_ms).await?;
    journal.push(versioned_event(json!({
        "event": "TimerScheduled",
        "workflow": workflow_name,
        "wake_at_ms": wake_at_ms,
    })));
    journal.push(versioned_event(json!({
        "event": "WorkflowParked",
        "workflow": workflow_name,
        "reason": "timer",
    })));
    host.respond(HostDecision::Park("timer".into()))?;
}
```

`park_timer` UPDATEs `status='parked'`, `wake_at_ms`, `parked_reason='timer'`, `lease_owner=NULL`, `lease_until_ms=NULL`.

`WorkflowWaker::tick`: `SELECT run_id FROM workflow_run_log WHERE status='parked' AND parked_reason='timer' AND wake_at_ms <= ?now`. For each id, set `status='running'` (do not claim the lease here — the resume tracker will). Return the ids.

Admit `TimerScheduled` and `WorkflowParked` in the schema enum in this PR. Extend `journal_schema_conformance` only if you have a source that emits them in that 3/4-arg test; otherwise add a dedicated assert in `durable_timer.rs`.

- [ ] **Step 4: Pass + mutation-verify lease release**

```bash
cargo test -p vox-workflow-runtime --test durable_timer
vox ci data-storage-guard
```

Mutation: leave `lease_owner` set on park; confirm the “park must clear lease_owner” assert fails. Restore.

- [ ] **Step 5: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-db crates/vox-workflow-runtime contracts
git commit -m "$(cat <<'EOF'
feat(workflow): park timers on workflow_run_log and wake by backdated clock

workflow_wait persists wake_at_ms, sets status parked, and clears the lease.
The waker resumes with stored args_json. No in-process sleep.
EOF
)"
```

---

## Phase 4 — Signals as `HostRequest::WaitSignal`

### Task 4.1: Consume-or-park on existing `workflow_signal_log`

Move signal consume **out** of `VoxDbTracker::on_activity_started`. One DB transaction: if an unconsumed row for `(run_id, key)` exists, mark consumed and `Replay` the payload; else park. Add `vox workflow signal`. `record_workflow_signal` has zero production callers today.

**Files:**
- Modify: `crates/vox-workflow-runtime/src/db_tracker.rs`
- Modify: `crates/vox-workflow-runtime/src/workflow/run.rs`
- Modify: `crates/vox-cli/src` workflow command surface (today `drain` / `ls` / `preview`)
- Modify: `contracts/workflow/workflow-journal.v1.schema.json` — add `SignalAwaited` in this PR
- Test: `crates/vox-workflow-runtime/tests/durable_signal.rs`

**Interfaces:**
- Consumes: existing `workflow_signal_log`; `HostRequest::WaitSignal { key }`.
- Produces: `tracker.consume_or_park_signal(key) -> SignalDecision::{Consumed(Value), Parked}`; CLI `vox workflow signal --run-id <id> --key <k> [--payload <json>]`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/vox-workflow-runtime/tests/durable_signal.rs
#![allow(missing_docs)]

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;

#[tokio::test]
async fn missing_signal_parks_the_run() {
    const SRC: &str = r#"
activity after() to int { return 1 }
workflow gated() to int {
    workflow_wait_signal("go")
    return after()
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "sig-1");
    let journal = interpret_workflow_durable(&hir, "gated", vec![], &mut tracker)
        .await
        .expect("park is not failed");
    assert!(
        journal.iter().any(|e| e["event"] == "SignalAwaited" || e["event"] == "WorkflowParked"),
        "got {journal:#?}"
    );
    let status = db.workflow_run_status("sig-1").await.expect("status");
    assert_eq!(status, "parked", "absent signal must park, not fail");
}

#[tokio::test]
async fn late_signal_after_park_completes() {
    const SRC: &str = r#"
activity after() to int { return 1 }
workflow gated() to int {
    workflow_wait_signal("go")
    return after()
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "sig-2");
    let _ = interpret_workflow_durable(&hir, "gated", vec![], &mut tracker)
        .await
        .expect("park");
    db.record_workflow_signal("sig-2", "go", "{}")
        .await
        .expect("signal");
    let mut resume = VoxDbTracker::new(db, "sig-2");
    resume.expire_lease_for_test().await.ok();
    let journal = interpret_workflow_durable(&hir, "gated", vec![], &mut resume)
        .await
        .expect("resume");
    assert!(journal.iter().any(|e| e["event"] == "WorkflowCompleted"));
}
```

- [ ] **Step 2: Run and watch fail**

```bash
cargo test -p vox-workflow-runtime --test durable_signal
```

Expected: FAIL — SQL tracker still `bail!`s in `on_activity_started`, or status is `failed`.

- [ ] **Step 3: Implement consume-or-park and CLI**

SQL (one transaction):

```sql
BEGIN;
SELECT payload_json FROM workflow_signal_log
 WHERE run_id = ? AND signal_key = ? AND consumed_at_ms IS NULL
 LIMIT 1;
-- if row: UPDATE consumed_at_ms = ?now; COMMIT; → Consumed
-- else: UPDATE workflow_run_log SET status='parked', parked_reason='signal',
--              lease_owner=NULL, lease_until_ms=NULL WHERE run_id=?; COMMIT; → Parked
```

Remove the signal `bail!` from `on_activity_started`.

CLI: add `signal` next to existing `vox workflow` subcommands. Call the existing `record_workflow_signal` facade. Register the command in the CLI catalog the same way `drain`/`ls` are registered so `vox ci command-sync` stays green.

Admit `SignalAwaited` in the schema enum in this PR.

- [ ] **Step 4: Pass + mutation-verify TOCTOU**

```bash
cargo test -p vox-workflow-runtime --test durable_signal
```

Mutation: split consume and park into two statements without a transaction; add a test that inserts the signal between them and confirm the test can fail. Restore the transaction. If you cannot race in-process, document the transaction in a comment and assert both outcomes still hold.

- [ ] **Step 5: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-workflow-runtime crates/vox-cli crates/vox-db contracts
git commit -m "$(cat <<'EOF'
feat(workflow): park on missing signals and add vox workflow signal

Consume-or-park is one transaction on the existing workflow_signal_log.
Absent signals set status parked instead of failing the run.
EOF
)"
```

---

## Phase 5 — Identity, SQL patches, duplicate explicit ids

### Task 5.1: Call-site side table, not a wider `HirExpr::Call` tuple

`HirExpr::Call` is `(Box<HirExpr>, Vec<HirArg>, bool, Span)`. Do **not** add `call_ordinal` to that tuple (G21). Stamp ordinals on a `HashMap<Span, u32>` (or a `HirFn` side table) at hook-install time. Identity becomes `(enclosing_fn, ordinal, iteration_index)`. Explicit `with { activity_id }` always wins; a duplicate explicit id in one run is an error. Completed-run resume **refuses**. Code-upgrade tests must use an in-flight (parked or crashed) `run_id`.

**Files:**
- Modify: `crates/vox-compiler/src/eval/activity_hook.rs` (or a new `crates/vox-compiler/src/eval/call_ordinals.rs`)
- Modify: `crates/vox-workflow-runtime/src/workflow/run.rs` (`derive_activity_id`)
- Test: `crates/vox-workflow-runtime/tests/activity_identity.rs` (create)

**Interfaces:**
- Consumes: `HirModule` at `WorkflowHost::spawn`.
- Produces: `pub fn stamp_call_ordinals(hir: &HirModule) -> HashMap<Span, u32>` walking each function body; `derive_activity_id(workflow, name, ordinal, iteration)`.

- [ ] **Step 1: Write the failing tests**

```rust
#![allow(missing_docs)]

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::counting_tracker::CountingTracker;
use vox_workflow_runtime::workflow::{DefaultTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

fn activity_ids(journal: &[serde_json::Value], name: &str) -> Vec<String> {
    journal
        .iter()
        .filter(|e| e["activity"].as_str() == Some(name))
        .filter_map(|e| e["activity_id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn inserted_leading_activity_does_not_rename_the_old_id() {
    const V1: &str = r#"
activity old() to int { return 1 }
workflow wf() to int { return old() }
"#;
    const V2: &str = r#"
activity fresh() to int { return 0 }
activity old() to int { return 1 }
workflow wf() to int {
    let _ = fresh()
    return old()
}
"#;
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "upgrade-1");
    // Crash after `old` completed so the run is in-flight, not completed.
    let mut t1 = CountingTracker::new(inner, /* fail_after: tune on first red run */ 4);
    let first = interpret_workflow_durable(&hir(V1), "wf", vec![], &mut t1).await;
    assert!(first.is_err(), "V1 must stop before WorkflowCompleted");
    let v1_journal = first.err().map(|e| format!("{e:#}")).unwrap_or_default();
    let _ = v1_journal;
    // Re-read persisted rows for `old` rather than depending on a completed journal.
    let v1_ids = db
        .workflow_activity_ids("upgrade-1", "old")
        .await
        .expect("v1 ids");
    assert_eq!(v1_ids.len(), 1, "V1 recorded one old activity");

    let mut t2 = VoxDbTracker::new(db, "upgrade-1");
    t2.expire_lease_for_test().await.expect("expire");
    let j2 = interpret_workflow_durable(&hir(V2), "wf", vec![], &mut t2)
        .await
        .expect("resume V2 on the in-flight run");
    let v2_old = activity_ids(&j2, "old");
    assert_eq!(
        v2_old.first(),
        v1_ids.first(),
        "inserting `fresh` must not rename `old`; V1={v1_ids:?} V2={v2_old:?}"
    );
    assert!(
        j2.iter().any(|e| e["activity"] == "fresh"),
        "V2 must also run the new leading activity"
    );
}

#[tokio::test]
async fn completed_run_resume_is_refused() {
    const SRC: &str = r#"
activity old() to int { return 1 }
workflow wf() to int { return old() }
"#;
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut t1 = VoxDbTracker::new(db.clone(), "done-1");
    let _ = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut t1)
        .await
        .expect("completes");
    let mut t2 = VoxDbTracker::new(db, "done-1");
    let err = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut t2)
        .await
        .expect_err("completed run must refuse");
    assert!(
        format!("{err:#}").contains("completed"),
        "got {err:#}"
    );
}

#[tokio::test]
async fn duplicate_explicit_activity_id_is_an_error() {
    const SRC: &str = r#"
activity step(n: int) to int { return n }
workflow wf() to int {
    let a = step(1) with { activity_id: "same" }
    let b = step(2) with { activity_id: "same" }
    return a + b
}
"#;
    let mut tracker = DefaultTracker;
    let err = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect_err("duplicate explicit id");
    assert!(format!("{err:#}").contains("same"));
}
```

Add `workflow_activity_ids(run_id, activity_name)` to the `vox-db` facade in this task if it does not exist (`SELECT activity_id FROM workflow_activity_log WHERE run_id=? AND activity_name=?`). Tune `fail_after` the same way as Task 2.1.

- [ ] **Step 2: Run and watch fail**

```bash
cargo test -p vox-workflow-runtime --test activity_identity
```

Expected: FAIL — per-name counters still shift when `fresh` is inserted, and duplicate explicit ids silently alias.

- [ ] **Step 3: Implement side-table ids**

Walk each `HirFn` body, assign `ordinal += 1` at every `HirExpr::Call` / `WorkflowVersion` / wait builtin. Key by `Span`. The hook looks up `callee_span` (pass it through `HostCall::Begin` as `span: Span` if needed — adding a field to `HostCall` is fine; adding a field to `HirExpr::Call` is not).

`derive_activity_id` becomes BLAKE3 of `workflow \0 enclosing_fn \0 ordinal \0 iteration`. Track loop iteration on the interpreter (increment on `for`/`while` header, include in Begin).

On `Begin`, if `options.activity_id` is set, use it; if that string was already used in this run, `bail!`.

Completed-run policy in `interpret_workflow_durable`: if `tracker.run_status() == "completed"`, `anyhow::bail!("run already completed")`.

- [ ] **Step 4: Pass + mutation-verify**

Temporarily keep per-name counters; confirm the insert test FAILS. Restore.

- [ ] **Step 5: Commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-compiler crates/vox-workflow-runtime
git commit -m "$(cat <<'EOF'
feat(workflow): identify activities by per-function call ordinals

Inserting a leading activity must not rename later ids. Explicit
activity_id still wins; duplicates in one run error. Do not widen
the HirExpr::Call tuple.
EOF
)"
```

### Task 5.2: `workflow_patch_log` via the data-storage pipeline

`VoxDbTracker` inherits no-op `record_workflow_patch` / `load_workflow_patch`. Schema green in Phase 0 does not mean patches survive resume.

**Files:**
- Modify: `crates/vox-db/src/schema/domains/execution.rs`
- Modify: `crates/vox-db/src/schema/manifest.rs` + `contracts/db/baseline-version-policy.yaml` + `contracts/db/retention-policy.yaml`
- Modify: `crates/vox-db/src/facade/workflow.rs`
- Modify: `crates/vox-workflow-runtime/src/db_tracker.rs`
- Test: `crates/vox-workflow-runtime/tests/workflow_patch.rs` (extend)

**Interfaces:**
- Consumes: `handle_workflow_patch` (already called from Task 1.4).
- Produces: table `workflow_patch_log(workflow_name, change_id, version, recorded_at_ms)` with PK `(workflow_name, change_id)`.

- [ ] **Step 1: Write the failing SQL resume test**

Append to `workflow_patch.rs`:

```rust
#[tokio::test]
async fn sql_tracker_replays_a_recorded_patch() {
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut t1 = VoxDbTracker::new(db.clone(), "patch-sql");
    let j1 = interpret_workflow_durable(&hir_with_version(), "wf", vec![], &mut t1)
        .await
        .expect("first");
    assert!(j1.iter().any(|e| e["event"] == "WorkflowPatch" && e["replayed"] == false));

    let mut t2 = VoxDbTracker::new(db, "patch-sql-2");
    // Same workflow_name, new run still loads the patch by (workflow, change_id).
    let j2 = interpret_workflow_durable(&hir_with_version(), "wf", vec![], &mut t2)
        .await
        .expect("second");
    assert!(
        j2.iter().any(|e| e["event"] == "WorkflowPatch" && e["replayed"] == true),
        "VoxDbTracker must persist patches; got {j2:#?}"
    );
}
```

- [ ] **Step 2: Run and watch fail**

```bash
cargo test -p vox-workflow-runtime --test workflow_patch -- sql_tracker_replays_a_recorded_patch
```

Expected: FAIL — no table / no-op persist.

- [ ] **Step 3: Add the table and implement tracker methods**

```sql
CREATE TABLE IF NOT EXISTS workflow_patch_log (
    workflow_name   TEXT NOT NULL,
    change_id       TEXT NOT NULL,
    version         INTEGER NOT NULL,
    recorded_at_ms  INTEGER NOT NULL,
    PRIMARY KEY (workflow_name, change_id)
);
```

Bump baseline, retention row (`keep_forever` or `manual`), facade insert/select, `VoxDbTracker::{record,load}_workflow_patch`. Run `vox ci data-storage-guard`.

- [ ] **Step 4: Pass + commit**

```bash
cargo test -p vox-workflow-runtime --test workflow_patch
vox run scripts/fmt.vox
git add crates/vox-db crates/vox-workflow-runtime contracts/db
git commit -m "$(cat <<'EOF'
feat(workflow): persist workflow.version patches in SQL

VoxDbTracker inherited no-op patch methods. Patches now survive resume
via workflow_patch_log.
EOF
)"
```

---

## Phase 6 — Generated path is the same engine

### Task 6.1: Tracker, env, sql features, `__vox_run_workflow`, delete `journal::execute`

Generated workflows already call `interpret_workflow_durable`. This task stops lying about how.

**Files:**
- Modify: `crates/vox-codegen/src/codegen_rust/emit/durability_lower.rs`
- Modify: `crates/vox-codegen/src/codegen_rust/emit/mod.rs` (generated `Cargo.toml`)
- Modify: `crates/vox-codegen/src/codegen_rust/emit/http.rs` (define or stop calling `__vox_run_workflow`)
- Modify: `contracts/config/env-vars.v1.yaml` (`VOX_WORKFLOW_RUN_ID`)
- Modify: `crates/vox-codegen/tests/durability_compiles.rs`
- Modify: `crates/vox-codegen/tests/durability_lowering.rs`
- Modify: `crates/vox-workflow-runtime/src/journal/execute.rs` (delete with its tests)
- Test: `crates/vox-codegen/tests/adr021_parity.rs` (create)

**Interfaces:**
- Consumes: 4-arg `interpret_workflow_durable`; `VoxDbTracker` behind `feature = "sql"`.
- Produces: generated workflow fn that constructs `VoxDbTracker` from `VOX_WORKFLOW_RUN_ID` (or a fresh id), passes generated args, and returns `extract_terminal_return::<T>(&journal)`. `__vox_run_workflow(name, args)` match dispatcher over every `workflow` in the module.

- [ ] **Step 1: Write the failing ADR-021 gate**

Create `crates/vox-codegen/tests/adr021_parity.rs`. Reuse the temp-crate + `CARGO_TARGET_DIR` pattern in `crates/vox-codegen/tests/emit_compile_harness.rs` (`generate_script`, `inject_workspace_patches`, shared target dir, `--config build.rustc-wrapper=""`). The test must **emit, `cargo build`, and `cargo run`** the generated binary — not call `interpret_workflow_durable` twice.

```rust
#![allow(missing_docs)]

use serde_json::Value;
use std::process::Command;
use vox_codegen::codegen_rust::emit::emit_fn;
use vox_codegen::codegen_rust::generate_script;
use vox_compiler::hir::{DurabilityKind, lower_module};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::parser::parse_script;
use vox_compiler::lexer::lex as lex_script;

const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Ok("tx") }
workflow checkout(amount: int) to Result[str] {
    workflow.version("add-audit-v2", 1, 2)
    return charge(amount)
}
"#;

#[test]
fn emit_contains_dispatcher_and_sql_tracker() {
    let module = parse(lex(SRC)).expect("parse");
    let hir = lower_module(&module);
    let func = hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Workflow))
        .expect("workflow");
    let rust = emit_fn(func, Some(&hir.inferred_types), &[]);
    assert!(
        rust.contains("VoxDbTracker"),
        "generated workflow must construct VoxDbTracker; got:\n{rust}"
    );
    assert!(
        !rust.contains("journal::execute"),
        "activity/workflow emit must not wrap journal::execute; got:\n{rust}"
    );
}

#[test]
fn generated_crate_defines_vox_run_workflow_and_sql_feature() {
    let module = parse_script(lex_script(SRC)).expect("parse");
    let hir = lower_module(&module);
    let runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vox-actor-runtime");
    let output = generate_script(&hir, "vox-adr021-emit", Some(&runtime)).expect("emit");
    let joined = output.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        joined.contains("fn __vox_run_workflow"),
        "emit_main already calls __vox_run_workflow; define it. got {} chars",
        joined.len()
    );
    let manifest = output
        .files
        .get("Cargo.toml")
        .or_else(|| {
            output
                .files
                .iter()
                .find(|(k, _)| k.ends_with("Cargo.toml"))
                .map(|(_, v)| v)
        })
        .expect("manifest");
    assert!(
        manifest.contains("features") && manifest.contains("sql"),
        "generated Cargo.toml must enable vox-workflow-runtime sql; got:\n{manifest}"
    );
    assert!(
        manifest.contains("vox-db"),
        "generated Cargo.toml must depend on vox-db for VoxDbTracker"
    );
}

#[tokio::test]
async fn generated_binary_journal_matches_interpreter() {
    // Copy compile_vox_script from emit_compile_harness.rs (same lock + target dir).
    // After `cargo build` succeeds, `cargo run` with:
    //   VOX_RUN_WORKFLOW=checkout
    //   VOX_WORKFLOW_RUN_ID=adr021-gen
    //   argv or env for amount=5
    // Generated main must print the journal JSON to stdout.
    //
    // Interp side (same process, different run_id so the two trackers do not
    // share rows — we compare *shape*, not the same SQL rows):
    let module = parse(lex(SRC)).expect("parse");
    let hir = lower_module(&module);
    let mut tracker = vox_workflow_runtime::workflow::DefaultTracker;
    let interp_journal = vox_workflow_runtime::workflow::interpret_workflow_durable(
        &hir,
        "checkout",
        vec![serde_json::json!(5)],
        &mut tracker,
    )
    .await
    .expect("interp");

    let gen_stdout = run_generated_workflow(SRC, "checkout", "adr021-gen", &[("amount", "5")])
        .expect("generated binary ran");
    let gen_journal: Vec<Value> = serde_json::from_str(&gen_stdout).expect("journal json");

    let ids = |j: &[Value]| -> Vec<String> {
        j.iter()
            .filter_map(|e| e.get("activity_id").and_then(Value::as_str).map(str::to_string))
            .collect()
    };
    assert_eq!(
        ids(&interp_journal),
        ids(&gen_journal),
        "ADR-021: same activity_id set\ninterp={interp_journal:#?}\ngen={gen_journal:#?}"
    );
    let ret = |j: &[Value]| {
        j.iter()
            .rev()
            .find(|e| e["event"] == "WorkflowCompleted")
            .and_then(|e| e.get("return_value"))
            .cloned()
    };
    assert_eq!(ret(&interp_journal), ret(&gen_journal));
}

fn run_generated_workflow(
    src: &str,
    workflow: &str,
    run_id: &str,
    _args: &[(&str, &str)],
) -> Result<String, String> {
    use std::path::PathBuf;
    use std::sync::Mutex;
    static COMPILE_LOCK: Mutex<()> = Mutex::new(());
    let module = parse_script(lex_script(src)).map_err(|e| format!("{e:?}"))?;
    let hir = lower_module(&module);
    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vox-actor-runtime");
    let output = generate_script(&hir, "vox-adr021-run", Some(&runtime))
        .map_err(|e| format!("emit: {e}"))?;
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    output
        .write_to_dir(dir.path())
        .map_err(|e| format!("write: {e}"))?;
    let _guard = COMPILE_LOCK.lock().unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target = std::env::temp_dir().join("vox-emit-harness-target");
    let out = Command::new(&cargo)
        .current_dir(dir.path())
        .args(["run", "--quiet", "--config", "build.rustc-wrapper=\"\""])
        .env("CARGO_TARGET_DIR", &target)
        .env("VOX_RUN_WORKFLOW", workflow)
        .env("VOX_WORKFLOW_RUN_ID", run_id)
        .env_remove("RUSTC_WRAPPER")
        .output()
        .map_err(|e| format!("spawn: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
```

Generated `main` must print the journal JSON on `VOX_RUN_WORKFLOW` (today it calls the undefined `__vox_run_workflow`). A test that only compares two `interpret_workflow_durable` calls is rejected at review. First red run: emit lacks the dispatcher / `VoxDbTracker` / stdout journal.

- [ ] **Step 2: Run and watch fail**

```bash
cargo test -p vox-codegen --test adr021_parity
cargo test -p vox-codegen --test durability_compiles
```

Expected: FAIL — `__vox_run_workflow` is called from `http.rs` ~600 and never defined; manifest is `default-features = false` without `sql`; activities still wrap `journal::execute`.

- [ ] **Step 3: Implement emit**

1. Register `VOX_WORKFLOW_RUN_ID` in `contracts/config/env-vars.v1.yaml` (string, optional, “durable run id for generated workflow binaries”).
2. In generated `Cargo.toml` for workflow-only modules: `vox-workflow-runtime = { ..., features = ["sql"] }` and `vox-db`.
3. `emit_workflow_body`: construct `VoxDbTracker` from `std::env::var("VOX_WORKFLOW_RUN_ID")` or `Uuid`; pass workflow args as `vec![ /* each param via to_journal_json equivalent */ ]`.
4. Emit `__vox_run_workflow(name: &str, args: &[Value])` that matches on workflow name and calls the generated wrapper. `http.rs` already calls it — define it in `durability_lower.rs` or a sibling emit file included from `emit_main`.
5. Stop wrapping activities in `journal::execute`. Delete `crates/vox-workflow-runtime/src/journal/execute.rs` and `journal/test_support` in **this** task. Update `durability_compiles.rs`, `durability_lowering.rs`, and any `journal_execute` test so they no longer require the symbol.
6. Stop emitting `compile_error!` for `WorkflowVersion` in codegen.

- [ ] **Step 4: Run gates**

```bash
cargo test -p vox-codegen --test adr021_parity
cargo test -p vox-codegen --test durability_compiles
cargo test -p vox-codegen --test durability_lowering
cargo test -p vox-workflow-runtime --lib
```

Expected: PASS. The parity test fails if you only call `interpret_workflow_durable` twice — mutation-verify by reverting the emit of `VoxDbTracker` and confirming `adr021_parity` goes red.

- [ ] **Step 5: Downgrade leftover ADR-041 codegen “Stable” wording** if any remains after Task 0.2, then commit

```bash
vox run scripts/fmt.vox
git add crates/vox-codegen crates/vox-workflow-runtime contracts/config
git commit -m "$(cat <<'EOF'
feat(codegen): emit VoxDbTracker, __vox_run_workflow, and a real ADR-021 gate

Generated workflows already called the interpreter; they now persist through
SQL, link the dispatcher emit_main already calls, and prove parity by
compiling the generated fn instead of invoking interp twice.
EOF
)"
```

---

## Phase 7 — Deferred

Transactional outbox is **not** in this plan.

---

## Self-review (spec coverage)

| Spec requirement | Task |
|---|---|
| Admit only today's missing journal names | 0.1 |
| ADR + doc honesty; research-index | 0.2 |
| Strict `__vox` encode/decode; `Interpreter: !Send` tripwire; extract | 1.1 |
| Intercept after callee eval; With; Version; waits; no Retry-on-Err | 1.2 |
| Host JSON bridge + Drop | 1.3 |
| Runner rewrite; Patch; mesh loud-error; `args_json`; callsites | 1.4 |
| Delete linearizer after 1.4 green | 1.5 |
| CountingTracker windows + expired lease + `@uses(fs)` | 2.1 |
| `HostRequest::Wait`; park clears lease; backdate waker | 3.1 |
| `HostRequest::WaitSignal`; consume-or-park; CLI `signal` | 4.1 |
| Side-table ordinals; duplicate explicit id; refuse completed resume | 5.1 |
| `workflow_patch_log` via data-storage pipeline | 5.2 |
| sql features; env var; `__vox_run_workflow`; delete `journal::execute`; real ADR-021 | 6.1 |
| Outbox | Phase 7 deferred |

No `Retry` decision. No `HirExpr::Call` 5-tuple. No pre-admitted Phase 3–4 event names in Task 0.1. No `Co-Authored-By: Claude Opus 5`.
