---
phase: 339-model-cards-v2
reviewed: 2026-11-09T03:05:00Z
depth: deep
files_reviewed: 11
files_reviewed_list:
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/README.md.tmpl
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/app.py.tmpl
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/requirements.txt.tmpl
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/src/mens/hub.rs
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/src/mens/hub_sync.rs
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/src/mens/hub_cards_push.rs
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/src/mens/hub_card/svg.rs
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-orchestrator-mcp/src/gui_ops/mens_corpus.rs
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-gui/src/commands/mens_run_reports.rs
  - /Users/brbrainerd/dev/vox-mens-compare/crates/vox-gui/ui/src/components/surfaces/Models/MensCorpusStudioPanel.tsx
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 339: Code Review Report (Iteration 2)

**Reviewed:** 2026-11-09T03:05:00Z
**Depth:** deep
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Deep adversarial review (Iteration 2) of Phase 339 following Iteration 1 fixes and the introduction of the Static WebGPU Playground Space (`crates/vox-populi/templates/hub/space/index.html.tmpl` and `README.md.tmpl`).

All 13 Iteration 1 findings (`CR-01`..`CR-06`, `WR-01`..`WR-05`, `IN-01`..`IN-02`) were verified fixed across both `/Users/brbrainerd/dev/vox-mens-compare` and `/Users/brbrainerd/dev/vox`. However, the newly introduced `index.html.tmpl` and Static Space migration contain 2 Critical issues, 4 Warnings, and 1 Info item:

1. **Broken Live Hub Evidence Explorer (`CR-01`)**: `index.html.tmpl` parses non-existent schema fields across all 6 Hub JSON/JSONL files (`ladder_index.json`, `hf_sync_manifest.json`, `gate_receipt.json`, `collateral_damage_report.json`, `heldout_*_nothink.json`, and `heldout_*_nothink.ckpt.jsonl`), resulting in 0 held-out samples rendered, `"not evaluated"` collateral tables, and `vundefined` manifest strings.
2. **DOM XSS via unescaped `innerHTML` (`CR-02`)**: `index.html.tmpl` interpolates remote JSON strings (`r.remote_gate_status`, `r.sync_status`, `gr.blocking_reasons`, `adapterServing`, `name`, etc.) directly into `innerHTML` without HTML escaping.
3. **WebGPU `env.fetch` interception leak & Cache API collision (`WR-01`)**: `env.fetch` is never restored in a `finally` block, does not handle `URL` instances, and leaves `env.useBrowserCache` enabled so `baseModel` and `adapterModel` can collide or cache mutable buffers.
4. **ONNX `MatMulNBits` shape / Asymmetric Quantization (`zero_points`) / `lm_head` handling (`WR-02`)**: `applyLoraToOnnxQ4F16` does not validate `q4Info.dims` (`[outDim, numBlocksPerRow, 16]`) or buffer bounds before writing in-place, ignores `*_zero_points` initializers if present, and silently skips `lm_head` without checking if an untied `lm_head.MatMul.weight_Q4` exists.
5. **Protobuf wire-format parser edge cases (`WR-03`)**: `indexOnnxInitializers` uses `>>> 3` (32-bit truncation) on varint tags, does not handle negative int64 varints, and throws on deprecated group wire types (`3`/`4`).
6. **Stale Gradio `app.py` / `requirements.txt` artifacts (`WR-04`)**: `crates/vox-populi/templates/hub/space/{app.py.tmpl,requirements.txt.tmpl}` and `mens/runs/hub_cards/vox-mens-playground/{app.py,requirements.txt}` remain on disk after switching the Space SDK to `static`.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `index.html.tmpl` Live Hub Evidence Explorer reads wrong JSON/JSONL schema keys for all 6 Hub artifacts (renders 0 samples and empty collateral table)

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl:481-699`
**Issue:** Every Hub JSON/JSONL artifact parser in `index.html.tmpl` reads property names that do not exist in the actual Phase 339 Hub schemas:
1. **`ladder_index.json` (lines 481-485)**: Reads `r.remote_gate_status || r.local_gate_status` and `r.remote_revision`. In `SizeSyncRow` (`hub_sync.rs:1137-1150`), the actual serialized fields are `r.gate_status` (e.g. `"passed"` or `"ungated (...)"`) and `r.latest_version` (already prefixed with `"v"`, e.g. `"v10"`).
2. **`hf_sync_manifest.json` (line 612)**: Reads `sm.hub_revision` and `sm.base_model`. In `HfSyncManifest` (`hub_sync.rs:242-252`), the fields are `sm.base_pin` and `sm.corpus_hash` (there is no `hub_revision` or `base_model`), so line 612 renders `"vundefined (commit )"`.
3. **`gate_receipt.json` (line 611)**: Reads `gr.passed` and `gr.blocking_reasons`. In `vox_mens_gate_receipt_v2`, the fields are `gr.overall_passed` (boolean) and `gr.gates` (array of `{ name, passed, block, message }`).
4. **`evidence/heldout_{base,adapter}_nothink.json` (lines 530-559, 577-587, 604-609)**:
   - Reads `br.constructed` and `br.pass_rate`; actual top-level fields are `br.passed_at_1`, `br.total`, and `br.pass_rate_at_1`.
   - Reads `ar.adapter_serving` from the JSON report; `adapter_serving` actually lives in the first line (header) of `heldout_adapter_nothink.ckpt.jsonl` at `header.inputs.model.adapter_serving`.
   - Reads `baseReport.samples` with `s.sample_id`, `s.constructed`, `s.error_Preview`, `s.hir_ok`, `s.first_typecheck_error`, `s.ast_check_failures`. In `heldout_*_nothink.json`, the array is `baseReport.results`, where each item has `{ id, category, description, pass_at_1, samples: [{ pass, checks: { non_empty, parse_ok, typecheck_ok, diag_errors, parse_error, anti_stub_pass, pass_compile, pass_ast }, completion_preview }] }`. Because `baseReport.samples` is `undefined`, `joined` is always empty (`[]`) and the Held-Out Benchmark selector always says `"No held-out samples match filter on Hub"`.
5. **`evidence/heldout_{base,adapter}_nothink.ckpt.jsonl` (lines 521-528, 547-558)**:
   - Assumes each JSONL line has `{ sample_id, prompt_json, output }`. In `vox_eval_item_checkpoint.v1`, line 0 is the header `{"schema":"vox_eval_item_checkpoint.v1","inputs":{"model":{"adapter_serving":"nf4_base_plus_lora_arm v2"},...}}` and lines 1..N are `{"item":"<blake3>","response":"{\"generated_text\":\"...\",\"prompt_tokens\":...}"}`. Just like `examples.rs:152-181`, checkpoint texts must be matched to `sample.completion_preview` by checking `answerRegion(text).slice(0, 240) === sample.completion_preview` (or falling back to `sample.completion_preview`), and the prompt text comes from `"Write a Vox program that: " + r.description`.
6. **`collateral_damage_report.json` (lines 664-686)**:
   - Checks `if (!col || !col.slices)`. In `collateral_damage_report.json`, slices live at `col.reasoning_slices.benches` (map of `sliceName -> { n, accuracy: { n, pre, post, losses, gains, p_value, regressed } }`) or `col.reports` (array). Because `col.slices` is `undefined`, the collateral table always displays `"No collateral_damage_report.json published"`.

**Fix:**
Update `loadRungEvidence`, `formatDiag`, `selectSize`, and `renderCollateralTable` in `index.html.tmpl` to match the exact Rust schemas in `hub_sync.rs`, `facts.rs`, and `examples.rs`:
```javascript
    function extractAnswerRegion(text) {
      if (typeof text !== "string") return "";
      const closeIdx = text.lastIndexOf("</think>");
      if (closeIdx !== -1) return text.slice(closeIdx + "</think>".length).trimStart();
      if (text.trimStart().startsWith("<think>")) return "";
      return text;
    }

    function extractShownCode(text) {
      const body = extractAnswerRegion(text);
      const start = body.indexOf("```");
      if (start === -1) return body.trim();
      const after = body.slice(start + 3);
      const nl = after.indexOf("\n");
      const code = nl === -1 ? after : after.slice(nl + 1);
      const end = code.indexOf("```");
      return (end === -1 ? code : code.slice(0, end)).trim();
    }

    function matchCkptText(preview, texts) {
      if (!preview || !Array.isArray(texts)) return null;
      const hits = texts.filter((t) => extractAnswerRegion(t).slice(0, 240) === preview.slice(0, 240));
      return hits.length === 1 ? extractShownCode(hits[0]) : extractShownCode(preview);
    }

    function parseCkptLines(rows) {
      if (!Array.isArray(rows) || rows.length === 0) return { texts: [], serving: null };
      const header = rows[0] || {};
      const serving = header?.inputs?.model?.adapter_serving || null;
      const texts = [];
      for (let i = 1; i < rows.length; i++) {
        const r = rows[i];
        if (!r || typeof r.response !== "string") continue;
        try {
          const parsed = JSON.parse(r.response);
          if (typeof parsed.generated_text === "string") texts.push(parsed.generated_text);
        } catch {}
      }
      return { texts, serving };
    }
