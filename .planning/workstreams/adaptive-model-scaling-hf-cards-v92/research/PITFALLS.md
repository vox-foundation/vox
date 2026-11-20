# Pitfalls Research

**Domain:** Adaptive Model Scaling (Training vs. Inference VRAM, Apple Silicon Unified Memory, Sticky Pins, PopuliMesh Routing) & Hugging Face Hub Model Cards / Sanitizer / Audit Compliance (`adaptive-model-scaling-hf-cards-v92`)
**Researched:** 2026-11-19
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: Confusing QLoRA Training `floor_mb` with Inference VRAM & Misleading Card Peak Memory

**What goes wrong:**
Local inference auto-selection, `vox mens serve` preflight checks, and Hub card readers treat QLoRA **training** peak memory (`floor_mb` / `peak_mb`) as the memory required for **inference**. On consumer hardware (e.g., 16 GB RTX 4080 Super or 16–32 GB Apple Silicon MacBooks), `select_for_free_memory` refuses to select *any* local model at all—or forces a 0.6B model when 4B/8B/9B inference easily fits—while Hub model cards display `27007 MiB` `Peak memory` on 2B/4B cards without distinguishing training peak from inference footprint.

**Why it happens:**
1. `crates/vox-orchestrator/src/models/auto_select.rs` (`select_with_ladder_from`, L69–86) delegates directly to `vox_populi::mens::tensor::size_select::select_size` (`size_select.rs:192-199`), which is the **training** size selector:
   - `select_size` hardcodes `"qwen3_code"` (`size_select.rs:198`), ignoring the active `"qwen3_5_code"` ladder (`Qwen3.5-0.8B/2B/4B/9B` and `Qwen3.8-27B`) in `mens/config/gpu-specs.yaml:340-345`.
   - `size_ladder_for_tag` (`size_select.rs:339`) filters `train_bases` for `methods.contains("qlora")` and checks `rung.floor_mb <= free.mb` (`size_select.rs:370`).
   - Those `floor_mb` numbers in `mens/config/gpu-specs.yaml:280-316` are `seq_len=3072` QLoRA **training** floors: `0.6B = 15319`, `1.7B = 27007`, `4B = 34460`, `8B = 122876` (conservative Metal estimate), `14B = 195382`, `32B = 295822` MiB (and `qwen3_5_code`: `0.8B = 17000`, `2B = 19500`, `4B = 34000`, `9B = 36000`, `27B = 74000` MiB). Even `0.6B` fails on any GPU/Mac with `< 15,319 MB` free, and `8B` demands ~120 GB free (`auto_select.rs:173-187`).
2. `crates/vox-ml-cli/src/commands/ai/serve/mod.rs` (`serve_fit_note`, L204–221) also calls `size_select::select_size(overlay, free, Some(base), force)` (`serve/mod.rs:217`), causing `vox mens serve` to fail closed unless `VOX_MENS_FORCE_TRAIN=1` is set (`serve/mod.rs:136, 312-331`). (Note that cloud dispatch in `crates/vox-populi/src/mens/cloud/resolver.rs:583-598` already had to isolate `JobKind::Infer` from training-memory sizing via `infer_jobs_keep_the_legacy_default_without_a_hub_lookup`.)
3. In `crates/vox-populi/tests/golden/hub_cards/passed.md` (L103, L383) and `index.md` (L31), fixture inheritance from `states.json` (`needs_re_gate_2b`, originally from a 1.7B run) prints `27007 MiB` on both the **2B** `passed.md` card and the **4B** `index.md` row under an unqualified `Peak memory` column—leading operators and Hub visitors to mistake QLoRA training peak VRAM for serving/inference VRAM.

**How to avoid:**
- Introduce an explicit **inference VRAM model** separate from `train_bases.*.floor_mb` (e.g., per-quantization weights + KV-cache `serve_floor_mb` / `estimate_inference_vram_mb` parameterized by dtype/quantization and target context length), and update `auto_select.rs` and `serve_fit_note` to size against inference budgets rather than QLoRA training `floor_mb`.
- Support `"qwen3_5_code"` alongside `"qwen3_code"` (filtering to gate-passed/serveable rungs rather than hardcoding `"qwen3_code"`).
- Update `auto_select.rs` unit tests (`auto_selection_picks_the_largest_fitting_pin`, `auto_selection_reads_the_workspace_ladder`, L173–218) so they assert against inference floors rather than `122876` MB.
- On Hub cards (`templates/hub/model.md`, `templates/hub/index.md`) and fixtures (`states.json`), explicitly label training memory (`Training peak`, e.g., `QLoRA training peak`) separately from recommended inference VRAM/hardware tiers, and eliminate the stale `27007 MiB` cross-size copy in `support.rs` / `states.json`.

**Warning signs:**
- `select_optimal_local_model_for_host()` fails with `smallest (0.6B) needs 15319 MB` on a 16 GB RTX 4080 Super or M-series MacBook.
- `vox mens serve` prints `refusing rather than shrinking` for a 4B/8B/9B model on a machine that runs `ollama` / `vllm` fine, requiring `VOX_MENS_FORCE_TRAIN=1`.
- `index.md` shows `Peak memory: 27007 MiB` for `4B` or `2B` without clarifying it was a training measurement.

**Phase to address:**
Phase 541 (Adaptive VRAM / Hardware Sizing & Training-vs-Inference Floor Separation) and Phase 544 (Hub Card Contracts & Metadata Clarity).

---

### Pitfall 2: Apple Silicon Unified Memory Blind Spots & Sticky Pin / `check_strict_pin` Bypass Traps

**What goes wrong:**
On Apple Silicon (`aarch64-apple-darwin` Metal), orchestrator scoring (`auto_score_parts`) receives **zero** VRAM signal (`VramFit::Unknown`), `vm_stat` localization/parsing failures silently disable Metal auto-selection, large allocations hit `maxBufferLength` before `working_set_bytes`, and sticky/strict model pins either bypass routing/privacy/health gates or fail `caps_ok` on incomplete synthesized `ModelSpec` stubs.

