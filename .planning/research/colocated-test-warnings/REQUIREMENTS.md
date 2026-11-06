# Requirements: Colocated Test Warnings

**Status:** Proposal. Not the active milestone.
**Active milestone stays v5.7** (`.planning/ROADMAP.md`, phases 324–331). Do not edit those files.
**Granularity:** standard (4–6 phases).
**Phase IDs:** sequential, local to this proposal (`Phase 1`…). These are not workspace phase numbers.

## v1

### Colocation

- **COL-01**: A `.vox` file can hold behavior and its tests together. `@test fn` in the same file is the unit-test form. Production lowering already drops those decls when `strip_tests` is set (`hir/lower/mod.rs`). This milestone documents and preserves that split. It does not add a second test syntax.
  - Note: `@test` stays a decorator on `fn`. Do not add a bare `test` keyword. The flat spelling is `@test fn name() { ... }` on one declaration. A `test_` name prefix is not a test marker.
- **COL-02**: A same-file `@test` is sufficient for a single-function behavior. A test in another file may also satisfy a warning when it names the symbol and carries a strong oracle. Same-file is the default suggestion, not a quota.

### Selective warnings

- **SEL-01**: `vox check` warns when a behavior-bearing declaration has no strong oracle. Behavior-bearing means exported or public logic with a branch, a computation, or a deterministic error path (`pub fn`, `@pure fn`, `query` / `mutation` / `server` / `activity`). It does not mean every `fn`.
- **SEL-02**: No warning for `main`, accessors, generated decls, type-only decls, one-line passthroughs, `component` markup with no logic, or `scripts/*.vox`.
- **SEL-03**: An author silences a specific declaration with `@untested(reason: "...")`. The reason is required. The diagnostic tells an agent to leave that annotation alone.
- **SEL-04**: Severity is warning. Existing files are baselined: a declaration that already existed without a test does not flood the report. A newly added behavior-bearing declaration warns. CI does not fail the workspace on this warning. An opt-in config may promote it later. This exists because `skeleton/untested-pub-api` already whole-file-scanned legacy modules and produced suppression spam.

### Strong oracles

- **ORC-01**: A test clears a missing-test warning only when it calls the behavior with a concrete input and asserts an expected value, error, or state change.
- **ORC-02**: These do not clear the warning, and they raise `vox/test/weak-oracle`: `assert(true)`, tautologies (`assert(x == x)`), a call with no assertion, and an assertion that would still pass if the function returned a different value of the same shape.
- **ORC-03**: When a test already calls the function but the assertion is too loose, the diagnostic says to tighten that assertion. It does not say to add a second test.

### Flakes do not count

- **FLA-01**: A `@test` that reads the clock, draws randomness, sleeps, or hits the network does not clear a missing-test warning. Reuse the idea of `vox/test/flaky-wait-and-unreasoned-skip`, `vox/test/wall-clock`, and `vox/test/in-loop`, extended to `.vox` tests.
- **FLA-02**: A test that fails and then passes with no source change is recorded flaky and stops counting as an oracle until it is stable on two consecutive runs.

### Hourglass shape

Google's hourglass is many small tests plus many end-to-end tests and almost no medium tests (`Just Say No to More End-to-End Tests`, 2015; `Fixing a Test Hourglass`, 2020). The 70/20/10 split is a first guess, not a CI quota.

- **SHAPE-01**: Each test is classified small (colocated, no clock/net/process), medium (multi-declaration, hermetic fakes), or large (UI, real process, network, full `vox run` of a multi-surface flow). `vox test` prints the three counts.
- **SHAPE-02**: A large test never clears a warning on a `@pure` or single-function declaration. The diagnostic names the layer that can observe the behavior: small for one function, medium for a cross-declaration contract. It does not suggest an end-to-end test.
- **SHAPE-03**: The report warns when small and large both grow while medium stays flat. That warning is informational. It does not fail CI on a numeric ratio.

### Agent reminder

- **LLM-01**: The diagnostic is the reminder. It names the symbol, why it qualifies, the one oracle that would clear it, and a line that forbids a touch test and forbids a new end-to-end test as the fix.
- **LLM-02**: `AGENTS.md` Test-First stops telling models to add a same-file test for every new Rust `pub fn` and every golden `fn`. Models act when this diagnostic fires, and they follow its oracle text. `skeleton/untested-pub-api` and `skeleton/no-test-for-pub-fn` stay as presence checks until this warning replaces their "any call counts" rule for `.vox`.
- **LLM-03**: A weak or flaky `@test` is not a positive MENS training example. `r_test` does not reward it.

### Opt-in mutant check

- **MUT-01**: An opt-in command (not `vox check`) flips one comparison or replaces a function body with a default, reruns only the tests that claim to cover that function, and refuses to count them as an oracle if they still pass. Equivalent mutants can be skipped with a reason, same spirit as `#[mutants::skip]`. `cargo mutants` on compiler/codegen PRs stays as it is.

## v2 (out of this proposal)

- Promoting the warning to a CI error by default.
- A numeric pyramid quota (any fixed 70/20/10 gate).
- Mutation testing on every check or on the whole workspace.
- Forcing tests on private helpers, generated code, or UI markup.
- GUI dashboards for the ratio.

## Traceability

Every v1 id maps to exactly one phase in this proposal's `ROADMAP.md`. These phases are local to the proposal. The live milestone remains v5.7.

| Requirement | Phase | Status |
|-------------|-------|--------|
| COL-01 | Phase 1 | Pending |
| COL-02 | Phase 1 | Pending |
| SEL-01 | Phase 2 | Pending |
| SEL-02 | Phase 2 | Pending |
| SEL-03 | Phase 2 | Pending |
| SEL-04 | Phase 2 | Pending |
| ORC-01 | Phase 1 | Pending |
| ORC-02 | Phase 1 | Pending |
| ORC-03 | Phase 1 | Pending |
| FLA-01 | Phase 3 | Pending |
| FLA-02 | Phase 3 | Pending |
| SHAPE-01 | Phase 4 | Pending |
| SHAPE-02 | Phase 4 | Pending |
| SHAPE-03 | Phase 4 | Pending |
| LLM-01 | Phase 2 | Pending |
| LLM-02 | Phase 5 | Pending |
| LLM-03 | Phase 5 | Pending |
| MUT-01 | Phase 1 | Pending |
