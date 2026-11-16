---
phase: 339-model-cards-v2
fixed_at: 2026-11-09T03:10:00Z
review_path: .planning/phases/339-model-cards-v2/339-REVIEW.md
iteration: 2
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 339: Code Review Fix Report (Iteration 2)

**Fixed at:** 2026-11-09T03:10:00Z
**Source review:** `.planning/phases/339-model-cards-v2/339-REVIEW.md`
**Iteration:** 2
**Verification environment:** Main checkouts (`/Users/brbrainerd/dev/vox-mens-compare` and `/Users/brbrainerd/dev/vox`, synchronized byte-for-byte; `vm.Script` syntax check + `rustfmt --edition 2024`)

**Summary:**
- Findings in scope: 7 (`CR-01`, `CR-02`, `WR-01`, `WR-02`, `WR-03`, `WR-04`, `IN-01`)
- Fixed: 7
- Skipped: 0

## Fixed Issues

### CR-01: `index.html.tmpl` Live Hub Evidence Explorer reads wrong JSON/JSONL schema keys for all 6 Hub artifacts (renders 0 samples and empty collateral table)

**Files modified:** `crates/vox-populi/templates/hub/space/index.html.tmpl`, `mens/runs/hub_cards/vox-mens-playground/index.html`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:**
1. Updated `renderLadderStrip` to read `r.gate_status || r.remote_gate_status || r.local_gate_status || "untrained"` and `r.latest_version || (r.remote_revision ? "v" + r.remote_revision : r.sync_status || "n/a")`.
2. Updated `selectSize` to read `sm.base_pin` and `sm.corpus_hash` (combined with `latest_version` from `ladder_index.json`), and to read `gr.overall_passed` (fallback `gr.passed`) and `gr.gates.filter(g => g && !g.passed && g.block).map(g => g.name + ": " + g.message)`.
3. Added `parseCkptLines`, `extractAnswerRegion`, `extractShownCode`, and `matchCkptText` to parse `vox_eval_item_checkpoint.v1` JSONL files (reading `header.inputs.model.adapter_serving` from line 0 and `JSON.parse(r.response).generated_text` from lines 1..N), join `baseReport.results` and `adapterReport.results` by `id`, read `passed_at_1` and `pass_rate_at_1`, format compiler diagnostics from `sample.checks`, and build `"Write a Vox program that: " + (b.description || itemId)`.
4. Updated `renderCollateralTable` to read `col.reasoning_slices.benches` (with fallback to `col.slices`) and extract `entry.n ?? acc.n`, `acc.pre`, `acc.post`, `(preRate - postRate) * 100`, `acc.losses`, `acc.gains`, `acc.p_value`, and `acc.regressed`.

---

### CR-02: DOM XSS in `index.html.tmpl` via unescaped `innerHTML` with remote JSON strings

**Files modified:** `crates/vox-populi/templates/hub/space/index.html.tmpl`, `mens/runs/hub_cards/vox-mens-playground/index.html`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:** Added `escapeHtml(str)` and escaped all dynamic/remote values interpolated into `.innerHTML` across `renderLadderStrip`, `selectSize`, and `renderCollateralTable`.

---

### WR-01: `env.fetch` in `runWebGpuSideBySide` is not restored on completion/error, ignores `URL` objects, and risks browser Cache API collision

**Files modified:** `crates/vox-populi/templates/hub/space/index.html.tmpl`, `mens/runs/hub_cards/vox-mens-playground/index.html`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:** Saved `prevFetch = env.fetch` and `prevUseBrowserCache = env.useBrowserCache`, set `env.useBrowserCache = false` while loading the in-memory `baseOnnxBuf` and `adaptedOnnxBuf` models, handled `input instanceof URL ? input.href : ...` in the custom `env.fetch` interceptor, and restored both `env.fetch` and `env.useBrowserCache` in a `finally` block.

---

### WR-02: `applyLoraToOnnxQ4F16` lacks ONNX initializer shape/bounds checks, ignores `zero_points`, and skips untied `lm_head`

**Files modified:** `crates/vox-populi/templates/hub/space/index.html.tmpl`, `mens/runs/hub_cards/vox-mens-playground/index.html`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:** Mapped `logical === "lm_head"` to `"lm_head.MatMul.weight"` so untied `lm_head.MatMul.weight_Q4` tensors are patched when present while tied embeddings remain untouched; validated `inDim % 32 === 0`, `q4Info.dataLength === expectedBlocks * 16`, `scalesInfo.dataLength === expectedBlocks * 2`, and buffer bounds against `u8.length`; and added support for reading and updating 4-bit `${onnxPrefix}_zero_points` initializers when present.

---

### WR-03: Protobuf parser (`readVarint` / `skipWireValue` / `indexOnnxInitializers`) truncates tags via `>>> 3` and lacks bounds guards

**Files modified:** `crates/vox-populi/templates/hub/space/index.html.tmpl`, `mens/runs/hub_cards/vox-mens-playground/index.html`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:** Added `if (shift > 63) throw new Error("Protobuf varint overflow");` in `readVarint` and `skipWireValue`, replaced bitwise `tag >>> 3` / `tag & 7` with `Math.floor(tag / 8)` and `tag % 8`, and clamped/validated `graphEnd`, `tEnd`, `dEnd`, `sEnd`, and `dataOffset + dataLength` against `onnxU8.length`.

---

### WR-04: Leftover Gradio `app.py` and `requirements.txt` in `mens/runs/hub_cards/vox-mens-playground/` and stale comment in `hub_sync.rs`

**Files modified:** `crates/vox-populi/src/mens/hub_sync.rs`, `mens/runs/hub_cards/vox-mens-playground/index.html`, `mens/runs/hub_cards/vox-mens-playground/README.md`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:** Updated `write_local_cards` in `hub_sync.rs` to remove stale `app.py` and `requirements.txt` from `mens/runs/hub_cards/vox-mens-playground` before writing `render_playground_space_files`, updated the comment at `hub_sync.rs:1402` from `Gradio Space` to `Static WebGPU Space`, removed stale `app.py` and `requirements.txt` from `mens/runs/hub_cards/vox-mens-playground/`, and regenerated `index.html` and `README.md` on disk.

---

### IN-01: `parseSafetensors` assumes all LoRA tensors in `candle_qlora_adapter.safetensors` are `F32`

**Files modified:** `crates/vox-populi/templates/hub/space/index.html.tmpl`, `mens/runs/hub_cards/vox-mens-playground/index.html`
**Commit:** uncommitted (workspace git-no-commit policy)
**Applied fix:** Updated `parseSafetensors` to decode `F32`, `F16` (via `fp16ToFp32`), and `BF16` (via `bf16ToFp32`), and throw a descriptive error on unsupported tensor dtypes.

---

_Fixed: 2026-11-09T03:10:00Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 2_
