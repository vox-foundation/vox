---
title: "MENS Reasoning Preservation — Does Our Qwen3 SFT Break Thinking Mode? (2026-09-21)"
description: "Audit of how the MENS Candle QLoRA pipeline formats and supervises Qwen3-8B training rows, cross-checked against Qwen, Unsloth, and 2024–2026 forgetting research, with a verdict on chain-of-thought damage and a ranked fix list."
category: "Architecture SSOTs"
status: "research"
training_eligible: false
---

# MENS Reasoning Preservation — Research Findings (2026-09-21)

**Question.** Does the way we fine-tune Qwen3 into MENS destroy its chain-of-thought / reasoning ability? Unsloth recommends keeping "at least 75% reasoning data" to preserve it.

**Verdict (short).**

- **"Destroy" is not established — nobody has measured it**, here or in the literature for hybrid Qwen3-8B specifically. Our only "collateral damage" report measures nothing relevant (§3.4).
- **But our pipeline has the exact shape that causes thinking-mode collapse**, and it is a *format* problem more than a *ratio* problem. We train Qwen3's thinking-mode entry point (`<|im_start|>assistant\n`) to emit the answer immediately, with no `<think>` block and no `/no_think` flag (§3.1). The closest controlled result (Qwen3-4B-Thinking, LoRA, answers-only) went to **0% valid reasoning**.
- **The damage is most likely mode suppression, not erased capability.** LoRA r=16, one epoch, and loss on only the last 64 tokens all limit forgetting. So the model probably still *can* reason but has learned not to *start*. That is plausible, not verified.
- **The 75% figure is a vendor heuristic.** It describes one notebook's data mix; no experiment backs it. Peer-reviewed work shows hybrid models keep think-mode accuracy even at 1:4 think:no-think — *when the no-think rows are formatted correctly and real think rows are present*. We have neither: 1.1% of rows are "reasoning", and all of them are a single fixed template (§3.2).

## 1. Method

- **Code audit** of the Candle QLoRA path. Qwen3-8B is the default base (`vox_populi::mens::DEFAULT_MODEL_ID`, pinned `b968826d…`). The pipeline runs mix → encode → mask → loss, then eval and the collateral gate.
- **Data measured directly** on `target/dogfood/train.jsonl`, the train file recorded in the `qwen3_8b_metal_v2` run manifest (9,268 rows). Token counts used that run's `tokenizer.json`.
- **External sources** are graded **E** (empirical, measured) or **H** (heuristic / vendor advice). Sources are listed in §5.

## 2. How reasoning gets damaged in hybrid Qwen3 (external evidence)

| # | Finding | Grade |
|---|---|---|
| R1 | **How Qwen trained thinking and non-thinking into one model (Stage 3 "Thinking Mode Fusion").** Non-thinking SFT rows carry `/no_think` in the user turn and keep an **empty** `<think>\n\n</think>\n\n` block. Thinking rows were **self-distilled**: rejection-sampled from the model's own Stage-2 checkpoint. | E (Qwen3 tech report) |
| R2 | **Even Qwen's careful fusion cost reasoning.** Think-mode AIME'24 fell 83.8 → 81.9 → 81.4 and LiveCodeBench v5 fell 68.4 → 67.2 → 65.7 across Stages 2–4. That is the *best case* for adding non-thinking data. | E (Qwen3 tech report, Table 22) |
| R3 | **What Qwen3's chat template actually renders.** The last assistant turn always gets a think block, empty if there is no reasoning. Prior turns have their think content stripped. `enable_thinking=False` pre-fills `<think>\n\n</think>\n\n`. `enable_thinking=True` (the default) leaves the generation prompt as bare `<|im_start|>assistant\n`. | E (tokenizer_config.json) |
| R4 | **The closest controlled test of answers-only LoRA.** Qwen3-4B-Thinking-2507, LoRA r=32, lr 1e-4, 1 epoch. Trained on answers only: **0% valid reasoning**, GSM8K 34.5. Trained on distilled traces: ~94.5 / 100% reasoning. Masking the empty think block plus a KL anchor: 96.0. Same pattern on Gemma and gpt-oss. | E, but a vendor blog, not peer-reviewed. Thinking-only model, not hybrid. |
| R5 | **Adding more no-think data did not hurt think-mode accuracy in a hybrid model.** Up to 1:4 think:no-think, MATH500 stayed ~86–87% — *with* paired think data present. Training reasoning first, then the mix, gives better mode control. The same paper shows Qwen3-8B leaks reasoning into no-think mode. | E (arXiv 2510.12680) |
| R6 | **LoRA forgets less than full fine-tuning.** Forgetting grows with rank and with training length. Measured on code instruction tuning, but the forgetting measured is commonsense QA, not CoT. | E (Biderman et al. 2024) |
| R7 | **A lower learning rate cuts general-capability loss** at similar in-domain gain. | E (Lin et al., ICLR 2026) |
| R8 | **Training on the model's own outputs forgets less.** Self-distilled / on-policy SFT (SDFT; "RL forgets less because on-policy") reduces forgetting vs vanilla SFT. | E (Yang et al. 2024; Chen et al. 2025) |
| R9 | **Caveat on self-distillation:** if the teacher is conditioned on privileged info (e.g. the gold answer), traces get shorter and more overconfident, and math/OOD reasoning can drop by up to 40%. | E (arXiv 2603.24472) |
| R10 | **Unsloth's "75% reasoning / 25% non-reasoning" rule** is a description of their notebook's mix (OpenMathReasoning + FineTome). No sweep or benchmark supports the number. | **H** |

