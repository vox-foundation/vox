# MENS End-to-End Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Revision 2 (2026-09-12).** Revision 1 was critiqued along eight parallel tracks against the codebase. It shrank from 25 tasks to 18, four tasks became deletions, five became human-only, three of its eight sample tests were found not to compile, and **a new Lane 0 was added ahead of everything** because the inference engine is ~100× too slow and Revision 1 contained a task that would have made it worse. What changed and why is in [Appendix A](#appendix-a--what-revision-1-got-wrong).

**Goal:** Take MENS from "a locally trained model emits text into a chat bubble" to "a locally trained fine-tune answers in seconds, writes code that compiles, is reachable and startable from the GUI, and is validated by gates that can actually fail."

**Architecture:** Six lanes. **Lane 0** makes inference fast enough to evaluate at all — nothing else is measurable until it lands. **Lane A** makes the gate stack capable of failing. **Lane B** opens the agent loop to VoxLocal, the change that converts "chats" into "writes code." **Lane C** makes the journey reachable from the GUI. **Lane D** takes one spoke end to end. **Lane E** is deletions. A seventh, **Lane H**, is human-operated hardware work that blocks nothing.

**Tech Stack:** Rust (candle, qlora-rs, axum, tauri), VoxScript via `vox run`.

**Spec:** [`docs/src/architecture/mens-program-status-and-requirements-audit-2026-09-12.md`](../../src/architecture/mens-program-status-and-requirements-audit-2026-09-12.md) — the requirements inventory (R1-R6, D1-D9, M1-M4) this plan executes against.

---

## Task -1: The kill test — run this before anything else

Two checks. Neither needs training, CUDA, a mesh, or any task in this plan. **Together they may reorder or cancel it.** Run Part B today; it is one curl.

### Part B — the latency floor (15 minutes)

The Qwen3-0.6B snapshot is already cached and the M4 demo's run directories already exist ([`mens-m4-live-demo-2026-09-12.md`](../../src/architecture/mens-m4-live-demo-2026-09-12.md) §4 has the exact commands). Serve it exactly as that demo did and change **one number** — ask for 256 tokens instead of 24:

```bash
VOX_PLUGINS_DIR=$S/plugins ./target/debug/vox-ml-cli mens serve \
  --model $S/run_base --port 18112 --temperature 0 --max-tokens 256
time curl -s -X POST http://127.0.0.1:18112/v1/generate \
  -H 'content-type: application/json' \
  -d '{"prompt":"Write a Vox function that merges two sorted integer lists.","max_tokens":256,"temperature":0}'
```

**Prediction: 40-90 minutes, and it will not stop early** (see L-1c). **Kill line: if this exceeds 60 seconds, Lane 0 is mandatory and blocks everything**, because Task B1 raises the default `max_tokens` to 2048 and would make chat strictly worse until Lane 0 lands.

### Part A — the corpus ceiling (half a day, no GPU training)

Ask the question fine-tuning is a *worse* channel for: **does this corpus contain enough signal to write compiling Vox at all?** In-context learning is an upper bound on what a fine-tune of the same bytes can extract. If the corpus in the prompt does not produce compiling Vox, baking it into weights will not.

1. Build a 50-task held-out bench from the existing `humaneval-vox` problems already marked `training_eligible: false`, plus any of the current 10 whose answers are **not** in the corpus (drop `fn_add`, `fn_greet`, `query_list_items`, `component_button` — see L-4).
2. Score three arms with the **existing, already-real** `eval_local` verifier (`run_frontend_str` + the anti-stub check) — write no new eval code:
   - **Arm 1 — zero-shot base.** Qwen3-0.6B, no adapter. Expected ≈ 0.
   - **Arm 2 — corpus in-context.** A frozen 7B-14B coder through the already-wired `ProviderType::Ollama` (real KV cache, fast), with the k nearest corpus rows retrieved into the prompt.
   - **Arm 3 — control.** The same 7B-14B, zero-shot, no corpus.

| Result | Read |
|---|---|
| Arm 2 ≫ Arm 3 and Arm 2 ≥ ~0.5 | The corpus carries real signal. Proceed. Arm 3 becomes the honest beat-base number the gate currently cannot compute. |
| **Arm 2 ≈ Arm 3** | **The corpus adds nothing even handed to the model directly.** A QLoRA of the same bytes into a smaller model will do worse. **Stop and build the corpus before building any lane below.** |
| Arm 2 ≥ 0.5 but Arm 1 ≈ 0 | Signal exists, 0.6B is the wrong target. Re-scope to the smallest model that clears the bar, and re-plan the hardware around that rather than around the 4 GB T1000. |

Part A's free byproduct is the base-model baseline that Lane D's `beat_base` gate silently skips today for lack of a producer (L-3).

---

## Standing execution rules — read before your first tool call

You are one subagent executing one task. You have no prior context and no follow-up turn. **Every rule here names a failure that already happened on this plan.**

**R1 — Never end a turn waiting on a background process.**
`Bash(run_in_background: true)` sends you no notification and no monitor wakes you. If you background a command and stop, your task is dead and a human must rescue it. **Five agents did this on this plan.** Run anything you need the result of in the foreground with an explicit timeout:

```bash
cd /Users/brbrainerd/dev/vox && timeout 540s cargo test -p vox-ml-cli --features gpu,execution-api -- eval_collateral 2>&1 | tail -40
```

Use GNU `timeout` (on PATH at `/opt/homebrew/bin/timeout`) *inside* the command as well as the tool's `timeout` parameter. Exit 124 means the inner timeout fired — that is data, not a crash.

**R2 — Anything that can exceed 600s must be warmed, then measured.**
The Bash tool's ceiling is 600000 ms. A cold `cargo` build of `vox-ml-cli --features gpu` exceeds it: `gpu` transitively enables `vox-populi/{mens,mens-train,mens-hf-hub}`, `vox-tensor`, `vox-plugin-host`, and `quantize` → `vox-quantize` → the candle/gemm subtree. Two calls:

1. `timeout 590s cargo build -p vox-ml-cli --features gpu,execution-api 2>&1 | tail -5 ; echo "exit=$?"` — repeat **at most three times** until it exits 0. A repeat that makes no progress (same last-compiled crate twice) is a stop, not a retry.
2. Then run the test, now incremental.

If you cannot get a green build in three warm calls, **stop and report the blocker**. Do not vary feature flags searching for a combination that is faster.

**R3 — Hard stop: one configuration, three attempts, then report.**
Retrying an identical command after a transient failure is fine. Changing parameters and retrying as a way of *searching* for something that works is not. If attempt 3 of the same command fails, write down the exact command, the exact error, your best theory, and stop. A clear blocker report is a successful turn. **One task on this plan burned ~800k tokens retrying a GPU run with a new config after each timeout, and never succeeded.**

**R4 — Driving a long-lived process: one foreground command, never two calls.**
The shell does not persist between Bash calls. Anything needing a server up *while* a request is sent does it in a single call:

```bash
cd /Users/brbrainerd/dev/vox && timeout 540s bash -c '
  set -o pipefail
  LOG=$(mktemp)
  ./target/debug/vox-ml-cli mens serve --model "$RUN_DIR" --host 127.0.0.1 --port 11435 >"$LOG" 2>&1 &
  SRV=$!
  trap "kill $SRV 2>/dev/null" EXIT
  for i in $(seq 1 60); do
    curl -sf http://127.0.0.1:11435/health >/dev/null && break
    kill -0 $SRV 2>/dev/null || { echo "SERVER DIED"; cat "$LOG"; exit 1; }
    sleep 1
  done
  curl -sf -X POST http://127.0.0.1:11435/generate \
    -H "content-type: application/json" \
    -d "{\"prompt\":\"...\",\"max_tokens\":512}" | tee /dev/stderr
'
```

The `trap` is required — a leaked server holds 11435 and the next agent's run fails mysteriously. The `kill -0` liveness check is required — without it a crashed server is indistinguishable from a slow one and you poll 60s into a false negative. **Port is 11435. Never 11434 — Ollama usually owns it.**

**R5 — Cite nothing you have not re-read this turn.**
Every path, line number, symbol, struct field, and signature in this plan may be stale. In Revision 1, **three sample tests cited types that do not exist**, and fifteen line numbers were off by 1-8. Before using a cited symbol:

```bash
git show main:<path> | sed -n '<start>,<end>p'
```

If the plan's sample disagrees with the file, **the file wins** and you note the discrepancy in your report. Never type a plan's sample code without confirming the types it names exist with the spellings it uses. Do not put a line number in a commit message or report unless you read that line this turn.

**R6 — Read ranges, not whole files.** Files over ~400 lines get `sed -n 'A,Bp'` or grep-then-window. Per-task ranges are in each task's Execution notes. `agent_loop.rs` is 1784 lines.

**R7 — A test that cannot fail is not a test.**
Before claiming a test passes, prove it can fail: break the **production** code it covers, re-run, confirm RED, restore, confirm GREEN, then confirm `git status --porcelain` shows only your intended changes. Paste both outputs. Two anti-patterns that already shipped here: a test that constructs its expected value inside the test body instead of calling the production function; and a test asserting "the command failed" that cannot distinguish *failed closed* from *never ran*. A concurrent `rustfmt` or another agent can revert your mutation and hand you a meaningless pass — hence the before/after `git status` check.

**R8 — Your report contains artifacts, not adjectives.** "Tests pass" is not a result. Paste the last 20 lines of the test output, the `git diff --stat`, the JSON the command wrote. If a step produced a file, `cat` it. If you could not verify a step, say **NOT VERIFIED** and why — an acceptable outcome, and far better than an unbacked claim.

**R9 — One task, one scope.** Commit only files the task's **Files:** block names. Found an unrelated bug? Write it in your report; do not fix it. If a step cannot be done without touching a file outside the block, stop and report — do not expand scope and justify it in the commit message.

**R10 — Repo hygiene under concurrent agents.**
- **Never `git stash`.** The stash stack is shared across worktrees; you will pop someone else's work.
- **Never `git checkout`/`switch` branches.** Work on the branch you were given.
- `git status --porcelain` before you start. If files outside your scope are dirty, **do not `git add -A`** — stage only your named paths, by explicit path.
- Format with `vox run scripts/fmt.vox`. **Never `cargo fmt --all`.**
- A fresh worktree has its own empty `target/` (cold build, see R2) and **cannot build `vox-gui`** until you run `vox run scripts/gui-build.vox` once *and* `pnpm install && pnpm build` in `crates/vox-gui/ui`. Budget a separate warm call for each.

**R11 — Route cargo through the broker.** Plain `cargo` on PATH only. Never `~/.cargo/bin/cargo`, never `rustup run … cargo`. That is what serializes builds across agents sharing this machine.

**R12 — Do not invent a missing decision.** Where a step says "decide," "choose," or "document the story" and this plan has not pre-decided it: stop, state the options and your recommendation, end the turn. An architectural coin-flip made silently by a subagent is worse than a blocked task.

---

## Global Constraints

