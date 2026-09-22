---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
# Testing Patterns

**Analysis Date:** 2026-09-22

Two test stacks: Rust (`cargo`/`nextest`, workspace-wide, dominant by volume — 744 files under `crates/*/tests/`, 1358 `#[tokio::test]` occurrences) and TypeScript (`vitest` for unit/component, `Playwright` for e2e, under `crates/vox-gui/ui/`).

## Test Framework

**Rust runner:**

- `cargo test` / `cargo nextest run` (nextest is used for CI tiers — see `contracts/budgets/test-tier-budgets.v1.yaml` and `docs/src/contributors/local-ci-pre-push.md`).
- No custom test config file; standard `#[test]` / `#[tokio::test]` attributes, config lives in each crate's `Cargo.toml` `[dev-dependencies]` / `[[test]]` blocks (e.g. `crates/vox-search/Cargo.toml` gates `crawler_test` behind `required-features = ["web-scrape"]`).

**Rust assertion style:** stdlib `assert!` / `assert_eq!` / `.expect("message")` — no third-party assertion crate.

**TS runner:**

- `vitest` for unit/component tests — config `crates/vox-gui/ui/vitest.config.ts`. Includes `src/**/*.{test,spec}.{ts,tsx}` plus `e2e/lib/**/*.test.ts` and `e2e/review/**/*.test.ts` (pure-logic e2e helpers get vitest coverage even though the specs themselves run under Playwright). Excludes `e2e/*.spec.ts` explicitly so Playwright specs are never double-collected. Setup file: `crates/vox-gui/ui/src/test-setup.ts`.
- `@testing-library/react` + `@testing-library/user-event` + `@testing-library/jest-dom` for component tests; `@vitest-environment jsdom` pragma at the top of DOM-touching test files (see `crates/vox-gui/ui/src/components/ui/Button.test.tsx`).
- `@playwright/test` for e2e — config `crates/vox-gui/ui/playwright.config.ts`, specs under `crates/vox-gui/ui/e2e/`.

**Run commands:**

```bash

# Rust — whole workspace

cargo test --workspace
cargo nextest run                       # CI-equivalent runner
cargo test -p vox-db                    # single crate
cargo test -p vox-search --features web-scrape --test crawler_test   # gated integration test

# Rust — local CI gate tiers (see AGENTS.md §Local CI Gate Tiers)

vox ci pre-push                         # fast tier (no tests/clippy)
vox ci pre-push --full                  # native gate tiers incl. tests
vox ci pre-push --act                   # full GitHub workflow suite via act/Docker

# TypeScript (from crates/vox-gui/ui/)

pnpm test                               # vitest run (unit/component)
pnpm test:e2e                           # playwright test
pnpm typecheck                          # tsc --noEmit
pnpm review:capture                     # visual stepper screenshot capture (chromium + firefox-review)
```

## Test File Organization

**Rust — two coexisting patterns, both used deliberately:**

1. **Inline `#[cfg(test)] mod tests`** at the bottom of the source file for unit tests of that file's private/internal logic (`crates/vox-search/src/policy.rs` — `mod tests { use super::*; ... }`).
2. **Separate `tests.rs` sibling file** referenced via `mod tests;` when the test module grows large enough to warrant its own file but is still logically "unit" (not integration) (`crates/vox-orchestrator-mcp/src/memory_tools/tests.rs`).
3. **`crates/<crate>/tests/*.rs` integration tests** — one file per behavior/feature area, black-box against the crate's public API (`crates/vox-db/tests/migration_tests.rs`, `crates/vox-search/tests/quota_tracker_test.rs`). This is where cross-crate / DB / network-mocked tests live.

**Naming:** integration test files are `snake_case_test.rs` or `snake_case_tests.rs` (`research_doc_io_test.rs`, `ops_codex_tests.rs`) — both suffixes occur, pick either but stay `snake_case` and end in `test`/`tests`.

**Shared helpers:** `crates/<crate>/tests/common/mod.rs` holds cross-test-file helpers, always doc-commented at the module level about what they're for and what gates them (`crates/vox-db/tests/common/mod.rs`: `remote_creds()` gates opt-in network integration tests behind an env var).

**TS:** component tests are co-located next to the component (`Button.tsx` / `Button.test.tsx`); non-component unit tests co-locate similarly (`transport.console.test.ts` next to `transport.ts`); Playwright specs live in `crates/vox-gui/ui/e2e/` as a flat list of `<feature>.spec.ts`, with e2e-only shared mocks/helpers under `crates/vox-gui/ui/e2e/lib/`.

