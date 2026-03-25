---
title: "Workflow enumeration (GitHub Actions)"
description: "Official documentation for Workflow enumeration (GitHub Actions) for the Vox language. Detailed technical reference, architecture guides,"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Workflow enumeration (GitHub Actions)

| File | Purpose |
|------|---------|
| `.github/workflows/ci.yml` | **`runs-on: [self-hosted, linux, x64]`** (basic Linux pool). `cargo build -p vox-cli`, then guards via **`vox ci`** (`cargo run -p vox-cli --quiet -- ci …`): `manifest`, `line-endings` (forward-only diff vs `GITHUB_BASE_SHA`…`GITHUB_SHA` on PRs), `check-codex-ssot`, `check-docs-ssot` (includes stale doc/workflow ref scan), `doc-inventory verify`, `workflow-scripts`, `toestub-scoped`, `feature-matrix`, `no-vox-dei-import`, `cuda-features`; `cargo fmt --check`, `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`, `cargo clippy --workspace -- -D warnings`, repository/orchestrator/MCP smoke, `cargo check -p vox-cli --features gpu,mens-qlora,stub-check`, `cargo test --workspace`, **`mens-gate --profile ci_full`** (full Mens gate matrix from `scripts/mens/gates.yaml`). Optional shell twins: [`scripts/README.md`](../adr/README.md). Intentional duals: [command-surface-duals](command-surface-duals.md). |
| `.github/workflows/docs-deploy.yml` | Build `vox-doc-pipeline`, run doc pair extraction, mdBook build, Pages artifact. |
| `.github/workflows/link_checker.yml` | Link validation for docs site. |
| `.github/workflows/ml_data_extraction.yml` | ML / corpus maintenance jobs. Grammar drift via **`vox ci grammar-drift --emit github`**; eval summary via **`vox corpus eval --print-summary`** (no Python). |
| `.github/workflows/release-binaries.yml` | Tag-only release publish (`v*`): build artifacts via **`vox ci release-build --target <triple> --version <tag>`** for Linux (`[self-hosted, linux, x64]`), Windows (`windows-latest`), macOS (`macos-latest`), then publish release assets with consolidated `checksums.txt`. |

**CUDA / GPU compile gates:** when a job needs `nvcc` or CUDA-enabled `cargo check`, use the **Docker** self-hosted profile (`[self-hosted, linux, x64, docker]`) per [runner contract](runner-contract.md); keep `runs-on` explicit per job.

GitLab: `.gitlab-ci.yml` mirrors Rust guards, tests, docs, and ML jobs. **Docker parity (optional):**

| Job | GitHub equivalent | Notes |
|-----|-------------------|--------|
| `mens-compose-config` | `mens-compose-config` in `ci.yml` | `docker compose -f examples/mens-compose.yml config` using `docker:26-cli` (no DinD if `config` is client-only). |
| `docker-vox-image-smoke` | `docker-vox-image-smoke` | `docker build` default + mens features; Docker-in-Docker service + `allow_failure: true` unless the runner allows **privileged** service containers (typical GitLab constraint). |

If your runner cannot run DinD, the smoke job fails soft; keep **`mens-compose-config`** green for compose YAML validation. See [deployment compose SSOT](../reference/deployment-compose.md).
