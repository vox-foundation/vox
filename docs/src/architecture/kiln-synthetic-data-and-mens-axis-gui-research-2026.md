---
title: "Kiln AI Architecture Audit & Vox Axis MENS All-Hubs/All-Spokes Studio Design (2026)"
description: "Technical evaluation of Kiln AI (Kiln-AI/Kiln) for synthetic dataset generation and fine-tuning, Rust vs. Python architecture analysis, feature adoption roadmap for vox-corpus/vox-populi, and the Vox Axis MENS Training Studio GUI specification covering all hubs and all spokes."
category: "Architecture SSOTs"
status: "research"
topic: "mens-training"
domain: "ml"
authorship: "llm-generated"
authorship_model: "google/gemini-4-argon"
code_anchors:
  - "crates/vox-corpus/src/training/contract.rs"
  - "crates/vox-populi/src/mens/tensor/domain_profiles.rs"
  - "crates/vox-populi/src/mens/tensor/domain_router.rs"
  - "crates/vox-populi/src/mens/tensor/telemetry_schema.rs"
  - "crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx"
  - "mens/config/domain-profiles.yaml"
  - "mens/config/gpu-specs.yaml"
---

# Kiln AI Architecture Audit & Vox Axis MENS All-Hubs/All-Spokes Studio Design (2026)

<!-- kb:summary -->
Evaluates Kiln AI (`Kiln-AI/Kiln`) to determine whether Vox MENS should depend on its libraries or port its high-leverage mechanisms natively into Rust. Concludes that Kiln has no Rust library or bindings (its core engine is 100% Python over Pydantic and LiteLLM, with Rust used only as a thin Tauri desktop wrapper), so direct code dependency is rejected in favor of native Rust adoption of three high-value patterns: hierarchical topic-tree synthetic data generation, teacher-to-student distillation with rating-to-DPO routing, and a customizable All-Hubs/All-Spokes MENS Training Studio in Vox Axis.
<!-- /kb:summary -->

## 1. Executive Summary & Architectural Verdict

| Question | Verdict | Rationale |
| --- | --- | --- |
| **Does Kiln have libraries or Rust bindings?** | **Python library only (`kiln_ai`); NO Rust library or bindings.** | `Kiln-AI/Kiln` is split into `libs/core` (MIT Python package `kiln_ai` built on Pydantic v2 + LiteLLM), `libs/server` (FastAPI local sidecar), and `app/desktop` (Svelte UI inside a thin Tauri shell). Its Rust code is strictly desktop window/sidecar lifecycle boilerplate, not an ML or synthetic-data crate. |
| **Should Vox depend on `kiln_ai` directly (via `vox-python-bridge` or sidecar)?** | **No — Reject direct code dependency.** | Calling `kiln_ai` directly would bypass `vox_actor_runtime::llm` (violating the Model-Agnostic LLM Boundary and `vox/llm_provider_call`), bypass Clavis (`vox_secrets`), and replace our deterministic compiler/cargo/MCP admission oracles (`OracleReceipt`) with weaker Python JSON-schema checks. |
| **Would our pipeline benefit from copying key Kiln features natively into Rust?** | **Yes — High leverage across 3 concrete subsystems.** | (1) **Hierarchical Topic Trees** in `vox-corpus` to eliminate cold-start scarcity and mode collapse on under-represented spokes; (2) **Interactive Rating + Teacher→Student Distillation + Failed-Oracle→DPO Pair Routing**; (3) **Vox Axis MENS Studio GUI** elevating `MensTrainingView.tsx` from a 3-card CLI stdout viewer into an interactive All-Hubs, All-Spokes workbench. |

---

## 2. Deep-Dive: How Kiln AI (`Kiln-AI/Kiln`) Works Under the Hood

### 2.1 Repository & Runtime Topology

Kiln AI is an open-source, local-first AI workbench for synthetic data generation, evaluation, RAG setup, and fine-tuning orchestration. Its codebase consists of three layers:

