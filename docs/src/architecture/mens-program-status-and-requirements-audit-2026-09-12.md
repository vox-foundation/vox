---
title: "MENS Program Status & Requirements Audit"
description: "Every requirement asked for across the MENS memory-model/quantization/cloud-training program, rated against the current codebase, with challenged assumptions and a prioritized forward plan."
category: "Architecture SSOTs"
status: "current"
---

## Why this document exists

This is the second, expanded pass over a question asked twice in this
program: *given everything that was asked for, how much of it actually
exists in the codebase today, with what confidence, and what's the fastest
correct path to the concrete milestone now in front of us* — a live,
confirmed, GUI-visible round-trip: **send text to a locally fine-tuned
model on this Mac and get back a response that is measurably different
from the unmodified base model.**

The first pass (an in-chat audit) established the shape of the answer. This
version replaces every estimate with a verified reference against the
actual tree, re-derives ratings from a fresh reading of the code (not from
memory of what a plan claimed to do), and folds in everything that changed
since: a full 8-angle code review of the entire session's merged diff, six
fixes from that review, and seven follow-up tasks (fixes, an audit, an LoC
audit, a live measurement attempt, and a verification pass) dispatched and
reviewed in parallel. One of those seven — the adapter-application fix
that this whole milestone depends on — is still in a fix round as this
document is written; that is flagged explicitly, not glossed over.

**Method.** Every row below cites a real file, and where a specific claim
matters, a real commit or test name on the branch it landed on. Where a
prior review already independently verified a claim (reran the code, ran
the tests, reproduced the bug), that is noted — this document does not
re-derive what has already been checked twice; it does re-derive anything
that was only ever checked once, or not at all.

## 1. Requirements inventory

Every explicit ask across this program, in the order it was made, given a
stable ID so it can be cited from the rating table.

