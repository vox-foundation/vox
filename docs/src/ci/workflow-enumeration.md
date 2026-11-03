---
title: "Workflow enumeration (GitHub Actions)"
description: "Official documentation for Workflow enumeration (GitHub Actions) for the Vox language. Detailed technical reference, architecture guides,"
category: "CI & Quality"
training_eligible: true

schema_type: "TechArticle"
---

# Workflow enumeration (GitHub Actions)

| File | Purpose |
|------|---------|
| `.github/workflows/ci.yml` | **The PR gate.** Triggers: `pull_request` + `merge_group`. Five jobs, all GitHub-hosted. **`linux`** (`ubuntu-latest`, 30 min) — `vox ci pre-push` fast tier, then `cargo clippy` + `cargo nextest run` on the **affected crates only**; additionally `cargo deny check licenses bans sources` when dependencies changed, and `cargo clippy`/`cargo doc` with `-D warnings` when a toolchain bump is detected (a bump swaps the affected-crate nextest step for those). **`ui`** (`ubuntu-latest`, 30 min) — typecheck + vitest + Playwright, and runs only when the PR touches `crates/vox-gui/**`. **`windows`** (`windows-latest`) — advisory, non-blocking compile check, `merge_group` only. **`gate`** (`ubuntu-latest`) — the branch-protection required context, named `Check, Build, and Test (Rust)`; one step, which asserts `linux` and `ui` both reported `success`. **`ssot-autoregen`** (`ubuntu-latest`) — non-required PR helper that regenerates the committed SSOT artifacts and commits them back to the PR branch; nothing `needs:` it, so it can never block the gate. Optional shell twins: [`scripts/README.md`](../adr/index.md). Intentional duals: [command-surface-duals](command-surface-duals.md). |
| `.github/workflows/nightly.yml` | **The slow lanes** (`ubuntu-latest`/`windows-latest`, 180-min cap, daily cron + `workflow_dispatch`). ~18 jobs, including the `full` job (full-workspace clippy / rustdoc / nextest / doctests / `cargo deny` / `cargo audit`) that is the **sole writer** of the `workspace` Swatinem cache the PR gate restores, `audits` (TOESTUB-full, `build-timings --crates`, `feature-matrix`, `cuda-features`, mens-gate), the GUI Playwright and cross-build lanes, and `windows-cache-seed`, which seeds the cache the advisory merge-queue Windows leg reads. |
| `.github/workflows/docs-deploy.yml` | Build `vox-doc-pipeline`, run doc pair extraction, mdBook build, Pages artifact. |
| `.github/workflows/deploy-hetzner.yml` | **`push: main`** Automated deployment to Hetzner Coolify VPS. Gate 1 is a thin **`cargo build -p vox-cli --locked`** on **`ubuntu-latest`** (fmt/clippy/tests are **`ci.yml`**); Gates 2–4 poll Coolify and probe public **`/health`**. |
| `.github/workflows/docs-quality.yml` | **`runs-on: ubuntu-latest`**. mdBook toolchain, **`cargo run -p vox-doc-pipeline -- --check`** (blocking), advisory mdBook build / markdownlint / internal link steps. |
| `.github/workflows/link_checker.yml` | Link validation for docs site. |
| `.github/workflows/ml_data_extraction.yml` | ML / corpus maintenance jobs. Grammar drift via **`vox ci grammar-drift --emit github`**; eval summary via **`vox corpus eval --print-summary`** (no Python). |
| `.github/workflows/release-binaries.yml` | Tag-only release publish (`v*`): matrix **`vox ci release-build --package all`** for Linux x64, Windows x64, macOS x64 + **Apple Silicon** (`aarch64-apple-darwin`), using **`cargo run --locked`**. Each matrix job builds and smoke-tests `vox` and `vox-ml-cli` archives (`vox --version`, `vox-ml-cli --help`) before upload; publish job merges `checksums.txt`. See [binary release contract](binary-release-contract.md). |
| `.github/workflows/mutation-nightly.yml` | **Schedule / `workflow_dispatch`:** **`cargo mutants -p vox-compiler`** with **`cargo-nextest`** (pilot; config `.cargo/mutants.toml`). |

**Every gate, nightly, and release lane runs on a GitHub-hosted runner.** There is no self-hosted fleet; the labelled pools this page used to describe were retired with it (Tasks 9/10/14 of `docs/superpowers/plans/2026-09-22-ci-audit-remediation.md`). The only `self-hosted` labels left are the GPU lanes in `ml_data_extraction.yml` and the skipped `mens-candle-cuda` plugin row, which have **zero registered runners** and therefore starve — see [runner contract](runner-contract.md) §Runners. Keep `runs-on` explicit per job.

> **GitLab mirror retired (2026-06-03).** The `.gitlab-ci.yml` mirror that
> once tracked these Rust guards, tests, docs, and ML jobs has been **deleted**;
> GitHub Actions (`.github/workflows/`) is the sole CI surface and GitLab CI is
> no longer a supported target. The GitLab `vox-ci-guards` job listing and the
> GitLab↔GitHub job-parity table that previously lived here are gone with it.

`vox-workflow-runtime` tests also validate representative interpreted journal event rows against `contracts/workflow/workflow-journal.v1.schema.json` (including retry and mesh event families across feature modes), so CI catches v1 contract drift in both event shape and replay paths. The compose smoke lanes (`mens-compose-config`, `docker-vox-image-smoke`) live in `ci.yml`; see [deployment compose SSOT](../reference/deployment-compose.md).

