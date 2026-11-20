# Stack Research

**Domain:** Cross-platform live memory probing, Qwen3.5/3.8 inference vs. QLoRA memory scaling, and deterministic zero-new-dependency dual-theme SVG → `.webp` rendering (Milestone v9.2: `adaptive-model-scaling-hf-cards-v92`)
**Researched:** 2026-11-19
**Confidence:** HIGH

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `objc2-metal` | `0.3.2` (pinned `Cargo.toml:435`, `Cargo.lock:9257`) | macOS Apple Silicon live Metal working-set (`recommendedMaxWorkingSetSize`, `currentAllocatedSize`, `maxBufferLength`) | Already wired behind `mens` in `crates/vox-populi/Cargo.toml:173-177`; zero C/CMake build deps; provides exact driver headroom paired with `vm_stat` (`macos_metal.rs:32-194`, `accel_budget.rs:70-86`). |
| `nvml-wrapper` (`vox-plugin-nvml-probe`) + `nvidia-smi` | `0.12.1` (pinned `Cargo.toml:331`, `Cargo.lock:9060`) | NVIDIA CUDA per-device live `memory.free` / `memory.total` / `memory.used` telemetry across Windows & Linux | Runtime-dlopen (`nvml.dll` / `libnvidia-ml.so`) in L4 plugin `crates/vox-plugin-nvml-probe/src/probe.rs:53-180` with `nvidia-smi` CSV fallback (`vram_autodetect.rs:45-57`), respecting `CUDA_VISIBLE_DEVICES` + `CUDA_DEVICE_ORDER=PCI_BUS_ID` (`size_select.rs:84-111`). |
| `sysinfo` | `0.39.3` (pinned `Cargo.toml:282`, `Cargo.lock:13868-13880`) | Cross-platform live host RAM (`available_memory()`, `total_memory()`) for CPU inference & Windows/Linux unified fallback | Already in `Cargo.lock` and `crates/vox-populi/Cargo.toml:200` (`[dev-dependencies]`) and `crates/vox-orchestrator/src/services/local_resources.rs:24-56`; covers Windows (`GlobalMemoryStatusEx`), Linux (`/proc/meminfo`), and macOS Mach VM without adding any crate to `Cargo.lock`. |
| Linux DRM sysfs + `/proc/meminfo` (Rust `std::fs`) | OS native | Linux AMD (`amdgpu`) / Intel dedicated VRAM (`mem_info_vram_total`, `mem_info_vram_used`) and unified RAM fallback | Zero dependencies; fixes `crates/vox-populi/src/mens/hardware/linux_drm.rs:39,59` returning `vram_mb: 0` without `libdrm`/`vulkan` FFI or `cmake`/`libclang`. |
| `windows-sys` | `0.59.0` (pinned `Cargo.toml:357`, `Cargo.lock:20566-20572`) | Windows live GPU/RAM native Win32 probing (DXGI adapter VRAM budget or `GlobalMemoryStatusEx` when NVML absent) | Already in `Cargo.lock` (`windows-targets 0.52.6` only; adding Win32 feature flags causes zero `Cargo.lock` churn and adds zero `cmake`/`nasm`/`libclang` deps). Replaces `reg query` stub (`windows_fallback.rs:45-85`). |
| `sharp` (`@img/sharp-libvips-*`) | `0.35.4` (pinned `docs-astro/package.json:23`, `docs-astro/pnpm-lock.yaml:6362`) | Deterministic SVG/PNG → `.webp` rasterization & optimization | Prebuilt native `libvips` + `librsvg` binaries across macOS/Linux/Windows; zero new npm or Cargo deps; supports lossless and lossy WebP (`quality`, `effort: 6`, `sharp_yuv`-class output) for Hub cards and docs assets. |
| `@playwright/test` (`chromium`) | `1.63.0` (`docs-astro/package.json:30`, `docs-astro/pnpm-lock.yaml:3966`) | Deterministic headless HTML/CSS/SVG dual-theme rendering (`data-theme="dark"` / `"light"`) when rich CSS Grid/Flexbox or `@fontsource/arimo` webfonts are required | Proven in Milestone v8.9 (`GFX-01`, `GFX-03`) and v9.0 (`PERF-04`, `frontpage-perf-and-cv-flywheel.spec.ts`); shares CSS token system (`reader-layout.css:6-69`, `vox-diagram.css:37-46`) and self-hosted `Arimo` WOFF2 (`docs-astro/public/fonts/arimo/`). |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `vox-diagram` | `0.6.0` (in-repo L2 crate `crates/vox-diagram/Cargo.toml`) | Deterministic pure-Rust architecture/flow graph layout (`dagre =0.1.1`) & SVG emission with `THEME_VARS` (`svg/mod.rs:76-99`) | Use for structured architecture/node-edge diagrams rendered inline in docs (`remark-vox-diagram.mjs`) or exported to SVG and rasterized to `.webp` for README/Hub collateral. |
| `vox-hf-layout` | `0.6.0` (in-repo `crates/vox-hf-layout/src/lib.rs`) | Parses `HfTransformerLayout` (`layer_types`, `num_key_value_heads`, `head_dim`, `linear_num_key_heads`, `linear_num_value_heads`, `linear_key_head_dim`, `linear_value_head_dim`, `tie_word_embeddings`) from `config.json` | Use to compute exact per-model inference weight + hybrid Gated DeltaNet/GQA KV-cache memory footprints from Hub metadata or local `config.json` instead of flat `0.5 * 1.15`. |
| `vox-quantize` | `0.6.0` (in-repo `crates/vox-quantize/src/policy.rs`) | Exact tensor-role `QuantMixture` (`Q4KM`, `Q5KM`, `Q6K`, `Q8_0`) and exact GGML block-size sizing (`Q4K = 4.5 bpw`, `Q6K = 6.5625 bpw`, `KeepF32` norms/conv1d) | Use when sizing quantized local artifacts (`plan_quantize` at `policy.rs:140-193`) so Q4 inference footprints reflect real tensor roles rather than a single constant. |
| `askama` | `0.12` (pinned `Cargo.toml:434`, `vox-populi/Cargo.toml:154`) | Deterministic markdown Hub card template rendering (`templates/hub/model.md`, `index.md`, `partials/*.md`) | Already powers `hub_card/render.rs:11-47`; update templates/assets cleanly alongside `hub_cards_push.rs:35-44` (which already admits `assets/*.{jpg,jpeg,png,webp}`). |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cwebp` / `img2webp` (`libwebp`) | CLI static/animated WebP verification & tuning (`/opt/homebrew/bin/cwebp`, `/opt/homebrew/bin/img2webp`) | Used in v9.0 (`533-01-SUMMARY.md:18-20`) with `-q 76..80 -m 6 -sharp_yuv`; optional developer/CI check tool, while `sharp` `0.35.4` handles cross-platform Node script generation without requiring system `libwebp`. |
| `vox-arch-check` & `vox ci crate-edges` | Enforces `docs/src/architecture/layers.toml` & `contracts/ci/crate-edges.allow.v1.json` | Critical for v9.2: `vox-populi` and `vox-orchestrator` are both Layer 3; `vox-plugin-nvml-probe` is Layer 4 (`layers.toml:243,248,295`). Never add an L3→L4 Cargo dependency. |
| `cargo test -p vox-populi --features mens-hf-hub --lib` | Runs `hub_card/lints.rs` duplicate-sentence, anchor, HF frontmatter, `.svg` ban, and golden parity checks | Any Hub card image change (`.jpg` → `.webp` or dual-theme `<picture>`) must update `hub_card/tests/model_card.rs:223-233`, `tests/index_card.rs:89-90`, and `tests/golden/hub_cards/*.md`. |

## Installation & Build-Toolchain Constraints

**Zero New Dependencies Required (`Cargo.lock` churn = 0, `pnpm-lock.yaml` churn = 0).**

Per `AGENTS.md:189-196` (*Build-toolchain invariant*), a clean clone must build with only the pinned Rust toolchain (`1.96`), the platform C compiler, and Node + `pnpm` — **no dependency may add `cmake`, `nasm`, `Go`, `perl`, or `libclang`**. All recommended libraries are already resolved in `Cargo.lock` and `docs-astro/pnpm-lock.yaml`:

```bash
# Verify existing Rust crates compile without adding Cargo.lock entries
cargo check -p vox-populi --features mens
cargo check -p vox-orchestrator --features "runtime,system-metrics"

# Verify existing Node/Playwright/Sharp pipeline in docs-astro
pnpm --dir docs-astro exec node -e "import('sharp').then(m => console.log('sharp', m.default.versions))"

# Verify Hub card golden & lint suite (must remain green on any asset/template update)
cargo test -p vox-populi --features mens-hf-hub --lib hub_card
```

*(Note: If promoting `sysinfo` in `crates/vox-populi/Cargo.toml` from `[dev-dependencies]` (`line 200`) to `mens` optional `[dependencies]`, `sysinfo = "0.39.3"` is already present in `Cargo.lock:13868-13880`, so `Cargo.lock` adds zero new crates. Same for enabling `Win32_System_SystemInformation` or `Win32_Graphics_Dxgi` feature flags on `windows-sys = "0.59.0"`, which depends solely on `windows-targets 0.52.6` in `Cargo.lock:20566-20572`.)*

---

## Verified In-Tree Gaps & Architectural Ground Truth

### 1. Cross-Platform Live Memory Probing Gaps

Inspection of the current probing stack reveals **four concrete split-brain gaps** between `vox-populi` and `vox-orchestrator`:

1. **`vox-orchestrator/src/models/vram.rs` is NVML-only (`lines 16-19, 174-209`):**
   - `refresh_free_vram_hint_from_nvml()` only queries `registered_vram_probe()` (which `crates/vox-orchestrator-mcp/src/server_state/vram_probe.rs:42-62` populates solely from L4 `vox-plugin-nvml-probe`).
   - On Apple Silicon (Metal), Linux AMD/Intel DRM, and Windows non-NVIDIA or CPU-only hosts, `free_vram_mb_hint()` returns `None` (`VramFit::Unknown`), disabling local-model VRAM scoring entirely.
2. **`vox-orchestrator/src/models/auto_select.rs` evaluates inference through the QLoRA training ladder (`lines 49-86, 173-187`):**
   - `select_for_free_memory()` delegates to `vox_populi::mens::tensor::size_select::select_size()` (`size_select.rs:192-199`), which hardcodes tag `"qwen3_code"` and compares live free memory against **QLoRA training `floor_mb`** (`15,319 MB` for `0.6B`, `34,460 MB` for `4B`, **`122,876 MB` for `Qwen3-8B`** in `mens/config/gpu-specs.yaml:280-316`).
   - Result: `auto_select.rs:173-187` requires `130,000 MB` free just to pick `Qwen3-8B` for local **inference**, and fails closed on a 16 GB RTX 4080 Super (`~12.6 GB` usable) or 16–24 GB Apple Silicon Mac where Q4 inference of `Qwen3.5-9B` (`~5.5 GB`) or `Qwen3.5-4B` (`~2.6 GB`) fits easily.
3. **`size_select::read_free_memory()` misses non-NVIDIA Linux/Windows GPUs and host RAM fallback (`size_select.rs:69-79`):**
   - On `target_os = "macos"`, `query_accel_budget()` (`accel_budget.rs:70-86`) combines `MTLDevice.recommendedMaxWorkingSetSize()` with `vm_stat` (`free + inactive + speculative - max(15%, 2 GiB)` in `macos_metal.rs:75-157`), and rejects `working_set_is_live == false` (`size_select.rs:60-65`).
   - On `not(target_os = "macos")`, `size_select.rs:75-77` **only** calls `vram_autodetect::query_nvidia_smi_free_mb()`. It never reads Linux DRM `mem_info_vram_*` or `/proc/meminfo`, Windows DXGI / `GlobalMemoryStatusEx`, or `sysinfo::System::available_memory()`.
4. **`linux_drm.rs` and `windows_fallback.rs` emit `vram_mb: 0` (`linux_drm.rs:39,59`, `windows_fallback.rs:45`):**
   - `linux_drm.rs:51-73` enumerates `/sys/class/drm/card*` but never reads `/sys/class/drm/card*/device/mem_info_vram_total` and `mem_info_vram_used` (exposed in bytes by Linux `amdgpu`), nor `/sys/class/drm/card*/device/vendor` (`0x1002` AMD, `0x8086` Intel, `0x10DE` NVIDIA).
   - `windows_fallback.rs:28-85` shells out to `reg query` for `DriverDesc` and hardcodes `vram_mb: 0` and `ComputeBackend::Cpu`.

### 2. Inference vs. QLoRA Training Memory Specs (Qwen3.5 & Qwen3.8)

#### Exact Architectural Parameters (Verified from Pinned HF Snapshots & `vox-hf-layout`)

All five active Nov 2026 rungs in `mens/config/gpu-specs.yaml:340-345` share `model_type: "qwen3_5"` (`architectures: ["Qwen3_5ForConditionalGeneration"]`), `vocab_size: 248320`, `max_position_embeddings: 262144`, `full_attention_interval: 4` (3:1 hybrid ratio: **75% `linear_attention` Gated DeltaNet**, **25% `full_attention` GQA**), `head_dim: 256`, `partial_rotary_factor: 0.25` (64 rotary dims), `linear_conv_kernel_dim: 4`, `linear_key_head_dim: 128`, `linear_value_head_dim: 128`, `linear_num_key_heads: 16`, and `mamba_ssm_dtype: "float32"`:

| Model (Pinned SHA) | Layers (`Lin` / `Full`) | `hidden_size` / `inter` | GQA `n_heads` / `n_kv_heads` (`head_dim=256`) | DeltaNet `v_heads` (`k_heads=16`, `d=128`) | `tie_word_embeddings` | Total Safetensors Index (`BF16` VLM+MTP) |
|---|---:|---:|---:|---:|:---:|---:|
| `Qwen/Qwen3.5-0.8B` (`2fc06364...`) | `24` (`18` / `6`) | `1024` / `3584` | `8` / `2` | `16` | `true` | `1,746,882,752 B` (`1.63 GiB`) |
| `Qwen/Qwen3.5-2B` (`15852e8c...`) | `24` (`18` / `6`) | `2048` / `6144` | `8` / `2` | `16` | `true` | `4,548,144,832 B` (`4.24 GiB`) |
| `Qwen/Qwen3.5-4B` (`851bf6e8...`) | `32` (`24` / `8`) | `2560` / `9216` | `16` / `4` | `32` | `true` | `9,319,737,856 B` (`8.68 GiB`) |
| `Qwen/Qwen3.5-9B` (`c2022362...`) | `32` (`24` / `8`) | `4096` / `12288` | `16` / `4` | `32` | `false` | `19,306,216,416 B` (`17.98 GiB`) |
| `Qwen/Qwen3.8-27B` (`37561816...`) | `64` (`48` / `16`) | `5120` / `17408` | `24` / `4` | `48` | `false` | `55,562,855,904 B` (`51.75 GiB`) |

*(Source: Cached pinned `config.json` & `model.safetensors.index.json` under `~/.cache/huggingface/hub/models--Qwen--Qwen3.*`; note `is_vision_or_mtp_key` in `crates/vox-hf-layout/src/lib.rs:82-87` strips `model.visual.*` and `mtp.*` for text-only loading, reducing resident weights below the full VLM index size.)*

#### Hybrid Inference Memory Formula (Weights + Constant DeltaNet State + Linear-in-T GQA KV Cache)

Replacing `crates/vox-orchestrator/src/models/vram.rs:47-54`'s flat `0.5 * 1.15` heuristic with architecture-aware inference sizing yields:

$$\text{Bytes}_{\text{inf}}(T, B) = \text{Weights}(Q) + B \cdot \Big(\underbrace{\text{GDN}_{\text{state}} + \text{GDN}_{\text{conv}}}_{\text{constant wrt } T} + T \cdot \text{KV}_{\text{GQA/tok}}\Big) + \text{Scratch}$$

Where per-sequence ($B=1$) hybrid state terms at `mamba_ssm_dtype = float32` (`4 B`) and `BF16/FP16` KV cache (`2 B`) are:

- **Gated DeltaNet Recurrent State** (fixed per sequence across $L_{\text{lin}}$ layers):
  `L_lin * linear_num_value_heads * linear_key_head_dim * linear_value_head_dim * 4` bytes
  Plus short causal Conv1D ring buffer (`conv_kernel - 1 = 3`):
  `L_lin * (2 * linear_num_key_heads * linear_key_head_dim + linear_num_value_heads * linear_value_head_dim) * 3 * 4` bytes
- **Full-Attention GQA KV Cache** (grows linearly with context $T$ across **only** $L_{\text{full}} = L/4$ layers):
  `L_full * 2 * num_key_value_heads * head_dim * 2 * T` bytes

| Rung | Q4 Weights (`~0.55 B/p` text, MiB) | BF16 Weights (`2.0 B/p` text, MiB) | Fixed DeltaNet + Conv State (F32, MiB/seq) | Full-Attn GQA KV Rate (BF16, KiB/tok) | Total Inference @ `Q4`, `T=8192` (+512 MiB scratch) | Total Inference @ `BF16`, `T=8192` (+512 MiB scratch) | Measured Metal QLoRA Training Floor (`gpu-specs.yaml:341-345`, `seq=1024`) |
|---|---:|---:|---:|---:|---:|---:|---:|
| `0.8B` | ~420 MiB | ~1,440 MiB | `19.3 MiB` | `12.0 KiB/tok` (`96 MiB` @ 8K) | **~1,047 MiB (~1.0 GiB)** | **~2,067 MiB (~2.0 GiB)** | `17,000 MiB` (peak `16,393 MiB`) |
| `2B` | ~1,050 MiB | ~3,815 MiB | `19.3 MiB` | `12.0 KiB/tok` (`96 MiB` @ 8K) | **~1,677 MiB (~1.6 GiB)** | **~4,442 MiB (~4.3 GiB)** | `19,500 MiB` (peak `18,740 MiB`) |
| `4B` | ~2,100 MiB | ~7,630 MiB | `50.3 MiB` | `32.0 KiB/tok` (`256 MiB` @ 8K) | **~2,918 MiB (~2.8 GiB)** | **~8,448 MiB (~8.3 GiB)** | `34,000 MiB` (peak `33,090 MiB`) |
| `9B` | ~4,720 MiB | ~17,166 MiB | `50.3 MiB` | `32.0 KiB/tok` (`256 MiB` @ 8K) | **~5,538 MiB (~5.4 GiB)** | **~17,984 MiB (~17.6 GiB)** | `36,000 MiB` (peak `33,648 MiB`) |
| `27B` | ~14,160 MiB | ~51,500 MiB | `149.6 MiB` | `64.0 KiB/tok` (`512 MiB` @ 8K) | **~15,334 MiB (~15.0 GiB)** | **~52,674 MiB (~51.4 GiB)** | `74,000 MiB` (peak `73,503 MiB`) |

*(Notes: (1) Q4 footprint uses `0.55 B/param` baseline; `vox-quantize` `Q4KM` outputs `4.5 bpw` = `0.5625 B/param` for `Matrix`, `6.5625 bpw` = `0.8203 B/param` for `DownProj`/`VProj`/`Embedding`/`Output`, and `F32` for norms/conv1d/A_log/dt_bias (`crates/vox-quantize/src/policy.rs:80-106,296-297`), putting quantized `.vox`/GGUF files around `0.56–0.65 B/param` depending on `248,320`-vocab untied head weight share. (2) Training floors are 3×–16× higher than Q4 inference because `memory_model.rs:287` sets `cache_dequantized=true` (`BF16` weights on GPU) for `approx_linear_params <= 10B`, plus `act_bytes_per_lht = 162.48240311940512` B/LHT in `contracts/mens/memory-model.v1.yaml:43`, plus `[seq_len, 248320]` backward logits documented in `2026-11-13-candle-metal-qwen35-calibration.md:52-53`.)*

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| **`sysinfo 0.39.3` + native OS probes (`objc2-metal`, NVML, DRM sysfs, `windows-sys`)** | Pull `wgpu` / `ash` (Vulkan) solely to query GPU VRAM | Only if Burn/wgpu compute were restored; `wgpu` was removed from `vox-populi` (`pipeline.rs:173`) because it adds heavy GPU shader/driver deps and fails headless without Vulkan ICDs. |
| **Pre-resolved `sharp 0.35.4` (and Playwright Chromium for complex HTML/Arimo plates) → dual-theme `.webp`** | Add Rust `resvg` + `usvg` + `tiny-skia` + `fontdb` + `webp` (`libwebp-sys`) crates | Only if the `vox` CLI binary itself must rasterize arbitrary runtime-authored SVGs in an environment with no Node/`pnpm` available. Avoid for static/build-time Hub & docs graphics: `resvg`/`libwebp-sys` bloat `Cargo.lock` and risk C toolchain churn (`AGENTS.md:189-196`). |
| **Direct `sharp(svgBuffer).webp()` for pure vector/inline-hex diagrams** | Playwright Chromium screenshot + `sharp` for every SVG | Use Playwright Chromium + `sharp` whenever graphics rely on HTML/CSS Flexbox/Grid, external stylesheets (`reader-layout.css`, `vox-diagram.css`), CSS `var(--sl-*)` theme switching, or `@fontsource/arimo` `.woff2` font metrics (`docs-astro/public/fonts/arimo/`). Use direct `sharp` when generating parameterized pure SVGs with resolved dark/light hex palettes (faster, zero Chromium launch). |
| **Single-theme high-contrast raster `.webp` on Hugging Face Hub (`assets/banner.webp`, `assets/overview.webp`)** | GitHub/HF `<picture><source media="(prefers-color-scheme: ...)">` dual-theme `.webp` | Hugging Face Hub's Markdown sanitizer allows `<picture><source media="(prefers-color-scheme: dark)">` in model cards, OR single-theme dark/light neutral plates (`#0b0f17` obsidian cards with light accents) work everywhere (HF Hub, GitHub, PyPI, plain Markdown viewers). If `<picture>` is used on HF Hub, verify HF Hub preview rendering and update `hub_card/lints.rs` / `tests/` accordingly; for `docs-astro`, dual-theme `<picture>` or `html[data-theme]` CSS swap over `-dark.webp` / `-light.webp` avoids contrast bleed. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Inline `<svg>` or `.svg` references in Hugging Face Hub model cards** | Violates `hub_card/tests/model_card.rs:223-233`, `tests/index_card.rs:90`, `tests/model_card/use_it.rs:244`, `hub_card/svg.rs:106`, and `AGENTS.md:762` (architecture graphics in READMEs must be raster `.webp`/`.png`, never raw `.svg`, and HF Hub sanitizers strip `<style>` / CSS variables inside SVGs). | Pre-rendered deterministic `.webp` rasters (`hub_cards_push.rs:39` already admits `assets/*.webp`). |
| **Adding `resvg`, `usvg`, `fontdb`, `tiny-skia`, `webp` (`libwebp-sys`), or `image` to `Cargo.toml`** | Unnecessary `Cargo.lock` churn, increases 3rd-party Rust compile bloat (91% of build time per `layers.toml:60`), and `libwebp-sys` risks violating `AGENTS.md:189-196` build-toolchain invariants. | `sharp 0.35.4` + `@playwright/test 1.63.0` already locked in `docs-astro/pnpm-lock.yaml`. |
| **Using `qwen3_code` QLoRA training `floor_mb` (`size_select.rs:198`) to gate inference** | Over-estimates inference memory by 3×–16× (`122,876 MB` for Qwen3-8B vs `~5.4 GB` Q4 inference for Qwen3.5-9B), causing `select_optimal_local_model_for_host()` (`auto_select.rs:90-92`) to fail closed on 16–32 GB GPUs and laptops. | Dedicated inference footprint estimator parameterized by `(quant_mode, active_layers, linear/full layers, kv_heads, head_dim, context_len)` alongside separate `qwen3_5_code` training gate. |
| **Static `MTLDevice.recommendedMaxWorkingSetSize` alone on macOS without `vm_stat` pressure** | Does not reflect memory consumed by browser/IDE/other processes (`accel_budget.rs:51-54`); `size_select.rs:56-65` explicitly rejects non-live readings (`D-01`). | `min(recommendedMaxWorkingSetSize, live_pressure_budget_bytes())` (`accel_budget.rs:59-85`), or `sysinfo::available_memory()` minus safety reserve when `vm_stat` fails. |
| **Static L3→L4 Cargo edge from `vox-populi` or `vox-orchestrator` to `vox-plugin-nvml-probe`** | Forbidden by `docs/src/architecture/layers.toml:243,248,295` and `AGENTS.md:456-477`; `vox ci crate-edges` fails on upward dependencies or unapproved `exceptions`. | Runtime plugin ABI via `vox-plugin-host` (`vram_probe.rs:42-62`) + `VramProbe` trait registration, or `nvidia-smi` CLI fallback in `vox-populi`. |
| **Generative AI image API (`image-generator`) for technical telemetry/scaling charts or CI assets** | Non-deterministic across runs, hits `429 RESOURCE_EXHAUSTED` (experienced in v8.9 `GFX-01`/`GFX-03`), cannot guarantee exact numeric tables/labels, and bloats `.jpg` hero files (`~745 KB` per banner in `crates/vox-populi/assets/hub/*.jpg`). | Deterministic SVG/HTML → `.webp` pipeline (`sharp 0.35.4` / Playwright Chromium) achieving `< 100 KB` crisp vector-derived rasters (`65–113 KiB` observed in `v8.9`/`v9.0`). |