1. **`libs/core` (`kiln_ai` on PyPI, MIT License):**
   - **Data Model (`kiln_ai.datamodel`):** Every project is stored on disk as a Git-friendly directory of `.kiln` JSON files validated by Pydantic v2 models. The hierarchy is:
     $$\text{Project} \rightarrow \text{Task} \rightarrow \{\text{TaskRequirement}, \text{Prompt}, \text{TaskRun}, \text{Eval}, \text{Finetune}\}$$
   - **Provenance & Ratings:** Every `TaskRun` records `input`, `output`, `DataSource` (`human`, `synthetic`, `model_run`), a 1–5 star `Rating`, freeform repair edits, and string `tags` (e.g., `fine_tune`, `eval_golden`, `dpo_rejected`).
   - **Adapter Registry (`kiln_ai.adapters`):** Wraps LiteLLM, OpenAI, Anthropic, Ollama, OpenRouter, Fireworks, Together, and Unsloth behind a Python `BaseAdapter` interface with structured JSON schema / tool-call output enforcement.
2. **`libs/server` (`kiln_server`):**
   - A Python FastAPI HTTP server that loads `.kiln` project directories and exposes REST endpoints for topic-tree expansion, batch synthetic sample generation, LLM-as-judge evaluations, and cloud/local fine-tuning job dispatch.
3. **`app/desktop` (Svelte + Tauri):**
   - A Svelte web frontend packaged in a Tauri desktop wrapper. **Crucial finding:** Tauri (`src-tauri`) is used exclusively to bundle and launch the Python `kiln_server` executable and open a webview. There are **no Rust crates, PyO3 bindings, or C-ABI libraries** for dataset synthesis, tokenization, or training.

### 2.2 Why Kiln Feels "Easy" to Operators

Kiln's user experience succeeds because it closes four loops inside one visual surface:

1. **Cold-Start Synthetic Data via Hierarchical Topic Trees:**
   - Standard LLM synthetic data generation suffers from **mode collapse** (asking an LLM for 500 examples yields 50 near-duplicates).
   - Kiln solves this in two phases:
     - **Phase A (Topic Tree Generation):** Given a task definition and optional seed examples, an LLM recursively generates a tree of sub-topics (e.g., $\text{Depth } D = 3, \text{Branching Factor } B = 4 \Rightarrow 64\text{ leaf topics}$). Users can inspect, prune, add, or regenerate branches in the UI before spending tokens on full examples.
     - **Phase B (Leaf-Conditioned Sample Synthesis):** For each leaf topic, a teacher model generates $N$ structured `(input, output)` pairs using configurable prompt strategies (Zero-Shot, Few-Shot Seed Ladder, Chain-of-Thought Reasoning, or RAG-grounded).
2. **Inline Human/Teacher Curation & Tag Routing:**
   - Generated runs appear in an interactive table where operators filter by topic, rate 1–5 stars, edit outputs in-place, and apply split tags (`fine_tune`, `eval`, `golden`).
3. **One-Click Teacher → Student Distillation:**
   - Outputs generated by a frontier teacher (or slow CoT prompt) are filtered to `rating >= 4` + `tag: fine_tune` and dispatched to fine-tune a small student model without writing glue scripts.
4. **Integrated Eval Comparison Matrix:**
   - Every base model, prompt strategy, and fine-tuned adapter is scored on the same frozen eval set with side-by-side diff inspection.

---

## 3. Comparative Gap Analysis: Kiln AI vs. Vox MENS

Vox MENS is already significantly stronger than Kiln in **deterministic verification, native Rust tensor execution, and multi-LoRA runtime routing**, while Kiln is ahead in **interactive GUI workflow and cold-start topic-tree diversity**.

