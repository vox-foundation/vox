# Roadmap: Colocated Test Warnings

## Overview

This is a proposal. The live milestone remains v5.7 (Python bridge, phases 324–331). Phase numbers here are local to this proposal (`Phase 1`, `Phase 2`, …). They are not workspace phases and must not be numbered 332 or higher.

`vox check` warns when a newly added behavior-bearing `.vox` declaration has no strong oracle. The only unit-test declaration is the existing `@test fn`. A call inside `@test`, `assert(true)`, a tautology, a same-shape assertion, a flake, or a large end-to-end run leaves the warning in place. The diagnostic names the one tighter assertion that would clear it. Legacy declarations stay quiet, CI stays green on this warning, and small/medium/large counts are printed as information.

## Locked decision

`@test` stays a decorator on `fn`. Do not add a bare `test` keyword. The flat spelling is `@test fn name() { ... }` on one declaration. A `test_` name prefix is not a test marker.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): planned proposal work, local to this directory
- Decimal phases (2.1, 2.2): urgent insertions (none planned)

- [ ] **Phase 1: Strong Oracles** - A test counts only when it asserts a concrete result, and mutation stays opt-in
- [ ] **Phase 2: Selective Warnings** - `vox check` warns on new behavior that lacks that oracle, and legacy files stay quiet
- [ ] **Phase 3: Flakes Excluded** - Clock, randomness, sleep, network, and fail-then-pass stop counting
- [ ] **Phase 4: Hourglass Shape** - `vox test` prints size counts, and a large test cannot close a single-function warning
- [ ] **Phase 5: Agent Alignment** - Standing orders and training follow the diagnostic

## Phase Details

### Phase 1: Strong Oracles
**Goal**: A behavior counts as tested only when a `@test fn` calls it with a concrete input and asserts an expected value, error, or state change.
**Depends on**: Nothing (first phase)
**Requirements**: COL-01, COL-02, ORC-01, ORC-02, ORC-03, MUT-01
**Success Criteria** (what must be TRUE):
  1. `vox check` accepts `@test fn name() { ... }` in the same file as the behavior. A bare `test` keyword is not a declaration. A `test_` name prefix is not a test marker. Production lowering with `strip_tests` still drops those declarations.
  2. Coverage is accepted when that `@test fn` calls the behavior with a concrete input and asserts an expected value, error, or state change. Same-file is the default suggestion. A test in another file is accepted when it names the symbol and carries that oracle. There is no quota on how many tests a file holds.
  3. `vox check` raises `vox/test/weak-oracle` for `assert(true)`, a tautology (`assert(x == x)`), a call with no assertion, and an assertion that still passes if the function returned a different value of the same shape. Those tests leave a missing-test warning in place.
  4. When a test already calls the function and the assertion is too loose, the diagnostic tells the author to tighten that assertion. It does not tell them to add a second test.
  5. An opt-in command, separate from `vox check`, flips one comparison or replaces one function body with a default, reruns only the tests that claim that function, and refuses to count them when they still pass. An equivalent mutant can be skipped with a reason. `cargo mutants` on compiler and codegen pull requests stays as it is.
**Plans**: TBD

### Phase 2: Selective Warnings
**Goal**: `vox check` warns when a new behavior-bearing declaration has no strong oracle, and stays quiet for everything else.
**Depends on**: Phase 1
**Requirements**: SEL-01, SEL-02, SEL-03, SEL-04, LLM-01
**Success Criteria** (what must be TRUE):
  1. `vox check` warns when a new exported or public declaration carries a branch, a computation, or a deterministic error path (`pub fn`, `@pure fn`, `query`, `mutation`, `server`, `activity`) and has no Phase 1 oracle. A declaration that is not behavior-bearing draws no warning.
  2. `vox check` stays silent for `main`, accessors, generated declarations, type-only declarations, one-line passthroughs, `component` markup with no logic, and `scripts/*.vox`.
  3. `@untested(reason: "...")` silences that one declaration. The reason is required. The diagnostic tells an agent to leave the annotation alone.
  4. Severity is warning. A declaration that already existed without a test does not appear in the flood. A newly added behavior-bearing declaration warns. The workspace CI run stays green on this warning. Promotion to an error remains opt-in and off by default.
  5. The diagnostic names the symbol, why it qualifies, and the one oracle that would clear it, and it forbids a touch test and a new end-to-end test as the fix.
**Plans**: TBD

### Phase 3: Flakes Excluded
**Goal**: A non-hermetic or unstable `@test` does not count as an oracle.
**Depends on**: Phase 2
**Requirements**: FLA-01, FLA-02
**Success Criteria** (what must be TRUE):
  1. A `@test` that reads the clock, draws randomness, sleeps, or hits the network leaves the missing-test warning in place. The same idea as `vox/test/flaky-wait-and-unreasoned-skip`, `vox/test/wall-clock`, and `vox/test/in-loop` applies to `.vox` tests.
  2. A test that fails and then passes with no source change is recorded flaky and stops counting as an oracle.
  3. That test counts again only after it is stable on two consecutive runs.
**Plans**: TBD

### Phase 4: Hourglass Shape
**Goal**: Test size is visible, and a large test cannot close a warning on a `@pure` or single-function declaration.
**Depends on**: Phase 3
**Requirements**: SHAPE-01, SHAPE-02, SHAPE-03
**Success Criteria** (what must be TRUE):
  1. `vox test` prints small, medium, and large counts. Small means colocated and free of clock, network, and process. Medium means multi-declaration with hermetic fakes. Large means a graphical surface, a real process, the network, or a full `vox run` of a multi-surface flow. The counts are informational.
  2. A large test leaves the warning in place on a `@pure` or single-function declaration. The diagnostic names small for one function and medium for a cross-declaration contract. It does not suggest an end-to-end test.
  3. The report warns when the small count and the large count both grow while the medium count stays flat. That warning is informational. CI does not fail on a numeric ratio.
**Plans**: TBD

### Phase 5: Agent Alignment
**Goal**: An agent following project instructions acts on this diagnostic, and a weak or flaky test is not a positive training example.
**Depends on**: Phase 4
**Requirements**: LLM-02, LLM-03
**Success Criteria** (what must be TRUE):
  1. `AGENTS.md` Test-First tells models to act when this diagnostic fires and to follow its oracle text. It no longer tells them to add a same-file test for every new Rust `pub fn` and every golden `fn`.
  2. `skeleton/untested-pub-api` and `skeleton/no-test-for-pub-fn` remain presence checks. For `.vox`, `vox check` uses this warning's oracle rule in place of "any call counts."
  3. A weak or flaky `@test` is not a positive MENS training example. `r_test` does not reward it.
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Strong Oracles | 0/TBD | Not started | - |
| 2. Selective Warnings | 0/TBD | Not started | - |
| 3. Flakes Excluded | 0/TBD | Not started | - |
| 4. Hourglass Shape | 0/TBD | Not started | - |
| 5. Agent Alignment | 0/TBD | Not started | - |

## Deferred (v2)

Kept out of this proposal: CI error by default, any numeric pyramid quota, mutation on every `vox check` or on the whole workspace, forcing tests on private helpers, generated code, or markup, and a ratio dashboard.