**Why it happens:**
1. **Apple Silicon blind spots across `vram.rs`, `macos_metal.rs`, `accel_budget.rs`, and `vram_autodetect.rs`:**
   - `crates/vox-orchestrator/src/models/vram.rs` (L16–19, L174–209) is **NVIDIA/NVML-only** (`VramProbe`): `free_vram_mb_from_probe()` and `refresh_free_vram_hint_from_nvml()` always yield `None` (`VramFit::Unknown`) on macOS, so `vram_score_delta` (`scoring.rs:681, 759-766`) contributes `0.0` on every Mac.
   - `vram.rs:47-91` (`estimate_vram_fit`) hardcodes `BYTES_PER_PARAM_DEFAULT_Q4 = 0.5` and `KV_CACHE_OVERHEAD_FRACTION = 0.15`, undercounting BF16/FP16 (`2.0 B/param`) and Q8 (`1.0 B/param`) checkpoints by **2x–4x**, while `MensCatalog::refresh` (`catalog.rs:1249-1290`) leaves `capabilities.param_count_b = None`—so every local MENS run evaluates to `VramFit::Unknown`.
   - `macos_metal.rs:89-114` (`parse_vm_stat`) requires English `vm_stat` stdout (`"Mach Virtual Memory Statistics: (page size of "`, `"Pages free:"`, `"Pages inactive:"`, `"Pages speculative:"`). When `vm_stat` fails or emits non-English output, `working_set_is_live` becomes `false` (`accel_budget.rs:84`) and `size_select::metal_reading` (`size_select.rs:60-65`) returns `None` (`NO_READING`), hard-refusing selection.
   - `accel_budget.rs:22-24, 138-142` notes that `max_alloc_bytes` (`MTLDevice.maxBufferLength`) binds *before* `working_set_bytes` (e.g., `86.6 GB` vs `115.4 GB` on M5 Max). Checking only total working set causes Metal buffer-allocation failures.
   - Conversely, `probe_metal()` (`macos_metal.rs:40-60`) and `vram_autodetect::get_system_vram_info()` (`vram_autodetect.rs:98-106`) report the static `recommendedMaxWorkingSetSize` ceiling with `used_gb: 0.0` and `free_gb = total_gb`, ignoring unified-memory pressure from browsers/IDEs/other processes and causing macOS swap thrashing or jetsam OOM if used for inference sizing.
2. **Sticky Pin & Strict Pin inconsistencies (`resolve.rs`, `policy.rs`, `select.rs`):**
   - **Strict pin vs unmerged `MensCatalog` (`resolve.rs:270-285, 447-462`):** `check_strict_pin` looks up `registry.get(pin)` only. If `pin` is `mens/<run>` and `MensCatalog` hasn't merged into `orch.models_handle()` yet (e.g., OpenRouter TTL skip or early startup), `check_strict_pin` **fails** (`not in the model registry`), whereas the soft `pref` branch (`resolve.rs:494-506`) calls `sticky_mens_vox_local_spec(id)` to synthesize it. Meanwhile `select.rs:863-875` (`VOX_MODEL_FORCE` in `select_inner`) silently warns and falls through to unpinned selection when missing from the registry!
   - **Gate bypass on `pref` and `sticky_mens_vox_local_spec` (`resolve.rs:472-506`):** Neither the `pref` registry-hit branch (`L475-489`) nor the `sticky_mens_vox_local_spec` branch (`L494-506`) calls `routing_allows(&m)`! Under `VOX_ROUTE_POLICY_PROFILE=restricted` / `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP=0` / `routing_policy.provider_filter_allows`, a sticky `pref` bypasses `routing_allows` entirely.
   - **Degraded capabilities on `sticky_mens_vox_local_spec` (`policy.rs:60-91`):** The synthesized spec hardcodes `max_tokens: 8192` and `ModelCapabilities::default()` (only `tier: Local, writes_vox: true`), leaving `supports_tool_use`, `supports_json`, `supports_reasoning`, and `param_count_b` unset/`false`. Any prompt that infers `Capability::SupportsToolUse` or `SupportsReasoning` (`resolve.rs:406-416, 495-499`) falsely rejects a sticky `mens/<run>` pin! Also, `sticky_mens_vox_local_spec` does not recognize `vox-local/mens/<run>` or `mens/runs/<run>` aliases that `selection_queries.rs:424-435` normalizes, nor does it verify the run exists on disk or passed gates.