| Capability Axis | Kiln AI (`kiln_ai`) | Vox MENS (`vox-corpus` + `vox-populi` + `vox-tensor`) | Advantage & Action |
| --- | --- | --- | --- |
| **Core Language & Runtime** | Python (`Pydantic` + `FastAPI` + `LiteLLM`) | 100% Native Rust (`vox-corpus`, `vox-tensor`, `vox-populi`, `vox-actor-runtime::llm`) | **Vox MENS.** Keep 100% Rust; do not import `kiln_ai`. |
| **Admission Verification** | JSON Schema validation + LLM-as-Judge | Deterministic per-spoke `OracleReceipt` (`vox check --interp`, `rustc`/`syn`, shell safety floor, MCP schema, harness envelope) + `xxh3_128` Trusty URIs | **Vox MENS.** Never downgrade deterministic compiler oracles to LLM judges. |
| **Synthetic Diversity / Cold-Start** | Recursive **Hierarchical Topic Tree** ($D \times B$) with human-editable branches + CoT seed ladder | Static combinatorial Rust templates (`crates/vox-corpus/src/synthetic_gen/bodies/`) + post-hoc `min_ast_diversity` check | **Kiln AI.** Port Hierarchical Topic Tree generation into `vox-corpus` per spoke. |
| **Failure / Negative Sample Handling** | Manual rating (1–2 stars) or discard | `ChosenRejectedPairV2` & `TrainingRecordV2.rejected_response` + `heal_to_dpo` | **Hybrid.** Combine Kiln's topic-tree teacher generation with automatic **Oracle-Fail → Teacher-Repair → DPO Pair** synthesis. |
| **Model Architecture & Routing** | Flat per-task fine-tune dispatch | **Hub-and-Spoke**: Pinned Qwen3 dense ladder (`0.6B`..`32B`), Retrieval Hub (`Qwen3-0.6B`), 8 Organic Spokes, 9 Domain Profiles, Champion/Challenger `DomainRouter` | **Vox MENS.** Expose the full Hub-and-Spoke topology in Vox Axis GUI. |
| **Desktop GUI Surface** | Full interactive workbench (Task, Topic Tree, Curation, Fine-Tune, Eval, Serve) | `MensTrainingView.tsx` has 3 read-only CLI stdout cards (`status`, `models`, `probe`), a path box for `mens_run_reports`, and a `port`/`model` box for `mens_serve` | **Kiln AI.** Upgrade Vox Axis `MensTrainingView` into a full All-Hubs, All-Spokes customizable studio. |

---

## 4. Features to Port into `vox-corpus` & `vox-populi` (Native Rust)

### 4.1 Hierarchical Topic Tree Generator (`crates/vox-corpus/src/topic_tree.rs`)

Instead of relying solely on static `.inc` templates in `crates/vox-corpus/src/synthetic_gen/`, introduce a declarative and LLM-expandable `SpokeTopicTreeV1`:

```rust
// vox:skip — illustrative Rust struct specification for crates/vox-corpus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicNodeV1 {
    pub id: String,
    pub label: String,
    pub description: String,
    pub spoke: OrganicSpokeKind,
    pub difficulty_range: (u8, u8),
    pub seed_examples: Vec<String>,
    pub target_sample_count: usize,
    pub verified_sample_count: usize,
    pub children: Vec<TopicNodeV1>,
}
```

- **How it works with `vox_actor_runtime::llm` and `OracleReceipt`:**
  1. For any of the 8 `OrganicSpokeKind` spokes (especially sparse spokes like `SpeechToCode`, `PopuliMesh`, `SkillCreation`, `AgentHarness`), `vox mens corpus topic-tree` expands a root domain into sub-topics and leaf scenarios via `vox_actor_runtime::llm::infer_with_retry`.
  2. Each leaf scenario is persisted in `mens/config/topic-trees/<spoke>.yaml` (Git-backed and editable in Vox Axis GUI).
  3. Leaf-conditioned synthesis prompts a Teacher model (configurable via `vox-orchestrator` model registry) with the leaf scenario, difficulty band ($1..10$), and $k$ workspace seed snippets.
  4. **Deterministic Oracle + Auto-DPO Loop:** Every generated candidate runs through the spoke's deterministic admission oracle (`verify_voxlang_roundtrip`, `verify_rust_workspace_roundtrip`, `verify_structured_shell_roundtrip`, `verify_mcp_roundtrip`, `verify_agent_harness_roundtrip`, `verify_speech_to_code_roundtrip`, `verify_populi_mesh_roundtrip`, `verify_skill_creation_roundtrip`).
     - **If Oracle Passes on Turn 1:** Admitted as a golden SFT `TrainingRecordV2` with `origin: "topic_tree_synth"`.
     - **If Oracle Fails on Turn 1 but Passes on Turn 2 (Compiler/Oracle Diagnostic Repair):** Admitted as a **both** an SFT record AND a `ChosenRejectedPairV2` (`response` = Turn 2 verified code, `rejected_response` = Turn 1 failed code, linked to exact compiler diagnostics). This turns every teacher mistake into high-signal DPO preference data automatically.

### 4.2 First-Class Teacher→Student Distillation & Split Tagging

Extend `RecordProvenance` in `crates/vox-corpus/src/training/contract.rs` (backward-compatibly via `#[serde(default)]`) with optional Kiln-inspired curation fields:

- `teacher_model_id: Option<String>` — records which teacher model synthesized or repaired the sample.
- `prompt_strategy: Option<String>` — `zero_shot` | `few_shot_seed` | `cot_distill` | `rag_grounded` | `oracle_repair`.
- `human_rating: Option<u8>` — 1–5 rating from Vox Axis Curation Studio (merges with `min_rating` in `domain-profiles.yaml`).
- `split_tags: Vec<String>` — `train_sft`, `train_dpo`, `eval_golden`, `canary`, `quarantine`.

---

## 5. Vox Axis MENS Training Studio Specification (All Hubs, All Spokes, Customizable)

### 5.1 Complete Hub & Spoke Inventory to Surface in GUI

Currently, Vox MENS has a rich Hub-and-Spoke architecture in Rust (`domain_profiles.rs`, `domain_router.rs`, `spoke_base_resolver.rs`, `contract.rs`) and YAML (`domain-profiles.yaml`, `gpu-specs.yaml`, `flywheel.yaml`), but 90% of it is invisible in `MensTrainingView.tsx`.

The redesigned **Vox Axis MENS Studio** exposes every Hub and every Spoke with live status and full customization:

#### A. All Hubs (3 Hub Tiers)

1. **Hub 1 — Retrieval & Embedding Hub (`mens/config/domain-profiles.yaml :: hub`):**
   - Pinned dense embedder (`Qwen/Qwen3-0.6B@c1899de...`), SHA pin verification status, hybrid `vox-search` RRF index health, unbiased Recall@5, MRR, and distractor resilience telemetry.
   - Customizable: embedder model/SHA selector, chunking & RRF top-$k$ parameters, and one-click decontamination audit.
2. **Hub 2 — Base Model Ladder & Checkpoint Merge Hub (`mens/config/gpu-specs.yaml :: train_bases` + Phase 333):**
   - Full Qwen3 dense ladder: **0.6B**, **1.7B**, **4B**, **8B**, **14B**, **27B (provisional)**, **32B** across capability tags (`qwen3_code`, `small_code_default`, `strong_code_default`, `agentic_default`).
   - Shows per-size **Local HF Cache status**, **Measured VRAM Floor (`floor_mb` & `floor_measured`)**, **Apache-2.0 License Gate**, **Phase 333 Release Receipt status** (`substantive_gate_count >= 2`, collateral damage check), and **`vox mens merge-qlora` status** (Adapter Add-On vs. Merged Complete Checkpoint).
3. **Hub 3 — Compute & Serving Dispatch Hub (`Local GPU` + `Populi Mesh` + `Cloud GPU` + `Multi-LoRA Serve`):**
   - **Local Hardware:** Live CUDA (`nvidia-smi`) / Apple Silicon Metal (`monitor_metal`) free memory (`memory_free_mb`), TFLOPS, and per-model fit verdict.
   - **Populi Mesh Hub:** Connected mesh nodes (`accelerator_free_mb`, QUIC heartbeat age, cohort capacity).
   - **Cloud Dispatch Hub:** Vast.ai / RunPod cost & time estimator (`TimeEstimator` from `gpu-specs.yaml` TFLOPS ratios) with budget ceiling guards.
   - **Multi-LoRA Serving & Champion/Challenger Router (`DomainRouter`):** Live `vox mens serve` / vLLM status, active Champion vs. Challenger adapter slots per domain, one-click **Promote Challenger** / **Rollback to Previous Champion**, and LRU eviction counter.

#### B. All Spokes (Unified 8 Organic Spokes + 9 Domain Profiles)

The studio provides a unified **Spoke Matrix & Customizer** bridging both the **8 Canonical Organic Spokes (`OrganicSpokeKind`)** and the **9 Domain Profiles (`mens/config/domain-profiles.yaml`)**:

| Organic Spoke (`SPOKE8-01`) | Core Admission Spoke (`CORPUS-01`) | Mapped Domain Profiles (`domain-profiles.yaml`) | Default Train Method | Deterministic Admission Oracle (`OracleReceipt`) |
| --- | --- | --- | --- | --- |
| **1. `vox_lang`** | `VoxLang` | `vox-lang` | `qlora` (`small_code_default`) | `lex -> parse -> HIR -> typeck -> fmt -> interp` |
| **2. `rust_workspace`** | `RustWorkspace` | `rust`, `rocks` | `qlora` (`strong_code_default`), `rag_only` | `syn::parse_file + rustc --emit=metadata + cargo_build` |
| **3. `structured_shell`** | `StructuredShell` | `harness` (shell lane) | `qlora` / `dpo` | `classify_shell_safety_floor` + `exec-policy.v1.yaml` |
| **4. `mcp_tool_chain`** | `McpToolChain` | `tool-selection`, `argument-generation` | `qlora` (`agentic_default`, trajectory boost $1.5\times$) | `vox-mcp-registry` JSON Schema + tool catalog resolution |
| **5. `agent_harness`** | `AgentHarness` | `harness`, `research`, `research-expert`, `populi-meta` | `qlora` / `dpo` / `rag_only` (`research_synthesis_eval`) | `TaskPlanEnvelope` + multi-turn trajectory validator |
| **6. `speech_to_code`** | `VoxLang` | `vox-lang` (`oratio` lane) | `qlora` | Oratio ASR transcript normalization $\rightarrow$ compilable `.vox` |
| **7. `populi_mesh`** | `AgentHarness` | `populi-meta` / `harness` | `qlora` | `PopuliMeshDispatchEnvelope` (iroh QUIC / A2A quorum schema) |
| **8. `skill_creation`** | `McpToolChain` | `tool-selection` (`agentskills` lane) | `qlora` | `agentskills.io` `SKILL.md` YAML frontmatter + `vox-*` metadata lint |

### 5.2 Studio Sub-Navigation (6 Dockable Views inside Vox Axis `mens` Surface)

Following the tabbed surface pattern in `ScientiaSurface.tsx` (`role="tablist"`, Roman display typography, Basalt & Travertine theme tokens), `MensTrainingView` is organized into 6 tabs:

1. **Topology & Hub Matrix (`topology`):**
   - Interactive Hub-and-Spoke radial/matrix map linking the Retrieval Hub, Base Model Ladder (`0.6B`–`32B`), Compute Targets (Local / Mesh / Cloud), and all 8 Organic Spokes + 9 Domain Profiles.
   - Live Champion/Challenger slot badges and Phase 333 `merge-qlora` readiness per size.
2. **Spoke Recipe Customizer (`spokes`):**
   - Select any Spoke or Domain Profile (or create a custom profile) and customize every parameter in `domain-profiles.yaml` and `gpu-specs.yaml`:
     - **Training Method:** `QLoRA`, `Full SFT`, `DPO`, `ORPO`, `RAG Only`, `Prompt Only`
     - **Base Model Rung & Preset:** `0.6B`..`32B`, preset (`tiny`, `safe`, `prosumer_12g`, `prosumer_16g`, `prosumer_24g`, `qwen3_16g`, `a6000`, `a100`, `h100`, or custom)
     - **LoRA & Optimizer Knobs:** Rank ($r \in \{8, 16, 32, 64\}$), Alpha ($\alpha$), Dropout, Learning Rate, Seq Len ($256..4096$), Micro-Batch, Grad Accum, Weight Decay, Max Grad Norm, `ce_last_k`, `min_rating`, `trajectory_weighting`, `trajectory_tool_trace_boost`, and `reward_hook`
     - **3-Epoch Curriculum Schedule Editor:** Visual sliders for Epoch 1/2/3 Max Difficulty ($1..10$) and phase ordering (`warmup`, `syntax`, `reasoning`, `tool_call`, `multi_turn`, `orchestration`, `agentic`)
     - **Inference Router Policy:** Glob & `lane:*` trigger chips, priority weight, `prefer_local` toggle, and live signal route tester (`route_by_signal` dry-run).
3. **Kiln Synthetic Studio & Topic Trees (`synthetic`):**
   - Visual **Hierarchical Topic Tree Builder** per spoke: expand root domain $\rightarrow$ sub-topics $\rightarrow$ leaf scenarios, adjust target sample counts, select Teacher model + Prompt Strategy (`CoT Distill`, `Few-Shot Seed Ladder`, `AST Mutator`, `Oracle-Fail → DPO Repair`), and inspect live `TrainingRecordV2` records with their `trusty://vox-corpus/<spoke>/<digest>` URI, `OracleReceipt` stage badge, and 1–5 star curation controls.
4. **Corpus Mix & Flywheel (`mix`):**
   - Interactive weight sliders for `mens/config/mix-*.yaml` data sources with a `require_sources` hard-gate indicator (eliminating silent missing-file drops), AST semantic diversity gauge (`flywheel.yaml :: min_ast_diversity`), sample floor progress bar, and decontamination check.