## Stack Patterns by Variant

**If running live memory probing on macOS (Apple Silicon unified memory):**
- Primary: `accel_budget::query_accel_budget()` (`objc2-metal 0.3.2` `MTLCreateSystemDefaultDevice` + `vm_stat` `live_pressure_budget_bytes()` with `max(15%, 2 GiB)` OS reserve, taking `min(driver_advisory, live)`).
- Live delta telemetry: `macos_metal::monitor_metal()` (`currentAllocatedSize()`).
- Fallback when `vm_stat` locale/spawn fails: `sysinfo::System::available_memory()` with the same `max(15%, 2 GiB)` reserve, capped by `recommendedMaxWorkingSetSize` (since Apple unified memory shares the pool).

**If running live memory probing on Linux:**
- NVIDIA discrete GPU: `registered_vram_probe()` (`vox-plugin-nvml-probe` via `nvml-wrapper 0.12.1`) → fallback `nvidia-smi --query-gpu=index,memory.free --format=csv,noheader,nounits`, taking `visible_min_free_mb(...) * SAFETY_FRACTION (0.88)`.
- AMD / Intel discrete or APU GPU: read `/sys/class/drm/card*/device/{vendor,mem_info_vram_total,mem_info_vram_used}` in `linux_drm.rs` (`free = (total - used) * 0.88`).
- CPU / host RAM inference fallback: read `/proc/meminfo` (`MemAvailable`) or `sysinfo::System::available_memory()` × `0.85` (keeping OS reserve).