**R-series — the original brainstorming ask** ("design a reusable, portable
lane for MENS training... auto-detects hardware, sizes to available
VRAM... catalogue of common memory tiers with safety headroom... extends
cleanly to CLI... auto mode as default"):

- **R1** — Auto-detect hardware; size to *available*, not nameplate, memory.
- **R2** — Safety headroom for collisions/peaks, specifically naming (a) the
  OOM class that hit a 4080 Super at 4B and (b) macOS GUI/other-app memory
  competition.
- **R3** — A catalogue of common memory tiers.
- **R4** — Clean CLI extension; pick a model, it just works.
- **R5** — Auto mode as the default path.
- **R6** — Portable to any macOS machine, not just this one.

**D-series — the deep-research mandate** ("ensure your measurement
benchmark is codified and reusable... quantization lanes to support
RUNNING... Ollama support and LoC audit... SSOT across CUDA and Metal...
expansion to vast.ai/RunPod/Google Cloud... cost and suitability... choose
quant/config for this local macOS 128GB, estimate costs, document
findings"):

- **D1** — Measurement benchmark, codified and reusable for all model
  size bands.
- **D2** — Quantization lanes to support *running* (not just training)
  fine-tuned models on other platforms.
- **D3** — Ollama support.
- **D4** — LoC audit against the codebase.
- **D5** — One SSOT spanning CUDA and Metal, and both training lanes.
- **D6** — Cloud expansion: vast.ai, RunPod, Google Cloud — most
  appropriate, currently-available hardware.
- **D7** — Cost/suitability/uptime analysis tailored to Vox's actual needs.
- **D8** — A concrete recommendation: best quant + config for *this*
  machine (128GB M-series Mac), with cost estimates.
- **D9** — Document findings across all platforms considered.

**P-series — process/governance asks** (largely satisfied by execution
process, included for completeness): SDD execution discipline, merge
choices, "complete all, address all including deferred, using parallel
subagents" (issued three times across the program), and a full `/code-review
high --fix` pass.

**M-series — the newest, most concrete ask** (this message):

- **M1** — Integrate and critique everything above into one document.
- **M2** — Rate and rank every requirement.
- **M3** — Produce an improved plan: up-to-date references, assumptions
  challenged and addressed.
- **M4** — **The milestone**: confirm, live, that text can be sent to and
  received from a model fine-tuned locally on this Mac (Metal, "MTX"/matrix
  optimizations — read as Apple's Metal Performance Shaders / MPS-class
  matrix kernels that `candle`'s Metal backend already uses, not a separate
  named toolkit; nothing in this codebase names an "MTX" library
  distinct from Metal itself, so this document treats the two as the
  same ask), wired all the way through the GUI, driven by the actual CLI
  harness underneath it — not a unit test standing in for the real thing.

## 2. Rating scale

| Symbol | Meaning |
| --- | --- |
| ✅ | Done, verified against current code (cites file + how it was checked) |
| ⚠️ | Real progress, with a specific, named gap |
| 🔧 | In progress right now (a dispatched fix is not yet landed) |
| ❌ | Not attempted |
| 🚫 | Structurally blocked on something outside this codebase (hardware access, another team's server) |

## 3. Rating table — R-series (the portable auto-sizing lane)

| ID | Requirement | Rating | Evidence | Gap |
| --- | --- | --- | --- | --- |
| R1 | Auto-detect real available memory | ✅ | `AccelBudget`/`query_accel_budget` in [`crates/vox-populi/src/mens/tensor/accel_budget.rs`](../../../crates/vox-populi/src/mens/tensor/accel_budget.rs) reads `MTLDevice.recommendedMaxWorkingSetSize` on macOS and `vram_autodetect::get_system_vram_info()` (nvidia-smi-backed) on other platforms — the latter added specifically because a full-branch review found the non-macOS path returned `None` unconditionally, silently disabling every downstream safety check on CUDA. | None structural. |
| R2a | Safety headroom — the 4080/4B OOM class | ✅, hardened | `plan_for`/`sweep`/`apply_auto_size_outcome` in [`memory_model.rs`](../../../crates/vox-populi/src/mens/tensor/memory_model.rs) + [`gpu.rs`](../../../crates/vox-ml-cli/src/commands/schola/train/gpu.rs), with a hard `bail!()` refusal and a `VOX_MENS_FORCE_TRAIN` override. This mechanism itself needed **three** internal fix rounds during its own development (a deletion sweep first removed it entirely, then a second round found the restore didn't cover explicitly-sized runs, then a third found it didn't work at all on non-macOS hosts) — see §4 for why "it exists" is not the same claim as "it always fires." | Only fires when the lane is calibrated — see D5 gap below. |
| R2b | Safety headroom — macOS GUI memory competition | ✅, restored this session | `combine_with_live_pressure()` in `accel_budget.rs` + `live_pressure_budget_bytes()`/`parse_vm_stat` in [`macos_metal.rs`](../../../crates/vox-populi/src/mens/hardware/macos_metal.rs), commits `5180c1223`/`9e217ba3f` on `main`. Takes the **minimum** of the driver's static advisory and a live `vm_stat`-derived reclaimable-memory estimate, restoring a mechanism that had been deleted earlier this session as part of unrelated dead-code removal. Independently reviewed: verified the parser against real `vm_stat` output on this machine, verified the margin formula (`max(15%, 2 GiB)`) matches the original pre-deletion code byte-for-byte, verified `min()` can only make the budget more conservative (never less safe), verified `query_accel_budget()` degrades gracefully to the old behavior if `vm_stat` parsing ever fails rather than propagating a new error. | None remaining — the two nits the reviewer found (a dropped `page_size > 0` guard, a stale user-facing message) were fixed in the same session, commit `9e217ba3f`. |
| R3 | Catalogue of memory tiers | ⚠️, superseded by design | The literal static catalogue (`QWEN3_LADDER`, `METAL_*_GIB`, etc.) was deleted wholesale, replaced by the live `plan_for` calculation. This is very likely the *better* architecture — a single measured number beats a lookup table that goes stale — but it means the literal deliverable "a catalogue" doesn't exist. What survives as a catalogue-shaped artifact: `mens/config/gpu-specs.yaml`'s preset table, pinned against real weight sizes by [`training_presets_yaml_contract.rs`](../../../crates/vox-populi/tests/training_presets_yaml_contract.rs). | If a literal static fallback catalogue is still wanted for the currently-uncalibrated case (R2a's gap), it would need to be rebuilt from scratch — nothing like the original survives. |
| R4 | Clean CLI extension | ✅ | `vox mens train --model X` with no sizing flags routes through `sweep` by default (`gpu.rs`); `vox mens probe --measure`/`--sweep --model-dir`/`--detailed --model` all real, wired to the same underlying `plan_for`. | Two cosmetic gaps, both known and deferred as genuinely minor: `--model` is silently ignored when combined with `--sweep`/`--measure`; `render_verdict` isn't wired into `--sweep`'s own terse output. |
| R5 | Auto mode as default | ✅ | Confirmed default: omitting `--batch-size`/`--seq-len` triggers `sweep`; this required its own fix round, since the first version of this task only wired `sweep` into a separate manual `probe` command, not the real `vox mens train` path — closed and independently re-verified. | None on the CLI. The GUI's own "auto" story is covered under M4 below — it exists but has not been exercised against a real fine-tuned model. |
| R6 | Portable to any Mac | ⚠️ | The *code path* is genuinely portable (real Metal API, not a hardcoded constant). The *calibration data* behind it is not: exactly one real measurement exists in the whole codebase (Qwen3-0.6B, one shape, this machine), and a second, careful attempt this session (commit-free, on branch `measure/metal-calibration-round2`) hit real GPU-bound wall-clock limits and correctly reported BLOCKED rather than fabricating a second point. `fit_a_lane` is working exactly as designed by refusing to fit a curve from one point — the honest state is "the mechanism is portable, the evidence that it's *tuned* correctly for any given Mac is not yet obtained." | Needs a second, unhurried, multi-session measurement effort — not fixable by more code, only by more (real, live, GPU-bound) time. |

## 4. Rating table — D-series (the deep-research mandate)

| ID | Requirement | Rating | Evidence | Gap |
| --- | --- | --- | --- | --- |
| D1 | Reusable measurement benchmark | ⚠️ | `CalibrationRecord`/`fit_a_lane`/`PeakSampler` in [`calibration.rs`](../../../crates/vox-populi/src/mens/tensor/calibration.rs) is real, reusable machinery — the framework, not a one-off script. | Only filled with real data for one (model, lane, shape) triple. See R6. |
| D2 | Quantization lanes for *running* elsewhere | ✅, code-verified; 🔧 live round-trip pending | Streamed `recombine`/`ArtifactWriter` ([`vox-quantize`](../../../crates/vox-quantize/src)), zero-fitted-parameter `plan_quantize`, MLX-input rejection, an Ollama publish lane (llama/gemma2 architectures), a llama.cpp GGUF lane (Qwen3 architectures, since Ollama's own converter refuses those) — all reviewed, all unit-tested. **A live, real-artifact round-trip verification was dispatched this session** (branch `verify/ollama-llamacpp-roundtrip`) and had not reported back as of this writing — see §6 pending items. | Until that verification lands: this row is "should work, extensively reviewed, never yet run against a real checkpoint end-to-end." |
| D3 | Ollama support | ✅ | Config-SSOT URL resolution (`vox_config::inference::local_ollama_populi_base_url()`), `/api/embed` migration off the deprecated `/api/embeddings`, a tool-calling fallback fix, the publish lane above. | None found in the full-diff code review specific to Ollama integration itself. |
| D4 | LoC audit | ✅, completed this session | [`mens-loc-audit-2026-09.md`](mens-loc-audit-2026-09.md), commit `377db27f0` on branch `docs/loc-audit-2026-09`. Headline numbers: MENS surface ≈ 52,245 lines (51,781 Rust across 217 files + 464 TS), roughly 5.8% of the ~890,600-line Rust workspace; largest single file `preset_schema.rs` at 1,368 lines. **This audit surfaced two new, previously-unknown findings**, not previously tracked anywhere in this program: 26 files exceed this repo's own God-Object 500-line error threshold, of which 23 are unsuppressed (no ledger entry, no `toestub-ignore`); and 23 files (~3,000 lines) are byte-for-byte identical between the CUDA and Metal plugin crates with **no** `// vox:defactored-from` marker, several far exceeding this repo's own ~50-line duplication carve-out. | This ask is *complete* — the audit exists — but it produced net-new, real technical debt that nothing in this program has yet triaged. Recommended as the next scoped follow-up (§7). |
| D5 | One SSOT across CUDA/Metal/both training lanes | ⚠️, structurally unified, data-asymmetric | `MemoryModels`/`Lane`/`CalKey`/`plan_for` is genuinely one entry point every consumer routes through — verified repeatedly across this session's reviews that `sweep`, `verify_request_with_budget`, and the cloud-dispatch sizing path all call the *same* function and cannot disagree with each other by construction. | The underlying *contract* (`contracts/mens/memory-model.v1.yaml`) has exactly one calibrated cell: `candle-cuda` with `gradient_checkpointing: true`. Every Metal run, and every CUDA run under the ~2.9B-parameter threshold that auto-enables checkpointing, resolves to "uncalibrated" and gets a warning instead of a real fit/refuse verdict. The SSOT is real; the data it's unified *over* is one-sixth filled in. This is the single most consequential remaining gap in the whole program, and it is a data-collection problem, not a code problem — see §7. |
| D6 | Cloud expansion (vast.ai/RunPod/GCP) | ✅ | `offer_is_suitable`/`UnsuitableReason` wired into `CloudResolver::rank_offers`; `vox mens cloud-estimate` (read-only cost/fit command reusing the *same* `plan_for` the local training path uses — this exact equivalence was independently verified, including a real bug found and fixed where cloud dispatch could rent a GPU sized for one shape and then the remote worker would silently re-derive a *different*, unverified shape from its own preset ladder: fixed this session, commit `6e1be9072`, closing the gap at both the transmission and the remote-consumption end). Google Cloud is covered in the survey doc (D9) but has no dedicated resolver/provider client the way vast.ai and RunPod do — the ask named it as a candidate to *consider*, and it was considered and priced, not integrated as a live provider. | If Google Cloud needs the same live integration as vast.ai/RunPod (a real `CloudProvider` implementation), that is unbuilt. |
| D7 | Cost/suitability/uptime tailored to Vox | ⚠️ | [`mens-cloud-training-economics-2026-09-11.md`](mens-cloud-training-economics-2026-09-11.md) covers 13 providers with per-row Verified/Reported/Unverified provenance flags — a real, disciplined survey, not a hand-wave. | The doc's own disclosed gaps stand: a promised source-URL column isn't delivered, and one budget section's arithmetic isn't shown against its own stated rates. Both are cosmetic, not load-bearing. |
| D8 | Concrete recommendation for this 128GB Mac | ❌ | No task in any of the three execution plans produced a written "here is the recommended quant + config for this specific machine" decision memo. The *infrastructure* to answer this now fully exists (`plan_for`, `sweep`, `cloud-estimate`) — nobody has run it end-to-end and written down the answer. | This is the one D-series item genuinely never attempted, not merely partial. It is now trivially answerable given everything else in this document — see §7, recommended as a 20-minute follow-up once M4 lands (M4's own live run will produce most of the needed evidence as a side effect). |
| D9 | Document findings across platforms | ✅ | Both the cloud economics doc (D7) and the calibration measurement doc ([`2026-09-11-candle-metal-calibration.md`](measurements/2026-09-11-candle-metal-calibration.md)) exist, are dated, and are explicit about what's measured versus estimated versus unverifiable. | Both are single-session snapshots; neither is a living document with a re-verification cadence. Acceptable for a research deliverable, worth noting for anyone treating them as permanently current. |

## 5. Assumptions challenged this pass

Four assumptions from earlier in this program turned out to be wrong or
incomplete once checked against the real, current tree — each is
recorded here specifically so a future reader doesn't re-inherit the
wrong belief silently.

1. **"The three-round VRAM-refusal fix means the safety net is closed."**
   Wrong as stated. The full-diff code review (§6) found the fix is
   structurally sound but only *reachable* for one calibration cell —
   large, checkpointed CUDA models. Every Mac run, and every small CUDA
   run, still gets a warning instead of a refusal. The mechanism is
   correct; its coverage is one-sixth of what the phrase "the safety net"
   implies. This is now stated plainly in D5 above rather than left
   implicit.

2. **"`ModelShape::from_model_dir`'s `artifact_bytes` reads real weight
   sizes."** Wrong, and this was the most severe bug found this session:
   `DirEntry::metadata()` does not follow symlinks, and every weight file
   in a Hugging Face Hub cache is a symlink into `blobs/`. This was
   independently reproduced on this exact machine (a real 1.5GB weight
   file measured as 76 bytes) before being fixed in commit `3ebc4d42c`.
   Had this shipped unfixed, the entire measured-safety story in this
   document would have been close to fictional for any real downloaded
   model — every fit calculation would have silently ignored the weights
   term. This is now fixed and covered by a regression test
   (`model_shape_follows_symlinked_weights_like_a_real_hf_cache`).

3. **"The Ollama and llama.cpp lanes work because they're reviewed and
   unit-tested."** Reviewed and unit-tested is not the same claim as
   "has ever produced a running model on another machine," and this
   document does not make that stronger claim — D2 above is explicit that
   this is still pending live verification, dispatched but not yet
   reported at time of writing.

4. **"LoRA adapters are applied at inference — the serving path was
   fixed."** This was true as of the fix landing (commit `c008e0064`),
   but an independent review found the fix itself has a real memory
   blow-up (materializes every layer's dense delta simultaneously,
   which would OOM on real hardware before the first token), a scope-creep
   change bundled in under a false justification, and — most importantly —
   **zero test coverage on the actual wiring**: the reviewer reverted one
   of thirteen fixed call sites back to the original bug and all 57 tests
   still passed. A fix round closing all three is in progress as of this
   writing (§6). This document treats M4 as blocked on that round landing
   and being re-verified, not as already satisfied by the first commit.

## 6. What changed this session (full-diff review + parallel follow-ups)

A `/code-review high --fix` pass was run over the entire session's merged
work (`git diff 7295b4470..main`, 62 commits, 103 files, ~12,500
insertions). Ten findings survived verification; six were fixed
immediately (commit `3ebc4d42c`), four were explicitly skipped with
reasons recorded (needs real hardware calibration, or needs a remote
contract this repo can't unilaterally invent). Following that, seven
parallel follow-up tasks were dispatched to close the skipped items and
the gaps this document's first draft surfaced:

| Task | Outcome |
| --- | --- |
| LoRA adapter application (the M4 prerequisite) | 🔧 Fix round in progress — see §5.4 |
| Cloud env transmission to remote workers | ✅ Merged, commit `6e1be9072` — found the real container entrypoint and fixed both the local and remote halves |
| Metal live memory-pressure restoration | ✅ Merged, commits `5180c1223`/`9e217ba3f` |
| `/v1/completions/stream` removal audit | ✅ Merged, commit `42eb0c797` — confirmed deliberate and well root-caused, fixed only the missing disclosure |
| LoC audit | ✅ Merged, commit `377db27f0` — see D4 |
| Second Metal calibration measurement | 🚫 Correctly self-terminated as BLOCKED after one bounded, careful attempt (real progress, too slow to finish in the safety-bounded window); no changes made, nothing fabricated |
| Ollama/llama.cpp live round-trip verification | 🔧 Pending as of this writing |

Two items remain outside this codebase's reach entirely: the CUDA hosts
(`bdesktop`, `bdesktop2`) are still unreachable (confirmed again this
session — connection timeout), and cross-machine verification
(`blaptop04`) is still blocked on an operator fixing a Windows OpenSSH
ACL issue on that machine's own console. Neither has a code-side
workaround.

## 7. Improved forward plan

Ordered by what actually blocks the next real milestone, not by
discovery order.

1. **Land the LoRA fix round (in progress).** Verify the memory fix with
   a real mutation test (revert a call site, confirm the new test catches
   it), confirm the scope-creep sampling change is split into its own
   honestly-labeled commit, merge.
2. **Run the M4 milestone for real**, once (1) lands: train or reuse a
   small real LoRA adapter on this Mac, serve it through `vox mens serve`,
   send a prompt through the actual GUI surface, and confirm the response
   differs measurably from the same prompt against the unmodified base
   model. This is the concrete, falsifiable version of "confirmed working"
   — not a passing test suite, an actual observed input/output pair. Full
   plan for this run in §8.
3. **Close the D4 LoC-audit fallout** as its own small, scoped task: triage
   the 23 unsuppressed God-Object files (either split the largest, or add
   documented `toestub-ignore`/suppression entries with real reasons), and
   either add `// vox:defactored-from` markers to the 23 duplicated
   CUDA/Metal files or fold the ones that exceed the ~50-line carve-out
   into a shared crate.
4. **Write the D8 decision memo** — this becomes nearly free once (2) has
   run, since the live run will produce real numbers (peak memory, tokens/
   sec) for at least one real configuration on this machine; extend that
   into the 2-3 configuration comparison the original ask wanted.
5. **Confirm the Ollama/llama.cpp verification's result** (dispatched,
   pending) and act on whatever real defect it surfaces, if any — per its
   own instructions it was told to stop and report rather than patch
   blind if it found one.
6. **Schedule, not rush, a second Metal calibration measurement** (R6/D1/D5's
   real remaining gap) as an explicitly unhurried, multi-hour or
   multi-session background effort — the two attempts made so far both
   correctly refused to fabricate a result under time pressure, which is
   the right behavior to keep, not a problem to route around with a
   longer timeout on the same rushed pattern.

## 8. The M4 milestone — exact plan

Once the LoRA fix round lands and is re-verified:

1. Use the existing real calibration checkpoint (Qwen3-0.6B, already
   locally cached) rather than downloading a new model — this keeps the
   run fast and reuses infrastructure already proven to work on this
   machine.
2. Produce a real, small LoRA adapter for it — either a genuine (even if
   brief) `vox mens train` run, or, if wall-clock needs to stay short, a
   deliberately large synthetic adapter delta on one or two layers,
   clearly labeled as synthetic-but-real (a real safetensors file with
   real `.lora_a.weight`/`.lora_b.weight` keys, not a mock) — the goal is
   to prove the *serving path* applies whatever adapter it's given, which
   a synthetic large delta demonstrates unambiguously via a measurably
   different output, without needing a lengthy real training run to
   converge on.
3. Serve it via `vox mens serve`.
4. Drive it two ways and compare: (a) directly via the CLI/HTTP surface
   (`vox mens serve` + a raw request) as a baseline, and (b) through the
   actual GUI surface — confirming the GUI's model-selection/probe
   surfaces correctly target the adapter-bearing model, not silently the
   base model.
5. Record the base-model response and the adapter-model response to the
   identical prompt side by side, showing a measurable difference, as the
   actual evidence this milestone asked for — not "tests pass," a real
   observed pair of outputs.
