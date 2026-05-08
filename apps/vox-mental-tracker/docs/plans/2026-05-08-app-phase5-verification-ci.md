# App Phase 5 — Hourglass verification + CI lanes — Implementation Plan

**Goal:** Make the app's CI lane match Vox's monorepo standard: separate jobs for typecheck (`vox check`), unit (vitest), e2e (Playwright with browser deps), and policy/contract guards.

**Why:** The existing `.github/workflows/vox-mental-tracker.yml` is a single sequential job. Modeled on the platform's `2026-05-03-local-ci-pre-push-and-job-split.md`, splitting into parallel lanes shortens wall-clock and isolates failure modes.

**Architecture:**
- 4 jobs, all on the standard self-hosted runner pool: `typecheck`, `unit`, `e2e`, `contracts`.
- Each restores from a shared cache key.
- `contracts` validates contract files (`mood_recorded.v1.json`, `csv-columns.v1.yaml`, `json-bundle.v1.yaml`) against their schemas; runs the existing `scripts/quality/doc-policy-lint.vox` if it covers app docs.
- Final aggregator `app-summary` job for branch protection.

**Tech Stack:** GitHub Actions YAML, existing `vox ci` framework.

**Out of scope:**
- sccache (separate plan).
- Cross-OS matrix (linux only — matches the rest of vox-foundation/vox).

---

## Tasks

- [ ] **A1.** Read existing `.github/workflows/vox-mental-tracker.yml`; note current step list.
- [ ] **A2.** Refactor into 4 jobs + 1 aggregator. Each job:
  - `typecheck`: `cargo build --release -p vox-cli` then `vox check apps/vox-mental-tracker/src/main.vox`.
  - `unit`: `pnpm install --filter vox-mental-tracker` then `pnpm --filter vox-mental-tracker test`.
  - `e2e`: `pnpm exec playwright install --with-deps` then `pnpm --filter vox-mental-tracker e2e`.
  - `contracts`: validate `apps/vox-mental-tracker/contracts/**` against `apps/vox-mental-tracker/contracts/*.schema.json` (add the schema if missing).
  - `app-summary`: depends on all four; required by branch protection.
- [ ] **A3.** Add `apps/vox-mental-tracker/contracts/schema/` with JSON Schemas matching the existing contract files.
- [ ] **A4.** Document the lane shape in `apps/vox-mental-tracker/docs/contributors/ci-lanes.md`.

## Verification

- [ ] **B1.** Push a small change; observe all 4 lanes run in parallel.
- [ ] **B2.** Confirm aggregator `app-summary` passes only when all four pass.
- [ ] **B3.** Confirm a deliberate test failure in `unit` only fails that lane.