## Test Structure

**Rust — flat function-per-case, no shared `describe` blocks:**

```rust
#[test]
fn test_budget_exhaustion_rejects_consumption() {
    let budget = TavilySessionBudget::new(10);
    assert!(budget.try_consume(10));
    assert!(!budget.try_consume(1), "Exhausted budget must reject consumption");
    let (used, remaining) = budget.usage_and_remaining();
    assert_eq!(used, 10);
    assert_eq!(remaining, 0);
}
```

(`crates/vox-search/tests/quota_tracker_test.rs`)

- Test function names are full sentences describing the behavior under test (`memory_config_for_state_matches_orchestrator_memory`, `retrieval_bundle_prefers_bm25_before_lexical_fallback`, `scientia_feedback_tightens_weak_source_policy_without_tavily`) — not `test_1`/`it_works`.
- Multi-step lifecycle tests use numbered inline comments to mark phases (`// 1. Acquire first lock`, `// 2. Attempt to acquire second lock...` in `crates/vox-search/src/policy.rs`'s `FileLock` tests) rather than splitting into separate test functions when the steps are inherently sequential/stateful.
- No `setUp`/`tearDown` hooks; setup is inline per-test using `tempfile::tempdir()` for filesystem isolation and unique temp-dir names seeded from `SystemTime::now()` nanos for parallel-safe isolation (see `retrieval_bundle_prefers_bm25_before_lexical_fallback`'s `unique` var).

**TS — `describe`/`it` with Testing Library queries:**

```tsx
describe('Button', () => {
  it('renders with type="button" by default', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'button');
  });
});
```

(`crates/vox-gui/ui/src/components/ui/Button.test.tsx`) — query by role/text/label (`getByRole`, `getByText`), not by test id, except where Playwright e2e specs need stable hooks (`getByTestId('workbench-tab-bar')` in `crates/vox-gui/ui/e2e/dashboard.spec.ts`).

## Mocking

**Rust HTTP mocking — `wiremock`:**

```rust
let server = MockServer::start().await;
Mock::given(method("GET"))
    .and(path("/usage"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "monthly_limit": 1000, "monthly_usage": 350
    })))
    .mount(&server)
    .await;
```

(`crates/vox-search/tests/quota_tracker_test.rs`) — used wherever a test needs to exercise real HTTP-client code against a fake upstream (Tavily budget sync, crawler tests). Declared in `[dev-dependencies]` per crate (`crates/vox-search/Cargo.toml`).

**Rust DB isolation:** `vox_db::VoxDb::connect(vox_db::DbConfig::Memory)` for an in-memory database per test — no shared test database, no fixtures loaded from disk by default.

**Rust — no general-purpose mock framework** (`mockall`/`mockito` are used only in ~10 specific crates: `vox-orchestrator-mcp`, `vox-gamify`, `vox-actor-runtime`, `vox-publisher`, `vox-gui`, `vox-orchestrator`, `vox-integration-tests`, `vox-llm-egress`, `vox-populi`, `vox-cli`). Default preference is a real implementation against an in-memory/temp-dir backend (`tempfile::tempdir()`, `VoxDb::connect(DbConfig::Memory)`) over mocking internal collaborators — only reach for `wiremock`/`mockall` at genuine external-network or trait-object boundaries.

**TS/Playwright — Tauri IPC mocking:** `installTauriMock` (basic) / `installTauriMockRich` (fuller surface) from `crates/vox-gui/ui/e2e/lib/tauriMock.ts` / `tauriMockRich.ts`, injected via `addMockInitScript(page, installTauriMock, 'dashboard')` from `crates/vox-gui/ui/e2e/lib/tauriMockShared.ts` before `page.goto('/')`. This is how every e2e spec fakes the Tauri desktop backend so the GUI can be tested as a pure web app in Chromium/Firefox. Per root `AGENTS.md` §GUI Visual Verification Invariant, every new/modified GUI surface must have a deterministic Playwright spec built on these mock harnesses, plus a viewport screenshot captured into `crates/vox-gui/ui/review-bundle/latest/`.

**What NOT to mock:** internal pure-Rust logic (budgets, parsers, policy structs) — test against the real type with real inputs, not a mock of it.

## Fixtures and Factories

**Rust:** no factory library; construct structs directly with `..Default::default()` spread for the fields under test (`SearchPolicy { tavily_enabled: false, ..SearchPolicy::default() }` in `crates/vox-search/src/policy.rs`) — the test module explicitly allows `#![allow(clippy::field_reassign_with_default)]` where a stylistic mutate-after-construct pattern is preferred for readability.

**Rust fixture files:** integration-test-only fixture directories live under a crate's own `tests/fixtures/` (e.g. `crates/vox-arch-check/tests/fixtures/missing-desc`, `crates/vox-plugin-host/tests/fixtures/noop-code`) — these excluded-from-workspace fixture crates are listed in the root `Cargo.toml` `[workspace] exclude`.

**TS:** Playwright fixtures are the mock-init-script pattern above, not `@playwright/test` custom fixtures; component tests build props inline per `render()` call, no shared factory module observed.

## Coverage

**Requirement:** Test-First Policy (root `AGENTS.md` §Test-First Policy) is the binding coverage rule — not a numeric %, but a structural one: every new `pub fn` in `crates/*/src/**` (excluding `main.rs`/`bin/`/`tests/`/files under 30 non-blank lines) requires an adjacent `#[test]`/`#[tokio::test]`/`#[cfg(test)] mod tests` in the **same file** before commit. Enforced by the `tdd-guard` lefthook pre-commit hook (`lefthook.yml`) running `toestub` in `enforce-strict` mode against staged files, and by CI via `vox-code-audit` (`skeleton/untested-pub-api`, `skeleton/no-test-for-pub-fn`, Warning severity in default `legacy` CI mode).

- Override only via `// toestub-ignore(skeleton/untested-pub-api) — <reason>` inline, or a structured entry in `contracts/toestub/suppressions.v1.json` with `owner` + `reason` — never to dodge the work.
- Vox golden examples (`examples/golden/**/*.vox`) carry the equivalent obligation via `@test` blocks in the same file.

**Coverage tooling:** numeric coverage/mutation gates exist at the CI tier level (`full+cov`, `cargo mutants` triggered on `crates/vox-compiler/**`/`crates/vox-codegen/**` PRs) — see `docs/src/contributors/local-ci-pre-push.md` for exact invocation; not restated here per that doc's SSOT note.

## Test Types

**Unit tests:** inline `#[cfg(test)] mod tests` or a sibling `tests.rs`, scoped to one module's internals, run with the default `cargo test` / `vitest run`.

**Integration tests:** `crates/<crate>/tests/*.rs` (Rust) exercising the crate's public API end-to-end, often against an in-memory DB or a `wiremock` server; gated integration tests behind `required-features` when they need optional heavy deps (`crawler_test` behind `web-scrape`).

**E2E tests:** Playwright (`crates/vox-gui/ui/e2e/*.spec.ts`) driving the built GUI in a real browser against Tauri-mocked IPC — this is the GUI's only true end-to-end layer; required before merging any new/changed GUI surface (root `AGENTS.md` §GUI Visual Verification Invariant).

## Common Patterns

**Async testing (Rust):**

```rust
#[tokio::test]
async fn test_upstream_usage_sync_wiremock() {
    let server = MockServer::start().await;
    // ... mount mock ...
    let budget = TavilySessionBudget::new(1000);
    let (used, remaining) = budget.sync_with_upstream("dummy_key", Some(&server.uri())).await.expect("sync");
    assert_eq!(used, 350);
}
```

`tokio::sync::Mutex` for shared async state in tests that stand up a full `Orchestrator`/`SessionManager` (see `crates/vox-orchestrator-mcp/src/memory_tools/tests.rs`).

**Error-path testing (Rust):** assert on the `Result`'s error shape directly rather than just `is_err()` where the error kind matters:

```rust
let lock2_res = FileLock::acquire(&target, Duration::from_millis(60));
assert!(lock2_res.is_err());
assert_eq!(lock2_res.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
```

(`crates/vox-search/src/policy.rs`)

**Regression-driven verification (repo-wide policy, not a code pattern):** per root `AGENTS.md` §PR & Review Discipline, a security-relevant guard must be verified by mutation (break it deliberately, confirm the test fails, restore it) — a test that passes with the guard deleted is worthless and has shipped bugs in this repo before.

---

*Testing analysis: 2026-09-22*
