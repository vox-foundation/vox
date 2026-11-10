---
title: "Cross-Repo Federation and Standalone Sovereignty SSOT"
description: "Architecture and verification contracts for multi-repository federation across vox, vox-ops, vox-axis, and voxlang.org while ensuring offline standalone sovereignty."
category: "Architecture SSOTs"
status: "current"
topic: "version-control"
domain: "platform"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Cross-Repo Federation and Standalone Sovereignty SSOT

<!-- kb:summary -->
This document defines the single source of truth for repository federation and standalone sovereignty across the Vox ecosystem. Multi-repository collaboration unites the public core (`vox`), private operations (`vox-ops`), desktop GUI client (`vox-axis`), and documentation portal (`voxlang.org`) through an explicit `.vox/repositories.yaml` contract spine. Shared wire, IPC, and telemetry contracts are verified with zero split-brain drift while absent optional checkouts are gracefully skipped. The public `vox` tree maintains strict standalone sovereignty with zero private runtime dependencies, decoupled default build targets, and offline-first default posture.
<!-- /kb:summary -->

## Overview

Vox coordinates multi-repository development across four federated surfaces while strictly preserving the standalone sovereignty of the open-source repository.

The federated topology comprises:
1. **`vox` (Core):** Public compiler, actor runtime, standard library, CLI, and contract schemas at `.`. Role: `core`.
2. **`vox-ops` (Private Operations):** Infrastructure, private cluster orchestration, deployment runbooks, and internal services at `../vox-ops`. Role: `private-ops`, marked `optional: true`.
3. **`vox-axis` (Desktop GUI Surface):** Tauri/Vite desktop application client at `crates/vox-gui`. Role: `gui-surface`.
4. **`voxlang.org` (Documentation Portal):** Starlight documentation and Astro web surface at `docs-astro`. Role: `docs-site`.

```vox-diagram
graph {
    direction: down

    container siblings: "~/dev (Single Master Folder)" {
        node vox: "vox (Core)"
        node ops: "vox-ops (Private Ops - Optional)"
    }

    container monorepo: "vox Workspace" {
        node gui: "vox-axis (crates/vox-gui)"
        node docs: "voxlang.org (docs-astro)"
    }

    siblings.vox -> siblings.ops: "shared wire/IPC/telemetry"
    siblings.vox -> monorepo.gui: "GUI contracts & telemetry"
    siblings.vox -> monorepo.docs: "documentation contracts"
}
```

## Contract Spine: `.vox/repositories.yaml`

The explicit multi-repository catalog manifest is stored at `.vox/repositories.yaml`. It replaces ambient directory crawling with an auditable topology specification:

```yaml
schema_version: 1
primary_repository_id: vox
repositories:
  - repository_id: vox
    display_name: vox
    root_path: .
    access_mode: local
    role: core
    optional: false
    capabilities: [read_file, list_files, text_search, history_search]
    shared_contracts:
      - contracts/wire/
      - contracts/ipc/
      - contracts/telemetry/
  - repository_id: vox-ops
    display_name: vox-ops
    root_path: ../vox-ops
    access_mode: local
    role: private-ops
    optional: true
    capabilities: [read_file, list_files, text_search, history_search]
    shared_contracts:
      - contracts/wire/
      - contracts/ipc/
      - contracts/telemetry/
    cargo_patch_crates:
      - vox-ops
```

### Optional Repository Grace Period

Repositories marked `optional: true` (such as `../vox-ops`) may be absent on standard developer machines, public forks, or CI runners. When absent:
- Catalog resolution reports status `skipped_absent_optional`.
- Cross-repository contract parity checks gracefully skip the checkout without failing the build.
- Query routines (`repo_query_text`, `repo_query_file`) list the repository in their `skipped` audit trace without emitting fatal errors.

## Cross-Repo Contract Parity Verification

The contract parity verification routine (`verify_cross_repo_contract_parity`) computes cryptographic BLAKE3 digests of shared wire, IPC, and telemetry contracts across all present repositories:
- Any file matching `shared_contracts` paths that appears in more than one present repository is compared.
- If all hashes match, the federation is in complete parity (`in_parity: true`).
- If an out-of-sync contract is detected, a `ContractDrift` record identifies the conflicting repositories and digests.

## Standalone Sovereignty Invariants

Standalone sovereignty (`verify_standalone_sovereignty`) ensures that the core `vox` repository remains fully functional, compilable, and auditable in isolation:
1. **Zero Private Dependency:** Root `Cargo.toml` has zero dependency on `vox-ops` or `vox_ops`.
2. **Lean Default Compilation:** Root `Cargo.toml` sets `default-members = ["crates/vox-cli"]`, ensuring heavy GUI surfaces (`vox-gui`) do not bloat standard terminal builds.
3. **Offline-Safe Posture:** Telemetry defaults to zero-surprise opt-in; local operations function without network access or cloud credentials (`VOX_OFFLINE=1` and `VOX_TELEMETRY=off` compliant).
4. **Self-Contained Subsurface Manifests:** `crates/vox-gui` and `docs-astro` maintain valid, independent package manifests.

## Local Cargo Patch Linking

For synchronized development across sibling repositories, `render_local_cargo_patch_block(workspace_root, enable)` generates the `[patch.crates-io]` manifest block:
- When enabled, it maps federated crates to their sibling checkout paths.
- When disabled, it comments out patch directives, preserving lockfile reproducibility.
