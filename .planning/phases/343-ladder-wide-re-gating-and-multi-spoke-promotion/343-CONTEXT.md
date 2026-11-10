# Phase 343: Ladder-Wide Re-Gating & Multi-Spoke Promotion (1.7B, 4B, 8B, 14B) - Context

**Gathered:** 2026-11-07
**Status:** Ready for planning
**Milestone:** v0.6 MENS Multi-Spoke & Full Ladder Gate Closure
**Source:** User-confirmed milestone decisions and full-ladder scaling objectives.

<domain>
## Phase Boundary

Phase 343 scales the validated spokes and gating protocols across the full Qwen3 model ladder (`1.7B`, `4B`, `8B`, `14B`, and cloud path for `32B`).

Key deliverables:
1. **Accelerated Re-Gating of Existing vox-lang Adapters**:
   - Re-evaluate existing `vox-lang` adapters (`1.7B`, `4B`, `8B`, `14B`) using Phase 337 eval throughput enhancements:
     - 128-item expanded collateral slices (GSM8K, MATH-500).
     - `suite:humaneval_vox` hidden-test suite.
     - BF16 Metal GEMM + sequential McNemar early stopping to gate overnight rather than taking days.
   - Generate receipt v2 with `policy_blake3` and input digests.
2. **Ladder-Wide Multi-Spoke Training & Promotion**:
   - Take the winning `harness`/agent architecture (mono or decomposed) from Phase 341 and scale training to `1.7B`, `4B`, `8B`, and `14B`.
   - Scale the `rust` spoke recipe from Phase 342 to `1.7B`, `4B`, `8B`, and `14B`.
   - Enforce memory floors and presets titrating across available hardware.
3. **Full 6-Size Ladder Coverage & 32B Strategy**:
   - `32B` dense rung exceeds the 64 GB local Apple Silicon memory ceiling for QLoRA training.
   - Phase 343 defines the reproducible cloud/cluster execution harness (or 4-bit quantized offloaded serving path) for `32B` while documenting local hardware constraints honestly.
4. **HuggingFace Repository Synchronization Across Ladder**:
   - Synchronize all passing `(size, spoke)` adapters to their respective `brbrainerd/vox-mens-qwen3-<size>` repos under `spokes/<spoke>/`.
   - Update `spoke_gates.json` on all repositories.
   - Generate and publish SVG visual strips (`overview.svg`, `scorecard.svg`) for each size.
</domain>

<decisions>
## Implementation Decisions

### D-01: Proven Recipe Scaling
- No new hyperparameters are introduced during ladder scaling; the exact `lr`, `alpha`, `rank`, and curriculum schedules validated on `0.6B` are mapped through `gpu-specs.yaml` presets for larger sizes.

### D-02: Fast Gate Gating
- Leveraging Phase 337's early-stopping McNemar test allows ladder models to reject failing runs within the first 30-40 items while requiring the full 128 items only for borderline comparisons.

### D-03: Honest 32B State
- Never invent mock scores or fake adapter files for `32B`. The `32B` model card and index explicitly report status as `cloud_pending` / `hardware_constrained` until cluster training completes.
</decisions>

<verification>
## Verification Requirements
- All intermediate sizes produce valid receipt v2 JSONs with non-empty policy hashes.
- `vox mens hub sync --push` pushes passing multi-spoke directories without overwriting top-level alias defaults.
- All model cards render green checks only for genuinely evaluated and passing spokes.
</verification>
