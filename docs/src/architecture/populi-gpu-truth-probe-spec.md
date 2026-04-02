---
title: "Populi GPU truth probe specification (NVML Layer A)"
description: "How probe-backed NVIDIA inventory flows into NodeRecord heartbeats and reconciles with ADR 018."
category: "architecture"
last_updated: 2026-03-29
training_eligible: true
---

# Populi GPU truth probe specification (NVML Layer A)

This document **implements** the probe slice of [ADR 018: Populi GPU truth layering](../adr/018-populi-gpu-truth-layering.md): **Layer A** fields on `NodeRecord` (`crates/vox-populi/src/node_registry.rs`) populated from the driver when NVML is available.

## Build / runtime

| Surface | Behavior |
| --- | --- |
| Default builds | No NVML link. `vox_repository::probe_nvidia_gpu_inventory_best_effort` (`crates/vox-repository/src/gpu_inventory.rs`) returns `None`; join/heartbeat behave as before (env advertisement only). |
| `vox-repository` feature `nvml-probe` | Links `nvml-wrapper`. At runtime, `Nvml::init()` must succeed (NVIDIA driver + NVML present). |
| `vox-populi` feature `nvml-gpu-probe` | Enables `vox-repository/nvml-probe`. |
| `vox-cli` feature `mesh-nvml-probe` | Pulls `vox-populi` with NVML probe for operators who want inventory on `node_record_for_current_process`. |

Typical build:

```bash
cargo build -p vox-cli --features populi,mesh-nvml-probe
```

## Fields populated

When the probe succeeds, `node_record_for_current_process` (`crates/vox-populi/src/lib.rs`) sets:

- `gpu_total_count`, `gpu_healthy_count`, `gpu_allocatable_count` — from NVML device enumeration (v1: healthy/allocatable match enumerated devices; refine with reservations in a later phase).
- `gpu_inventory_source` — `"nvml"`.
- `gpu_truth_layer` — `"layer_a_verified"`.
- `capabilities.min_vram_mb` — minimum **total** VRAM in MiB across devices, only if not already set by config.

## Heartbeat reconciliation

Operators should send the **same** [`NodeRecord`] shape on **join** and **heartbeat** (existing Populi HTTP contract). Rebuilding the record each tick via `node_record_for_current_process` (or equivalent) automatically refreshes Layer A after **GPU hotplug**, driver restart, or VM attach — subject to NVML visibility.

**Layer B** (allocatable after local reservations) and **Layer C** (labels/policy) remain separate; this spec does not merge operator lies with probe facts — ADR 018 precedence still applies when schedulers consume both.

## Related

- [ADR 018](../adr/018-populi-gpu-truth-layering.md)
- [Populi GPU mesh implementation plan 2026](populi-gpu-mesh-implementation-plan-2026.md)
- [Mens cloud GPU strategy](../reference/mens-cloud-gpu.md) (boundary vs Populi)