- Training is **Candle on CUDA and Metal only**. No second ML framework. SafeTensors on disk.
- **No new `.ps1` / `.sh` / `.py` glue.** VoxScript via `vox run`. Lane H's host bootstrap is the documented exception — it is interactive and recorded in a doc, not scripted.
- **Crypto only via `vox-crypto`.** No direct `sha2`/`blake3`/`ed25519-dalek`/`chacha20poly1305` imports outside that crate.
- **Crate-edge additions require a USER-AUTHORIZED ledger entry** in `contracts/ci/crate-edges.allow.v1.json`. Propose in the PR description; never self-author. (`vox-ml-cli` already depends on `vox-compiler` and `vox-gui` already depends on `vox-orchestrator-mcp` — neither needs one.)
- **Every new `pub fn` in `crates/*/src/**` needs a same-file test** before the commit lands (`skeleton/untested-pub-api`, `tdd-guard` pre-commit hook).
- **New docs under `docs/src/` need frontmatter** (`title`, `description`, `category`). Verify: `cargo run -p vox-doc-pipeline -- --lint-only --paths <path>`.
- **Never run `cargo fmt --all`.** Use `vox run scripts/fmt.vox`.
- `vox mens serve` has **no authentication** (`serve/mod.rs:117-124` — `Router::new()` with zero middleware). It may only ever be bound beyond loopback on a tailnet or through an SSH tunnel. **Do not add a `0.0.0.0` default anywhere.**
- Verify security-relevant guards **by mutation** (R7), not by observing green tests.
- **`patches/qlora-rs-1.0.5/` is a vendored patch tree.** Changes there are ours to make, but they must carry a comment saying why, because an upstream bump will drop them.

---

## Discoveries that reshaped this plan

Each verified against `main`. Several tasks exist *only* because of these.

### The four from Revision 1 — all confirmed true

**D-1. `blaptop04` was never blocked.** Three sessions recorded it as blocked on an operator fixing a Windows OpenSSH ACL. The real cause was the client invocation: bare `ssh blaptop04` defaults to local username `brbrainerd`, but the authorized key lives under `iacch`. `ssh iacch@blaptop04.tail4f69a0.ts.net` connects instantly. **No operator action is required.** *(Host fact; not verifiable from the repo. Its consequence is.)*

**D-2. There is reachable CUDA hardware, and it is small.** `blaptop04` carries an NVIDIA Quadro T1000, 4096 MiB. `bdesktop`/`bdesktop2` (the 4080 Supers) remain offline. 4 GB fits exactly one rung in `mens/config/gpu-specs.yaml` — `qwen3_code`'s Qwen3-0.6B at `floor_mb: 2000`; the next lowest rung anywhere is 6000. Enough to validate the CUDA lane, which has had **zero live hardware validation** across this program. Not enough to train a production spoke.

**D-3. The serve gate cannot fail.** `eval_collateral.rs:24-26` contains verbatim `// TRACKED: run inference, currently assuming pre == post for structural demo` / `let post = pre;`. The file is 85 lines with no adapter load and no test module. `dispatch.rs:354-370` refuses any adapter whose report lacks `"status": "pass"` — and every report passes.

**D-4. The agent loop is closed to VoxLocal by construction.** `agent_loop.rs:106-117` puts `ProviderType::VoxLocal` (at `:108`) in the `None` arm alongside nine other providers. `message.rs:302` `?`-propagates it out of an `Option`-returning fn, so `run_agent_turn` — the only path that calls `select_tools_for_turn` (`:531`) and matches `resp.tool_calls` (`:594`) — is never entered. **A locally served MENS model cannot write a file or run a command today.**

### The new ones

**L-1. The inference engine is ~100× too slow, from three compounding defects. This is the program-killer.** All three verified directly:

- **(a) No KV cache, and the doc comment claims there is one.** `metal/src/inference.rs:~350` says *"Autoregressive generation loop with KV cache."* The loop at `:552` is `for _ in 0..max_tokens { let input = Tensor::new(tokens.as_slice(), …)?.unsqueeze(0)?; … model.forward(&input)? }` — **the full context is re-forwarded every step, O(N²)**. The machinery exists and is deliberately not passed: `Qwen2Attention::forward` fully implements `kv_cache: Option<&mut (Tensor, Tensor)>` and `Qwen35LayerCache` is defined, but `Qwen35Model::forward` calls every layer with position 0 and cache `None`.
- **(b) NF4 dequantization runs on the CPU, every weight, every forward pass — on Metal only.** `patches/qlora-rs-1.0.5/src/quantization.rs:451`: `if device.is_cuda() && quantized.zero_points.is_none() { … dequantize_nf4_gpu … }` then falls through to `dequantize_nf4_cpu`. The vectorized path is gated on **`is_cuda()`**, so Metal always takes a scalar loop with a GPU→CPU→GPU roundtrip. The escape hatch exists — `QLoraConfig::cache_dequantized`, with an unused inference preset at `qlora.rs:196` setting it `true` — and **the Metal plugin contains zero references to it**, while the CUDA plugin sets it `true` at `model.rs:809`. The asymmetry is the bug.
- **(c) The EOS token is wrong, so nothing ever stops early.** The loop breaks on `next_token == 151643`, commented *"EOS token for Qwen2 family."* The cached `config.json` says `bos_token_id: 151643`, **`eos_token_id: 151645`**. It breaks on BOS. Every generation runs the full `max_tokens`. The M4 demo capped at 24 and never noticed.

Calibrating from the M4 demo's own measured ≈0.128 s per token-position: a 500-token reply with a 200-token prompt is **~8 hours**. **Revision 1's Task 1.1 raised the default `max_tokens` from 256 to 2048 and presented it as a bug fix** — against (a)+(b)+(c) that is ~4.6 days per reply. Even fixing only the quadratic leaves a ~0.65 tok/s linear floor that cannot terminate early: **52 minutes for one 2048-token reply.**

**L-2. Four more gates cannot fail, and Revision 1's Phase 0 touched none of them.**
- `eval_gate` → `rust_compile_rate` / `clippy_clean_rate`: a missing key yields `(true, "not applicable (no rust_authoring rows)")`. **A run with no `eval_results.json` passes a `block: true` gate.**
- `eval_gate` → `eval_local`: guarded by `&& let Some(ref eval) = eval_json` — no file means the gate is never emitted at all.
- pass@k regression and bfcl beat-base: both skip silently when their baseline file is absent.
- `vox ci mens-gate`: every step is `cargo test -p X <substring>`. **`cargo test` with a filter matching zero tests exits 0.** Rename or delete a test and the step silently becomes a no-op.
- `run_train.rs:300` wraps the `MIN_CORPUS_PAIRS` check in `if train_jsonl.exists() { … }` — **a missing `train.jsonl` skips the corpus-size gate entirely, silently.**

**L-3. Two gate policies read files that have no producer.** `eval-gates-rust.yaml` gates on `rust_compile_rate` / `clippy_clean_rate` from `eval_results.json`; `eval-gates-agents.yaml` on `tool_call_valid_json_rate`; `beat_base` reads `base_eval_dir/bfcl_results.json`. **Every reference to `eval_results.json` and `bfcl_results.json` outside tests is a consumer.** `eval_local.rs:370-398` writes a *different* schema (`pass_rate_at_1`, `category_stats`, …). `capture_baseline` is correctly fail-closed and `beat_base`'s Wilson CIs are unit-tested — they simply never run. This is D-3's defect class in a second place.

**L-4. The held-out bench is 10 tasks, and 4 of its answers are in the training corpus.** `mens/data/heldout_bench/manifest.json` has 10 entries. Their `semantic_expected_contains` strings occur in the 8,058-row training corpus: `fn add` ×2, `fn greet` ×3, `query list_items` ×1, `component Button` ×2. There is no `split_manifest.json` anywhere and no split step in the pipeline. `eval_gate/leakage.rs` opens with `#![allow(dead_code)] // eval-gate helpers, not yet wired` and `assert_no_leakage` has **zero non-test callers** — and it compares tool names by 3-gram Jaccard, so it would not catch text overlap anyway. pass@1 on n=10 has a ±30-point confidence interval; it cannot distinguish a 40% model from a 70% one.
**Revision 1 stepped on this rake: its e2e demo prompt was "Write a Vox function named `add`"** — bench task `fn_add`, whose answer is in the training data. It would have demoed a memorized answer as proof the model writes code.

**L-5. The corpus teaches retrieval, and 3.9% of it teaches code the compiler rejects.** 1,733 rows (not the "~20K" claimed in `training_contract.yaml`), 1,471 scraped from `examples/golden/*.vox` with reverse-engineered captions as prompts (`"Write Vox code as shown in … documentation"`, `"- **Usage**"`). 67 rows contain retired syntax that is now a **hard parse error** — 96 `@query`, 96 `@mutation`, 50 `@component` occurrences; the corpus was built 2026-09-05, those spellings became errors 2026-06-30 (`cd7cc96874`). Nothing checks for this. The repo's one measured failure taxonomy says the hard class is `Option[int]` indexing semantics — a control-flow restructuring a 0.6B model will not learn from 1,733 examples.

**L-6. The GUI cannot start a server, and probably cannot run the MENS cards it already ships.**
- `execute_command` calls `.output().await`, which **waits for process exit**. `vox mens serve` never exits — a serve card would hang its promise forever and parent an untracked server to the GUI with no stop control.
- `vox mens` execs `Command::new("vox-ml-cli")` from `PATH`, but `tauri.conf.json` bundles only `vox` as an `externalBin`. A `.app` launched from Finder inherits no dev-shell `PATH`.
- `vox-ml-cli`'s `default = ["mens-base"]`. `serve` is behind `execution-api`; `models` and `eval-*` are behind `gpu`. **Neither is default**, so a stock install has no `mens models` subcommand at all.
- `isModelSelectable()` computes *why* a local model is uncallable, and `filterPickerModels` then **silently drops the row**. Drive's API 409s with a reason; the click path throws it away.

**L-7. Metal↔CUDA inference drift has reversed since the M4 demo.** Metal now lacks `resolve_inference_device` (CUDA has it) and inlines a bound-and-discarded `let _device = … .unwrap_or(Device::Cpu)` — **silent CPU fallback with no warning**, where CUDA's version logs and errors on an explicit device kind. This is the exact class that produced the 9×-slow Metal mis-measurement. Metal's file still carries the comment *"Kept byte-for-byte in sync."* 22 of 44 shared plugin files are byte-identical; 22 have diverged. `model_card.rs` and `manifest.rs` each exist in **three** copies (vox-populi + both plugins).

**L-8. `vox mens serve` defaults to Ollama's port.** `inference_defaults.rs:8` — `DEFAULT_INFERENCE_PORT: u16 = 11434`, which is `LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT`'s port. With Ollama running, serve fails to bind; without it, MENS squats the port and Vox's own Ollama resolution dials the MENS server and 404s on `/api/embed`. Revision 1 knew to use 11435 — **but only in a GUI card's argv**, leaving every non-GUI path colliding.

