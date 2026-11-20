# Project Research Summary

**Milestone:** v9.5 MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases
**Workstream:** `mens-flywheel-hub-release-v95`
**Synthesized:** 2026-11-20 from STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md (same directory)
**Confidence:** MEDIUM-HIGH overall (Stack HIGH, Architecture HIGH, Features MEDIUM-HIGH, Pitfalls MEDIUM-HIGH)

> Written by the orchestrator from the synthesizer's returned text: the synthesizer's own file write
> was refused by the harness. Content is the synthesizer's; nothing was added.

## Key Findings

### Contradictions between sources and how they were resolved

Live or code-verified findings win over the handoff and over unverified statements.

1. **Tags exist.** FEATURES says "zero tags"; live refs and `hub_sync.rs:1191-1376` show `0.8b` v1,v2; `2b` v1,v2;
   `4b` v1; `9b` v1; none on `27b` or `vox-mens-index`. `0.8b` main is 2 commits past v2 (card-only pushes after
   the tag). Real gaps: untagged 27B/index, tags created even for `Ungated` uploads, no tag→SHA release map, no
   SHA-pin guidance.
2. **Tag scheme.** Keep the existing monotonic `v<N>`; record the commit SHA. Do not introduce `v9.5.0`/`YYYY.MM`.
3. **`MODEL_CARD.md`.** Delete it (the Hub renders only `README.md`; both are one string written twice at
   `hub_sync.rs:737-738,1072-1073`). Do it as one declared atomic commit with a file-list contract test; first
   check `hub pull` / `pull_adapter_revision_with_spoke` for dependencies, and migrate the Space's root-evidence
   readers before deleting root duplicates of `evidence/`.
4. **Sequential gate.** Sequential McNemar already exists (`eval_collateral.rs:391`), selected by policy `rule:`.
   Add a separate `eval-gates-ladder-fast.yaml` plus a `--policy` option. Editing the main ladder changes
   `policy_blake3` and invalidates every existing `gate_receipt`.
5. **Router.** `mens-models.md` claims the HF router serves our checkpoints; the live probe returned "not served by
   any provider". The router serves base 2B/4B/9B/27B only (not base 0.8B). Correct the doc.
6. **Space WebGPU path is likely broken.** The template pins transformers.js 3.5.0 (no `qwen3_5`) and fetches
   `onnx/model_q4f16.onnx`, which does not exist in `onnx-community/Qwen3.5-*-ONNX`. Not run end to end.
   transformers.js has no runtime LoRA. Never show another model's output as ours.
7. **Spoke stamping is writer-side.** Readers exist (`hub_card/facts.rs:657-685`, `eval_gate/mod.rs:55-95`) and
   look for `spoke` / `domain_profile`; no writer emits either. Fix in `train_arm.rs`, `train_record.rs`, new
   `run_stamp.rs`, using the existing `spoke` key. Then remove `default_spoke`; missing spoke = `unknown`, which
   blocks release.
8. **"Lane" has seven meanings.** Do NOT rename the serving lane (`codegen|hub|agent`), router `lane:` triggers, or
   orchestrator lanes. Rename scope: compute-backend `lane` (serde alias), corpus row slice → `data_slice`, the
   "Lane G" aliases, and populating `train_lanes`.
9. **Prefix cache is a new feature, not a bug fix.** No cache is retained today. `Qwen35LayerCache` clones are
   shallow, so a snapshot must deep-copy. Split prefill can change numerics at temperature 0.
10. **`.eval_results`.** Dataset ids must be registered Hub Benchmarks; `vox-mens/*` are not. Treat as
    machine-readable evidence only; no leaderboard or "verified" language. Rendering unverified.
11. **Display rule.** Keep "verified wins only" for code blocks; add a counts-based "where it still fails" table.
12. **Hub bugs found.** `hub_sync.rs:1141` stages `populi_train_record.json` (real file: `train-record.json`);
    `app.py.tmpl` and `requirements.txt.tmpl` are dead; a failed Space upload is only a warning; cards do not
    record which benchmark produced a held-out number, so v1 and v2 are indistinguishable.

### Stack

No new dependencies. Hub tags, branches, refs, commit-with-delete and Collections work through the pinned
`hf-hub` 1.0.0 plus raw `reqwest`. The Hub has no Releases feature: use tag message + generated "Release history"
and "How to pin" tables in the card + a release map in `vox-mens-index`. Space tests use the `@playwright/test`
already in `docs-astro`.

### Features (table stakes)

- Three release classes from paired evidence: Improved / Neutral / Regressed. Only Improved moves `main` and is
  tagged; the others are recorded in the index release log, neither hidden nor shipped.