**If running live memory probing on Windows:**
- NVIDIA discrete GPU: `registered_vram_probe()` (`vox-plugin-nvml-probe` / `nvml.dll`) → fallback `nvidia-smi.exe` × `0.88`.
- AMD / Intel / multi-adapter GPU (without NVML): DXGI `CreateDXGIFactory1` → `IDXGIAdapter3::QueryVideoMemoryInfo` (`DXGI_MEMORY_SEGMENT_GROUP_LOCAL` -> `Budget - CurrentUsage`) via `windows-sys 0.59` (or `GetDesc1` `DedicatedVideoMemory` capped conservatively when `IDXGIAdapter3` is absent).
- CPU / host RAM inference fallback: `sysinfo::System::available_memory()` (backed by `GlobalMemoryStatusEx` via `windows 0.62.2` already in `Cargo.lock`) × `0.85`.

**If rendering dual-theme `.webp` graphics for Docs (`docs-astro`) vs. Hub Cards (`vox-populi`):**
- **Docs (`docs/src/assets/`, `docs-astro/public/assets/`):** Render both `*-dark.webp` (`#0b0f17` canvas, `#38bdf8` cyan, `#f59e0b` amber, `#f8fafc` text — matching `reader-layout.css:24-44`) and `*-light.webp` (`#f8fafc` canvas, `#0284c7` cyan, `#d97706` amber, `#0f172a` text — matching `reader-layout.css:47-68`) via Playwright + `sharp` or direct `sharp(svg).webp({ quality: 86, effort: 6 })`. Enforce `frontpage-perf-and-cv-flywheel.spec.ts:384-404` budgets (`width`, `height`, `loading="lazy"`, `decoding="async"`, `object-fit: contain`, `aspectDrift <= 0.03`).
- **HF Hub Cards (`crates/vox-populi/assets/hub/*.webp` + `hub_card/svg.rs`):** Keep `svg.rs` outputting raster bytes (update `assets/banner.jpg` / `assets/overview.jpg` to `.webp` or emit `<picture>` pairs if dual-theme Hub markup is chosen; note `hub_cards_push.rs:39` already whitelists `.webp`, while `model.md:6`, `index.md:27`, `card_facts.rs:157`, and `hub_card/tests/` must be updated together in the same PR so `goldens` and `lints` stay green).

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `objc2-metal@0.3.2` | `candle-core@0.10.2` (`patches/candle-core-0.10.2`) | Single pinned version in `Cargo.lock:9257`; `MTLDevice.rs:722,1740` verified in `accel_budget.rs:66-68`. |
| `nvml-wrapper@0.12.1` | `vox-plugin-nvml-probe` (`abi_stable@0.11`) | Runtime dynamic load of host `nvml.dll`/`libnvidia-ml.so`; degrades cleanly to `LibraryUnavailable` on non-NVIDIA hosts (`probe.rs:56`). |
| `sysinfo@0.39.3` | `windows@0.62.2`, `libc@0.2` | Already unified in `Cargo.lock:13868-13880`; note `Cargo.lock:13854` also carries `sysinfo@0.38.4` transitively—depend on `sysinfo = { workspace = true }` (`0.39`) to avoid a third version. |
| `windows-sys@0.59.0` | `windows-targets@0.52.6` | Workspace pin (`Cargo.toml:357`); adding feature flags inside `windows-sys` (e.g., `Win32_System_SystemInformation`, `Win32_Graphics_Dxgi`) adds zero crates to `Cargo.lock`. |
| `sharp@0.35.4` | `@playwright/test@1.63.0`, `astro@7.4.0` | Both pinned in `docs-astro/package.json:23,30` and `docs-astro/pnpm-lock.yaml`; prebuilt `libvips@1.3.3` handles SVG→WebP natively across macOS/Linux/Windows. |
| `vox-hf-layout@0.6.0` | `Qwen/Qwen3.5-{0.8B,2B,4B,9B}` & `Qwen/Qwen3.8-27B` | Tested in `crates/vox-hf-layout/src/lib.rs` against `qwen3_5` VLM (`text_config`) + `linear_attention`/`full_attention` hybrid fields. |

