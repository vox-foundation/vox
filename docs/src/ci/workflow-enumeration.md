---
title: "Workflow enumeration"
category: ci
last_updated: 2026-03-21
---

# Workflow enumeration (GitHub Actions)

| File | Purpose |
|------|---------|
| `.github/workflows/ci.yml` | **`runs-on: [self-hosted, linux, x64]`** (basic Linux pool). `cargo build -p vox-cli`, then guards via **`vox ci`** (`cargo run -p vox-cli --quiet -- ci …`): `manifest`, `line-endings` (forward-only diff vs `GITHUB_BASE_SHA`…`GITHUB_SHA` on PRs), `check-codex-ssot`, `check-docs-ssot` (includes stale doc/workflow ref scan), `doc-inventory verify`, `workflow-scripts`, `toestub-scoped`, `feature-matrix`, `no-vox-dei-import`, `cuda-features`; `cargo fmt --check`, `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`, `cargo clippy --workspace -- -D warnings`, repository/orchestrator/MCP smoke, `cargo check -p vox-cli --features gpu,populi-qlora,stub-check`, `cargo test --workspace`, **`populi-gate --profile ci_full`** (full Populi gate matrix from `scripts/populi/gates.yaml`). Optional shell twins: [`scripts/README.md`](../../../scripts/README.md). Intentional duals: [command-surface-duals](command-surface-duals.md). |
| `.github/workflows/docs-deploy.yml` | Build `vox-doc-pipeline`, run doc pair extraction, mdBook build, Pages artifact. |
| `.github/workflows/link_checker.yml` | Link validation for docs site. |
| `.github/workflows/ml_data_extraction.yml` | ML / corpus maintenance jobs. Grammar drift via **`vox ci grammar-drift --emit github`**; eval summary via **`vox corpus eval --print-summary`** (no Python). |

**CUDA / GPU compile gates:** when a job needs `nvcc` or CUDA-enabled `cargo check`, use the **Docker** self-hosted profile (`[self-hosted, linux, x64, docker]`) per [runner contract](runner-contract.md); keep `runs-on` explicit per job.

GitLab: `.gitlab-ci.yml` mirrors Rust guards, tests, docs, and ML jobs. **Docker parity (optional):**

| Job | GitHub equivalent | Notes |
|-----|-------------------|--------|
| `mesh-compose-config` | `mesh-compose-config` in `ci.yml` | `docker compose -f examples/mesh-compose.yml config` using `docker:26-cli` (no DinD if `config` is client-only). |
| `docker-vox-image-smoke` | `docker-vox-image-smoke` | `docker build` default + mesh features; Docker-in-Docker service + `allow_failure: true` unless the runner allows **privileged** service containers (typical GitLab constraint). |

If your runner cannot run DinD, the smoke job fails soft; keep **`mesh-compose-config`** green for compose YAML validation. See [deployment compose SSOT](../architecture/deployment-compose-ssot.md).
