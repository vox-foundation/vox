# Technology Stack

**Project:** Vox v9.5 "MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases"
**Researched:** 2026-11-20
**Scope:** Hub release mechanics, static-Space inference, Space E2E testing. Candle QLoRA, `hub_sync`/`hub_card` internals and the v92 card research are NOT re-researched.
**Method note:** Hub endpoints were read from the live OpenAPI spec (`https://huggingface.co/.well-known/openapi.json`, fetched today) and Hub docs; live repo/Space state and WebGPU behaviour were probed today with read-only HTTP and a throwaway Playwright script (scratchpad, not in repo). Confidence tags: HIGH = official spec/docs or directly probed today; MEDIUM = single secondary source; LOW/UNVERIFIED = stated explicitly.

## Headline findings (read these first)

1. **No new dependency is needed for Hub versioning.** The release primitives (tag, branch, refs, commit/delete, collections) are already reachable through `hf-hub 1.0.0` (workspace pin `hf-hub = "1"`) and raw `reqwest` (already used for Collections in `hub.rs:606-748`). Only **Discussions** need raw `reqwest` (hf-hub 1.0.0 has no discussion API; verified by grep of its source).
2. **Versioning code already exists but is under-applied.** `hub_sync.rs:1232 create_repo_tag` + `next_version_tag` + legacy baseline `v1` (HFVER-01) are implemented. Live refs today (contradicts the brief's "no tags"): `qwen3.5-0.8b` v1,v2; `2b` v1,v2; `4b` v1; `9b` v1; **`qwen3.8-27b` and `vox-mens-index` have none**. `0.8b` `main` (`7120f866`, "Update ... model card (no weights)") is 2 commits past `v2` (`6842003f`) because card-only pushes land after the tag. So the gap is: untagged 27B/index, card-only commits after tags, no release notes, no immutable-pin guidance.
3. **The Space's WebGPU path for the Qwen3.5 rungs is almost certainly broken today** (template `index.html.tmpl:1684-1721`): it pins `@huggingface/transformers@3.5.0` (bundle contains zero `qwen3_5` strings; 4.3.2 does) and fetches `onnx/model_q4f16.onnx`, which does **not exist** in `onnx-community/Qwen3.5-{0.8B,2B,4B}-ONNX` (they ship `decoder_model_merged_q4f16.onnx` + `embed_tokens_*` + `vision_encoder_*`; file listing fetched today). That file does exist for `onnx-community/Qwen3-0.6B-ONNX` (570 MB). A "live inference is proven" check would catch this in seconds.
4. **The HF router cannot serve our fine-tunes.** `GET router.huggingface.co/v1/models/brbrainerd/vox-mens-qwen3.5-0.8b` returns `{"error":"Chat completion model not served by any provider"}`; `inferenceProviderMapping` is `{}`. Mapping is created by providers (not by repo owners). The router *can* serve the **base** models `Qwen/Qwen3.5-2B|4B` (featherless-ai), `9B` (deepinfra), `Qwen3.8-27B` (akashml); **not** `Qwen3.5-0.8B`. So `hf-router` mode in the Space can only ever show the base side of a base-vs-adapter comparison.
5. **Headless Chromium has no WebGPU adapter by default; one flag fixes it** (probed today against the live Space): default Playwright headless -> `navigator.gpu` exists, `requestAdapter()` = `null`, Space badge "WASM SIMD Fallback (No WebGPU Adapter)"; with `--enable-unsafe-webgpu --enable-features=Vulkan` -> adapter `{vendor: google, arch: swiftshader}` (software), badge "WebGPU Ready". A "WebGPU Ready" badge in CI therefore proves nothing about speed or real-GPU behaviour.

## Recommended Stack

### A. Hub release mechanics (no new crates)

| Mechanism | Endpoint / API (verified) | Use in v9.5 | Why | Confidence |
|-----------|---------------------------|-------------|-----|------------|
| Immutable pin | commit SHA via `GET /api/models/{ns}/{repo}/refs` -> `tags[].targetCommit`; consumers use `revision=<sha>` / `resolve/<sha>/<file>` | Release manifest records `{tag, commit_sha}`; docs tell users to pin the SHA, treat the tag as the human label | Tags are mutable: spec exposes `DELETE /api/models/{ns}/{repo}/tag/{rev}` and branch `overwrite:true`. A tag can be deleted and re-created onto another commit | HIGH |
| Tag (release marker) | `POST /api/models/{ns}/{repo}/tag/{rev}` body `{tag, message}`; hf-hub `.create_tag().tag().maybe_revision().message()` (already used, `hub_sync.rs:1232`); 409 if tag exists | Keep `vN` monotonic tags (`next_version_tag`). Create the tag **after** the final card+evidence commit | The tag message is the only per-release note stored with the ref | HIGH |
| Refs listing | `GET /api/models/{ns}/{repo}/refs` -> `{tags, branches, converts, pullRequests}` each `{name, ref, targetCommit}`; hf-hub `.list_refs()` (already used, `list_repo_tags`) | Add a `list_repo_refs` returning `targetCommit` (currently names only) | Needed to detect "main is ahead of latest tag" and to record SHAs in the manifest | HIGH |
| Branch | `POST /api/models/{ns}/{repo}/branch/{rev}` body `{startingPoint, emptyBranch, overwrite}` (path `rev` = the NEW branch name); hf-hub `.create_branch()` | Optional `candidate/<run-id>` branch: upload there, run gate + Space check, then fast-forward/tag main. Do not use branches as the primary version scheme | Lets "gated on eval evidence" be literal: nothing reaches `main` before evidence passes | HIGH (API); MEDIUM (promotion flow: no merge-branch endpoint exists, only PR merge `POST .../discussions/{num}/merge` or re-commit, so promotion = a PR merge or a second upload) |
| Commit with deletions | `POST /api/models/{ns}/{repo}/commit/{rev}` (NDJSON: header, `file`, `lfsFile`, `deletedEntry`, `parentCommit` optional); hf-hub `CommitOperation::{Add,Delete}`, `upload_folder(... delete_patterns ...)` | One atomic commit that adds the canonical files and deletes stale duplicates (`MODEL_CARD.md`, root-level evidence copies) | Single-commit release = tag points at a self-consistent tree; `parentCommit` guards against races | HIGH |
| Eval results | `.eval_results/*.yaml` (list of `{dataset:{id,task_id}, value, date, source, notes}`); also `model-index:` in README front matter | Keep `.eval_results/` (already published; Hub parses it: `?expand[]=evalResults` returns entries with `verified:false` today). Keep `model-index` only if card must also render the widget | Docs: `.eval_results` dataset ids "must be a Benchmark" (a Hub dataset with `eval.yaml`, allow-list is Beta). Our ids `vox-mens/*` are not Hub datasets, so **no leaderboard aggregation or badge will occur** (UNVERIFIED how the model page renders unregistered ids). Treat it as machine-readable evidence, not as a leaderboard feature | HIGH (docs) / UNVERIFIED (rendering) |
| Model card file | `README.md` ONLY is the card (docs: "model card as the `README.md` file") | Stop writing `MODEL_CARD.md` (`hub_sync.rs:738, 1073-1075`) and delete it in the next release commit | Hub never renders `MODEL_CARD.md`; it only creates drift | HIGH |
| Version linking | README front matter `new_version: <repo>` (docs: shows a link to latest, follows chains) | Only useful across repos (e.g. superseded rung); not for same-repo versions | | HIGH |
| Collections | `POST/GET/PATCH /api/collections...`, items add/batch (already in `hub.rs:606-748`) | One "VoxMens vN" collection per release grouping the rung repos + eval dataset + Space; slug keeps history | Collections are the only cross-repo "release bundle" primitive on the Hub | HIGH |
| Changelog / release notes | **The Hub has no Releases feature.** OpenAPI has no releases path; docs have no release-notes page (searched Model Cards, repo docs, hf_api reference). Closest primitives: tag message, README "Changelog" section, a pinned Discussion | (1) Tag message = one-line note. (2) README gets a generated `## Release history` table (tag, commit SHA, date, gate status, pass@1, one-line delta) rendered by `hub_card` from the manifest. (3) Optionally one pinned Discussion per release via `POST /api/models/{ns}/{repo}/discussions` `{title (3-200 chars), description (<=65536), pullRequest:false}` then `POST .../discussions/{num}/pin` | A card table is versioned with the tag; Discussions are mutable/off-tree so they are a notification channel, not the record | HIGH (endpoints) / MEDIUM (recommendation is ours) |
| Downstream pinning | `AutoModel.from_pretrained(repo, revision="v2" or sha)`, `hf download --revision`, `https://huggingface.co/<repo>/resolve/<rev>/<file>`, Space `preload_from_hub: - repo file sha` | Card "How to pin" block generated from the manifest; Space template `hfUrl()` (`index.html.tmpl:589`, hard-codes `resolve/main`) takes the pinned SHA from `ladder_index.json` | Space evidence tab must show the evidence of the release it links, not whatever `main` is today | HIGH |

**Where it lands in code (no new files required):**
- `crates/vox-populi/src/mens/hub_sync.rs` ~1232-1376: extend `create_repo_tag` callers (index repo, 27B repo, and the card-only push path in `hub_cards_push.rs` should either tag a patch version or fold the card into the release commit). Add `release_manifest` fields (`tag`, `commit_sha`, `gate_status`, eval digests) to the existing `hf_sync_manifest.json`.
- `crates/vox-populi/src/mens/hub.rs`: add `create_discussion`/`pin_discussion` beside the Collections raw-`reqwest` helpers (same `bearer_auth` pattern, `HuggingFaceToken` via Clavis; no new secret).
- `crates/vox-populi/src/mens/hub_card/{facts,render}.rs`: render Release history + How-to-pin from manifest facts; delete duplicate `MODEL_CARD.md` emission.
- `hub_sync.rs:32-62 OPTIONAL_BUNDLE_FILES`: pick ONE evidence location (`evidence/`). The Space reads `gate_receipt.json`, `collateral_damage_report.json` at root and `evidence/heldout_*` (template 794-799); `eval_local_report.json`/`benchmark_passatk.json` at root are duplicates of `evidence/benchmark_passatk.json` per the live file list. Migrate readers first, then delete roots via `delete_patterns` in a single commit.
- Release gating ("eval evidence required"): the `gate_receipt` check already exists (HFPUB-02); add "Space proof receipt" (see section C) as a second receipt consumed by the same gate.

### B. Static Space inference

| Option | Verdict | Evidence |
|--------|---------|----------|
| **transformers.js `@huggingface/transformers` 4.3.2 (npm latest, published 2026-10-27; `4.4.0-next.0` 2026-10-30)** | **Use, pinned exactly to an npm version, vendored or CDN-pinned with SRI** | `qwen3_5`, `qwen3_5_text` are present in the 4.3.2 bundle (grep today). Qwen3.5 ONNX model card (onnx-community) uses `Qwen3_5ForConditionalGeneration` + `AutoProcessor` with per-component dtype `{embed_tokens:"q4", vision_encoder:"fp16", decoder_model_merged:"q4"}` and `device:"webgpu"` (card says `npm i @huggingface/transformers@next` at fetch time) |
| Space's current `@3.5.0` + `AutoModelForCausalLM` + `onnx/model_q4f16.onnx` | **Replace** | 3.5.0 has no `qwen3_5`; filename absent for Qwen3.5 repos |
| **Runtime LoRA in browser (transformers.js)** | **Do not claim / do not build** | `grep -ic lora` on the 4.3.2 web bundle = 0. No public adapter option. onnxruntime (core) has a `LoraAdapter` API and `.onnx_adapter` format, but it is not surfaced by transformers.js, and WebGPU-EP support plus a model exported with adapter inputs are UNVERIFIED |
| Existing hand-rolled client-side merge (parses ONNX protobuf, patches `MatMulNBits` `_Q4/_scales/_zero_points` initializers; template ~1172-1340) | **Retire from the default path** | It depends on one exporter's initializer naming, re-quantizing deltas into q4 is lossy, and it only ever targeted the single-file `model_q4f16.onnx` layout. It is unproven for the split Qwen3.5 layout |
| **Ship merged ONNX** (LoRA merged into base, exported, q4) as separate Hub model repos, loaded by stock transformers.js | **Recommended for in-browser, 0.8B only to start** | Payload (text-only decoder+embeddings, q4, sizes listed today): 0.8B = 485 MB + 163 MB = ~0.65 GB; 2B = 1.21 GB + 0.33 GB = ~1.5 GB; 4B = 2.7 GB + 0.41 GB = ~3.1 GB. Vision encoder can be skipped for a text-only coder. 4B+ is not a sane browser download. Note a `-ONNX-OPT` variant exists for 0.8B (newer) |
| HF Inference Providers router for the adapter | **Not possible**; provider mappings are created by providers, and the staff answer on the forum for custom fine-tunes was "use Inference Endpoints" (MEDIUM, forum). Router is fine for **base** model comparison on 2B/4B/9B/27B | Live probe above |
| Inference Endpoints (dedicated) | Out of scope for a free static Space; mention only as the paid path for "chat with the adapter" (supports vLLM/TGI, autoscaling) | docs: https://huggingface.co/docs/inference-endpoints/index (LoRA specifics UNVERIFIED) |
| Local serve (`vox mens serve`, template default `http://127.0.0.1:11435`) | Keep as the honest "full fidelity" path | HTTPS-page -> `http://127.0.0.1` fetch (mixed content / Private Network Access) is UNVERIFIED in current Chrome; needs a real test |

**The unresolved hard part (flag for phase research):** producing a merged, q4-quantized ONNX of the Candle-trained adapter needs a PEFT-compatible merge and an ONNX exporter. The only mature exporter is Python (optimum / HF conversion tooling). AGENTS.md forbids new `.py` glue outside `crates/vox-python-bridge/worker/**`. Options: (a) run the export inside `vox-python-bridge` worker (allowed scope, uv-locked); (b) do the merge in Rust (safetensors out, Candle already has the base+LoRA) and export via a pinned external HF Space/Job; (c) scope v9.5 to NOT ship browser inference of the adapter: keep the Space honest ("evidence viewer + base-model WebGPU demo + local serve") and make the inference claim exactly what the automated check proves. **Recommendation: (c) for v9.5 with the check in section C, and (a) as a research-flagged stretch for 0.8B only.** Do not add heavyweight deps.

**Space config facts (HIGH, docs):** `sdk: static`, `app_file`, optional `app_build_command` (runs in a Job, output served from `refs/convert/build`), `custom_headers` accepts only COEP/COOP/CORP (all lowercase), `preload_from_hub` and `datasets/models` front-matter lists. Live Space already returns `cross-origin-opener-policy: same-origin` and `access-control-allow-origin: *`. `crossOriginIsolated` was `false` in the probe, so multi-threaded WASM (SharedArrayBuffer) needs `cross-origin-embedder-policy: require-corp` plus CORP on every cross-origin fetch; whether `huggingface.co/resolve` and the jsdelivr CDN send CORP is UNVERIFIED. Default to single-thread WASM / WebGPU and avoid COEP.

### C. Testing the live Space automatically

| Layer | Tool | What it proves | Cost |
|-------|------|----------------|------|
| C0 (no browser, `cargo test`, network-ignored lane) | existing `reqwest` in `vox-populi` | Extract every URL/filename the template fetches (`onnxRepo`, `onnx/...` file names, `transformers@x.y.z` CDN URL, evidence paths) and `HEAD` them (`https://huggingface.co/<repo>/resolve/<sha-or-tag>/<file>` -> 2xx; npm/CDN package exists and contains the model `model_type`). Catches the missing `onnx/model_q4f16.onnx` and the 3.5.0/qwen3_5 mismatch with zero JS | seconds |
| C1 (deploy gate) | poll `GET /api/spaces/{ns}/{space}` until `runtime.stage == "RUNNING"` and `sha == <pushed commit>` (both fields present today) | "Space is the version we pushed" before testing; avoids testing the old build | seconds |
| C2 (browser smoke) | `@playwright/test` ^1.63 (already in `docs-astro/package.json`; chromium-1255 installed locally; no new dependency). Target the direct origin `https://<ns>-<space>.static.hf.space/` (302 -> `/index.html`), NOT the `huggingface.co/spaces/...` iframe wrapper | Page loads, tabs render, evidence tab populated from Hub, every network response to `huggingface.co`/CDN is 2xx, zero `console.error`/`pageerror` | ~10 s |
| C3 (opt-in nightly/pre-release) | same, with `launchOptions.args: ["--enable-unsafe-webgpu","--enable-features=Vulkan"]` (+ `--use-angle=vulkan --disable-vulkan-surface` per Chrome's headless guide, MEDIUM) | Real generation: load the 0.8B merged/base ONNX, greedy-decode N tokens, assert non-empty, tokens>0, and (if adapter shipped) output differs from base. Record `adapter.info` (swiftshader vs real GPU) and wall time in the receipt | minutes; UNVERIFIED on SwiftShader at 0.65 GB (may exceed a sane timeout) |

Verified details: WebGPU requires a secure context (the `.static.hf.space` origin is HTTPS; `isSecureContext` was `true`). Default headless gave `requestAdapter() === null`. `--enable-unsafe-webgpu --enable-features=Vulkan` gave a SwiftShader adapter on this macOS host (even on Apple Silicon; a real-Metal adapter was not obtained in headless, UNVERIFIED headed). Chrome's own guide for Linux CI: `--headless=new --use-angle=vulkan --enable-features=Vulkan --disable-vulkan-surface --enable-unsafe-webgpu` (https://developer.chrome.com/blog/supercharge-web-ai-testing).

**Where it lands:** put the spec in `docs-astro/tests/hub-space/space-live.spec.ts` (already has Playwright and the dual-theme test conventions) or `crates/vox-gui/ui/e2e/` if the GUI-visual-verification rule is judged to apply; invoke it from a `.vox` script / `vox ci` subcommand (no `.sh`/`.py`), write a `space_proof_receipt.json` (`space_sha`, `transformers_version`, `adapter_kind`, `tokens`, `wall_ms`, `device`) and make `hub_sync` publish refuse to tag a release whose receipt is missing/failed. TypeScript only, no `.js`/`.mjs`.

## Core Technologies (additions/changes only)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `hf-hub` (existing) | 1.0.0 (`"1"` workspace pin) | tags, refs, branches, upload_folder `delete_patterns`, `CommitOperation::Delete` | Already in workspace and already used for tags/refs; no bump needed |
| `reqwest` (existing) | workspace | Discussions, pin, Space status polling, URL HEAD checks | Same pattern as Collections in `hub.rs`; `hf-hub 1.0.0` lacks discussions |
| `@huggingface/transformers` (Space, browser) | 4.3.2 exact (not `@next`) | in-browser Qwen3.5 ONNX | Only version line with `qwen3_5`; pin exactly, never `@latest` |
| `@playwright/test` (existing, docs-astro) | ^1.63.0 | Space smoke | Already present; zero new deps |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Versioning | tag + SHA in manifest + card table | Branch-per-version | Branches are not listed in the UI as releases; no merge-branch endpoint; more refs to garbage-collect |
| Release notes | README Release-history table + tag message (+ optional pinned Discussion) | Discussions only | Off-tree and editable; not pinned to the tag |
| Eval metadata | `.eval_results` + minimal `model-index` | Register `vox-mens/*` as Hub Benchmark datasets | Allow-list is Beta (docs say "get in touch"); worth a later issue, not a v9.5 dependency |
| Browser adapter | merged ONNX repos | runtime LoRA / in-browser q4 patching | No public API; unproven; fragile |
| Router for adapters | Do not offer | Inference Providers | Mapping empty and not owner-controllable |
| Space proof | C0 URL contract + C2 smoke (+C3 opt-in) | Full generation on every CI run | SwiftShader is slow and flaky as a gate |
| Space framework | keep `sdk: static` | Gradio/Docker Space with real compute | Paid/CPU-bound and out of milestone scope |

## What NOT to add

- No new Python (`optimum`, `onnx`, scripts) outside `crates/vox-python-bridge/worker/**`; no `.sh` glue.
- No new Rust crates (no `huggingface_hub` Python bridge, no `octocrab`-like clients, no browser-driver crate); `hf-hub` + `reqwest` + Playwright already cover it.
- No second model-card file (`MODEL_CARD.md`), no duplicate root/`evidence/` JSON.
- No `@next` / unpinned `@latest` transformers.js, no unpinned CDN URLs.
- No claim of "live LoRA merging in your browser" until a C3 receipt proves it for that rung.
- No direct vendor LLM hostnames in workspace code: the Space is a deliverable HTML artifact and the router call there is user-supplied-token browser JS, but keep it out of Rust crates (the `llm_provider_call` detector covers Rust/TS/Vox sources, so confirm the template is not scanned or add a documented marker).

## Installation

```bash
# Nothing to install for Hub release work (hf-hub 1.0.0 + reqwest already in vox-populi).
# Space proof (existing devDependency, run from docs-astro):
pnpm --dir docs-astro exec playwright install chromium   # only if browsers are missing
```

## Sources

- Hub OpenAPI spec (HIGH): https://huggingface.co/.well-known/openapi.json (paths `tag/{rev}`, `branch/{rev}`, `refs`, `commit/{rev}`, `super-squash`, `discussions`, `discussions/{num}/pin`, `collections`)
- huggingface_hub `HfApi` reference (HIGH): https://huggingface.co/docs/huggingface_hub/package_reference/hf_api (`create_tag`, `create_branch`, `create_commit`, 25k LFS file / `parent_commit`)
- Model cards (HIGH): https://huggingface.co/docs/hub/model-cards (README.md is the card, `new_version`, `base_model`, `model-index`)
- Evaluation results (HIGH): https://huggingface.co/docs/hub/eval-results (`.eval_results/*.yaml`, Benchmarks Beta allow-list)
- Static Spaces + config (HIGH): https://huggingface.co/docs/hub/spaces-sdks-static , https://huggingface.co/docs/hub/spaces-config-reference
- Inference Providers (HIGH): https://huggingface.co/docs/inference-providers/index , https://huggingface.co/docs/inference-providers/hub-api (`expand[]=inferenceProviderMapping`)
- Provider registration (MEDIUM): https://huggingface.co/docs/inference-providers/register-as-a-provider ; forum "How can I make my fine-tuned model supported by inference providers?" https://discuss.huggingface.co/t/how-can-i-make-my-fine-tuned-model-supported-by-inference-providers/154765 (answer: use Inference Endpoints)
- Inference Endpoints: https://huggingface.co/docs/inference-endpoints/index
- Qwen3.5 ONNX card (HIGH): https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX ; file sizes from `https://huggingface.co/api/models/onnx-community/<repo>/tree/main/onnx` (fetched today)
- transformers.js: npm registry dist-tags (latest 4.3.2, next 4.4.0-next.0); bundle greps of 3.5.0 (`qwen3_5` = 0) and 4.3.2 (`qwen3_5` present, `lora` = 0)
- onnxruntime LoRA adapter API (MEDIUM): https://onnxruntime.ai/docs/api/js/interfaces/LoraAdapter.html
- Headless WebGPU (MEDIUM): https://developer.chrome.com/blog/supercharge-web-ai-testing
- Local probes today: live Space `brbrainerd/vox-mens-playground` (RUNNING, sha `d5d3a431`), `refs` of the six model repos, router `/v1/models/*`, Playwright WebGPU probe (scratchpad only).
- Repo: `crates/vox-populi/src/mens/{hub.rs,hub_sync.rs,hub_cards_push.rs}`, `crates/vox-populi/templates/hub/space/index.html.tmpl`, `docs/src/architecture/mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md`.

## Unverified / needs phase research

- How the Hub model page renders `.eval_results` entries whose dataset ids are not registered Benchmarks.
- Merged-ONNX export pipeline for a Candle-trained LoRA (tooling, Python scope, q4 quality delta vs Candle outputs).
- Whether SwiftShader WebGPU can run a 0.65 GB q4 decode within a CI timeout; real-GPU headless on Apple Silicon.
- COEP/CORP behaviour of `huggingface.co/resolve` and CDN for cross-origin-isolated multithreaded WASM.
- HTTPS Space -> `http://127.0.0.1:11435` fetch under current Chrome Private Network Access.
- Whether any Inference Providers partner offers self-serve LoRA/fine-tune hosting mapped to a user repo.
- The `research-plan`/`research-store` seam was not used for this run (direct docs/API/live probes instead); no digests were cached.