**L-9. Corpus data is gitignored.** `mens/data/**/*.jsonl` is covered by `.gitignore` and `target/` is per-worktree. **A fresh worktree has no corpus.** All Lane D data work runs in the primary checkout.

---

# Lane 0 — Make inference fast enough to evaluate

**Blocks everything.** Until this lands, no quality measurement is affordable and Task B1 is actively harmful. Three defects, one task, because they compound and their fix shares a test.

### Task Z1: KV cache, Metal dequant cache, and the right EOS token

**Files:**
- Modify: `crates/vox-plugin-mens-candle-metal/src/inference.rs` (the generation loop and the model construction)
- Modify: `crates/vox-plugin-mens-candle-metal/src/model.rs` (`Qwen35Model::forward` — thread the existing cache)
- Modify: `crates/vox-plugin-mens-candle-metal/src/inference.rs` (read `eos_token_id` from `config.json`)
- Test: same-file test modules

**Interfaces:**
- Consumes: `Qwen2Attention::forward(&self, x, pos, kv_cache: Option<&mut (Tensor, Tensor)>)` and `Qwen35LayerCache` — **both already implemented**. This task passes them; it does not write them.
- Consumes: `QLoraConfig::cache_dequantized` (`patches/qlora-rs-1.0.5/src/qlora.rs:99`), default `false`, with an inference preset at `:196` already setting it `true`.
- Produces: a throughput floor the serve gate can enforce.

> **Execution notes.**
> **Do not write a KV cache.** It exists. `Qwen35Model::forward` currently calls every layer with position 0 and cache `None`; the work is threading the cache through and feeding only the new token after the prefill. Read `model.rs` around `Qwen2Attention::forward` and `Qwen35Model::forward` before touching the loop.
> **Do not write a GPU dequant kernel.** Prefer setting `cache_dequantized = true` on the *inference* config — one line, and the preset at `qlora.rs:196` shows the intended shape. Extending `dequantize_nf4_gpu` past the `is_cuda()` gate is the larger, riskier alternative; if you believe caching is insufficient, **stop and report** rather than opening that up (R12). Any edit under `patches/` carries a comment saying why, because an upstream bump drops it.
> **The EOS fix is three lines and must not be hardcoded again.** Read `eos_token_id` from the model's `config.json`. `151643` is BOS. Note that Qwen3 configs can carry a *list* of EOS ids — handle both shapes.
> Metal GPU work: hold `/tmp/vox-gpu.lock`.

- [ ] **Step 1: Write the failing test — throughput, as a number**

```rust
#[test]
fn generation_sustains_a_usable_token_rate() {
    // 0.6B on Metal. The pre-fix engine does ~0.06 tok/s at this length;
    // the floor below is deliberately far under what a cached engine gives,
    // so this asserts "not quadratic", not "fast".
    let engine = load_demo_engine();
    let t0 = std::time::Instant::now();
    let out = engine.generate("Write a Vox function that adds two ints.", 128, 0.0).unwrap();
    let rate = 128.0 / t0.elapsed().as_secs_f64();
    assert!(rate >= 15.0, "only {rate:.2} tok/s — the KV cache or the dequant cache is not engaged");
    assert!(!out.is_empty());
}

#[test]
fn generation_stops_at_the_real_eos_token() {
    // 151643 is BOS. The real EOS is 151645 and must come from config.json.
    let cfg = load_demo_config();
    assert_eq!(eos_token_ids(&cfg), vec![151645],
               "EOS must be read from config.json, not hardcoded");
}
```

- [ ] **Step 2: Run to verify both fail.** Expected: the rate assertion fails at ~0.06 tok/s; `eos_token_ids` not found. **Paste the measured rate** — it is the before-number for the report.
- [ ] **Step 3: Thread the existing KV cache** through `Qwen35Model::forward`; prefill the prompt once, then feed one token per step at the correct position.
- [ ] **Step 4: Enable `cache_dequantized` on the inference config.**
- [ ] **Step 5: Read `eos_token_id` from `config.json`** (handling the scalar and list shapes) and break on any of them.
- [ ] **Step 6: Run to verify both pass. Paste the measured rate.** Report the before/after as the headline number.
- [ ] **Step 7: Correct the lying comment.** `inference.rs:~350` claims a KV cache that was not there. Make it describe what the code now does.
- [ ] **Step 8: Mutation-verify (R7).** Revert the cache threading, confirm the rate assertion fails; restore. Revert the EOS read to `151643`, confirm the second test fails; restore. Paste all four outputs.
- [ ] **Step 9: Re-run the kill test's Part B** (256 tokens, one curl, R4) and paste the wall clock. **If it is still over 60s, stop and report** — the remaining bottleneck is something this task did not find, and that is the finding.
- [ ] **Step 10: Commit.**

```bash
git commit -m "fix(mens-metal): engage the KV cache, cache dequant weights, stop at real EOS

Three compounding defects made a 13-token reply take 20s and a 500-token
reply take hours. The generation loop re-forwarded the full context every
step despite Qwen2Attention implementing kv_cache; NF4 dequant fell to a
CPU scalar loop because the fast path is gated on is_cuda(); and the loop
broke on 151643, which is BOS, so nothing ever stopped early."
```

---

### Task Z2: Make the throughput floor a gate

**Files:** Modify `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (serve preconditions); `scripts/populi/gates.yaml`

> **Execution notes.** Without a number attached, "faster" gets deferred forever. Z1 proves the engine *can* hit the floor; this makes a regression visible. Serialize after Z1 and after Task A2 (same file).

- [ ] **Step 1: Write the failing test** — a run whose recorded `tokens_per_second` is below the floor is refused.
- [ ] **Step 2: Verify it fails.** — [ ] **Step 3: Implement.** — [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Mutation-verify.** — [ ] **Step 6: Commit.**

---

# Lane A — Make the gates able to fail

Nothing downstream is trustworthy until this lands. Four tasks, all CPU-only.

### Task A1: Make the collateral-damage eval actually run inference

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (85 lines — read whole)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` **line 358** (the wrong remediation string; the gate block is `:351-370`)
- Test: same-file `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `vox_eval::eval_collateral_damage_suite(scores: &[(&str, f64, f64)], config: &CollateralDamageConfig) -> Result<Vec<CollateralDamageReport>, CollateralDamageReport>` (`vox-eval/src/lib.rs:315-327`) — already implemented and unit-tested (`large_degradation_fails`, `improvement_never_fails`, `suite_returns_err_on_first_failure`, `suite_passes_when_all_ok`). **Do not modify it and do not re-test its arithmetic.**
- Produces: `run_collateral_damage_with(...) -> anyhow::Result<i32>`, writing `collateral_damage_report.json`.

> **Execution notes.**
> **Revision 1's sample code for this task did not compile.** It cited a `vox_eval::BenchRef` type, a `report.status` field, and a `report.degradations` collection. **None exist.** `CollateralDamageReport` (`lib.rs:251`) has `benchmark_name / pre_training_score / post_training_score / degradation / degradation_rate / exceeds_threshold`, and failure is `Err(first_failing_report)`. The `"status": "pass"` string the serve gate reads is built in an ad-hoc `json!` literal — an **envelope key, not a type**.
> **`run_collateral_damage` calls `std::process::exit(1)` on the fail branch.** Any test driving the real function down that path kills the test binary. Lifting that `exit` to the CLI boundary is part of this task.
> **Define the pre-score file format here.** `eval_collateral.rs:21-27` iterates `pre_json.as_object()` and treats **every f64-valued key** as a benchmark name. `eval_local.rs:370-398` writes `{model, bench, max_tokens, temperature, k, seed_base, total, passed_at_1, pass_rate_at_1, category_stats, results}`. Feeding one to the other fabricates "benchmarks" named `max_tokens`, `temperature`, `k`, `seed_base`. Either normalize on read or require a named sub-object — **decide and state which in your report**, then implement it.
> Build `--features gpu,execution-api`, warmed per R2.
> **Do not write a pure `build_report_from_scores(pre, post)` function.** A function *handed* `post` cannot observe where `post` came from — and "`post` came from `pre`" is the entire bug. The seam must be the scorer.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_degraded_adapter_writes_a_failing_report() {
    let dir = tempfile::tempdir().unwrap();
    let pre = dir.path().join("pre.json");
    std::fs::write(&pre, r#"{"general_bench":0.85,"code_bench":0.90}"#).unwrap();

    let mut asked: Vec<String> = Vec::new();
    let mut scorer = |bench: &str| {
        asked.push(bench.to_string());
        Ok(match bench { "general_bench" => 0.40, _ => 0.89 })
    };
    let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();

    assert_eq!(code, 1, "a 53% collapse must not pass");
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("collateral_damage_report.json")).unwrap(),
    ).unwrap();
    assert_eq!(report["status"], "fail");
    assert_eq!(report["failed_on"], "general_bench", "operator must know what broke");
    // THE assertion that catches "never loaded the adapter". Exact sequence, not `>= 1`:
    // deleting a bench or short-circuiting the loop fails it.
    assert_eq!(asked, ["general_bench", "code_bench"],
               "every bench in the baseline must be re-scored against the adapter");
}

#[test]
fn an_unloadable_adapter_fails_loudly_and_writes_no_passing_report() {
    let dir = tempfile::tempdir().unwrap();
    let pre = dir.path().join("pre.json");
    std::fs::write(&pre, r#"{"general_bench":0.85}"#).unwrap();
    let mut scorer = |_: &str| anyhow::bail!("adapter not found");
    assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
    assert!(!dir.path().join("collateral_damage_report.json").exists(),
            "a load failure must never leave a report the serve gate would accept");
}
```

- [ ] **Step 2: Run to verify it fails.** `timeout 540s cargo test -p vox-ml-cli --features gpu,execution-api -- eval_collateral 2>&1 | tail -40`

- [ ] **Step 3: Implement**

```rust
/// Scores every benchmark in the baseline against the adapter and writes the
/// envelope the serve gate reads. The scorer is a parameter so tests can drive
/// a degradation without a GPU — and so `post` can never be `pre`.
pub(crate) fn run_collateral_damage_with(
    pre_score_path: &Path,
    run_dir: &Path,
    score_bench: &mut dyn FnMut(&str) -> anyhow::Result<f64>,
) -> anyhow::Result<i32> {
    let pre: BTreeMap<String, f64> =
        serde_json::from_str(&std::fs::read_to_string(pre_score_path)?)?;
    let mut scores: Vec<(String, f64, f64)> = Vec::new();
    for (bench, pre_score) in &pre {
        let post = score_bench(bench)?;          // fails loudly; no unwrap_or(pre)
        scores.push((bench.clone(), *pre_score, post));
    }
    let borrowed: Vec<(&str, f64, f64)> =
        scores.iter().map(|(n, a, b)| (n.as_str(), *a, *b)).collect();
    let cfg = CollateralDamageConfig::default();
    let (status, failed_on, reports) = match eval_collateral_damage_suite(&borrowed, &cfg) {
        Ok(rs) => ("pass", None, rs),
        Err(bad) => ("fail", Some(bad.benchmark_name.clone()), vec![bad]),
    };
    std::fs::write(
        run_dir.join("collateral_damage_report.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "status": status, "failed_on": failed_on, "reports": reports,
        }))?,
    )?;
    Ok(if status == "pass" { 0 } else { 1 })
}
```