5. **5-Spoke & 8-Spoke Scorecards & Release Gates (`evals`):**
   - Preserves and upgrades `MensRunReportsPanel`: 5-Spoke `pass@1` progression (`Before → Mid → After`), 8-Spoke Organic Search-to-Compilation matrix (`Recall@5`, `Unbiased R@5`, `MRR`, `Distractor Resilience`, `Compile Pass`, `E2E pass@1`), Anti-Stub verification, Collateral Damage guard, and Phase 333 Substantive Gate Receipt (`substantive_gate_count >= 2`) with one-click `vox mens merge-qlora` promotion.
6. **Multi-LoRA Serve & Compute Fleet (`serve`):**
   - Preserves and upgrades `MensServePanel`: supervised `vox mens serve` start/stop, active multi-LoRA slot table, Champion vs. Challenger A/B traffic split & rollback, and Local / Populi Mesh / Cloud GPU job queue.

### 5.3 Live Job Progress & Configurable Hardware Time-Series Oscilloscope (`telemetry_schema.rs`)

Currently, `crates/vox-populi/src/mens/tensor/telemetry_schema.rs` defines `TRAIN_START`, `TRAIN_STEP`, `TRAIN_COMPLETE`, and `GPU_FALLBACK` with step-level progress keys (`epoch`, `step`, `loss`, `learning_rate`, `tokens_per_sec`, `valid_tokens`, `supervised_ratio_pct`, `planned_steps_per_epoch`, `planned_steps_total`, `eta_seconds_remaining`, `progress_fraction`, `steps_per_sec_ema`), while `MensTrainingView.tsx` only renders static post-hoc JSON files or a raw `vox mens status` stdout dump.

To give operators full real-time visibility into active training, distillation, and synthetic topic-tree jobs, the Vox Axis MENS Studio includes a **Configurable Multi-Trace Time-Series Oscilloscope & Job Progress Monitor**:

1. **Telemetry Schema Extension (`telemetry_schema::keys`):**
   - Add `S0` numeric resource keys sampled alongside `TRAIN_STEP` and a 1 Hz background hardware heartbeat:
     - `gpu_util_pct`: Accelerator compute utilization (`0.0..100.0`, via NVML on CUDA or `monitor_metal` on Apple Silicon).
     - `gpu_vram_used_mb` & `gpu_vram_reserved_mb`: Resident tensor + allocator footprint vs. hardware ceiling.
     - `cpu_util_pct`: Host CPU utilization across data-loader, tokenizer, and deterministic compiler-oracle worker threads.
     - `sys_ram_used_mb` & `sys_ram_total_mb`: Host system RAM / unified memory consumption.
2. **Configurable Time-Series Graph Controls in Vox Axis:**
   - **Multi-Series Channel Toggles:** Operators can independently toggle and overlay any subset of 6 live time-series channels:
     - `GPU Compute %` (`--color-accent-default` Brass)
     - `VRAM Used MB` (`--color-status-pass` Verdigris, with dashed hardware ceiling line)
     - `CPU Utilization %` (`--color-status-info` Sky)
     - `System RAM GB` (`--color-text-secondary` Steel)
     - `Training Loss (CE / DPO)` (`--color-status-warn` Terracotta, linear or log scale)
     - `Token Throughput (tok/s)` (`--color-gold-300` Gold)
   - **Display & Axis Customization:**
     - **View Mode:** `Overlay Multi-Trace (0–100% Normalized + Dual Axis)` vs. `4-Lane Stacked Horizon Strips (GPU · VRAM · CPU/RAM · Loss/Tok/s)`.
     - **Time Window / Horizon:** `1m Live`, `5m`, `15m`, `Full Run (All Epochs)`.
     - **X-Axis Basis:** `Wall-Clock Time (s)` vs. `Optimizer Step (global_step)`.
     - **Signal Smoothing:** `Raw + EMA Envelope` vs. `EMA Smoothed (steps_per_sec_ema)`.
     - **Interactive Crosshair & Event Pins:** Hovering anywhere over the graph scrubs exact correlated metrics at that step/timestamp and highlights `Epoch 1→2→3` transitions, checkpoint saves, and `gpu_fallback` warnings.