*Gap:* no published controlled study fine-tunes **hybrid** Qwen3-8B with LoRA on code-only answers and then measures thinking-mode benchmarks. R4 is the nearest proxy.

## 3. What our pipeline actually does

### 3.1 Row format trains "skip thinking" at the thinking-mode entry point *(main risk)*

- **How rows are rendered.** `try_encode_training_step` (`crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/encoding.rs`) builds every row with `chatml_supervised_text` (`crates/vox-plugin-mens-candle-core/src/training_text.rs`). That renders `<|im_start|>assistant\n{response}<|im_end|>`: no empty think block, no `/no_think`.
- **Why that is the worst format for thinking mode.** Our training prefix is byte-identical to Qwen3's thinking-mode generation prompt (R3). So each supervised answer teaches: *in thinking mode, the first token after `assistant\n` is code, not `<think>`.* This is exactly the collapse mechanism in R4.
- **Why it doesn't cleanly teach non-thinking mode either.** The canonical non-thinking prefix (`<think>\n\n</think>\n\n`) never appears in training. So we are not adapting the non-thinking mode the way Qwen's fusion data did (R1). We are overwriting the thinking mode.

### 3.2 Reasoning share is ~1%, and all of it is boilerplate

- **Measured share:**

  | | Rows | Share |
  |---|---|---|
  | All rows | 9,268 | 100% |
  | Rows with `<think>` | 104 | 1.1% |
  | `response_mode` = `prose_only` | 5,728 | 62% |
  | `response_mode` = `code_only` | 2,540 | 27% |
  | `response_mode` = `structured` | 1,000 | 11% |

- **All 104 think rows come from one template.** `generate_cot_organic_corpus` (`crates/vox-corpus/src/codegen_vox/part_03.rs`) emits the same three lines for every row, changing only the construct name: *"1. Plan: The user wants to generate a `X` construct… 2. Syntax considerations… 3. Implementation…"*.
- **This teaches that thinking is fixed filler unrelated to the problem.** Arguably that is worse than no think rows. The code in the samples we inspected is also not semantically coherent — e.g. `let created_at = fn(user: int) to metric` inside an `activity` that returns a string literal as `Result[str]`.

### 3.3 `ce_last_k = 64` limits the damage, but only partly

- **What gets loss.** Loss is masked to the **last 64 tokens** of each sequence (default `qlora_ce_last_k: 64`; mask in `training_loop/forward.rs`, identical on CUDA and Metal).
- **Effect on the "skip thinking" decision.** For a response longer than 64 tokens, the first assistant tokens are **not** in the loss, so the decision is only indirectly shaped.
- **How many rows still train it directly.** Measured response length: p50 = 137 tokens, p90 = 510. **20.2% of rows (~1,870) are ≤ 64 tokens.** Those rows directly supervise "first assistant token = answer". A single-token mode switch with ~1.9k direct examples per epoch is plenty to move.
- **Side effect on the template think rows.** Their think block sits at the start of the response, outside the last-64 window for most rows. So even the ~1% "reasoning" rows mostly train only their code tail.

### 3.4 Nothing measures reasoning retention

- **The gate exists but can't see reasoning.** `vox mens serve` refuses adapters without a passing `collateral_damage_report.json` (`crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`).
- **The only report is vacuous.** `mens/runs/qwen3_8b_metal_v2/collateral_damage_report.json` passes, but its "benchmarks" are eval config fields (`max_tokens`, `temperature`, `seed_base`, `k`) plus Vox pass-rates over 10 items. Every metric is identical pre and post.
- **No general-reasoning baseline anywhere.** There is no GSM8K / MATH500 / LiveCodeBench baseline in `mens/data/`. The gate code supports such benches (`vox-eval` tests use `gsm8k` / `mmlu` names), but none are wired.
- **Eval uses a third, non-canonical prefix.** `eval_local_prompt.rs` prompts with `<think>\n</think>\n` (single newlines) plus "Do not output reasoning". That differs from both the training prefix (no block) and Qwen's canonical `<think>\n\n</think>\n\n`, and it cannot observe thinking-mode behaviour at all.

### 3.5 Factors that *reduce* forgetting (these are why "destroy" overstates it)