The CLI arm builds the real scorer from the same `MlBackend` plugin dispatch `vox mens eval-local` uses, then `std::process::exit(code)` — **the `exit` lives there, not in the core.**

- [ ] **Step 4: Run to verify it passes.**
- [ ] **Step 5: Fix the remediation string at `dispatch.rs:358`.** It tells the operator to run the command that produced the stub. Read `sed -n '340,380p'` first; quote before/after in your report. **The flag is `--post-adapter`** — `mens_tail_subcommands.rs:52-53` has `#[arg(long, id = "post")]` on a field named `post_adapter`, and clap processes attrs in source order, so `long` resolves against the field name and `id` only renames the arg id. **Note in your report that swapping those two attributes would silently change the flag to `--post`** — it is a latent trap, not something to fix here (R9). Correct references: `scripts/mens-macos-metal-e2e.vox:130`, `docs/src/reference/how-to-train-mens-macos-metal.md:64`.
- [ ] **Step 6: Mutation-verify (R7).** Restore `let post = pre;`, confirm the first test FAILS, restore, confirm GREEN, confirm `git status --porcelain` is clean of strays. Then try the subtler regression that will actually happen later — `let post = scorer(b).unwrap_or(pre_score);` — and confirm the second test catches it. **Paste all outputs.**
- [ ] **Step 7: Commit.**

---

### Task A2: Refuse to serve a model whose gates all silently skipped

**Files:** Modify `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (the existing gate block, `:351-370`); test in the same file

> **Execution notes.** **The highest-leverage change in the plan, and Revision 1 did not contain it.** Per L-2, `eval_gate` has four branches that turn a missing artifact into a *passing* gate. Rather than edit all four (a large diff across `check_run.rs`, easy to regress), **count the substantive gates at the serve boundary** — ~15 lines, one file, converting every present *and future* silent-skip into a visible refusal.
> **Serialize after A1** (same file), **before Z2** (same file).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_receipt_whose_every_gate_skipped_is_not_evidence() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("gate_receipt.json"), r#"{
      "overall_passed": true,
      "gates": [
        {"name":"rust_compile_rate","passed":true,"message":"not applicable (no rust_authoring rows)"},
        {"name":"pass_at_k","passed":true,"message":"baseline file missing (skipped regression check)"}
      ]}"#).unwrap();
    let err = assert_serve_preconditions(dir.path()).unwrap_err().to_string();
    assert!(err.contains("0 substantive gates"), "got: {err}");
}

#[test]
fn a_receipt_with_two_real_gates_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("gate_receipt.json"), r#"{
      "overall_passed": true,
      "gates": [
        {"name":"throughput","passed":true,"message":"142 tok/s >= 100"},
        {"name":"supervised_ratio","passed":true,"message":"0.71 >= 0.60"}
      ]}"#).unwrap();
    assert!(assert_serve_preconditions(dir.path()).is_ok());
}
```

- [ ] **Step 2: Run to verify it fails.**
- [ ] **Step 3: Implement, inside the existing `if manifest_path.exists() || adapter_path.exists()` block**

```rust
let receipt_path = run_dir.join("gate_receipt.json");
let receipt: serde_json::Value = serde_json::from_str(
    &std::fs::read_to_string(&receipt_path).with_context(|| format!(
        "no gate_receipt.json in {} — run `vox mens eval-gate --run-dir {} --policy <p>` first",
        run_dir.display(), run_dir.display()))?)?;
anyhow::ensure!(receipt.get("overall_passed") == Some(&serde_json::Value::Bool(true)),
    "eval-gate did not pass for this run; refusing to serve");
// A receipt whose gates all said "not applicable"/"skipped"/"missing" is not evidence.
let substantive = receipt["gates"].as_array().map_or(0, |g| g.iter().filter(|r| {
    let m = r["message"].as_str().unwrap_or("");
    !m.contains("not applicable") && !m.contains("skipped") && !m.contains("missing")
}).count());
anyhow::ensure!(substantive >= 2,
    "gate_receipt.json has {substantive} substantive gates — every check was skipped for a \
     missing artifact. Produce eval_results.json and baseline_report.json, then re-gate.");
```