**How to avoid:**
- Expose an Apple Silicon live unified-memory provider (combining `recommendedMaxWorkingSetSize`, `maxBufferLength`, and `vm_stat` / Mach `host_statistics64` fallback so non-English locales don't break) into `vram.rs` / `auto_select.rs`, and populate `param_count_b` + `quantization` (bytes-per-param) on `MensCatalog` and `sticky_mens_vox_local_spec`.
- Enforce `single_alloc_fits` (`max_alloc_bytes`) in Metal inference-fit checks alongside total free working set.
- Route `pref` and `sticky_mens_vox_local_spec` through `routing_allows(&m)` (checking `route_policy_allows_model`, `provider_filter_allows`, `privacy_allows`), hydrate synthesized `mens/<run>` specs with real `MensCatalog` capabilities (`supports_tool_use`, `supports_json`, `supports_reasoning`, `max_tokens` from `config.json`, `param_count_b`), normalize `mens/runs/` and `vox-local/` prefixes consistently with `selection_queries.rs:424-435`, and make `check_strict_pin` vs `select_inner` fail closed uniformly.

**Warning signs:**
- `vram_score_delta` always returns `0.0` in macOS logs (`no GPU probe registered`).
- On localized macOS (`LANG != en_*`), `read_free_memory()` returns `None` even when plenty of unified memory is free.
- Pinning `mens/<run>` on a tool-use or reasoning prompt fails with `"Sticky MCP model does not satisfy inferred capability requirements"`.
- A `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP=0` test still selects a sticky `pref` local model.

**Phase to address:**
Phase 541 (Apple Silicon Unified Memory Probing & Quantization-Aware `vram.rs`) and Phase 542 (Sticky Pin Gate Unification & Capability Hydration).

---

### Pitfall 3: `PopuliMesh` Availability Optimism, Local Endpoint Coupling & Privacy/Egress Leakage

**What goes wrong:**
`PopuliMesh` is simultaneously treated as "always available with no key" (`key_guard.rs`), gated against the *local* Ollama HTTP port (`local_health.rs`), wiped whenever *local* `vox mens serve` is unreachable (`reconcile_local_endpoints`), allowed to bypass `VOX_INFERENCE_PRIVACY=local_only` (`route_policy.rs`), and silently rewritten into an invalid **OpenRouter** request if it reaches `llm_config_for_spec`.

**Why it happens:**
1. **`key_guard.rs` (`L58–62, L107–112, L143–165`):** `provider_secret_is_available` and `native_key_available` unconditionally return `true` for `ProviderType::PopuliMesh`, so `inference_wire_for` always returns `Some(InferenceWire::Native)` regardless of whether `mesh_directory::trusted_peers()` is empty or unreachable.
2. **`local_health.rs` (`L18–26, L51–53, L199–210`):** `is_populi_backed_local` groups `PopuliMesh` with `Ollama` and probes `local_ollama_populi_base_url()` (`http://localhost:11434`), NOT `iroh` QUIC `trusted_peers()`. Worse, `local_gate_allows` returns `true` when cached health is `None` (`health != Some(false)`), allowing unprobed/offline `PopuliMesh` entries through on cold starts or TTL expiry.
3. **`selection_queries.rs` (`reconcile_local_endpoints`, L51–66):**
   ```rust
   if !vox_local_reachable {
       self.models.retain(|_, m| {
           !matches!(m.provider_type, ProviderType::VoxLocal | ProviderType::PopuliMesh)
       });
   }
   ```
   This deletes live remote `PopuliMesh` peers whenever local `vox mens serve` is down, while leaving stale/dead `PopuliMesh` peers alive whenever local `vox mens serve` happens to be running!
4. **`selection_queries.rs:724` (`llm_config_for_spec`):**
   `ProviderType::PopuliMesh => LlmConfig::openrouter(spec.id.clone())` — if a `mesh/<endpoint_id>/<kind>` candidate slips past upstream `StreamRoute` handling (e.g., via `ModelRegistry::get_llm_config`), it dispatches a mesh peer ID to OpenRouter over the public internet!
5. **`route_policy.rs:17-22, 92-97` & `resolve.rs:551-568` & `scoring.rs:604-611`:**
   - `is_local_http_provider` includes `ProviderType::PopuliMesh`, so `VOX_INFERENCE_PRIVACY=local_only` permits remote network traffic to mesh peers (`local_health.rs:327-330` explicitly asserts this today).
   - `VOX_LOCAL_PREFERRED_TASKS` in `resolve.rs:551-568` short-circuits `CodeGen/Testing/Parsing/TypeChecking` straight to `ProviderType::VoxLocal` before `decide()` ever scores `PopuliMesh` or scaled cloud models.
   - `scoring.rs:604-611` applies `ZERO_COST_BASE_BONUS` to `PopuliMesh` even at high complexity (`complexity >= COMPLEXITY_HIGH_CUTOFF`), unlike `Ollama` and `VoxLocal` (`scoring.rs:613`).

**How to avoid:**
- Decouple `PopuliMesh` health and reconciliation from `VoxLocal` / `Ollama` (`reconcile_local_endpoints` must check `PopuliMeshCatalog` / `trusted_peers()` liveness independently from `vox_local_reachable`, and `local_health.rs` must check actual mesh peer availability rather than `localhost:11434`).
- Make `selection_key_available` / mesh gate require at least one live probed peer (`!trusted_peers().is_empty()` or per-peer liveness cache) before `PopuliMesh` enters `decide()` candidates, while keeping `available_inference_providers()` honest.
- Split `VOX_INFERENCE_PRIVACY` cleanly (`local_only` = loopback `VoxLocal`/`Ollama` only; `mesh_only` = loopback + trusted `PopuliMesh`; `any` = all), updating `route_policy.rs` and `local_health.rs:327-330`.
- Remove the dangerous `ProviderType::PopuliMesh => LlmConfig::openrouter(...)` fallback in `selection_queries.rs:724` (fail loudly or route via dedicated mesh transport config so mesh IDs never leak to OpenRouter).
- Remove pre-scorer `VOX_LOCAL_PREFERRED_TASKS` bypasses (`resolve.rs:551-568`) so `decide()` scores `VoxLocal`, `PopuliMesh`, and cloud candidates uniformly.

**Warning signs:**
- `vox_credentials_status` or candidate lists report `PopuliMesh` ready on machines with zero paired `mesh_trust.json` peers.
- Turning off local `vox mens serve` unexpectedly drops remote GPU mesh peers from `ModelRegistry`.
- Running with `VOX_INFERENCE_PRIVACY=local_only` sends QUIC/HTTP requests to a peer node, or a `PopuliMesh` model ID (`mesh/...`) appears in OpenRouter error logs (`400 Bad Request`).

**Phase to address:**
Phase 542 (Unified Auto Model Scoring, Sticky Pins & Local/Mesh Routing Policy).

---

### Pitfall 4: Hub Card Lints, `hub-cards.yaml` Rules, Metadata Rules & Doc Anchor Drift

**What goes wrong:**
Editing Hub card templates (`templates/hub/`), `mens/config/hub-cards.yaml`, `SIZES`, or `docs/src/reference/mens-models.md` triggers cascading build/test failures across `vox-populi` (`config.rs`, `lints.rs`, `lints_tests.rs`, `model_card.rs`, `index_card.rs`)—even when every file looks like valid Markdown and YAML.

**Why it happens:**
Five non-obvious compile/test invariants enforce strict coupling across those files:
1. **Exact `SIZES` bijection & digit ban in `config.rs` (`L65–160`):**
   - `mens/config/hub-cards.yaml` uses `#[serde(deny_unknown_fields)]` and must define **every** entry in `hub_repo::SIZES` (`["0.8B", "2B", "4B", "9B", "27B"]`) and **no** other size (`config.rs:99-107`). Adding/renaming a rung—or passing a `LEGACY_SIZES` token like `"8B"`/`"14B"` without a mapping—triggers `why_is_error = true` and injects `<!-- hub card config error: ... -->` into the card (`card_facts.rs:356-361`).
   - `has_stray_digit` (`config.rs:79-87, 114-118, 142-147`) removes strings in `SIZES` and rejects **any remaining ASCII digit `0..=9`** in `sizes.<S>.why` and `spokes[].purpose`. Prose like `"16 GB VRAM"`, `"v2"`, `"Qwen3.5"`, `"60s"`, or `"0.6B"` (since `0.6B` is in `LEGACY_SIZES`, not `SIZES`!) immediately fails `hub_cards()`.
   - `sentence_count` (`config.rs:65-76, 108-113`) splits on `.`, `!`, `?` and demands `2..=3` sentences in `why`. Periods inside abbreviations (`e.g.`) or decimal numbers count as extra sentences.
   - `index_card.rs:134-138` additionally asserts **zero** `SpokeStatus::Planned` entries exist in `hub-cards.yaml`, even though `config.rs:44` still parses `planned`.
2. **Global 4+ word duplicate-sentence ban (`lints.rs:155-181`, `lints_tests.rs:71-117`):**
   - Across all `SIZES.len() + 1` rendered cards (5 model cards + `vox-mens-index`), `sentences()` strips frontmatter, fenced code blocks, and **only** the final non-empty line if it contains `FOOTER_MARKER` (`"generated by \`vox mens hub sync\`"`).
   - Every heading, list item, table row, and paragraph sentence is normalized (link URLs stripped, backticks/emphasis removed, `|` replaced with spaces, lowercased) and checked: **any normalized sentence of `>= 4` alphanumeric words appearing twice—across two cards or twice on one card—fails.**
   - Therefore, *every* prose line or table caption in shared templates (`model.md`, `partials/onboarding.md`, `partials/use_it.md`, `partials/examples.md`) of 4+ words MUST interpolate `{{ f.size }}` (or per-size facts), and every `##` heading on model cards must stay `<= 3` words (`model_card.rs:208`).
3. **Hardcoded `mens-models.md` heading slug list (`lints_tests.rs:129-152`, `lints.rs:184-242`):**
   - `DOCS_MD` embeds `docs/src/reference/mens-models.md` at compile time, and `slug_matches_site_ids` asserts the exact 16-item heading slug list (`what-mens-is` .. `links`). Changing *any* `##`/`###` heading in `mens-models.md` fails `lints_tests.rs` unless the expected list in `lints_tests.rs:131-148` is updated in the same commit, and `every_card_anchor_exists_in_mens_models` fails if any card `#fragment` points to a missing heading.
4. **Strict HF metadata rules & substring bans (`hf-metadata-rules.v1.yaml`, `lints.rs:286-392`, `model_card.rs:482`):**
   - `check_hf_metadata` rejects any frontmatter key not listed in `hf-metadata-rules.v1.yaml` `known_keys`.
   - `base_model` forbids `@` (`base_model_revision` carries the 40-char SHA).
   - `library_name` must be in `[transformers, peft, vox]`; `Trained` cards must have **exactly one** `gate-` tag and **exactly one** `spoke-` tag; `NotTrained` cards must have `not-trained` and zero `gate-`/`spoke-` tags; `model-index` metric `value` must be a YAML number (`!v.is_number()` fails); and `model_card.rs:482` forbids the substring `"verified"` anywhere in model card frontmatter!
5. **Live ` ```vox ` doctests & ignored rendered-dir lint (`lints.rs:395-406`, `lints_tests.rs:410-448`):**
   - Every ` ```vox ` code block on any card (including `partials/onboarding.md`!) is compiled via `verify_vox_admission`; failing/unadapted outputs MUST use ` ```text `.
   - Golden snapshots in `crates/vox-populi/tests/golden/hub_cards/*.md` (9 files) break on any template wording change unless regenerated with `VOX_UPDATE_GOLDEN=1` and audited; `lints_hold_on_rendered_cards_dir` is `#[ignore]` and only catches real `mens/runs/hub_cards` violations when explicitly run with `VOX_HUB_CARDS_DIR=...`.

**How to avoid:**
- Keep all numbers (VRAM GB, latencies, benchmark counts, loss, peaks) out of `mens/config/hub-cards.yaml` `why` prose and derive them via `CardFacts` / `IndexFacts` records or template variables.
- Parameterize every shared 4+ word template sentence with `{{ f.size }}` and keep `##` headings `<= 3` words.
- Whenever modifying YAML frontmatter keys, `mens-models.md` headings, or `hub_repo::SIZES`, update `hf-metadata-rules.v1.yaml`, `lints_tests.rs:131-148`, and `hub-cards.yaml` in lockstep, then run both standard `vox-populi` tests (without `VOX_UPDATE_GOLDEN` after reviewing golden diffs) AND `VOX_HUB_CARDS_DIR=mens/runs/hub_cards cargo test -p vox-populi --features mens-hf-hub --lib lints_hold_on_rendered_cards_dir -- --ignored`.

**Warning signs:**
- Card test failures mentioning `holds a digit`, `needs two or three sentences`, `sentence found twice`, `unknown key`, `slug_matches_site_ids`, or `golden ... drifted`.
- `<!-- hub card config error: ... -->` appearing in rendered markdown for legacy sizes or newly added sizes.

**Phase to address:**
Phase 544 (Hub Card Contracts, Config, Frontmatter Metadata & Sentence-Uniqueness Lints) and Phase 546 (Golden Regeneration & Rendered-Cards Gate Closure).

---

### Pitfall 5: Hugging Face Hub Markdown HTML Sanitizer Stripping (`<picture>`, `<source>`, `<svg>`, `<style>`)

**What goes wrong:**
Embedding responsive or dual-theme visuals in HF model/index cards using `<picture>`, `<source media="(prefers-color-scheme: dark)">`, inline `<svg>`, or `<style>` blocks gets silently stripped, flattened, or mangled by Hugging Face Hub's XSS/HTML sanitizer—leaving broken blank regions, missing dark-mode banners, or unstyled fallback fragments in production on `huggingface.co` even though local Markdown previews look fine.

**Why it happens:**
1. **Verified HF Hub sanitizer behavior (`huggingface.co/docs/hub/model-cards`):**
   - **`<style>` blocks:** Blocked/stripped to prevent global Hub CSS injection and UI clickjacking/redress.
   - **Inline `<svg>` tags:** Stripped or aggressively sanitized because raw SVGs can harbor `<script>`, `<foreignObject>`, `onload`/`onerror`, or `javascript:` URI vectors (which is why `crates/vox-populi/src/mens/hub_card/svg.rs` and `hub_sync.rs:1877` switched to raster `assets/banner.jpg` / `assets/overview.jpg` and `model_card.rs:223` / `index_card.rs:90` forbid `.svg`).
   - **`<picture>` and `<source>` tags:** Stripped or unsupported by the Hub's strict README markdown sanitizer; responsive `<source media="(prefers-color-scheme: ...)">` rules are dropped.
   - **`<img>` and Markdown image syntax:** Permitted (with `on*` event handlers and `javascript:` URIs stripped). For theme-aware images, Hugging Face natively supports only:
     - Markdown URL fragment modifiers: `![Alt](...#hf-light-mode-only)` and `![Alt](...#hf-dark-mode-only)`
     - HTML `<img>` tags with Tailwind CSS classes: `<img class="hidden dark:block" src="..." alt="..." />` and `<img class="dark:hidden" src="..." alt="..." />`
2. **Codebase coupling traps:**
   - `crates/vox-populi/src/mens/hub_cards_push.rs:35-44` (`is_card_collateral`) only recognizes `assets/*.{jpg,jpeg,png,webp}` for card-only sync (`--cards-only`), omitting `.svg` or CSS files.
   - `crates/vox-populi/src/mens/hub_card/lints.rs:67-89, 108-145` (`units` and `normalize`) does NOT skip raw HTML tags (`<img>`, `<div>`, `<picture>`), so putting multi-word prose with punctuation inside HTML blocks or `alt="..."` attributes feeds them straight into the 4-word duplicate-sentence lint!

**How to avoid:**
- Never emit `<picture>`, `<source>`, `<style>`, or inline `<svg>` in `crates/vox-populi/templates/hub/**` or `hub_card/svg.rs`.
- Use standard Markdown raster image syntax (`![...](assets/banner.jpg)` / `assets/overview.jpg`) with neutral/balanced high-contrast artwork, OR—if dual-theme assets are needed—use HF-sanctioned `<img class="hidden dark:block" ... />` / `<img class="dark:hidden" ... />` (or `#hf-dark-mode-only` / `#hf-light-mode-only`, verified on a live test repo on HF Hub).
- Add a unit test in `lints_tests.rs` / `model_card.rs` / `index_card.rs` asserting rendered cards contain zero `<picture`, `<source`, `<svg`, or `<style` tags, and ensure any `<img>` `alt` text stays `<= 3` words or includes `{{ f.size }}`.

**Warning signs:**
- Rendered `README.md` looks fine in VS Code / GitHub Markdown preview, but images disappear or double-render on `huggingface.co/<ns>/vox-mens-*`.
- Card assets under `assets/` fail to upload during `vox mens hub sync --cards-only` because their extension is not in `is_card_collateral`.

**Phase to address:**
Phase 545 (HF Hub Visual Assets, Sanitizer-Safe Markdown Rendering & Card Push Sync).

---

### Pitfall 6: Flywheel 60-Cell Spoke Routing Ratchet (`verify_full_flywheel_routing_and_spoke_ratchet`) & `vox-code-audit` `LlmProviderCallDetector` Traps

**What goes wrong:**
(a) Changing `auto_score_parts`, `vram_score_delta`, `select_local_first`, or `MensCatalog` registration breaks the 60-cell `(5 spokes × 4 clutches × 3 risk postures)` MENS ratchet in `verify_full_flywheel_routing_and_spoke_ratchet`, and (b) adding Hub API calls or HF Router endpoints in `vox-populi` / `vox-orchestrator` triggers `Severity::Error` (`vox/llm/direct-provider-call`) in `vox audit code` / pre-push CI even though `cargo test` passes.

**Why it happens:**
1. **Flywheel 60-cell ratchet (`mens_spoke_routing.rs:145, 216-310`, `registry/tests.rs:270-378`):**
   - `register_calibrated_mens_adapter` (`mens_spoke_routing.rs:145`) **hardcodes `param_count_b: Some(27.0)`** for every registered MENS adapter, regardless of whether the checkpoint is 0.8B, 2B, 4B, 9B, or 27B!
   - Meanwhile, the baseline local Ollama competitor in `registry/tests.rs:289-318` (`"ollama/qwen3:8b"`) has `param_count_b: None`.
   - In `verify_full_flywheel_routing_and_spoke_ratchet` (`mens_spoke_routing.rs:261-310`), every one of the 60 `(SpokeKind, ClutchProfile, RiskPosture)` cells—including `ClutchProfile::Free`, `Efficiency`, `Balanced`, and `Genius`—must route to `expected_mens_id`.
   - If Phase 541–542 populates `free_vram_mb_hint()` globally or applies a stronger `VRAM_EXCEEDS_PENALTY` (currently `-0.05`, `scoring.rs:64`), a hardcoded `27.0B` calibrated MENS adapter looks oversized on a 16 GB host (`Exceeds`), while the unmeasured `ollama/qwen3:8b` (`param_count_b: None` -> `VramFit::Unknown` -> `0.0` penalty!) wins or breaks the ratchet!
   - Furthermore, `route_spoke_task_with_policy` (`mens_spoke_routing.rs:235-247`) relies on `intent.prefer_local = true` and `intent.allow_free_in_performance_mode = true`. Removing `select_local_first` (`select.rs:878-882`) without preserving `QualitySource::Measured` + spoke-strength superiority in `auto_score_parts` across `Balanced` and `Genius` clutches will regress the 60-cell ratchet to cloud or Ollama models.
2. **`LlmProviderCallDetector` (`crates/vox-code-audit/src/detectors/llm_provider_call.rs`):**
   - Rule `vox/llm/direct-provider-call` (`Severity::Error`) flags any `.rs`, `.vox`, or `.ts` file outside `crates/vox-llm-egress/` (`is_facade_file`, L66–69) that pairs a `HOSTNAMES` match (including `"huggingface.co/api"`, `"api.openai.com"`, `"openrouter.ai"`, `"generativelanguage.googleapis.com"`, etc., L26–40) or `vox_config::*base_url(` within ±3 lines of `reqwest::Client`, `reqwest::get`, `.get(`, `.post(`, `ureq`, etc. (L52–59, L253–263).
   - It skips `#[cfg(test)]` blocks and comments (`L208–212`), and ONLY exempts read-only `/api/models`, `/api/collections`, `/v1/models"`, `/v1beta/models?` (without `generateContent`) (`L218–224`) or lines within ±5 lines of `// vox-arch-check: allow llm-egress` (`L73–75, L237–244`).
   - Adding new HF Hub REST calls (e.g., `https://huggingface.co/api/repos/...`, `https://huggingface.co/api/spaces/...`, card/metadata verification endpoints) via `reqwest` in `vox-populi` or `vox-cli` immediately fails `vox audit code` unless routed through `hf-hub` or explicitly exempted as read-only catalog discovery; conversely, any new inference routing (HF Router) MUST go exclusively through `vox_actor_runtime::llm` / `crates/vox-llm-egress/`.

**How to avoid:**
- Parse actual `param_count_b` from the adapter's `base_model` / `model_family` in `register_calibrated_mens_adapter` (`mens_spoke_routing.rs:145`) and populate `param_count_b` on local baselines too—never hardcode `Some(27.0)` for all sizes.
- Run `verify_full_flywheel_routing_and_spoke_ratchet` with both `free_vram_mb_hint = None` and realistic host VRAM values (16 GB, 32 GB, 128 GB) after modifying `scoring.rs`, `vram.rs`, or `select.rs`, and keep `#[cfg(test)]` thread-local VRAM/privacy overrides isolated (`vram.rs:103-107`).
- Route all inference calls strictly through `vox_actor_runtime::llm` / `crates/vox-llm-egress/`; for read-only HF Hub metadata APIs in `vox-populi`, prefer `hf-hub` or update `llm_provider_call.rs` read-only path exemptions (`L218-222`) with matching unit tests (`L425-451`) rather than sprinkling `allow llm-egress` suppressions.

**Warning signs:**
- `local_to_mens_regression_and_clutch_risk_matrix_across_all_five_spokes` fails on `ClutchProfile::Balanced` or `Genius`, or when `free_vram_mb_hint` is non-`None`.
- `cargo test` passes locally (because `#[cfg(test)]` is skipped by `LlmProviderCallDetector`), then pre-push / CI fails in `vox audit code` with `vox/llm/direct-provider-call`.

**Phase to address:**
Phase 543 (Flywheel Spoke Ratchet & Policy-Aware Multi-Factor Routing Verification) and Phase 545/546 (`vox-code-audit` & CI Gate Closure).

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keeping `auto_select.rs` and `serve_fit_note` wired to `size_select::select_size` (QLoRA training `floor_mb`) | Avoids maintaining an inference VRAM estimator | Refuses all models `<15.3 GB` free, blocks `8B` under `120 GB`, misleads operators into setting `VOX_MENS_FORCE_TRAIN=1` | Never (fix in Phase 541) |
| Hardcoding `param_count_b: Some(27.0)` in `register_calibrated_mens_adapter` | Single constant in test/registry code | Smaller calibrated MENS models (`0.8B`–`9B`) receive 27B `Exceeds` VRAM penalties once live host VRAM hints are active | Never (fix in Phase 543) |
| Leaving `sticky_mens_vox_local_spec` as a bare `ModelCapabilities::default()` stub | Quick synthesis before `MensCatalog` refresh | Rejects `SupportsToolUse`/`SupportsReasoning` turns, uses wrong `max_tokens: 8192`, bypasses `routing_allows` | Never (fix in Phase 542) |
| Keeping `ProviderType::PopuliMesh => LlmConfig::openrouter(...)` fallback in `llm_config_for_spec` | Satisfies exhaustive `match` without QUIC wiring | Leaks internal `mesh/<endpoint>/<kind>` IDs to OpenRouter API and breaks privacy invariants | Never (fix in Phase 542) |
| Running `VOX_UPDATE_GOLDEN=1` blindly after editing `templates/hub/` | Makes `vox-populi` tests pass fast | Commits stale/misleading metrics (`27007 MiB` on wrong sizes) or HF-sanitizer-unsafe HTML into goldens | Only after manual line-by-line golden diff review in Phase 544/546 |
| Silencing `LlmProviderCallDetector` with `// vox-arch-check: allow llm-egress` on Hub API calls | Avoids updating detector patterns | Masks real direct-egress regressions and accumulates architectural debt | Only for deliberate non-OpenAI egress in sanctioned adapters; never for inference |

## Integration Gotchas

Common mistakes when connecting to external services or subsystems.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **Hugging Face Hub README Markdown** | Using `<picture><source media="(prefers-color-scheme: dark)">`, inline `<svg>`, or `<style>` blocks | Use raster `.jpg`/`.png`/`.webp` assets in `assets/` via `![alt](assets/banner.jpg)`, or `<img class="hidden dark:block" ...>` / `#hf-dark-mode-only` |
| **Hugging Face Hub YAML Frontmatter** | Putting `@<sha>` in `base_model`, putting string `"0.625"` in `model-index` `value`, or leaving root `adapter_config.json` alongside `model.safetensors` | Keep `owner/name` in `base_model` and 40-hex SHA in `base_model_revision`; keep `model-index` values numeric; place PEFT under `peft/` when merged (`library_name: transformers`) |
| **Apple Silicon Metal (`objc2_metal` + `vm_stat`)** | Using `recommendedMaxWorkingSetSize` alone (`probe_metal()` / `get_system_vram_info()`) as free VRAM | Combine `recommendedMaxWorkingSetSize` with live memory-pressure reclaimable pages (`vm_stat` / Mach `host_statistics64`) and enforce `maxBufferLength` (`max_alloc_bytes`) |
| **NVML / `nvidia-smi` Multi-GPU** | Summing VRAM across GPUs or indexing without `CUDA_DEVICE_ORDER=PCI_BUS_ID` | Follow `size_select::visible_min_free_mb`: take the minimum free MiB across visible GPUs (`x 0.88` safety margin), treating `0` as unreadable fallback |
| **`vox-code-audit` + HF APIs** | Writing `reqwest::Client::new().get("https://huggingface.co/api/...")` directly in `vox-populi` | Use `hf-hub` crate methods, or for read-only catalog paths align with `LlmProviderCallDetector`'s exempted paths (`/api/models`, `/api/collections`) |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Probing `vm_stat` / `nvidia-smi` / NVML FFI synchronously on every `auto_score_parts` call | Chat routing latency spikes 20–150 ms per candidate turn | Refresh once per catalog/health TTL cycle into `vram::set_free_vram_mb_hint` via `spawn_blocking` (`catalog_refresh.rs:309`) | Every interactive chat/autocomplete turn |
| Probing `iroh` QUIC `trusted_peers()` synchronously inside `resolve_mcp_chat_model_sync` | MCP chat hangs up to QUIC timeout when a peer is offline | Keep `mesh_directory` probe results in a short-TTL async cache (`OnceCell` endpoint + peer snapshot), never blocking the sync resolver | Any time `>= 1` paired mesh peer is offline or sleeping |
| Scanning `snapshots/` via `DirEntry::metadata()` instead of `std::fs::metadata()` | HF cache symlinks report ~60 bytes instead of multi-GB `.safetensors` size, causing OOM on load | Always use `std::fs::metadata()` to follow HF blob symlinks (`memory_model.rs:178-184`) | Every HF Hub cached checkpoint load |
| Loading `BF16` weights via `load_f32` or treating Q4/BF16 identically in `estimate_vram_fit` | 2x–4x VRAM underestimation causes swap thrashing on Metal or CUDA OOM | Track exact quantization (`Q4`, `Q8`, `BF16`) / on-disk `artifact_bytes` in `ModelSpec` and `vram.rs` | Any `>= 7B` BF16 or `>= 14B` Q8 model on 16–32 GB hosts |

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Allowing `ProviderType::PopuliMesh` under `VOX_INFERENCE_PRIVACY=local_only` (`route_policy.rs:17-22, 92-97`) | User prompts silently leave the machine over QUIC to remote peer nodes when operator explicitly demanded local-only privacy | Split `local_only` (loopback `VoxLocal`/`Ollama` only) from `mesh_only` (loopback + trusted `PopuliMesh`) and `any` |
| Sticky `pref` bypassing `routing_allows(&m)` (`resolve.rs:475-506`) | Restricted network/provider policies (`VOX_ROUTE_POLICY_PROFILE=restricted`) or `VOX_INFERENCE_PRIVACY` can be bypassed via user/session preference string | Apply `routing_allows(&m)` uniformly across `strict_pin`, `pref` registry hits, and `sticky_mens_vox_local_spec` |
| `ProviderType::PopuliMesh => LlmConfig::openrouter(...)` fallback (`selection_queries.rs:724`) | Leaks private mesh peer endpoint IDs and prompt payloads to an external cloud gateway if `StreamRoute` is bypassed | Remove OpenRouter fallback on `PopuliMesh`; fail closed if mesh transport route is absent |
| Host filesystem paths or secrets rendered into HF Hub cards (`model_card.rs:57`) | Leaks operator `/Users/...` or `/home/...` paths or local endpoints in public `README.md` | Run `sanitize_json_paths` and assert `!card.contains("/Users/") && !card.contains("/home/")` across all card and golden tests |
| Directly calling LLM inference endpoints outside `crates/vox-llm-egress/` | Bypasses cost accounting, telemetry, Clavis secrets, and retry/privacy policy | Enforce `vox_actor_runtime::llm` / `vox-llm-egress` and gate with `vox audit code` (`vox/llm/direct-provider-call`) |

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Displaying `Peak memory: 27007 MiB` on 2B/4B HF cards without saying "QLoRA training peak" | Users on 8–16 GB laptops think they cannot run 2B/4B inference and abandon the model | Label `Training peak` explicitly alongside estimated `Inference VRAM` (`BF16` vs `Q4`) |
| Failing `auto_select` on an 8–16 GB MacBook/GPU with `"smallest (0.6B) needs 15319 MB"` | Local-first UX fails immediately out of the box | Size local serving with inference footprints (`~0.5–1.5 GB` for 0.8B Q4/BF16, `~2–4 GB` for 2B/4B) |
| Silent fallback on `VOX_MODEL_FORCE` (`select.rs:874`) vs hard error on `check_strict_pin` (`resolve.rs:276`) | Operator thinks strict pin is running when agent tasks silently switched to cloud/default | Unify strict-pin fail-closed semantics across both MCP chat (`resolve.rs`) and agent task (`select.rs`) paths |
| Reporting `PopuliMesh` as "available" in status panels when zero peers are online | Operators see dead mesh options and hit timeouts when routing tries mesh | Only report `PopuliMesh` ready when `trusted_peers()` returns at least one reachable peer |

## "Looks Done But Isn't" Checklist

- [ ] **Inference vs Training Separation:** `auto_select.rs` and `serve_fit_note` pass on a simulated 8 GB and 16 GB host without `VOX_MENS_FORCE_TRAIN=1`, while `train` (`size_select`) still enforces measured training `floor_mb`.
- [ ] **Apple Silicon Unified Memory:** `vram_score_delta` returns non-`Unknown` verdicts on macOS Metal (`recommendedMaxWorkingSetSize` + live `vm_stat` pressure + `maxBufferLength`), and quantization/BF16 models don't use the `0.5 B/param` Q4 shortcut.
- [ ] **Sticky & Strict Pin Parity:** Sticky `mens/<run>` pins pass `routing_allows`, `privacy_allows`, `caps_ok` (for `ToolUse`/`Reasoning` prompts), and normalize `mens/runs/` + `vox-local/mens/` prefixes cleanly.
- [ ] **PopuliMesh Truthfulness:** Disconnecting all mesh peers removes `PopuliMesh` from `decide()` candidates even while local `vox mens serve` is up, and `VOX_INFERENCE_PRIVACY=local_only` blocks `PopuliMesh`.
- [ ] **Hub Card Lints & Anchors:** `cargo test -p vox-populi --features mens-hf-hub` passes *without* `VOX_UPDATE_GOLDEN=1`, `hub-cards.yaml` has zero stray digits / `planned` spokes, and `VOX_HUB_CARDS_DIR=... lints_hold_on_rendered_cards_dir -- --ignored` passes on real rendered output.
- [ ] **HF Sanitizer Safety:** Rendered model cards and index cards contain zero `<picture>`, `<source>`, `<svg>`, or `<style>` tags, and all `assets/*.jpg` entries appear in `is_card_collateral` and `hf_sync_manifest.json`.
- [ ] **Flywheel Ratchet & Audit Gates:** `verify_full_flywheel_routing_and_spoke_ratchet` passes 60/60 cells under active VRAM hints, and `vox audit code` + `vox audit arch` + `vox-doc-pipeline --lint-only` pass with `0` errors.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Inference refuses 16 GB GPU because of training `floor_mb` | LOW | Immediate operator workaround: `VOX_MENS_FORCE_TRAIN=1` for `vox mens serve`; permanent fix: decouple `auto_select.rs` / `serve_fit_note` from `size_select::select_size` |
| Hub card push introduces broken `<picture>`/`<svg>` or duplicate sentence | LOW | Fix template in `crates/vox-populi/templates/hub/`, re-run `vox mens hub sync --cards-only --push` (`hub_cards_push.rs` updates cards/assets/manifest in-place without re-uploading `.safetensors`) |
| 60-cell flywheel ratchet breaks after scoring/VRAM change | MEDIUM | Inspect failing `(spoke, clutch, risk)` cell via `route_spoke_task_with_policy`, verify `param_count_b` on `register_calibrated_mens_adapter`, and adjust `auto_score_parts` / `VRAM_EXCEEDS_PENALTY` magnitude vs `Measured` quality bonus |
| `VOX_INFERENCE_PRIVACY=local_only` leaks to `PopuliMesh` or OpenRouter fallback triggers | MEDIUM | Separate `Mesh` from `is_local_http_provider` in `route_policy.rs`, purge `LlmConfig::openrouter` on `PopuliMesh` in `selection_queries.rs:724`, and add regression tests in `local_health.rs` |
| `mens-models.md` heading edit breaks `vox-populi` compile/tests | LOW | Update `slug_matches_site_ids` in `crates/vox-populi/src/mens/hub_card/lints_tests.rs:131-148` and verify all `card_anchors` resolve |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| **Pitfall 1:** Training `floor_mb` used for inference (`auto_select.rs`, `serve_fit_note`, `gpu-specs.yaml`) | **Phase 541** (Adaptive Inference VRAM Sizing & Ladder Separation) | Unit tests in `auto_select.rs` & `serve/mod.rs` confirming 8 GB / 16 GB hosts pick fitting inference rungs while training keeps `floor_mb` |
| **Pitfall 2a:** Apple Silicon unified memory blind spots (`macos_metal.rs`, `accel_budget.rs`, `vram.rs` Q4/BF16 mismatch) | **Phase 541** (Unified Metal/CUDA Inference VRAM Probe & Quantization Model) | `vram.rs` & `macos_metal.rs` tests covering BF16/Q8/Q4 footprints, `maxBufferLength` cap, and `vm_stat` graceful fallback |
| **Pitfall 2b:** Sticky pin & `check_strict_pin` gate bypasses / missing capabilities (`resolve.rs`, `policy.rs`) | **Phase 542** (Sticky Pin Gate Unification & Capability Hydration) | `resolve.rs` tests verifying `routing_allows` + `SupportsToolUse`/`SupportsReasoning` on `sticky_mens_vox_local_spec` and unified strict-pin fail-closed checks |
| **Pitfall 3:** `PopuliMesh` keyless optimism, `reconcile_local_endpoints` coupling, `local_only` leak & OpenRouter fallback | **Phase 542** (Local/Mesh Availability, Privacy Tier Split & Route Unification) | `local_health.rs`, `selection_queries.rs`, and `route_policy.rs` tests verifying `local_only` blocks `PopuliMesh`, unreachable peers drop cleanly, and no OpenRouter fallback |
| **Pitfall 6a:** Flywheel 60-cell `(clutch, risk, spoke)` ratchet regression & hardcoded `27.0B` adapter size | **Phase 543** (Flywheel Spoke Ratchet & Multi-Factor Scoring Calibration) | `local_to_mens_regression_and_clutch_risk_matrix_across_all_five_spokes` + `verify_full_flywheel_routing_and_spoke_ratchet` under live VRAM hints |
| **Pitfall 4:** Hub card `hub-cards.yaml` digit/bijection rules, 4-word duplicate lint, `hf-metadata-rules.v1.yaml`, doc anchors, card `Peak memory` clarity | **Phase 544** (Hub Card Contracts, Metadata Rules & Training-vs-Inference Copy) | `cargo test -p vox-populi --features mens-hf-hub` (`config.rs`, `lints_tests.rs`, `model_card.rs`, `index_card.rs`) passing without `VOX_UPDATE_GOLDEN` |
| **Pitfall 5:** HF Markdown HTML sanitizer stripping (`<picture>`, `<source>`, `<svg>`, `<style>`) & `is_card_collateral` gaps | **Phase 545** (Sanitizer-Safe Hub Visuals & `--cards-only` Push Sync) | Rendered card HTML/Markdown sanitizer-safety assertions + `hub_cards_push_tests.rs` verifying dual-theme / raster asset collateral sync |
| **Pitfall 6b & Integration Closure:** `vox-code-audit` `LlmProviderCallDetector`, `VOX_HUB_CARDS_DIR` ignored lints, golden regeneration & doc sync | **Phase 546** (End-to-End Verification, Golden Closure & CI Gates) | `vox audit code`, `vox audit arch`, `vox-doc-pipeline --lint-only`, and `lints_hold_on_rendered_cards_dir -- --ignored` passing with 0 errors |

## Sources

- `mens/config/gpu-specs.yaml` (L238–351: `train_bases`, `qwen3_code`, `qwen3_5_code`, `floor_mb`, `floor_measured`)
- `crates/vox-orchestrator/src/models/auto_select.rs` (L49–92, L173–257)
- `crates/vox-populi/src/mens/tensor/size_select.rs` (L1–252, L327–458)
- `crates/vox-ml-cli/src/commands/ai/serve/mod.rs` (L96–140, L204–221, L312–366)
- `crates/vox-populi/src/mens/cloud/resolver.rs` (L583–598)
- `crates/vox-populi/src/mens/hardware/macos_metal.rs` (L21–200)
- `crates/vox-populi/src/mens/tensor/accel_budget.rs` (L9–120, L138–156)
- `crates/vox-populi/src/mens/tensor/vram_autodetect.rs` (L73–156)
- `crates/vox-orchestrator/src/models/vram.rs` (L1–227)
- `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs` (L270–302, L445–627)
- `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/policy.rs` (L41–91)
- `crates/vox-orchestrator-mcp/src/llm_bridge/local_health.rs` (L15–140, L199–351)
- `crates/vox-orchestrator/src/models/key_guard.rs` (L45–165)
- `crates/vox-orchestrator/src/models/registry/selection_queries.rs` (L49–66, L402–459, L681–734)
- `crates/vox-orchestrator/src/models/select.rs` (L119–388, L861–950)
- `crates/vox-orchestrator/src/models/scoring.rs` (L42–65, L590–766)
- `crates/vox-orchestrator/src/route_policy.rs` (L15–98, L113–137)
- `crates/vox-orchestrator/src/catalog.rs` (L880–960, L967–1296)
- `crates/vox-orchestrator/src/models/mesh_directory.rs` (L1–75)
- `crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs` (L66–310)
- `crates/vox-orchestrator/src/models/registry/tests.rs` (L270–378)
- `crates/vox-populi/src/mens/hub_card/{lints.rs,lints_tests.rs,config.rs,card_facts.rs,index_facts.rs,svg.rs,render.rs}`
- `crates/vox-populi/src/mens/hub_card/tests/{support.rs,model_card.rs,index_card.rs}`
- `crates/vox-populi/tests/fixtures/hub_cards/{hf-metadata-rules.v1.yaml,cases/states.json}`
- `crates/vox-populi/tests/golden/hub_cards/{passed.md,index.md}`
- `crates/vox-populi/src/mens/{hub_repo.rs,hub_cards_push.rs,hub_sync.rs}`
- `mens/config/hub-cards.yaml` and `docs/src/reference/mens-models.md`
- `crates/vox-code-audit/src/detectors/llm_provider_call.rs` (L24–268, L425–451)
- Hugging Face Hub Model Card Markdown / Dark-Light Mode Documentation (`huggingface.co/docs/hub/model-cards`, verified via web search for `#hf-light-mode-only`, `#hf-dark-mode-only`, Tailwind `hidden dark:block` / `dark:hidden` on `<img>`, and HTML sanitizer stripping of `<style>`, `<svg>`, `<picture>`, `<source>`).

---
*Pitfalls research for: adaptive-model-scaling-hf-cards-v92*
*Researched: 2026-11-19*