| Setting | Our run | Why it helps |
|---|---|---|
| LoRA rank | r=16 (α=32), NF4 base | LoRA forgets less than full FT; low rank forgets least (R6) |
| Epochs | 1 (`qwen3_8b_metal_v2`); presets default to 3 | Fewer steps, less forgetting (R6, R7) |
| Optimizer steps | 1,100; ~7.2M tokens total | Small |
| Loss | Masked to the answer tail | Prompt tokens are never trained |

Net: expect **suppressed thinking-mode initiation** plus possibly modest general-reasoning drift. Complete loss of underlying capability is unlikely. Forcing a `<think>\n` prefix at inference would likely recover much of it — **untested**.

### 3.6 Minor: stale doc comment

The module doc in `training_text.rs` (both copies) says Candle QLoRA uses `plain_system_prompt_response`. The encoder actually uses the ChatML functions.

## 4. Recommendations (cheapest and highest-leverage first)

1. **Measure before changing anything.**
   - Run base Qwen3-8B and the `qwen3_8b_metal_v2` adapter on ~200-item slices of GSM8K and MATH500 in **thinking mode** (bare `assistant\n` prefix, temp 0.6 / top-p 0.95 per Qwen).
   - Record accuracy **and think-initiation rate** (share of outputs opening a non-empty `<think>`).
   - Wire these as real benches into the collateral gate, replacing the config-field "benchmarks".
   - This turns §3's "likely" into a number, and it is what the gate was built for.
2. **Fix the row format: use Qwen's own fusion format (R1).**
   - Render direct-answer rows as `<|im_start|>assistant\n<think>\n\n</think>\n\n{response}<|im_end|>`.
   - Append `/no_think` to the user turn.
   - Optionally mask the empty-think tokens from loss (R4's best variant pairs this with a KL anchor).
   - Align `eval_local_prompt.rs` to the canonical `<think>\n\n</think>\n\n`.
   - This alone moves our adaptation from the thinking mode to the non-thinking mode, where it belongs for "respond with only Vox code". It is a small change in `training_text.rs` plus eval.
3. **Delete the templated CoT generator** (`generate_cot_organic_corpus`). Fixed filler reasoning is anti-signal.
4. **If MENS should reason about Vox** (agentic, debugging, planning lanes), add *real* think rows via rejection-sampled self-distillation (R1, R8):
   - Run Qwen3-8B, or a stronger teacher, in thinking mode on Vox prompts.
   - Keep only traces whose final code passes `vox check` / tests.
   - **Do not** put the gold answer in the teacher prompt (R9).
   - Add a slice of general reasoning replay (e.g. OpenMathReasoning).
   - These rows need `qlora_ce_last_k = 0` (full-assistant loss) and a `seq_len` well above 512, or the trace is truncated or unsupervised.
   - Start around 25–50% think rows and let step 1's benches choose the ratio. 75% is not a requirement (R5, R10).
5. **Keep training light:** 1–2 epochs, preset LR (1.5e-4) or lower, rank ≤ 16 unless evals justify more (R6, R7).

**Product decision this depends on.** If MENS is only ever served in no-think mode as a Vox code emitter, thinking-mode damage has no product impact. Steps 1–3 are still worth doing, because the current format also fails to properly adapt the no-think path. If MENS is expected to reason (the hub / agent lanes), steps 1–4 are required.

## 5. Sources

- Qwen3 Technical Report, arXiv:2505.09388 — Stage 3 Thinking Mode Fusion; Table 22. <https://arxiv.org/abs/2505.09388>
- Qwen3-8B model card and `tokenizer_config.json` chat template. <https://huggingface.co/Qwen/Qwen3-8B>
- Unsloth, "Qwen3: How to Run & Fine-tune" (75/25 guidance). <https://unsloth.ai/docs/models/tutorials/qwen3-how-to-run-and-fine-tune>
- Crusoe, "Preserving the Trace: fine-tuning chain-of-thought models" (vendor blog). <https://www.crusoe.ai/resources/blog/preserving-the-trace-a-guide-to-fine-tuning-chain-of-thought-models>
- "Demystifying Hybrid Thinking", arXiv:2510.12680. <https://arxiv.org/abs/2510.12680>
- Biderman et al., "LoRA Learns Less and Forgets Less", arXiv:2405.09673. <https://arxiv.org/abs/2405.09673>
- Lin et al., ICLR 2026, arXiv:2509.20758 (learning rate and forgetting; TALR). <https://arxiv.org/abs/2509.20758>
- Yang et al., "Self-Distillation Bridges Distribution Gap in Language Model Fine-Tuning" (SDFT), ACL 2024, arXiv:2402.13669. <https://arxiv.org/abs/2402.13669>
- Chen et al. 2025, on-policy data explains why RL forgets less than SFT, arXiv:2510.18874. <https://arxiv.org/abs/2510.18874>
- Self-distillation degradation with privileged teachers, arXiv:2603.24472. <https://arxiv.org/abs/2603.24472>
- Not relied on: Chu et al. 2025, "SFT Memorizes, RL Generalizes" (arXiv:2501.17161). It is about OOD generalization on non-CoT tasks and is often mis-cited as CoT-forgetting evidence.
