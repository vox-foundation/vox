---
title: "CI/CD Compute Placement Policy"
description: "Where each CI, CD, and nightly job runs — GitHub-hosted (the default for vox since the 2026-09 fleet retirement) vs Hetzner VPS (always-on) vs a local fleet — chosen by the job's gating resource."
category: "CI & Quality"
status: "current"
---

# CI/CD Compute Placement Policy

Three compute tiers are available. Place each job where its **gating resource** is
cheapest, subject to the free-tier economics below.

## Decision rule — classify by gating resource, then place

| Gating resource | Host | Why |
|---|---|---|
| CPU-parallel (compile, `clippy --all-targets`, mutation, test matrix) | Local fleet | Many cores vs few shared vCPU |
| Disk-IO / cache (cargo target, Docker layers, sccache, graphify) | Local fleet | 4 TB NVMe vs ~160–240 GB VPS disk |
| GPU (inference, LoRA/QLoRA, qwen nightly, ComfyUI) | Local fleet | RTX-class GPU; the VPS has none |
| RAM-heavy (Next.js build, Playwright/Stagehand) | Local fleet | 64 GB vs ~16 GB VPS |
| Uptime / network (deploy CD, health/TLS probes, DB maintenance, dep-bots) | Hetzner VPS | Must fire regardless of workstation state |
| Reproducibility on neutral infra (portability gate, cross-OS release, security scans) | GitHub-hosted | No private-hardware dependency |

## Free-tier economics

- **vox = PUBLIC** → unlimited free GitHub-hosted minutes. Run the entire **deploy
  critical path** (image build, Coolify trigger, Gate-3 probe) on `ubuntu-latest`
  so deploys never wait on the workstation. Since minutes are free and the fleet is
  retired (see below), run everything else hosted too — the only thing a local host
  would buy back is GPU, which no hosted runner has.
- **FableForge = PRIVATE** → 2,000 free min/mo. Keep only the light merge gate +
  the deploy trigger on hosted; push all heavy jobs to the local fleet to conserve
  minutes.

## vox placement

> **The local fleet row is gone (2026-09).** The hosted-primary migration moved
> every workflow to GitHub-hosted runners and Tasks 9/10/14 of
> `docs/superpowers/plans/2026-09-22-ci-audit-remediation.md` deleted the fleet
> and its tooling. The decision rule above still holds *in principle* — it is
> how you would place a job if a fleet existed — but for vox today the "Local
> fleet" column has no host behind it, and picking it would mean standing a new
> one up. Assume GitHub-hosted unless the job needs always-on uptime, which is
> the VPS.

| Tier | Jobs |
|---|---|
| GitHub-hosted | **Every gate, nightly, and release lane**: the `ci.yml` PR gate (`linux` + `ui` under `gate`, plus the advisory merge-queue `windows` leg), `nightly.yml`'s slow lanes, `mutation-nightly`, `compile-matrix`, `bench-nightly`, `docker-eval` image builds, `release-*` cross-OS, mobile EAS, `codeql`/`scorecard`/`gitleaks` |
| Hetzner VPS | deploy triggers + Gate-3 probes (`deploy-hetzner`), nightly ClickHouse maintenance (TTL/OPTIMIZE, backup → object storage), live-endpoint uptime, link/dep bots |
| GPU (no host) | `ml_data_extraction.yml`'s `extract`/`train` and the `mens-candle-cuda` plugin row still name `self-hosted` GPU labels. **No runners are registered**, so they starve; both lanes are nightly/dispatch-only and the plugin row is `if:`-skipped, so nothing on the PR path waits on them. Run CUDA work locally. |
| Local fleet | **None.** Retired; see the note above. `bench-nightly` timings are now hosted-runner timings, so they are comparable only in aggregate. |

> The telemetry workflows (`docker-telemetry.yml`, `deploy-telemetry.yml`) are
> parked to `workflow_dispatch` only (Task 9) pending a telemetry-deploy revival;
> both still use `runs-on: ubuntu-latest`.

## FableForge placement

| Tier | Jobs |
|---|---|
| Local fleet | `nextjs-build-check`, `e2e-*`/stagehand/semantic-vrt, full test suites, studio-pipeline (GPU), test-coverage |
| Hetzner VPS | `deploy-hetzner`/convex-deploy/deploy-guard triggers, nightly-live-audit, archive-cron, coderabbit-ingest |
| GitHub-hosted | lint/typecheck merge gate, coverage-check reporting |

## Invariants

1. The merge gate never hard-depends on the workstation — `ci.yml` runs entirely
   on GitHub-hosted runners, so no local outage can block PRs.
2. DB maintenance + backups run where the data lives (Hetzner → object storage).
3. The telemetry and eval deploy critical paths stay on GitHub-hosted runners;
   nothing private-hardware is ever on the path between a green `main` and a
   live deploy.
4. If a local fleet is ever reintroduced, it may not host the merge gate or any
   deploy step — that is what invariants 1 and 3 protect, independent of whether
   a fleet exists.