## Sources

- `crates/vox-populi/src/mens/hardware/macos_metal.rs:32-194` — Metal `recommendedMaxWorkingSetSize`, `currentAllocatedSize`, `vm_stat` live pressure (`max(15%, 2 GiB)` reserve) [HIGH — read directly]
- `crates/vox-populi/src/mens/tensor/accel_budget.rs:59-120` — `combine_with_live_pressure`, `maxBufferLength`, CUDA `AccelBudget` [HIGH — read directly]
- `crates/vox-populi/src/mens/hardware/{linux_drm.rs,windows_fallback.rs,probe.rs,pipeline.rs,mod.rs}` — Current probe stubs (`vram_mb: 0` on Linux/Windows) & L3/L4 layer boundary [HIGH — read directly]
- `crates/vox-plugin-nvml-probe/src/probe.rs:53-180` & `crates/vox-orchestrator-mcp/src/server_state/vram_probe.rs:42-62` — NVML runtime plugin probe [HIGH — read directly]
- `crates/vox-populi/src/mens/tensor/size_select.rs:37-200,305-325` & `crates/vox-orchestrator/src/models/{vram.rs,auto_select.rs,scoring.rs}` — Training-floor vs inference auto-selection mismatch & NVML-only `VramFit` [HIGH — read directly]
- `crates/vox-orchestrator/src/services/local_resources.rs:14-67` — Existing cached `sysinfo::System::available_memory()` probe [HIGH — read directly]
- `mens/config/gpu-specs.yaml:270-346`, `contracts/mens/memory-model.v1.yaml:35-44`, `crates/vox-populi/src/mens/tensor/memory_model.rs:201-314,695-843`, `docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.md:20-64` — Calibrated `162.4824 B/LHT` Metal QLoRA floors & `cache_dequantized` policy [HIGH — read directly]
- `~/.cache/huggingface/hub/models--Qwen--Qwen3.{5-0.8B,5-2B,5-4B,5-9B,8-27B}/snapshots/*/config.json` & `model.safetensors.index.json` — Exact pinned Qwen3.5/3.8 layers, GQA/DeltaNet head shapes, vocab (`248,320`), tied/untied embeddings, and safetensors byte totals [HIGH — read directly]
- `crates/vox-hf-layout/src/lib.rs:37-117,221-330` & `crates/vox-quantize/src/policy.rs:66-193,292-298` — Hybrid layout parsing & `Q4K` (`4.5 bpw`) / `Q6K` (`6.5625 bpw`) quantization sizing [HIGH — read directly]
- `crates/vox-populi/src/mens/hub_card/{svg.rs,render.rs,card_facts.rs,lints.rs}` & `hub_cards_push.rs:35-44` & `hub_card/tests/{model_card.rs,index_card.rs}` — Hub card asset pipeline, `.webp` upload support, `.svg` ban invariants [HIGH — read directly]
- `crates/vox-diagram/{Cargo.toml,src/svg/mod.rs}` & `docs-astro/{package.json,pnpm-lock.yaml,src/styles/vox-diagram.css,src/styles/reader-layout.css,tests/frontpage-perf-and-cv-flywheel.spec.ts}` & `AGENTS.md:189-196,456-477,762` — Zero-new-dep dual-theme CSS tokens, Playwright + `sharp 0.35.4` pipeline, and build-toolchain invariants [HIGH — read directly]

---
*Stack research for: Adaptive Model Scaling & Deterministic Dual-Theme Hub/Docs Graphics (v9.2)*
*Researched: 2026-11-19*