- One card file, one evidence copy, card numbers template-bound to evidence JSON.
- Card = use-it-or-not and what changed; `mens-models.md` = shared definitions; YAML/JSON = what tooling reads.
- Paired statistics (exact McNemar, Wilson/bootstrap intervals), per-category counts without tests on tiny n,
  one pre-declared primary comparison, failure funnel whose buckets sum to n, truncation reported separately.
- Space labels recorded output as recorded and live output as live, never blended.

### Pitfalls that order the work

- The handoff's distillation plan targets zero-pass `heldout_bench_v2` categories: that leaks by construction.
  Seal a blind v3 slice (commit its hash) before any data generation; enforce the firewall in code.
- The 64-task canary is a subset of the 166: not independent evidence. n=64 is about ±8 pp.
- The dirty v9.4 tree edits the verifier: freeze a proof manifest and re-score baselines before any claim.
- Do not compare step-capped runs with full runs.
- `--allow-ungated` exists: the release path must be non-bypassable, with evidence hash == uploaded-weights hash.
- New metric fields must be `Option`, never default 0 (plugin dylib and CLI are built separately).
- Record the resolved teacher model id per distilled row (licence history differs by version).

## Implications for Roadmap

**Wave 0 (serial):** commit the ~157-path v9.4 tree in path-scoped commits for a clean base SHA; pre-register
`mod` lines in `mens/mod.rs`; add a GPU serialization lock; freeze four contracts (plugin response fields,
`exec_error_kind` enum, run-record keys, funnel schema).

**Wave 1 (parallel, file-disjoint groups; spikes first):**

| Group | Scope | Notes |
|---|---|---|
| Spike A | CPU prefix-cache design + equivalence on 0.8B config | before P1's cache work |
| Spike B | Space in-browser feasibility | before any Space inference work |
| P1 | plugin timing, then prefix KV (metal + cuda `inference.rs`) | one owner for both backends |
| P2 | host timing (`eval_timing.rs`) | |
| P3 | failure funnel (`metrics.rs`, `eval_funnel.rs`) | |
| P4 | lane/config/docs cleanup; wire `verified_vox_codegen_v2.jsonl` as `optional: false` | starts with a full `lane` inventory |
| P5 | run stamping (`run_stamp.rs`, `train_arm.rs`, `train_record.rs`) | |
| P6a | hub hardening (bugs above, dedupe, release map) | |
| P6b | remove `default_spoke` | after P5 |
| P7 | live `eval_spoke` | |
| P8 | backend `lane` → `backend` rename | low value |

**Wave 2 (no GPU):** Space headline on the v2 benchmark; broken WebGPU path fixed or disabled and relabelled;
router claim corrected; Space contract tests (URL HEAD check, browser smoke).

**Wave 3 (single GPU, strictly serial, nothing before PID 34157 exits):** G1 Metal parity of cached vs uncached
decode → G2 Tier-1 64-item canary with timing and funnel (freeze proof manifest, re-score baselines) → G3 live
`agent_episodes` (72) → G4 27B 30-minute preflight → G5 CAP-27B-04 (~30 h train + v2 eval) → v2 gate-floor
calibration in its own reviewed commit.

**Wave 4:** seal blind v3 slice and pre-register the primary metric → gated distillation → before/after proof →
gated release (hash-bound evidence, non-regression vs last tag, NOTICE/attribution, real `hub pull` by SHA).

**Research flags:** prefix KV design, Space in-browser feasibility, proof statistics and blind-slice protocol need
phase research. Hygiene, timing, funnel, hub hardening, run stamping and the rename are standard patterns.

### Decisions for the project owner

1. **27B repo without weights:** remove the public repo until weights pass the gate (recommended), or keep an
   evidence-only repo with a first-line disclosure.
2. **Space scope:** honest evidence viewer + base-model demo + local serve (recommended), or add merged-ONNX
   in-browser inference for 0.8B (needs a Python export path under `vox-python-bridge/worker/**`), or a paid
   live Space.
3. **Teacher-distillation spend:** total cap, allowed teacher ids, and approval to start only after the blind
   slice is sealed and the baseline frozen.

Smaller defaults: bench v2 items never public; one canonical vox-lang mix; prefix cache may ship behind a new
decode-protocol tag if bitwise identity fails; drop `MODEL_CARD.md`.

## Sources

See the Sources sections of STACK.md, FEATURES.md, ARCHITECTURE.md and PITFALLS.md in this directory. Unverified
items are marked there (`[VERIFY]` / "unverified"): model-page rendering of unregistered eval ids, merged-ONNX
export, SwiftShader timing, COEP/CORP for multithreaded WASM, HTTPS Space to localhost serve.