3. **Job Progress & Step ETA Ribbon:**
   - Displays active job ID, target spoke & base rung, `progress_fraction` bar segmented by curriculum epoch (`Epoch 1: Syntax` $\rightarrow$ `Epoch 2: Reasoning` $\rightarrow$ `Epoch 3: Agentic`), `global_step / planned_steps_total`, `steps_per_sec_ema`, countdown `eta_seconds_remaining`, `supervised_ratio_pct`, and one-click `Checkpoint Now` / `Pause` / `Abort` controls.

### 5.4 Full Ladder Versioning, Master-to-Local Sync, and AFK Serial Training Queue (`size_train.v1` + `harness_before_after.v1`)

To eliminate manual CLI bookkeeping across the 6 pinned Qwen3 sizes (`0.6B`, `1.7B`, `4B`, `8B`, `14B`, `32B` in `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`) and 8 Organic Spokes, Vox Axis MENS Studio exposes a **Full Ladder Versioning & AFK Orchestration Controller**:

1. **Deterministic Version & Gate State per Rung (`size_train.v1` & `harness_before_after.v1`):**
   - Each rung row displays its exact pinned repository & 40-hex revision (`Qwen/Qwen3-<size>@<sha40>` from `mens/config/gpu-specs.yaml :: train_bases.qwen3_code`), license class (`apache-2.0`), measured VRAM floor (`floor_mb`), and live state resolved by `crates/vox-ml-cli/src/commands/mens/size_train.rs` and `harness_before_after.rs`:
     - `TRAINED + GATED` (`status == "trained"` + `adapter_manifest.json` matches pinned snapshot + `assert_collateral_gate` OK + `assert_serve_preconditions` `substantive_gate_count >= 2`): Ready for `vox mens serve` or `vox mens merge-qlora`.
     - `TRAINED · UNGATED` (`status == "trained"`, awaiting `vox mens eval-gate --five-spokes` / `eval-collateral-damage`): One-click `[Run Gate Verification]`.
     - `TRAIN_NOT_RUN` (`reason == "train_not_run"`): Base snapshot verified on local disk (`snapshot_complete == true`), no matching adapter for this SHA pin yet.
     - `STALE_LOCAL_SHA`: Local adapter `adapter_manifest.json :: base_model` or remote master adapter revision differs from current repo `gpu-specs.yaml` master pin.
     - `BASE_SNAPSHOT_ABSENT` (`reason == "base_snapshot_absent"`): Pinned base weights not yet cached in `HF_HUB_CACHE`.
     - `DOES_NOT_FIT_LOCAL` (`reason == "does_not_fit"`, `floor_mb > free_mb`): Automatically routes to **Populi Mesh** or **Cloud GPU (`cloud-estimate`)** when queued.
2. **One-Click Master-to-Local Version Sync (`Update All From Remote`):**
   - Compares local `HF_HUB_CACHE` base snapshots and `mens/runs/` adapter manifests against the repository master pins (`mens/config/gpu-specs.yaml` and upstream Hugging Face Hub repos via `vox mens hub download`).
   - Provides both per-rung `[⬇ Pull Master]` buttons and a global **`[⟳ Update All Local ← Repo Master]`** batch action that downloads missing/updated base snapshots and published master adapters without blocking the UI thread.
3. **Unattended Serial Queue (`AFK Train All Missing in Series`):**
   - Operators can click **`[🌙 AFK Train All Missing in Series]`** (or select any subset of missing/stale `Rung × Spoke` cells) to build an unattended sequential execution queue ordered smallest-floor-first (`0.6B → 1.7B → 4B → 8B`, matching `write_size_train_records` ordering in `size_train.rs`).
   - For each queued rung/spoke item, the AFK runner executes the deterministic stage chain in series:
     1. **Ensure Base Snapshot:** `vox mens hub download` if `base_snapshot_absent`.
     2. **Refresh Corpus & Kiln Topic Tree:** `vox mens pipeline --skip-train` (when `TrainDataModeCli::Strict` detects a stale corpus fingerprint).
     3. **Train Adapter:** `vox mens train --model <hf_id@sha> --domain <profile> --checkpoint-every 500` (supports `--background` mode: `--process-priority low --vram-limit-fraction 0.8`).
     4. **Run Substantive & Collateral Gates:** `vox mens eval-local`, `vox mens eval-collateral-damage`, and `vox mens eval-gate --five-spokes`, followed by `vox mens harness-before-after`.
     5. **Optional Auto-Merge / Auto-Serve:** `vox mens merge-qlora` and hot-swap into `MensServePanel` (`resolve_harness_serve_target`) only if all blocking gates pass.

