# Research: colocated test warnings

**Date:** 2026-11-02
**Scope:** How Vox can warn when behavior lacks a real test, without paying people or models to sprinkle touch tests.

## What already exists

- `@test fn` parses in the same file as other decls (`parser/descent/decl/head.rs` `parse_test`). Lowering pushes it to `hir.tests` and drops it when `strip_tests` is set (`hir/lower/mod.rs`). Colocation is already the language form. D's `unittest` and Zig's `test` blocks are the same idea; Rust uses `#[cfg(test)] mod tests` in the same file and keeps integration tests outside.
- `skeleton/untested-pub-api` (Warning, Rust only) fires when a non-tiny library file has a `pub fn` and no `#[test]` / `#[cfg(test)]` module, unless a sibling `*_tests.rs` or a `tests/` file merely contains the function name. The pre-commit `tdd-guard` blocks new hits. It checks presence, not assertion strength. Phase 6.3 and phase 03.07 summaries record agents adding a small inline test, or fighting the hook, solely to commit.
- `skeleton/no-test-for-pub-fn` (Warning) applies only to `examples/golden/**/*.vox`. A function counts as tested when its name is called inside an `@test` block. `assert(true)` satisfies the detector. User projects are not checked.
- Flake-shaped Rust/TS tests are already flagged: `vox/test/flaky-wait-and-unreasoned-skip`, `vox/test/wall-clock`, `vox/test/in-loop`. Those detectors do not look at `.vox` `@test` blocks and do not judge assertion strength.
- PRs that touch `vox-compiler` or `vox-codegen` run `cargo mutants`. That is a Rust mutation run, not a `.vox` oracle, and it is not on the default check path.
- `AGENTS.md` Test-First tells every agent to add a same-file test for every new Rust `pub fn` and every new golden `fn`. Planned GRPO weight `r_test` is 30%. A weak test therefore becomes a positive training example.

## Prior art that should constrain the design

- **Hourglass is a shape to notice, not a quota to hit.** Google Testing Blog (2015-04, "Just Say No to More End-to-End Tests") defines the hourglass as many unit tests and many end-to-end tests with few integration tests between them. The 70/20/10 unit/integration/end-to-end split is "a good first guess," and the article says the mix differs by team. The 2020-11 post "Fixing a Test Hourglass" fixes the shape by adding medium tests with fakes and deleting end-to-end tests that duplicate them. Ham Vocke's Practical Test Pyramid (martinfowler.com) makes the same point: more high-level tests means a slower, flakier suite.
- **Coverage is the wrong signal.** cargo-mutants documents that a test can reach a line and still miss a behavior change (a function replaced by `Ok(())` still "passes"). Flaky or non-hermetic tests make mutation results lie. The useful outcome is "missed," and the follow-up is a public-behavior assertion, not a test aimed at the mutant.
- **Weak assertion vs missing case.** Testland's mutant-survival triage splits those two: if a test already ran the input that separates the mutant, tighten that assertion; do not add a second test. A flaky test is not a mutation finding. No-coverage is not a weak-assertion finding.
- **Rust does not warn on untested public APIs.** Clippy's `cfg_not_test` warns about `#[cfg(not(test))]` hiding code from the test build (false coverage). It does not demand a test per function. That matches "do not force tests on everything."
- **D's best practice** is a `unittest` immediately after the function it checks, compiled out of normal builds. Vox already lowers `@test` onto a separate `hir.tests` list.

## Implications for the roadmap

1. Do not build a new test syntax. Preserve `@test` plus `strip_tests`.
2. The first delivery is "what counts as an oracle," because today's presence checks are what create touch tests.
3. Warnings are selective and baselined. A legacy wall of warnings will be suppressed or answered with `assert(true)`.
4. Hourglass maintenance is a classification plus a refusal: a large test cannot close a small-scope warning, and the three counts are printed. No numeric ratio gate.
5. The diagnostic text is the LLM reminder. Narrow `AGENTS.md` so the standing order is no longer "test every pub fn."
6. Keep mutation opt-in and single-function. Do not put it on `vox check`.
7. A tombstoned note exists at `docs/src/archive/research-2026-q1/ai-augmented-testing-hourglass-research-2026.md`. It was not read. Do not mine `docs/src/archive/`.

## Non-goals

Forcing a test on every declaration. Rewarding call-only or tautological tests. Using an end-to-end or GUI test to silence a pure-function warning. Failing CI on a 70/20/10 ratio. A new dashboard.
