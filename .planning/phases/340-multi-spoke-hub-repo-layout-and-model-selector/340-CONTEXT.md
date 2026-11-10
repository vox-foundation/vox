# Phase 340: Multi-Spoke Hub Repo Layout & Spoke-Aware Local Model Selector - Context

**Gathered:** 2026-11-07
**Status:** Ready for planning
**Milestone:** v0.6 MENS Multi-Spoke & Full Ladder Gate Closure
**Source:** User-confirmed milestone decisions and architectural specifications.

<domain>
## Phase Boundary

Phase 340 establishes the multi-spoke HuggingFace repository topology and the unified spoke-aware local model selector across the entire Vox MENS Brain.

Key deliverables:
1. **Multi-Spoke Hub Repo Layout (`spokes/<spoke>/`)**:
   - Single HuggingFace repository per size (`brbrainerd/vox-mens-qwen3-<size>`).
   - Adapters stored in subdirectories: `spokes/vox-lang/`, `spokes/rust/`, `spokes/harness/`, `spokes/tool-selection/`, `spokes/argument-generation/`.
   - Backward compatibility: top-level `adapter.safetensors` and `vox-adapter-manifest.json` maintained as aliases for the default `vox-lang` spoke during transition.
   - Unified `spoke_gates.json` published at repository root recording gate verdict for every spoke.
   - Unified Model Card (`README.md` + programmatic 2-panel SVG strips `assets/overview.svg` and `assets/scorecard.svg`).
2. **Spoke-Aware Local Model Selector**:
   - Integrated into `vox-orchestrator::models` and `vox-populi` (`mens::tensor::size_select` + `spoke_route`).
   - Selects based on live accelerator VRAM / unified memory floor titrating across all 6 sizes (`0.6B`, `1.7B`, `4B`, `8B`, `14B`, `32B`).
   - Dispatches requests by file type / lane trigger:
     - `*.vox`, `lane:vox_codegen`, `lane:vox_lang_tier_b` -> `vox-lang` spoke
     - `*.rs`, `lane:vox_rust_authoring` -> `rust` spoke
     - `lane:vox_harness` -> `harness` spoke
     - `lane:vox_tool_selection`, `lane:vox_tooling` -> `tool-selection` spoke
     - `lane:vox_argument_generation` -> `argument-generation` spoke
   - Enforces `spoke_gates.json` pass requirement: never auto-picks an ungated or failing `(size, spoke)` adapter; falls back to base model or next qualified rung with clear explanation.
3. **Retrieval & Knowledge Lane Routing**:
   - Coordinates with B3 retrieval embedder `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca` for tool-schema and documentation retrieval.
   - Preserves non-fine-tuned retrieval routing for `populi-meta`, `research`, and `rocks`.
   - Fences Lane G `research-expert` GRPO+RLVR hook without disrupting standard QLoRA adapter selection.

Out of scope: Retraining spokes (Phases 341, 342, 343) or deleting retired code (Phase 344).
</domain>

<decisions>
## Implementation Decisions

### D-01: Single Repo per Size with Subdirectories
- User Decision: Each size ladder rung uses a single repository (`brbrainerd/vox-mens-qwen3-<size>`).
- Subdirectory hierarchy:
  ```
  brbrainerd/vox-mens-qwen3-<size>/
  ├── README.md                      # Unified Model Card (v2 template)
  ├── spoke_gates.json               # SSOT matrix of spoke gate states
  ├── assets/
  │   ├── overview.svg               # Router topology & Pareto chart (1200x320)
  │   └── scorecard.svg              # Loss trajectory & metric bars (1200x300)
  ├── adapter.safetensors            # Symlink/copy alias to spokes/vox-lang/
  ├── vox-adapter-manifest.json      # Symlink/copy alias to spokes/vox-lang/
  └── spokes/
      ├── vox-lang/
      │   ├── adapter.safetensors
      │   ├── adapter_config.json
      │   └── training_manifest.json
      ├── rust/
      ├── harness/
      ├── tool-selection/
      └── argument-generation/
  ```

### D-02: Backward Compatibility Alias
- To avoid breaking existing clients pulling from root (`vox mens run`, `vox mens serve`), top-level files point to `spokes/vox-lang/`.
- Pull and sync commands accept `--spoke <spoke>` (defaulting to `vox-lang`).

### D-03: Strict Gate Verification in Model Selector
- `select_model_for_task(task_context, free_vram)` inspects `spoke_gates.json`.
- A spoke marked `failed`, `stale`, or `missing` is ineligible for automatic dispatch.
- If no adapter passes for the desired `(size, spoke)`, the selector falls back to base model or reports exact missing gate without crashing.

### D-04: Full 6-Size Ladder Coverage
- Ladder rungs: `0.6B`, `1.7B`, `4B`, `8B`, `14B`, `32B`.
- `32B` handling: Hardware-blocked for local training on 64 GB Mac; selector marks local execution as `requires_external_host` or `quantized_4bit` and emits cloud/remote dispatch advisory.
</decisions>

<verification>
## Verification Requirements
- `vox mens hub sync --dry-run` validates directory structure matching `spokes/<spoke>/`.
- Golden tests for `spoke_gates.json` schema serialization and deserialization.
- Unit and integration tests for `size_select` + `spoke_route` verifying all 5 spokes and lane triggers.
- CLI tests for `vox mens pull --size 0.6b --spoke harness` pulling the expected subdirectory.
</verification>