### 5.5 End-to-End Real Backend Plumbing Matrix (Zero Placeholders)

Every visual control, dial, badge, and time-series trace in the MENS Studio is directly wired to a concrete Rust type, CLI command, or Tauri IPC command:

| GUI Control / Visualization | Concrete Backend Source / Command (`crates/`) | Persisted Artifact / Config SSOT |
| --- | --- | --- |
| **Qwen3 Ladder Rung Status (`0.6B`..`32B`)** | `vox mens size-train` (`crates/vox-ml-cli/src/commands/mens/size_train.rs`) + `resolve_qwen3_size` (`spoke_base_resolver.rs`) | `mens/runs/size_train/Qwen3-<size>-<sha12>.json` (`size_train.v1`) |
| **Before/After Reasoning & Intelligence Matrix** | `vox mens harness-before-after` (`crates/vox-ml-cli/src/commands/mens/harness_before_after.rs`) | `mens/runs/harness_before_after.json` (`harness_before_after.v1`) |
| **5-Spoke & 8-Spoke Organic Scorecards + Gate Receipt** | `mens_run_reports` Tauri cmd (`crates/vox-gui/src/commands/mens_run_reports.rs`) + `vox mens eval-gate --five-spokes` | `gate_receipt.json`, `mens_spoke_scorecard.json`, `organic_spoke_scorecard.json` |
| **Update All From Remote / Publish to Hub** | `vox mens hub download` / `vox mens hub upload` (`crates/vox-ml-cli/src/commands/mens/hub.rs`, `vox_populi::mens::hub`) | `HF_HUB_CACHE` snapshots + `adapter_manifest.json` |
| **AFK Serial Training Queue & Background Mode** | `vox mens train` (`PopuliAction::Train`: `--model`, `--domain`, `--preset`, `--background`, `--vram-limit-fraction`, `--process-priority`, `--checkpoint-every`, `--resume`) | `mens/runs/<run_id>/checkpoint_state.json` + `adapter_manifest.json` |
| **Rotary Hyperparameter Dials (`rank`, `alpha`, `seq_len`, `grad_accum`, `lr`, `ce_last_k`, `trajectory_*`)** | `DomainProfileConfig` (`crates/vox-populi/src/mens/tensor/domain_profiles.rs`) + `vox mens probe --detailed --sweep --model <id>` (`memory_model.rs`) | `mens/config/domain-profiles.yaml` + `contracts/mens/training-presets.v1.yaml` |
| **3-Epoch Curriculum Staircase & Difficulty Caps** | `CurriculumSchedule` (`domain_profiles.rs`) + `--curriculum` flag in `vox mens train` | `mens/config/domain-profiles.yaml :: curriculum_schedule` |
| **Kiln Topic Tree (`SpokeTopicTreeV1`) & Auto-DPO Repair** | `vox mens corpus generate` / `heal-to-dpo` / `rft` (`PipelineStage` in `action_prelude.rs`) + `TrainingRecordV2` & `ChosenRejectedPairV2` (`vox-corpus/src/training/contract.rs`) | `mens/config/topic-trees/<spoke>.yaml` + `target/dogfood/train.jsonl` |
| **Phase 333 QLoRA Merge & GGUF Export** | `vox mens merge-qlora` (`PopuliAction::MergeQlora`: `--base-shard`, `--adapter`, `--meta`, `--quantize`, `--gguf-out`) | `recombined_full/` + quantized `.safetensors` / `.gguf` |
| **Supervised Multi-LoRA Serve & Routing** | `mens_serve_status` / `mens_serve_start` / `mens_serve_stop` (`crates/vox-gui/src/commands/mens_serve.rs`) + `DomainRouter` (`domain_router.rs`) | Live HTTP server (`:8089`) + `resolve_harness_serve_target` |
| **Live Job Progress & Hardware Time-Series Oscilloscope (`GPU %`, `VRAM`, `CPU %`, `RAM`, `Loss`, `Tok/s`)** | `vox mens watch-telemetry` / `train_log.rs` (`telemetry_schema.rs`: `TRAIN_STEP`, `loss`, `tokens_per_sec`, `eta_seconds_remaining`, `progress_fraction`, `gpu_util_pct`, `gpu_vram_used_mb`, `cpu_util_pct`, `sys_ram_used_mb`) | `mens/runs/latest/telemetry.jsonl` |