```
And in `loadRungEvidence`:
- Join `baseReport.results` and `adapterReport.results` by `r.id`, reading `r.samples?.[0]`.
- Set `promptText = "Write a Vox program that: " + (b.description || b.id)`.
- In `renderCollateralTable`, read `const benches = col?.reasoning_slices?.benches;` and for each `[name, entry]` read `const acc = entry?.accuracy || entry;` with `acc.pre`, `acc.post`, `((acc.pre - acc.post) * 100)`, `acc.losses`, `acc.gains`, and `acc.p_value`.

---

### CR-02: DOM XSS in `index.html.tmpl` via unescaped `innerHTML` with remote JSON strings

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl:486-492, 595, 614-621, 666, 669-702`
**Issue:** `index.html.tmpl` interpolates untrusted strings fetched over HTTP (`r.gate_status`, `r.latest_version`, `r.sync_status`, `gr.gates[].message`, `adapterServing`, `name`, `bundle.size`) directly into `.innerHTML` templates without HTML entity escaping. If any gate failure message or JSON field contains `<`, `>`, or `"`, it breaks the DOM or executes arbitrary script in the user's browser origin (`huggingface.co` Space iframe).
**Fix:**
Add a `function escapeHtml(s)` helper and pass every interpolated dynamic value through `escapeHtml(...)` (or build nodes using `textContent`):
```javascript
    function escapeHtml(str) {
      return String(str ?? "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
    }
```

---

## Warnings

### WR-01: `env.fetch` in `runWebGpuSideBySide` is not restored on completion/error, ignores `URL` objects, and risks browser Cache API collision

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl:1034-1065`
**Issue:**
1. **Unrestored global `env.fetch` hook**: `env.fetch` is overwritten with a closure capturing the multi-hundred-MB `baseOnnxBuf` and `adaptedOnnxBuf` `ArrayBuffer`s and `serveAdapted = true`, and is never restored to `nativeFetch` after `AutoModelForCausalLM.from_pretrained` finishes (or throws). If a subsequent call or another size fails mid-load, `env.fetch` remains hijacked and leaks both buffers.
2. **`URL` instance handling**: `const urlStr = typeof input === "string" ? input : (input && input.url) || "";` evaluates to `""` when `@huggingface/transformers` passes a `URL` object (`input instanceof URL`), bypassing the interceptor and downloading the unpatched model from the network. Use `typeof input === "string" ? input : (input instanceof URL ? input.href : (input && input.url) || "")`.
3. **Browser Cache API collision**: `@huggingface/transformers` v3 uses the browser Cache API (`env.useBrowserCache = true` by default) keyed by request URL. When `env.useBrowserCache` is active, `@huggingface/transformers` checks `caches.open('transformers-cache')` *before* calling `env.fetch`, or caches the synthetic `__vox_lora_adapter__` ONNX response across page reloads even when a new adapter version is published on Hub. Setting `env.useBrowserCache = false;` around the custom `from_pretrained` calls ensures the live merged `adaptedOnnxBuf` is always used.

**Fix:**
```javascript
          const prevFetch = env.fetch;
          const prevUseBrowserCache = env.useBrowserCache;
          let serveAdapted = false;
          env.useBrowserCache = false;
          env.fetch = async (input, init) => {
            const urlStr =
              typeof input === "string"
                ? input
                : input instanceof URL
                  ? input.href
                  : (input && input.url) || "";
            if (urlStr.endsWith("onnx/model_q4f16.onnx")) {
              const buf = serveAdapted ? adaptedOnnxBuf.slice(0) : baseOnnxBuf.slice(0);
              return new Response(buf, {
                status: 200,
                headers: { "Content-Type": "application/octet-stream" },
              });
            }
            if (urlStr.includes("__vox_lora_adapter__/")) {
              const rewritten = urlStr.replace("__vox_lora_adapter__", onnxRepo);
              return nativeFetch(rewritten, init);
            }
            return nativeFetch(input, init);
          };
          try {
            serveAdapted = false;
            const baseModel = await AutoModelForCausalLM.from_pretrained(onnxRepo, {
              dtype: "q4f16",
              device: "webgpu",
            });
            serveAdapted = true;
            const adapterModel = await AutoModelForCausalLM.from_pretrained("__vox_lora_adapter__", {
              dtype: "q4f16",
              device: "webgpu",
            });
            cached = { baseModel, adapterModel, patchedCount };
            state.cachedSessions[size] = cached;
          } finally {
            env.fetch = prevFetch;
            env.useBrowserCache = prevUseBrowserCache;
          }