- [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Mutation-verify** (`>= 2` → `>= 0`, confirm the first test fails, restore; paste both). — [ ] **Step 6: Commit.**

---

### Task A3: Give the gate policies a producer

**Files:** Modify `crates/vox-ml-cli/src/commands/mens/eval_local.rs`; `crates/vox-ml-cli/src/commands/mens/eval_gate/check_run.rs`

> **Execution notes.** Per L-3, `eval-gates-rust.yaml` and `eval-gates-agents.yaml` gate on keys in `eval_results.json`, and `beat_base` reads `bfcl_results.json`. **Neither file is written by anything in this repo.** So the two policies Lane D depends on read files nobody produces — D-3's defect class in a second place, which Revision 1 did not find.
> Three parts: (a) make `eval_local` emit `eval_results.json` with the keys the policies actually gate on (`rust_compile_rate`, `clippy_clean_rate`, `tool_call_valid_json_rate`); (b) add a `--base` / no-adapter mode so the **same command** produces both sides of the comparison; (c) make a **missing baseline a hard failure**, not a skip.
> `eval_local.rs` is the one genuinely real instrument in the program — `verify_completion` runs `run_frontend_str` (real parse + typecheck) plus an anti-stub check. **Build on it; do not write a second verifier.**

- [ ] **Step 1: Write the failing test** — a policy gating on `rust_compile_rate` fails when the run's candidate does not compile, and **fails** (not skips) when the baseline is absent.
- [ ] **Step 2: Verify it fails.** — [ ] **Step 3: Implement (a), (b), (c).** — [ ] **Step 4: Verify it passes.**
- [ ] **Step 5: Mutation-verify the way A1 does** — hand the gate a baseline the candidate loses to, **confirm the gate fails**, restore. Paste both.
- [ ] **Step 6: Commit.**

---

### Task A4: Make the gate runner able to notice a deleted test

**Files:** Modify `crates/vox-cli/src/commands/ci/.../matrix.rs` (`run_mens_gate_steps` — grep for it); `crates/vox-cli/src/commands/ci/run_body_helpers/guards.rs` (~45-48); `scripts/populi/gates.yaml`; `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:300`

> **Execution notes.** Four independent silent-pass bugs, one small task.
> **(a)** `cargo test -p X <substring>` with a filter matching zero tests **exits 0**. Every step in `gates.yaml` is that shape. Switch to `cargo nextest run … --no-tests=fail`.
> **(b)** `guards.rs` downgrades any `spoke-check` violation mentioning `eval-gates-rust.yaml` or `eval-gates-agents.yaml` to a warning as "known-pending." **Both files now exist**, so the carve-out only hides real errors — in the two spoke gates Lane D introduces. Delete it.
> **(c)** `run_train.rs:300` wraps the `MIN_CORPUS_PAIRS` check in `if train_jsonl.exists()`, so a **missing** `train.jsonl` skips the corpus-size gate entirely and silently. Make absence a hard error.
> **(d)** Add Lane 0 / A / B's tests to `ci_full` — all CPU-only thanks to the injected-scorer and worker-channel seams.

- [ ] **Step 1:** Demonstrate (a): run `cargo test -p vox-ml-cli -- definitely_no_such_test_xyz; echo $?` and paste the `0`.
- [ ] **Step 2:** Fix (a), (b), (c); add (d).
- [ ] **Step 3: Mutation-verify.** Rename one test the gate names, run the gate, **confirm it now fails**, restore. Delete `train.jsonl` from a fixture run, confirm the corpus gate now errors. Paste all four.
- [ ] **Step 4: Commit.**

---

# Lane B — Make a local model write code

### Task B0: PRE-FLIGHT — can a 0.6B fine-tune emit a parseable tool call? *(gates B2)*

> **Execution notes.** The single riskiest assumption in the plan, and unproven. If a Qwen3-0.6B-class model cannot reliably emit well-formed tool-call JSON, then B2's dispatch assertion and any future `tool-selection` spoke rest on nothing. **This is cheaper than discovering it in B2's Step 7.** A spike: its output is an answer, not code.
> **Run after Z1** — before it, 10 prompts is hours.
> **Do not use a prompt from the held-out bench** (L-4).

- [ ] **Step 1:** Stand up the M4 demo run directory per R4.
- [ ] **Step 2:** Send 20 prompts in the OpenAI tool-calling shape, greedy, each requiring one obvious tool call. Twenty, not ten — this is a **rate**, and it becomes `tool_call_valid_json_rate`.
- [ ] **Step 3:** Count how many replies contain JSON that parses into `{name, arguments}` **and** name a tool that exists. Record the raw replies verbatim.
- [ ] **Step 4:** Write the finding into `mens-m4-live-demo-2026-09-12.md` as a new section.
- [ ] **Step 5: RULING GATE.** ≥0.7 → proceed to B2 as written. 0.3-0.7 → proceed, but B2 must add a tolerant extraction shim **and surface every salvage to the user as a repair**. **≤0.3 → STOP and report**: B2's route is still correct (it is the only way to send tool schemas at all), but its assertion and any tool-selection spoke must be re-scoped against a larger base. **Do not silently downgrade the assertion.**

---

### Task B1: Send the real `max_tokens` and stop dropping the caller's system prompt

**Files:** Modify `crates/vox-orchestrator-mcp/src/llm_bridge/provider_adapter.rs` (453 lines — readable whole); test in the same file

> **Execution notes.**
> **This task is blocked on Lane 0.** Raising the effective `max_tokens` against a quadratic, non-terminating loop makes chat worse, not better (L-1). Do not dispatch it before Z1 is merged and its throughput assertion is green.
> **Revision 1's sample test did not compile.** The real type is `InferRequest<'a> { system_prompt: &'a str, user_prompt: vox_openai::ChatMessageContent<'a>, max_t: u64, temperature: Option<f32>, top_p: Option<f32>, json_mode: bool, tools: Option<Value>, tool_choice: Option<Value> }` (`:27-36`) — lifetime-parameterized, **no `Default` impl**, and every field name in Revision 1 was wrong. **Use the existing `make_infer_request(...)` helper**; extend it rather than writing a struct literal. `extract_prompt_text` takes `&ChatMessageContent<'_>`, not `&InferRequest`. `VoxLocalGenerateRequest.model` is `Option<String>`.
> **The server already injects its own system prompt.** `serve/mod.rs:105-108` falls back to `vox_corpus::training::generate_training_system_prompt()`. So the model *is* steered — just not by the caller. **Revision 1 said "the system prompt is dropped" and would have shipped a fix that stacks two system prompts.** Decide the precedence (caller wins, or caller is appended), state it in your report, and test it.
> The rest of the intent is verified: `VoxLocalGenerateRequest` is `{prompt, validate, max_retries, model}` (`:232-239`); `GenerateRequest` (`serve/schema.rs:8-28`) has `max_tokens: usize` defaulting to **256** (`:31-33`), and **no `validate` field**. Removing `validate` is a genuine no-op on the wire. The catalog advertises `max_tokens: 8192` (`catalog.rs:681`).

- [ ] **Step 1: Write the failing test.** Do **not** assert `v.get("validate").is_none()` — a tautology about your own struct. The load-bearing assertion is the round-trip: the adapter's body must deserialize as the **server's** `GenerateRequest`. That is the only thing in the repo that fails if the two structs drift again.

```rust
#[tokio::test]
async fn the_server_honors_max_tokens_and_exactly_one_system_prompt() {
    let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(1);
    let seen = std::thread::spawn(move || {
        let ir = rx.recv().expect("handler must reach the worker");
        let snapshot = (ir.prompt.clone(), ir.max_tokens, ir.temperature);
        let _ = ir.reply.send(Ok("fn add(a: int, b: int) to int { a + b }".into()));
        snapshot
    });
    let state = AppState { tx, model_name: "mens/demo".into(),
                           ready: Arc::new(AtomicBool::new(true)) };
    let body = build_vox_local_request(&make_infer_request(/* … */), "mens/demo");
    let req: GenerateRequest = serde_json::from_value(serde_json::to_value(&body).unwrap())
        .expect("the adapter's body must deserialize as the server's GenerateRequest");
    let (status, Json(resp)) = do_generate(State(state), Json(req)).await;

    let (prompt, max_tokens, temperature) = seen.join().unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(max_tokens, 4096, "the 256 default must not reach the worker");
    assert_eq!(temperature, 0.2);
    assert!(prompt.contains("You are a Vox expert."), "caller's system prompt must reach the model");
    assert_eq!(prompt.matches("You are").count(), 1,
               "the server injects its own system prompt — do not stack two");
    assert_eq!(resp.model, "mens/demo");
}
```

- [ ] **Step 2: Verify it fails.** `--features gpu,execution-api` (the schema types are `#[cfg(feature = "execution-api")]`; without it the code under test does not compile).
- [ ] **Step 3: Implement** `build_vox_local_request`, **and call it from `VoxLocalAdapter::infer`** (`:279-284`). A builder the adapter does not use is theater — the defeating mutation is to add the function and leave `infer` building its body inline.
- [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Mutation-verify** (delete the call site in `infer`, confirm failure, restore; paste both). — [ ] **Step 6: Commit.**

---

### Task B2: Open the agent loop to VoxLocal *(Route A — decided, not delegated)*

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (1784 lines — read only `sed -n '55,125p'`, `'520,610p'`, `'1375,1400p'`)
- Modify: `crates/vox-ml-cli/src/commands/ai/serve/{mod.rs,handlers.rs,schema.rs}`
- Modify: `contracts/orchestration/providers.v1.yaml` (the `VoxLocal` row)
- Test: `agent_loop.rs` test module

> **Execution notes.**
> **Route A is decided. Route B is deleted.** Revision 1 delegated the choice with a stated preference; under subagent-driven development that is a coin flip, and B3 and Lane D both depend on the answer.
> **Delegate, do not duplicate.** `serve/mod.rs:117-124` already fans **three** routes (`/v1/generate`, `/generate`, `/v1/completions`) onto one `do_generate`. Add a fourth with a thin adapter that flattens `messages[]` into the existing prompt and re-wraps `GenerateResponse` into `choices[].message`, delegating to the same worker channel. **Non-goal: a second sampling/validation/retry path.**
> **`contracts/orchestration/providers.v1.yaml` is codegen input**, not documentation — `vox-orchestrator-types/build.rs` generates Rust from it and `data_ssot_guards.rs` gates it. Its `VoxLocal` row asserts `supports_openai_compat: false` and the comment *"Speaks OpenAI v1/completions (text-prompt) — NOT v1/chat/completions (messages)."* **This task makes both false.** Flip the flag, rewrite the description, run `vox ci ssot-drift`, **all in the same commit**. Revision 1 never mentioned this file.
> **Resolve the base URL through the config SSOT**, mirroring the Ollama arm above it, which deliberately uses `vox_config::inference::local_ollama_populi_base_url` with a comment on why an unset env must not disable tool-calling. Use `vox_config::inference::vox_local_endpoint_probe_candidates()`. **Do not inline a URL** (see Task E3, and note `VOX_LOCAL_ENDPOINT` *replaces* the candidate list entirely — `vox-config/src/inference.rs:187-193`).
> The helper is `fn model_spec(provider_type: ProviderType, id: &str)` at `agent_loop.rs:1383`. Revision 1 cited `:1375` and swapped the arguments.
> **New silent failure this route creates:** the server must *emit* `choices[].message.tool_calls`. A small model will emit prose about the tool, or near-miss JSON. If parsing fails, `resp.tool_calls` is empty, the loop takes the text branch, and the user sees a fluent reply that did nothing — **indistinguishable from a model that chose not to call a tool.** Per B0's ruling: either salvage a tool-shaped blob **and surface it as a repair**, or fail the turn loudly. Silent degradation to "chatted about the tool" is the worst option and is the default you get by doing nothing.
> **Serialize after B1 and after B0's ruling; before B3.** Owns `serve/{handlers,schema,mod}.rs` exclusively.

- [ ] **Step 1: Write the failing test.** `.is_some()` is the weakest possible assertion — the minimal change satisfying it is `VoxLocal => Some(LlmConfig::default())`, which makes the turn proceed and then fail against a nonexistent base URL, **worse** than today's clean fallback. Assert the wire:

```rust
#[tokio::test]
async fn a_mens_turn_dispatches_a_tool_and_feeds_the_result_back() {
    // Fake /v1/chat/completions: turn 1 requests a tool, turn 2 answers.
    let server = httpmock::MockServer::start_async().await;
    let calls = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    /* mount a handler that records each request body and replies with
       choices[0].message.tool_calls on the first call, plain content on the second */

    let spec = model_spec(ProviderType::VoxLocal, "mens/demo-run");
    let cfg = model_spec_to_llm_config(&spec).expect("VoxLocal must map");
    assert!(cfg.base_url.as_deref().is_some_and(|u| u.ends_with("/v1/chat/completions")),
            "the arm must point at chat-completions, not /generate: {:?}", cfg.base_url);
    assert_eq!(cfg.max_tokens, Some(spec.max_tokens), "the catalog's claim must reach the wire");

    let out = run_agent_turn(/* … */).await.unwrap();

    let bodies = calls.lock().unwrap();
    assert_eq!(bodies.len(), 2, "a tool call must produce a second round-trip");
    assert!(bodies[0]["tools"].as_array().is_some_and(|t| !t.is_empty()),
            "tool schemas must be sent, or the model can never request one");
    assert_eq!(bodies[1]["messages"].as_array().unwrap().last().unwrap()["role"], "tool",
               "the tool RESULT must be fed back — this is what is_some() cannot see");
    assert_eq!(out.tool_calls_dispatched, 1);
}
```

`bodies.len() == 2` is exact, not `>= 1`: a regression that sends schemas but drops the result-feedback leg fails it.

- [ ] **Step 2: Verify it fails.**
- [ ] **Step 3: Add the `/v1/chat/completions` route**, delegating to `do_generate`. Shapes into `schema.rs` behind `execution-api`.
- [ ] **Step 4: Add the `ProviderType::VoxLocal` arm**, base URL from `vox-config`.
- [ ] **Step 5: Handle the empty-`tool_calls` case per B0's ruling** — salvage-and-surface, or fail loudly. Never silent.
- [ ] **Step 6: Flip `providers.v1.yaml`** and run `cargo run -p vox-cli -- ci ssot-drift`.
- [ ] **Step 7: Verify it passes.**
- [ ] **Step 8: Emit the rate.** Write `tool_call_valid_json_rate` and `tool_name_exists_rate` into `eval_results.json` (A3 created the producer) so `eval-gates-agents.yaml` can gate on the metric it already names.
- [ ] **Step 9: Mutation-verify.** Drop the tool-result feedback leg, confirm the `bodies[1]` assertion fails, restore. Paste both.
- [ ] **Step 10: Commit** — one commit, all files: the contract flip and the route must never be separable.

---

### Task B3: Make the e2e script unable to pass on a lie

**Files:** Modify `scripts/axis-drive-metal-e2e.vox`

> **Execution notes.** The existing openrouter-cost check (`:158-163`) is good because it is a negative control on a specific failure mode. Its weakness is the general weakness of absence assertions: **"nothing ran" also satisfies it.**
> **Change the demo prompt first.** Revision 1 used "Write a Vox function named `add`" — that is held-out bench task `fn_add`, and `fn add` appears **twice in the training corpus** (L-4). It would demo a memorized answer as proof the model writes code. Pick something provably absent; grep the corpus to prove it and paste the zero-count.
> **This task owns this file exclusively** — B2 does not touch it. Revision 1's "or extend the e2e script" alternative in B2 is deleted.
> **Split:** this task writes assertions and proves the script type-checks. The live run happens in Lane D under R4. A subagent handed "run it end to end" will fake it or deadlock.

- [ ] **Step 1: Replace the demo prompt** with one provably absent from the corpus. Paste the grep count.
- [ ] **Step 2: Nonce echo.** The prompt is a fixed string today, so a cache, a stub handler, or a hardcoded fallback all pass:

```vox
let nonce = "vx" + str(time.now_unix())
let _ = run_json(bin, ["gui","drive","send","--text",
    "Reply with exactly this token and nothing else: " + nonce])
if not assistant_text.contains(nonce) {
    fail(bin, "reply did not echo the per-run nonce — cached or canned response")
}
```

- [ ] **Step 3: Positive provider assertion + negative control.** Assert an event *names* the provider (`provider == "vox_local"`, or `model_used == pin`) rather than only asserting openrouter's absence. Then the control no absence-assertion can fake: **stop the serve process, re-send, assert the turn now fails.** If it still succeeds, the reply was never coming from serve.
- [ ] **Step 4: Adapter-vs-base.** Nothing in the repo checks this anywhere. Same prompt, greedy, output must differ from the base model's. Without it, a serve that silently fell back to base weights passes every other assertion.
- [ ] **Step 5: Extract and compile.** Specify the extraction rule (first ```` ```vox ```` fence) and **make extraction failure a failure**. "No fence found → skip" is theater and is the shape this repo has shipped before.
- [ ] **Step 6: Assert the GUI's own view.** `gui drive state` already returns `probe.reachable`, `probe.models`, and `catalog[].reason`. Assert `probe.reachable` and that `probe.models` contains the pin, replacing the direct curl — this tests what the GUI sees, not what the server says.
- [ ] **Step 7:** `vox check scripts/axis-drive-metal-e2e.vox`. Paste the output.
- [ ] **Step 8: Commit.**

---

# Lane C — Make the journey reachable from the GUI

Ordered C1 → C2 → C3 → C4 → C5. **C1 first, because C2 is untestable in a packaged build without it.**

### Task C1: Make `vox mens` actually runnable from a packaged GUI

**Files:** Modify `crates/vox-gui/tauri.conf.json` (`externalBin`); `scripts/gui-build.vox`

> **Execution notes.** Per L-6, the GUI's three existing MENS cards likely render `Error: vox-ml-cli is not installed or not in PATH` outside a dev checkout, and `mens models` is broken even *with* the binary because `models` is behind the non-default `gpu` feature. Everything else in Lane C is dead without this.
> **Pre-decided (R12): bundle `vox-ml-cli` built with `--features gpu,execution-api`.** If the bundle-size cost is prohibitive — **measure it and say so** — the fallback is to detect the missing binary once on the `mens` surface and render an install instruction, **not** three identical opaque errors.

- [ ] **Step 1:** Reproduce. Build the app, launch from **Finder**, paste what the three cards render.
- [ ] **Step 2:** Add `vox-ml-cli` to `externalBin`; build with `--features gpu,execution-api` in `gui-build.vox`.
- [ ] **Step 3:** Re-launch from Finder; confirm all three cards render real output. Paste it.
- [ ] **Step 4:** Record the bundle-size delta. — [ ] **Step 5: Commit.**

---

### Task C2: A `mens serve` supervisor, not a card

**Files:**
- Create: `crates/vox-gui/src/commands/mens_serve.rs`, `crates/vox-gui/ui/src/components/surfaces/Models/MensServePanel.tsx`
- Modify: `crates/vox-gui/src/main.rs`, `Models/MensTrainingView.tsx` (81 lines — read whole), `MensTrainingView.test.tsx`

> **Execution notes. This supersedes Revision 1's Task 1.3, which was not implementable.** Revision 1 called it "a card definition, not new plumbing." That is wrong at the seam: `execute_command` calls `.output().await`, which **waits for process exit**, and `vox mens serve` never exits. The card's promise would hang forever and an untracked server would be parented to the GUI with no stop control.
> **`CommandCardsView` is the wrong component**, not just the wrong trigger — its contract is "arg-free read-only reads, all cards, on mount" (its own doc comment; `useEffect` at `:79-81`). Do not add a second execution mode. Add one line to its doc comment saying why serve is not a card.
> **The correct template exists: `crates/vox-gui/src/commands/daemon.rs`** — resolves a managed binary via `resolve_managed_binary_path`, spawns detached with stderr to `~/.vox/run/*.stderr.log`, holds the child in a slot, polls health, refuses unsafe restarts, kills on shutdown, and **already forwards `VOX_LOCAL_ENDPOINT`**.
> **The readiness probe exists too:** `llm_settings.rs::probe_vox_local()`. **Do not write a second probe.**
> Port **11435**, resolved from `vox-config`, never typed as a literal in TSX (Task E3).
> The load-bearing frontend assertion is the **negative** one: zero invokes after render, non-zero after a click.

- [ ] **Step 1: Write the failing tests.** Rust: start spawns and reaches ready; stop kills. TSX: zero invokes on mount, one after click.
- [ ] **Step 2: Verify they fail.**
- [ ] **Step 3: Implement `mens_serve.rs`** — `mens_serve_status` / `mens_serve_start { model, port }` / `mens_serve_stop`, modeled on `daemon.rs`. Register in `main.rs`.
- [ ] **Step 4: Implement `MensServePanel.tsx`**, above the `CommandCardsView`: model, port, Start/Stop, live status from `inference_provider_status`'s VoxLocal row (already probed live — reuse it).
- [ ] **Step 5: Verify they pass.** `npx vitest run src/components/surfaces` — fast, no cargo build.
- [ ] **Step 6:** Launch the app, start a server from the panel, confirm `BackendAvailability` flips to `online · N models`. Paste a screenshot.
- [ ] **Step 7: Commit.**

---

### Task C3: Say *why* a MENS model is unselectable instead of deleting the row

**Files:** Modify `crates/vox-gui/ui/src/lib/modelPicker.ts`, `Chat/ChatModelPicker.tsx`; test alongside

> **Execution notes. The single highest-value fix in the GUI audit.** `isModelSelectable()` (`modelPicker.ts:104-124`) already computes the reason (`local_reachable !== true`, or the id absent from `local_models`) — and `filterPickerModels` **silently drops the row**. The user who just trained `mens/foo` opens the picker, does not see it, and is told nothing; they conclude the feature does not work. Drive's API 409s `model_not_selectable` **with a reason**; the click path throws it away.
> Smallest shape: add `unselectableReason(model, statuses): string | null`, leave `filterPickerModels` alone, render failing local models as a dimmed non-clickable row with the reason and a "Start server" link to the `mens` surface.
> `ChatModelPicker.tsx:14-17`'s comment documenting that `model_override` and `set_active_model` are deliberately different is **documenting a bug as a design** — note it in your report; do not fix it here (R9).

- [ ] **Step 1: Write the failing test** — an unreachable local model renders with its reason and is not clickable.
- [ ] **Step 2: Verify it fails.** — [ ] **Step 3: Implement.** — [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Commit.**

---

### Task C4: Make "this turn was local and free" visible

**Files:** Modify `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx`

> **Execution notes.** `ModelBadge` **already accepts** `provider`, `costUsd`, `reqTokens`/`respTokens`, `selection`. `ChatTranscript.tsx:67-71` passes only `model`, `latencyMs`, `selectionReason`. The only locality cue today is an unlabelled `mens/` prefix. <15 lines, and it is the GUI counterpart to B3's `cost_incurred provider=openrouter` honesty check.
> Derive `provider` from the id (`mens/` → `local`), pass `costUsd={0}` for local. Threading `provider` + `cost_usd` through `ParsedChatReply` / `parse_chat_message_envelope` (`chat.rs:397-460`) is the more honest source but a larger diff — **recommend it in your report; do not do it here** (R9).

- [ ] **Step 1: Write the failing test.** — [ ] **Step 2: Verify it fails.** — [ ] **Step 3: Pass the props.** — [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Commit.**

---

### Task C5: Surface the gates Lane A repaired

**Files:** Create a `mens_run_reports` Tauri command beside `mens_serve.rs`; modify `Models/MensTrainingView.tsx`

> **Execution notes.** Lane A makes these reports able to fail. If nothing renders them, a user still cannot tell a passed adapter from a stubbed one — `eval-local`, `eval-collateral-damage`, `eval_local_report.json`, `gate_receipt.json`, and `collateral_damage_report.json` have **zero references anywhere under `crates/vox-gui`**.
> **Do not add `mens eval-*` cards.** Both are expensive GPU runs and `CommandCardsView` runs every card on mount. Read the JSON from the active run dir directly — no CLI invocation, no GPU.
> Show the substantive-gate count from A2 alongside the status: a receipt that passed with 0 substantive gates must not render as a green check.

- [ ] **Step 1: Write the failing test** — a run dir with `status: "fail"` renders as failed and names the degraded bench; a receipt with 0 substantive gates does not render green.
- [ ] **Step 2: Verify it fails.** — [ ] **Step 3: Implement.** — [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Commit.**

---

# Lane D — One spoke, end to end

### Task D1: `--domain` selects base and preset on the local path

**Files:** Modify `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` (914 lines — read `sed -n '380,470p'` and `sed -n '594,615p'`)

> **Execution notes.**
> **Scope this to the local branch.** Revision 1 said only `vox mens pipeline --profile` routes through `resolve_training_selection`. Not quite: `train_arm.rs:615` also calls it inside `resolve_cloud_spoke_base` (`:594-615`), invoked at `:104-109`. So **`vox mens train --domain rust --cloud <x>` already resolves correctly** — the bug is specific to `cloud == "local"`, where `:388` is literally `let effective_model = model;`. **`resolve_cloud_spoke_base` is the in-repo template to copy**; Revision 1 did not mention it exists.
> **Do not duplicate existing coverage.** `training_selection.rs:95-112` (`rust_resolves_qwen_qlora`) already asserts the resolver yields a Qwen model + `CandleQlora`. The missing test is at the **`train_arm.rs` wiring level**, not the resolver.
> Pure Rust, no GPU. Safely parallel with everything in Lanes A-C. Warm the build first (R2).

- [ ] **Step 1: Write the failing test** at the wiring level — `--domain rust --cloud local` yields the spoke's base and preset, not the defaults.
- [ ] **Step 2: Verify it fails.** — [ ] **Step 3: Implement, mirroring `resolve_cloud_spoke_base`.** — [ ] **Step 4: Verify it passes.** — [ ] **Step 5: Mutation-verify.** — [ ] **Step 6: Commit.**

---

### Task D2: Diagnose why 19,664 lines of Rust corpus emit zero pairs

**Files:** Modify `mens/config/mix-rust.yaml`; produces `target/dogfood/rust_authoring.validated.jsonl` (untracked)

> **Execution notes.**
> **Runs in the primary checkout only** (L-9). `mix-rust.yaml:19-21` marks `rust_authoring.validated.jsonl` `optional: false` and the file is absent; `train_mixed.mix_report.json` records `"input_lines": 19664, "emitted_lines": 0, "skipped_reason": "no_lines_passed_filters"`.
> Step 1 is the good subagent work: read the `skipped_reason` chain and `cat` it into your report. Generation runs can be long — wrap in `timeout` (R1), apply R3.
> **The floor is ambiguous by design.** The *enforced* gate is `MIN_CORPUS_PAIRS = 100` (and A4 made its absence-skip a hard error); the target is ≥500. **State the count you achieved as a number.** Do not report done at 100-499 without flagging it. Revision 1 cited a research doc for the 500 figure that lives under `docs/src/archive/` — **tombstoned, and agents must not ingest it.** Treat 500 as a given constant.
> **Also purge the retired syntax (L-5):** 67 rows carry `@query` / `@mutation` / `@component` spellings that are hard parse errors since 2026-06-30. Filter them out and report the count removed. The corpus currently teaches code the compiler rejects.

- [ ] **Step 1: Diagnose.** Paste the `skipped_reason` chain.
- [ ] **Step 2: Fix the filter.** — [ ] **Step 3: Purge retired-syntax rows; report the count.** — [ ] **Step 4: Regenerate; report the pair count.** — [ ] **Step 5:** `vox ci spoke-check` (now able to fail, per A4). — [ ] **Step 6: Commit.**

---

### Task D3: A non-leaked held-out bench

**Files:** Modify `mens/data/heldout_bench/manifest.json`; create `split_manifest.json`; modify `crates/vox-ml-cli/src/commands/mens/eval_gate/check_run.rs`; `eval_gate/leakage.rs`

> **Execution notes.** Per L-4: the bench is **10 tasks** and **4 of its answers are verbatim in the training corpus**. pass@1 on n=10 has a ±30-point interval — it cannot distinguish a 40% model from a 70% one, so it cannot support any claim Lane D makes.
> `assert_no_leakage` exists, is marked `#![allow(dead_code)] // not yet wired`, and has **zero non-test callers**. It also compares **tool names only**, by 3-gram Jaccard — useless for a code corpus.
> Three parts: expand the bench to ≥50 non-leaked tasks (the `humaneval-vox` held-out problems are already marked `training_eligible: false` — reuse them); emit a `split_manifest.json` at corpus-build time; wire `assert_no_leakage` into `check_run.rs` as a hard precondition and extend it to **normalized n-gram overlap between bench answers and corpus completions**.
> **State the bar numerically here so Lane D cannot move it later: pass@1 ≥ 0.5 with the Wilson lower bound above the base model's upper bound.** Kill-test Part A supplies the base number.

- [ ] **Step 1: Write the failing test** — a bench whose answer appears in the corpus is rejected by `assert_no_leakage`.
- [ ] **Step 2: Verify it fails** (it will: nothing calls the function).
- [ ] **Step 3: Expand the bench to ≥50; emit `split_manifest.json`.**
- [ ] **Step 4: Extend `assert_no_leakage` to text n-grams; wire it into `check_run.rs` as a hard precondition.**
- [ ] **Step 5: Verify it passes, and that the 4 known-leaked tasks are now caught.** Name them in your report.
- [ ] **Step 6: Mutation-verify** — re-add one leaked task, confirm the gate fails, remove. Paste both.
- [ ] **Step 7: Commit.**

---

### Task D4: Train and gate the `rust` spoke — **HUMAN-OPERATED**

> **Do not dispatch this as a subagent task.** Every step is a GPU training or eval run exceeding the 600s tool ceiling with no in-turn completion signal. This is the exact shape that produced five background-and-wait deadlocks and the 800k-token retry loop.
> **Split:** a human performs the training (or babysits a detached, logged run). A subagent then runs the gate, reads the receipts, and commits the run card, and may run B3's live half under R4.
> **Fail-the-program checkpoint.** If the first spoke misses D3's bar, **this plan stops and re-scopes.** Do not proceed to further spokes or to any distribution work on a model that did not clear it.

- [ ] **Step 1 (human):** Train `rust` on Metal.
- [ ] **Step 2 (agent):** `vox mens eval-gate`. Confirm the receipt has ≥2 substantive gates (A2). If it does not, **that is the finding** — the eval artifacts were never produced.
- [ ] **Step 3 (agent):** Score against D3's bench. **Report pass@1 with its Wilson interval against the base**, not a bare number.
- [ ] **Step 4 (agent):** Run B3's e2e live under R4. Paste the reply and the `vox check` result.
- [ ] **Step 5 (agent):** Commit the run card. **If the bar was missed, commit the finding and stop.**

---

# Lane E — Deletions

### Task E1: Delete the `VOX_MODEL` env write

**Files:** Modify `crates/vox-gui/src/commands/models.rs` **`:235-255`** (811 lines — read only that range)

> **Execution notes. Revision 1 proposed wiring this up. That was backwards.** Model selection already lives in **five** places: `AgentTask.model_override` (read by routing at `runtime.rs:822`), `ServerState.mcp_chat_model_override` (read by seven call sites), a per-call MCP param, a DB `user_preference` row (written, read back only for GUI display), and this `unsafe { std::env::set_var("VOX_MODEL", …) }`. Revision 1's Task 1.2 would have added a **sixth** write.
> This line is **provably inert** — the daemon is a separate process and never sees the GUI's `set_var` — and it is an `unsafe` env mutation inside an async fn in a multi-threaded Tauri process: a data-race hazard, not merely dead code. **Delete it.**
> Revision 1 also told the implementer to "mock `OrchDaemonClient` the way the existing tests in this file mock their dependencies." **No such mocking exists** — `models.rs:723-741` is pure `ModelCardDto` serde round-trips. The implementer would have hunted for a pattern that is not there.
> **Out of scope, recommend in your report (R9):** make the daemon's `mcp_chat_model_override` the sole runtime SSOT with the DB row as its persistence, and write the precedence ladder down once in `chat_model_resolve.rs`, whose module doc already explains a problem of exactly this shape.

- [ ] **Step 1:** Prove it is inert — grep every reader of `VOX_MODEL` and show none is in the daemon process. Paste the grep.
- [ ] **Step 2:** Delete the `set_var`. — [ ] **Step 3:** `cargo test -p vox-gui`. — [ ] **Step 4: Commit.**

---

### Task E2: Fold the duplicated plugin trees and fix the Metal CPU-fallback regression

**Files:** Create `crates/vox-plugin-mens-candle-core`; modify both plugin crates; add a layer row to `contracts/ci/crate-layers.v1.json`

> **Execution notes.**
> **Revision 1 said "mark or fold." Marking is not permitted.** AGENTS.md §Dependency Discipline rule 3: a helper **under ~50 lines** may be duplicated with a `// vox:defactored-from` comment; *"Larger shared surfaces get split into `-types`/`-core` crates. Never fork 100+ line chunks."* There is no suppression clause. 22 files are byte-identical, including `qlora_preflight.rs` (**720** lines), `merge.rs` (516), `hf_keymap.rs` (399). A marker on a 720-line file suppresses a rule that has no suppression. **Folding is the smaller end state** — ~3,000 lines deleted versus an annotation exercise that must be redone.
> **L-7 must be fixed here:** Metal lacks `resolve_inference_device` and silently falls back to CPU with no warning, where CUDA logs and errors; CUDA also carries two tests Metal does not. The device-dispatch seam (`resolve_inference_device` / `compute_dtype_for_device`) is the natural trait boundary: trait in the core crate, impls in the plugins.
> **`model_card.rs` and `manifest.rs` each exist in three copies** (vox-populi + both plugins). Fold them here — otherwise Task E3's license fix lands in one and leaves two drifted, which is exactly the hazard this task exists to remove.
> **Merge hazard — the one-side-wins trap.** Before folding any pair, `diff -u` cuda vs metal and **enumerate every divergent hunk**; assert the folded file contains *both* lanes' unique symbols. Taking one side silently reintroduces the drift this task removes.
> **Runs alone.** No other task in flight — 6 of the files it touches are >500 lines and are edited by Lanes 0/A/B/D. Splitting a file under another agent's edit produces the marker-free duplicate-definition merge this repo has already shipped once.
> **The new crate needs a layer row at creation** (rule 4).

- [ ] **Step 1:** `diff -u` all 44 pairs; write the identical/divergent inventory into your report.
- [ ] **Step 2: Write the failing test** for `resolve_inference_device` on Metal (port CUDA's two).
- [ ] **Step 3: Verify it fails on Metal, passes on CUDA.**
- [ ] **Step 4:** Create the core crate with its layer row; move the 22 identical files; put the device trait there. Fold the three `model_card.rs` and three `manifest.rs` copies.
- [ ] **Step 5:** Implement Metal's `resolve_inference_device` with warn-and-error behavior.
- [ ] **Step 6: Verify both plugins' suites pass.**
- [ ] **Step 7: Add the sameness gate** — a test (or `vox-code-audit` detector) asserting no file in the two plugin `src/` trees is byte-identical to its counterpart. Without it this is archaeology that will need redoing. **Delete the now-false "Kept byte-for-byte in sync" comment.**
- [ ] **Step 8: Commit.**

---

### Task E3: Give the port and the license one home each

**Files:** Modify `crates/vox-ml-cli/src/commands/ai/inference_defaults.rs:8`; `mens/config/gpu-specs.yaml`; `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs:375`; the folded `model_card.rs`

> **Execution notes.** Two unrooted values, one task, because both are "a literal that already has a home."
> **(a) Port (L-8).** The SSOT exists: `vox-config/src/inference.rs:165-173` — `VOX_LOCAL_ENDPOINT_DEFAULT` (11434), `…_OLLAMA_CONFLICT_ALT` (11435), `…_PROBE_CANDIDATES`. The **serve** side declares an independent `DEFAULT_INFERENCE_PORT = 11434` in `inference_defaults.rs:8`, which is **Ollama's port** — so `vox mens serve` collides with Ollama by default, and `mens-serving-ssot.md` documents the footgun instead of fixing it. **Change the serve default to 11435 and reconcile against the config SSOT.** One constant, and the collision is gone on every path — Revision 1 fixed it only in a GUI card's argv.
> **(b) License.** `license_class` is an unrooted `Option<String>`; **every non-test call site passes `None`** (`mens/pipeline.rs:498-499`, `populi/dispatch.rs:128-129`, `mens/tensor/execution_planner.rs:251-252`, `mens/tensor/preset_schema.rs:1208-1209`), it is hashed into contract identity at `finetune_contract.rs:268-269`, nothing reads `attribution_required`, and `merge_qlora.rs:375` hardcodes `apache-2.0`. **`mens/config/` and `contracts/mens/` contain the string "license" zero times** — there is no base-model→license mapping anywhere. Revision 1 threaded the field from the manifest, which just moves the hardcode to whoever types the flag. **Add `license:` and `attribution_required:` to each `train_bases` rung in `gpu-specs.yaml`** (already the SSOT for base selection), populate from the resolved base, make the flag an override, and make `None` a **hard error** instead of a silent `apache-2.0`.
> **Serialize after E2** — `model_card.rs` must be one file before this edits it.

- [ ] **Step 1: Write the failing tests** — the serve default equals the config SSOT's non-Ollama port; a base with no `license:` is a hard error.
- [ ] **Step 2: Verify they fail.** — [ ] **Step 3: Implement.** — [ ] **Step 4: Verify they pass.** — [ ] **Step 5: Mutation-verify the license error path.** — [ ] **Step 6: Commit.**

---

# Lane H — Hardware (human-operated, blocks nothing)

**Do not dispatch any of these as subagent tasks.** Each is a remote-host build, a GPU run, or a cross-machine serve that exceeds the 600s ceiling with no in-turn completion signal. They run on a different machine and block nothing in Lanes 0-E.

**H1 — Stand up `blaptop04`.** Connect as `ssh iacch@blaptop04.tail4f69a0.ts.net` (D-1). A cold Rust + CUDA build on a Quadro T1000 with no toolchain installed is a multi-hour operation behind an SSH pipe, with installers that may prompt.

Revision 1's instruction "use one persistent remote shell, not a chain of one-shot commands" **is not achievable through the Bash tool** — each call is an independent process. The `vcvars` footgun documented at `mens-training.md:171` is specifically the *nested* `cmd.exe /c "vcvars64.bat && cargo build"` form; the single-process form that works is `Enter-VsDevShell` (which sets the environment in the *current* PowerShell process) followed by `cargo build` in that same process. Verify the VS install path with `vswhere` first.

Record `tokens/sec` **with and without** `mens-candle-cuda` into a **named JSON artifact** beside the host doc. Without a named path the measurement is unfalsifiable — and the plugin's `default = []` means a build without the feature compiles against CPU candle and silently reports garbage (the crate's own Cargo.toml comment says so). That trap is how the Metal 9× figure was nearly missed. A subagent may write the host-setup doc (with frontmatter) once a human reports results.

**H2 — First real CUDA calibration.** Blocked on H1. Two GPU training measurements. Two bounded attempts at the equivalent Metal measurement already hit wall-clock limits this session. If any of it is dispatched, dispatch only "fit the lane, update the test, commit" — **after** a human supplies the two measured points as data.

**Do not overclaim what it proves.** `CalKey { lane, gradient_checkpointing }` has **no device, no dtype, and no parameter-size band**, so the row transfers to the 4080 Supers *by construction of the contract, not by evidence*. A T1000 is Turing — no bf16 tensor cores, 4 GB — and candle may select different kernels and a different compute dtype than an Ada 4080. Fitting a slope from **two points at 0.6B** and then governing 8B and 32B sizing with it is a 13-50× extrapolation. Mark the row `source: measured-t1000-0.6b`, **not** bare `measured`, and record in `memory-model.v1.yaml`'s provenance that it is an extrapolation beyond its measured band. Add a `params_b` band to `CalKey`, or at minimum warn when `plan_for` is asked to size more than ~4× outside it. **The real value of Lane H is that no CUDA code path in this program has ever executed on a CUDA device** — say that, and stop claiming the calibration generalizes.

**H3 — Cross-machine serve.** Blocked on H1 + H2. `vox mens serve` has no auth (`serve/mod.rs:117-124`), so binding beyond loopback is a security-relevant decision a human makes each time — never a subagent's.

**H4 — The mesh question (R12: human decides, agent may write).** `interp_executor.rs:478` refuses any `kind != TaskKind::VoxScript`; `TaskKind::TrainQLoRA` exists with no executor, and `vox-workflow-runtime`'s `dispatchable_source()` is a bare `Err(anyhow!(…))`. Whether "SSH-over-tailnet is the supported cross-machine path" is an architectural decision. Once a human states it, an agent writes the banner and the doc — re-verifying every supporting fact per R5.

---

# Execution: waves, conflicts, and merge order

## Wave plan

Width is capped by file conflicts and three singleton physical resources: **one Apple GPU, one `blaptop04`, one port 11435.**

| Wave | Tasks | Notes |
|---|---|---|
| **W0** | **Kill test Part B** | One curl. **May reorder everything below.** |
| **W1** | **Z1** | Alone on the GPU. Lane H's H1 runs alongside on another machine. |
| **W2** | **A1, C1, D1, E1** + **Kill test Part A** | 4-wide, CPU-only, zero overlap. Part A uses Ollama, not the Metal GPU. |
| **W3** | **A2, A3, B0, C2, D2** | A2 after A1 (`dispatch.rs`). B0 needs the GPU + Z1. D2 in the primary checkout (L-9). |
| **W4** | **Z2, A4, B1, C3, D3** | Z2 after A2 (`dispatch.rs`). B1 after Z1 — **never before**. |
| **W5** | **B2, C4** | B2 after B1 and after B0's ruling. |
| **W6** | **B3, C5, E3** | B3 owns the e2e script exclusively. E3 after E2 — **so E3 moves to W8**. |
| **W7** | **D4** | Human-operated. GPU lock. B3's live half rides here. **Fail-the-program checkpoint.** |
| **W8** | **E2, then E3** | **E2 alone. Nothing else in flight, no other worktrees, no background shells.** |

**Revision 1's phase numbering hid most of this.** Phases 1, 3, 4.1, 5.1 and 6.2 had no dependency on Phase 0 at all; sequencing them behind it cost most of the available parallelism. It also had no Lane 0, so its Phase 1 would have shipped a `max_tokens` increase into a quadratic loop.

## Conflict matrix

| File | Tasks | Rule |
|---|---|---|
| `mens/populi/dispatch.rs` | A1, A2, Z2 | Strict serial: A1 → A2 → Z2. |
| `metal/src/{inference,model}.rs` | Z1, E2 | E2 last, alone. |
| `ai/serve/{handlers,schema,mod}.rs` | B2 only | B3 does **not** touch these. |
| `provider_adapter.rs` | B1 only | |
| `MensTrainingView.tsx` | C2, C5 | Serial, C2 first. |
| `model_card.rs` (×3 copies) | E2, E3 | E2 folds them first; E3 then edits one file. |
| 6 files >500 lines | E2 vs Z1/A1/B2/D1/E1/C2 | **E2 runs last, alone.** |
| `mens/data/**`, `target/dogfood/**` | D2, D3, D4 | Untracked + per-worktree `target/` → primary checkout, serial. |

## Shared-resource locks

| Resource | Rule |
|---|---|
| **Apple GPU** | `flock /tmp/vox-gpu.lock` for the whole task. **Never dispatch two GPU tasks in one wave** — worktree isolation does not isolate the GPU. |
| **Port 11435** | `flock /tmp/vox-port-11435.lock`. **11434 is Ollama's — never bind it** (and E3 fixes the default that does). |
| **blaptop04** | One remote session, `flock /tmp/vox-blaptop04.lock`. H1-H3 are one chain. |
| **HF cache** | Shared across worktrees; concurrent downloads of the same repo race on the blob path. Pre-warm once in the primary checkout; **do not** set a per-worktree `HF_HOME` (re-downloads multi-GB bases N times). |
| **Cargo broker** | `VOX_BROKER_MAX_CONCURRENT=3`, `VOX_BROKER_RESERVED_SLOTS=1`. A slow `cargo` call is the broker working, not a hang. |

## Merge order and post-merge assertions

Every task **rebases** onto current `main` before its review gate. Nothing merges with `-X ours|theirs`; nothing is union-resolved except append-only registries (clap enums, `mod` lists, the command-catalog baseline txt). **Generated `.md` files are deleted and regenerated, never hand-merged** — fix drift at the generator's input.

After every merge touching Rust:

```bash
git diff --check                                      # no markers
rg -n '^(pub )?fn <each-new-fn-name>' <file> | wc -l   # must be exactly 1
cargo check -p <crate> --features <task features>
cargo test -p <crate> -- <task's test names>           # against the MERGED tree
```

The one-defined-once check is not ceremony: a move+reformat merge produces a duplicate definition **with no conflict markers**, and this repo has shipped one.

## Detecting a falsely-successful subagent

In rough order of how often each fires here:

1. **Inspect the committed object, never the working tree** — `git show <sha>:<path>`. A clean tree can hide a poisoned commit, and orphaned background shells keep writing after "completed."
2. **`0 tests run` reads as green.** Reject any "tests pass" report unless the controller's own run shows `N passed` with `N ≥ 1` **and** the named test appears in the output.
3. **Test exists but cannot fail.** For every task with a mutation step, **the controller performs the mutation itself.** A guard that passes both ways = rejected.
4. **No diff.** `git log main..HEAD --oneline` empty → the agent narrated work it did not do.
5. **Stub markers.** `rg -n 'TODO|TRACKED|unimplemented!|todo!' $(git diff --name-only main...HEAD)` — a new one is an automatic reject. Task A1 exists because of a `// TRACKED` stub.
6. **GPU claims without GPU evidence.** Any training or inference result must carry a tokens/sec figure **with and without** the accel feature flag. A build without it runs on CPU and silently reports garbage.
7. **A citation not re-read this turn.** Fifteen of Revision 1's line numbers were stale and three of its type names were invented. A report that quotes a line number without having read it this turn is not evidence.

---

## Appendix A — What Revision 1 got wrong

Recorded so the mistakes are not re-made and the cuts are not quietly re-added. Revision 1's four Discoveries (D-1…D-4) and every load-bearing *structural* claim were independently confirmed true. The rot was elsewhere.

**Missed the program-killer.** Zero tasks about inference throughput. All six phases could have landed green with a single chat reply still taking hours (L-1). Worse, Revision 1's Task 1.1 raised the default `max_tokens` 256 → 2048 **as a bug fix**, which against a quadratic non-terminating loop is a ~100× regression. Lane 0 now precedes everything and the kill test precedes Lane 0.

**Missed the quality question entirely.** No task asked whether any MENS model can reach a usable bar. The verification instrument is a 10-task bench with a ±30-point confidence interval, **4 of whose answers are verbatim in the training corpus** — and Revision 1's own e2e demo prompt was one of them (`fn add`). No MENS run has ever been evaluated; `mens/runs/` contains only `e2e-smoke`. Task D3 and the kill test's Part A exist because of this.

**Fabricated code — 3 of 8 sample tests would not compile.** `vox_eval::BenchRef`, `report.status`, `report.degradations` — none exist. `InferRequest`'s every field name, plus `..Default::default()` on a lifetime-bound struct with no `Default`. `model_spec(id, provider)` with the arguments swapped. Two independent critique tracks found all three without seeing each other's work. Fifteen further line numbers were stale by 1-8. **R5 exists because of this.**

**Cut as unnecessary (4 tasks).** Wiring `set_active_model` to the daemon (it would have added a sixth source of truth; the fix is a deletion — E1). A `vox_source` output mode (gold-plating; its labels are triplicated across three files with no parity gate, so adding it to one is a silent no-op — and the repair prompt it would feed is hardcoded to `"Fix the JSON."`). A multi-spoke runner (`scripts/mens/full-pipeline.vox` **already is** a 5-stage pipeline keyed on `VOX_MENS_DOMAIN`; Revision 1 cited that file and then proposed reimplementing it). A God-Object LoC audit (unrelated debt).

**Deferred (the whole distribution phase).** `hub.rs`'s `download_model` assumes a base-model safetensors layout, not an adapter repo — publishing produces an artifact Vox cannot read back. And publishing a model that has never cleared a quality bar is the wrong order regardless. Only the hardcoded-license compliance bug survives, in E3. The hf-hub/Xet research was verified accurate and stays on the shelf.

**Missed entirely.** Four more can't-fail gates plus the `cargo test` zero-filter exit-0 bug and the `if train_jsonl.exists()` skip (L-2) — so fixing only D-3 would not have produced a gate that catches a garbage model. Two gate policies reading files with **no producer anywhere in the repo** (L-3). `providers.v1.yaml` as **codegen input** whose `VoxLocal` row B2 falsifies. `execute_command`'s `.output().await` (L-6), which made Task 1.3 unimplementable as written, plus the `vox-ml-cli` sidecar/feature gap that leaves three shipped GUI cards dead in a packaged build, and the silently-dropped picker row. The reversed Metal CPU-fallback regression (L-7). That `vox mens serve` defaults to **Ollama's port** on every non-GUI path (L-8). That `mens/data/**` is gitignored (L-9). That the server **already injects its own system prompt**, so Revision 1's fix would have stacked two. That `train_arm.rs`'s cloud path already resolves the spoke base correctly, making Task 4.1 twice as broad as the bug. That `model_card.rs` and `manifest.rs` each exist in three copies. A cited research doc that lives under tombstoned `docs/src/archive/`. A mocking pattern it told the implementer to follow that does not exist.

**Structural blind spot.** Four separate tasks introduced a **new copy** of a value that already had a home in `vox-config`, `contracts/`, or `mens/config/`. Revision 1 was unusually honest about *behavioral* lies and largely blind to *structural* ones.
