# Phase 426 Context: Parallel HF Text-Tower Prefetch & Multi-Spoke Corpus Regeneration

- **Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)
- **Goal:** Prefetch text-only HF snapshots (`Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) concurrently across 3 network workers while 1 CPU corpus worker regenerates compiler-verified `mens/data/` and spoke mixes.
- **Disjoint Ownership:**
  - Network I/O workers (`426-N1`, `426-N2`, `426-N3`): `~/.cache/huggingface/hub/models--Qwen--*` (read-only verification on disk)
  - CPU corpus worker (`426-C1`): `mens/data/`, `target/dogfood/`