```

---

### WR-02: `applyLoraToOnnxQ4F16` lacks ONNX initializer shape/bounds checks, ignores `zero_points`, and skips untied `lm_head`

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl:852-928`
**Issue:**
1. **Unchecked initializer dimensions & buffer bounds**: `applyLoraToOnnxQ4F16` assumes `q4Info` has `outDim * (inDim / 32) * 16` bytes and `scalesInfo` has `outDim * (inDim / 32) * 2` bytes without verifying `q4Info.dataLength === outDim * numBlocksPerRow * 16` or `scalesInfo.dataLength === outDim * numBlocksPerRow * 2` or `(inDim % 32 === 0)`. If an ONNX model uses `block_size = 64` or `128`, or stores `scales` in FP32, `dv.getUint16(scByteOff, true)` and `u8[q4BlockOff + byteIdx]` silently corrupt adjacent protobuf tensors or throw a `RangeError`.
2. **Asymmetric quantization `zero_points` ignored**: ONNX `MatMulNBits` supports an optional 4th input (`${onnxPrefix}_zero_points`). While symmetric `MatMulNBits` omits `zero_points` and uses a constant midpoint of `8`, asymmetric `MatMulNBits` packs 4-bit zero-points per block in `${onnxPrefix}_zero_points`. If `${onnxPrefix}_zero_points` exists in `inits`, dequantizing with `(q0 - 8)` instead of the stored zero-point (and requantizing to midpoint `8` without updating `zero_points`) shifts the weight distribution.
3. **`lm_head` handling**: Line 854 unconditionally `if (logical === "lm_head") continue;`. While Qwen3-0.6B/1.7B/4B tie `lm_head` to `model.embed_tokens.weight`, Qwen3-8B/14B/32B (or untied ONNX exports that emit `lm_head.MatMul.weight_Q4`) have a dedicated `lm_head` `MatMulNBits` node. Instead of skipping `lm_head` unconditionally, map `logical === "lm_head"` to `lm_head.MatMul.weight` (never `embed_tokens`) so untied `lm_head.MatMul.weight_Q4` is patched when present in `inits` while tied embeddings remain untouched.

**Fix:**
- Map `const onnxPrefix = logical === "lm_head" ? "lm_head.MatMul.weight" : baseKey.replace(".self_attn.", ".attn.").replace(/\.weight$/, ".MatMul.weight");`.
- Guard each layer with `const expectedBlocks = outDim * numBlocksPerRow; if (inDim % 32 !== 0 || q4Info.dataLength !== expectedBlocks * 16 || scalesInfo.dataLength !== expectedBlocks * 2) continue;`.
- Check `const zpInfo = inits[\`${onnxPrefix}_zero_points\`];` and read/update the 4-bit zero point (`(u8[zpInfo.dataOffset + (blockIdx >> 1)] >> ((blockIdx & 1) * 4)) & 0x0f`) when `zpInfo` is present.

