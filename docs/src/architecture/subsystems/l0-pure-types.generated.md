---
title: "Layer 0 — Pure Types & Contracts (Generated)"
description: "Zero-I/O pure data structures, AST nodes, database row types, and KB facet schemas."
category: "Architecture SSOTs"
status: "current"
sort_order: 10
code_anchors:
  - "docs/src/architecture/layers.toml"
---

# Layer 0 — Pure Types & Contracts

Zero-I/O pure data structures, AST nodes, database row types, and KB facet schemas.

## Subsystem Topology

```vox-diagram
graph {
    direction: down
    node l0_pure_types: "Layer 0 — Pure Types & Contracts"
    node vox_arch_check: "vox-arch-check (binary)"
    l0_pure_types -> vox_arch_check
    node vox_ast: "vox-ast (library)"
    l0_pure_types -> vox_ast
    node vox_build_meta: "vox-build-meta (library)"
    l0_pure_types -> vox_build_meta
    node vox_config_derive: "vox-config-derive (library)"
    l0_pure_types -> vox_config_derive
    node vox_container_types: "vox-container-types (library)"
    l0_pure_types -> vox_container_types
    node vox_db_types: "vox-db-types (library)"
    l0_pure_types -> vox_db_types
    node vox_kb_facets: "vox-kb-facets (library)"
    l0_pure_types -> vox_kb_facets
    node vox_llm_config: "vox-llm-config (library)"
    l0_pure_types -> vox_llm_config
    node vox_mesh_types: "vox-mesh-types (library)"
    l0_pure_types -> vox_mesh_types
    node vox_orchestrator_types: "vox-orchestrator-types (library)"
    l0_pure_types -> vox_orchestrator_types
    node vox_rename_registry: "vox-rename-registry (library)"
    l0_pure_types -> vox_rename_registry
    node vox_shell_stdlib_types: "vox-shell-stdlib-types (library)"
    l0_pure_types -> vox_shell_stdlib_types
}
```

## Crates in Layer

| Crate | Kind | Fan-In Cap | LoC Baseline |
|---|---|---|---|
| [`vox-arch-check`](../../api/vox-arch-check.generated.md) | `binary` | — | — |
| [`vox-ast`](../../api/vox-ast.generated.md) | `library` | 40 | — |
| [`vox-build-meta`](../../api/vox-build-meta.generated.md) | `library` | — | — |
| [`vox-config-derive`](../../api/vox-config-derive.generated.md) | `library` | — | — |
| [`vox-container-types`](../../api/vox-container-types.generated.md) | `library` | — | — |
| [`vox-db-types`](../../api/vox-db-types.generated.md) | `library` | — | — |
| [`vox-kb-facets`](../../api/vox-kb-facets.generated.md) | `library` | — | — |
| [`vox-llm-config`](../../api/vox-llm-config.generated.md) | `library` | — | — |
| [`vox-mesh-types`](../../api/vox-mesh-types.generated.md) | `library` | — | — |
| [`vox-orchestrator-types`](../../api/vox-orchestrator-types.generated.md) | `library` | — | — |
| [`vox-rename-registry`](../../api/vox-rename-registry.generated.md) | `library` | — | — |
| [`vox-shell-stdlib-types`](../../api/vox-shell-stdlib-types.generated.md) | `library` | — | — |