---

### WR-03: Protobuf parser (`readVarint` / `skipWireValue` / `indexOnnxInitializers`) truncates tags via `>>> 3` and lacks bounds guards

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl:754-836`
**Issue:**
1. In `readVarint`, if a protobuf field (`int64` / `int32`) encodes a negative number (10-byte two's-complement varint) or a corrupt varint, `shift` grows past `70` without bound.
2. In `indexOnnxInitializers` (lines 787, 793, 805), `const fieldNum = tag >>> 3;` and `const wireType = tag & 7;` coerce `tag` to a signed 32-bit integer before shifting. For standard ONNX field numbers (`1..15`) this works, but `Math.floor(tag / 8)` and `tag % 8` avoid 32-bit bitwise truncation if a custom protobuf extension uses a large field number.
3. In `indexOnnxInitializers`, `graphEnd = p.pos + graphLen`, `tEnd = p.pos + tLen`, and `p.pos += dataLength` are not clamped or validated against `onnxU8.length`, so a truncated buffer can loop past `tEnd`.

**Fix:**
Add `if (shift > 63) throw new Error("Varint overflow");` in `readVarint`, validate `if (graphEnd > onnxU8.length || tEnd > graphEnd) break;`, and check `if (p.pos + dataLength <= tEnd)` before recording `initializers[name]`.

---

### WR-04: Leftover Gradio `app.py.tmpl`, `requirements.txt.tmpl`, and `mens/runs/hub_cards/vox-mens-playground/{app.py,requirements.txt}`

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/src/mens/hub_sync.rs:1222-1227, 1399-1406`
**Issue:**
1. `write_local_cards` in `hub_sync.rs:1222-1227` writes `README.md` and `index.html` into `mens/runs/hub_cards/vox-mens-playground/`, but does not remove stale `app.py` and `requirements.txt` left over from the earlier Gradio Space generation (`/Users/brbrainerd/dev/vox-mens-compare/mens/runs/hub_cards/vox-mens-playground/app.py` and `requirements.txt` still exist on disk).
2. Unused templates `crates/vox-populi/templates/hub/space/app.py.tmpl` and `crates/vox-populi/templates/hub/space/requirements.txt.tmpl` remain in both workspaces even though `hub_sync.rs` now uses `index.html.tmpl` (`sdk: static`).
3. Comment at `hub_sync.rs:1399` still says `// Also update the interactive Hugging Face Gradio Space`.

**Fix:**
- In `write_local_cards` (`hub_sync.rs:1222-1227`), remove stale `app.py` and `requirements.txt` from `sp` if present:
```rust
    let sp = dir.join("vox-mens-playground");
    std::fs::create_dir_all(&sp).with_context(|| format!("create {}", sp.display()))?;
    for stale in ["app.py", "requirements.txt"] {
        let _ = std::fs::remove_file(sp.join(stale));
    }
    for (rel_name, content) in render_playground_space_files(namespace, entries) {
        std::fs::write(sp.join(rel_name), content)?;
    }
```
- Update the comment at `hub_sync.rs:1399` from `Gradio Space` to `Static WebGPU Space`.

---

## Info

### IN-01: `parseSafetensors` assumes all LoRA tensors in `candle_qlora_adapter.safetensors` are `F32`

**File:** `/Users/brbrainerd/dev/vox-mens-compare/crates/vox-populi/templates/hub/space/index.html.tmpl:744-750`
**Issue:**
`parseSafetensors` constructs `new Float32Array(arrayBuffer.slice(dataStart + begin, dataStart + end))` unconditionally without checking `meta.dtype`. While Candle QLoRA v2/v3 currently serializes all `lora_a` and `lora_b` tensors as `F32`, if a future adapter checkpoint saves `F16` or `BF16` tensors, `new Float32Array(...)` will misinterpret the bytes.
**Fix:**
Support `F16` / `BF16` conversion in `parseSafetensors` when `meta.dtype === "F16"` or `"BF16"`, or throw a descriptive error if `meta.dtype !== "F32"`.

---

_Reviewed: 2026-11-09T03:05:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: deep_
