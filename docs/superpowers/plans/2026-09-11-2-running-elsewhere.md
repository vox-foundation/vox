---
title: "Running elsewhere: making a trained MENS model usable off this machine"
description: "Quantize a trained MENS checkpoint without OOMing, publish it through the lane its architecture actually supports, and repair the Ollama routing defects in between."
category: "Implementation Plans"
status: "planned"
---

# Running Elsewhere — Make a Trained MENS Model Usable Off This Machine

> **For agentic workers:** one fresh subagent per task. **Step 0 of every task is to read the Executor Preamble below.** Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A trained MENS hub + adapter can be (a) quantized on this host without exhausting RAM, (b) handed to a third-party runtime that actually exists, and (c) consumed back through Ollama without silent routing failures. Supersedes `2026-09-11-B-quantization-lanes.md` and `2026-09-11-C-ollama.md`, both of which cited symbols, flags and constants that do not exist at `origin/main` (`7295b4470`).

**Architecture: two publish lanes, chosen by base architecture — not one.** An earlier draft of this plan claimed Ollama's own converter could take a merged SafeTensors directory and do both the GGUF conversion and the quantization (`ollama create -q q4_K_M`), so that Vox needed no GGUF writer at all. **That premise was tested against the real binary and is false for MENS's default base.** Ollama 0.33.3:

```
$ ollama create probe -q q4_K_M -f Modelfile     # config.json architectures=["Qwen3ForCausalLM"]
Error: unsupported architecture "Qwen3ForCausalLM"

$ ollama create probe --experimental -q q4_K_M -f Modelfile
Error: unsupported --quantize "q4_K_M": supported types are int4, int8, nvfp4, mxfp4, mxfp8
```

MENS's default base is `Qwen/Qwen3-8B` → `Qwen3ForCausalLM`. The legacy converter has no entry for that architecture; the experimental converter accepts it but **cannot emit k-quants**. On 0.33.3 those two are mutually exclusive, so there is no `ollama create` invocation that produces a k-quantized Qwen3 model. `LlamaForCausalLM` and `Gemma2ForCausalLM` **do** pass the architecture check (they then fail on a stub tokenizer, which is the expected next error for a fixture with no real tokenizer).

The consequences are structural, and this plan carries all of them:

- **The Ollama lane survives, scoped to what it supports** (Task 6): llama- and gemma2-family bases. It is the cheapest route where it applies and it stays.
- **The llama.cpp GGUF lane is restored as a real task** (Task 12), because for Qwen3 there is no alternative. It shells out to `convert_hf_to_gguf.py` + `llama-quantize` from a user-supplied llama.cpp checkout, located by a **CLI flag** rather than a new env var — see Task 12 for why the flag is an order of magnitude cheaper.
- **Everything else in this plan is defect repair on paths that already exist**, plus three capacity fixes (Tasks 8, 9, 10) without which this host can train a model it cannot then quantize.

**Tech Stack:** Rust (`vox-quantize`, `vox-ml-cli`, `vox-populi`, `vox-orchestrator-mcp`, `vox-gamify`, `vox-code-audit`, `vox-actor-runtime`, `vox-config`), Ollama Modelfile, llama.cpp (third-party, invoked as a subprocess).

---

## Executor Preamble — every task's Step 0 is to read this

**You are a fresh subagent.** You have not read the other tasks and do not need to. Everything your task consumes is stated inside it. If you need a symbol, signature, file, or decision that is not written down in your task, that is **a defect in the task**, not a gap for you to fill by inference: stop and report what is missing. Do not guess a signature, do not invent a helper, do not adapt the code to match the plan.

**Checkout.** Work in a branch off `origin/main` `7295b4470`. Confirm before Step 1:

```bash
git merge-base --is-ancestor 7295b4470 HEAD && echo OK || echo "WRONG CHECKOUT — stop and report"
```

Every line number below was verified at that commit. If a cited line is off by more than ±5, or a cited symbol is absent, **stop and report** — you are in the wrong tree or the plan is stale.

**Read before you edit.** For every existing file your task modifies, read the cited region first (`sed -n 'START,ENDp' <file>`) and confirm the symbol is where the task says. Never edit a region you have not read in this session.

**House rules that will bite you** (from `AGENTS.md`):

- `cargo test` takes **exactly one** positional TESTNAME filter. Two is a hard error (`unexpected argument found`).
- **Never** `cargo fmt --all` — it overflows the Windows command-line limit. Use `cargo fmt -p <crate>`.
- **Cargo features are per-package and nothing is on by default.** `vox-populi` has `mens` / `mens-cloud` / `mens-train` / `mens-hf-hub`; `vox-ml-cli` has `gpu` / `cloud` / `execution-api`; `vox-quantize` has only `cuda` / `metal`, **no** `mens`, and **no `anyhow` dependency** — all fallible code there returns `Result<_, QuantizeError>`. A test run without the right `--features` compiles none of your work and exits 0.
- Every new `pub fn` needs a same-file `#[test]` or the `tdd-guard` pre-commit hook blocks the commit.
- No new `.py` / `.sh` / `.ps1` glue **authored by us**. Automation is VoxScript (`.vox`, via `vox run`). Task 12 *invokes* a third-party project's own `convert_hf_to_gguf.py`; that is calling an external tool, not authoring glue, and is no more a policy exception than calling `ollama` or `cargo`. Do not "fix" it by rewriting llama.cpp's converter in Vox.
- Do **not** create or edit `docs/src/architecture/research-index.md` — retired 2026-09-06. Frontmatter alone makes a doc discoverable.
- Do **not** run `git add -A` or `git commit -am`. Commit only the paths your task's **Files** block names; other agents are editing neighbouring files.

**Timeouts.** Unbounded commands are killed at 120 s with no output, which looks like a failure. Bound them:

```bash
timeout 900s  cargo test -p <crate> --features <f> --lib <one-filter>
timeout 1800s cargo run -p vox-cli -- ci pre-push --complete
timeout 2700s cargo run -p vox-ml-cli --features gpu --release -- <subcommand>
```

`/opt/homebrew/bin/timeout` is GNU coreutils; an expired command exits **124** — that is a timeout, not a test failure. Cargo builds are serialized machine-wide by the build broker, so a cold build is slow even when nothing is wrong. On a 124, re-run **once** with double the budget, then report.

**You cannot watch remote CI.** `gh pr checks`, `gh run watch`, and polling loops are blocked by a hook. You cannot run interactive terminal dialogs (`/permissions`, `/config`, `/hooks`) or drive another machine's console.

**Verification discipline — the part that matters most.**

1. Run the exact command the step gives. Do not substitute a broader or narrower one.
2. Compare output **literally**. `PASS` is not an expectation; `2 passed; 0 failed` is.
3. **Zero tests run is never success.** `0 passed` means the code was not compiled — almost always a missing `--features`.
4. **A test that should not compile must not compile.** If a step says "Expected: FAIL to compile — `cannot find function X`" and you get a clean run or an assertion failure instead, **the premise is wrong.** Stop and report. Do not proceed to implement against a test you never saw fail.
5. **When reality and the step disagree, reality wins and you stop.** Report the step number, the command, the actual output, the promised output, and your one-line reading. Do not repair the plan by rewriting the test to match the code, loosening an assertion, or adding a wrapper that makes a missing symbol resolve.
6. **Verify each guard by mutation.** Break it deliberately, confirm red, restore, confirm green. A test that passes against both fixed and unfixed code is worse than none. Confirm the file is actually restored — a concurrent `rustfmt` can revert your edit and hand you a meaningless pass.

**Report back:** `STATUS: DONE | BLOCKED | PREMISE-FALSE`, files changed, each test with whether it failed-as-expected and then passed, every command run verbatim with exit status, and any deviation from the plan. A report that says "all steps complete, tests pass" without the counts is not a report.

---

## Global constraints

- **Verify before you write.** Every symbol, path, flag and line number below was confirmed against `/Users/brbrainerd/dev/vox-audit2` at `7295b4470`; the commands are in §Verification log. If a step's premise no longer holds, stop and re-verify rather than adapting the code to the plan.
- Attribution: `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

## The architecture split, stated once as verified fact

| Base architecture | `ollama create -q q4_K_M` | Lane |
|---|---|---|
| `LlamaForCausalLM` | passes the architecture check | **Task 6** (Ollama) |
| `Gemma2ForCausalLM` | passes the architecture check | **Task 6** (Ollama) |
| `Qwen3ForCausalLM` (MENS default, `Qwen/Qwen3-8B`) | `Error: unsupported architecture "Qwen3ForCausalLM"` | **Task 12** (llama.cpp) |
| any architecture, `--experimental` | `Error: unsupported --quantize "q4_K_M": supported types are int4, int8, nvfp4, mxfp4, mxfp8` | k-quants unavailable — **Task 12** |

Evidence: ollama 0.33.3, driven directly against a fixture directory whose `config.json` declared each architecture in turn. **Do not hardcode this table into a runtime gate.** Ollama's converter enumeration lives upstream in `convert/convert.go`, is not in this tree, and a hardcoded copy would rot silently. `ollama create` reports an unsupported architecture itself, immediately and legibly; Task 6 lets it.

## CLI-surface gate (Tasks 6 and 12)

Tasks 6 and 12 each add a flag to `vox mens merge-qlora`, which **is** registered (`contracts/cli/command-registry.yaml:2193`, `contracts/operations/catalog.v1.yaml:7726`, `docs/src/reference/cli-command-surface.generated.md:211`). Those rows record `path` / `status` / `feature_gate` and **not** individual flags, so a flag addition is *expected* to produce no drift. Expected is not verified, and `ssot-drift` runs in the **fast** pre-push tier, so an unregistered surface change fails the first `git push` with an error that looks unrelated. Both tasks therefore run `cargo run -p vox-cli -- ci command-sync` and commit whatever it regenerates, as an explicit step.

`vox mens export-gguf` has **no** row in the command registry, the operations catalog, or the generated surface doc (verified: zero matches across `contracts/` and `docs/src/reference/`).

## Deliberately not in this plan

| Rejected | Why |
|---|---|
| `AGENTS.md` VoxScript-First amendment | **REQUIRES HUMAN AUTHORIZATION — do not execute.** `AGENTS.md` is the always-loaded cross-tool policy surface; an agent must not rewrite it unilaterally. It is also unnecessary: Task 12 invokes a third-party tool's own script, which the rule does not cover (it bans glue *we author*). Left out of the dependency chain entirely. |
| MLX lane (`mlx_lm.convert`) | Candle-Metal already serves Macs through the existing `vox-quantize` path. A speed win, not a capability unlock. Task 11 handles the *other* MLX question — refusing an MLX checkpoint as **input** — which is a correctness issue, not a lane. |
| `QuantRecipe` fields on the handoff | Advisory prose in a JSON payload whose schema is `contracts/eval/external-serving-handoff.schema.json`; a schema change plus a `contracts/index.yaml` row to ship three `llm-compressor` command strings that belong in a doc. |
| Plan B Task 6 (`0.22` → `0.3033`, `VoxMixture::label`) | **Already done / never existed.** `crates/vox-quantize/src/policy.rs:109` is `0.303`; the string `0.22` appears nowhere under `crates/vox-quantize/src/`; the type is `QuantMixture`, not `VoxMixture`; there is no `.label()`; `fits_target_tier` takes `&QuantMixture` by reference. Deleted. (Task 10 removes the constant altogether, which is a different and better outcome than correcting it.) |
| Ollama API surface in `vox mens serve` | `vox mens serve` and Ollama both default to port 11434; serving `/api/tags` there double-registers every MENS model. Unchanged from Plan C's rejection, which was correct. |
| Ollama provider adapter | Already shipping at `stable` maturity (`contracts/orchestration/providers.v1.yaml`). |
| A runtime `ollama_can_convert(arch)` gate | Ollama's architecture enumeration is upstream and not in this tree; a hardcoded copy rots silently. `ollama create` reports it itself. |

**The ceiling that decides whether local quantization is possible at all.** Three independent unbounded allocations stand between a trained checkpoint and a quantized artifact, and **all three must go** — fixing any two still OOMs:

| Site | What is resident | 27B cost |
|---|---|---|
| `read.rs:33-36` (`open`, no-index path) | the whole single-file checkpoint, loaded **only to enumerate tensor names**, then discarded | ~111 GB on an unsharded intermediate |
| `recombine.rs:28-45` | every tensor upcast to F32 in one `HashMap` before any write | ~111 GB |
| `write.rs:31` + `:105` | every quantized tensor's bytes in `ArtifactWriter.raw`, serialized only in `finish()` | ~18.4 GB, unbounded in model size |

Tasks 1, 8 and 9 fix them respectively. Task 10 replaces the three fitted/heuristic size estimators with one header-walk that has no fitted parameters, so the plan can *say in advance* whether a run fits. Until Tasks 8 and 9 land, run Task 6's and Task 12's acceptance criteria on a small model only.

---

## Task 1: Read each shard once, and enumerate names from the header

**Files:**
- Modify: `crates/vox-quantize/src/read.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: no new public API. `SafeTensorsSource::open` gains shard-grouped ordering **and a header-only name scan**; `SafeTensorsSource::load_f32` gains a one-slot shard cache; a private `fn shard_loads(&self) -> usize` exists for the test only.

### The defect — two halves, same file

**Half one, `load_f32` (`read.rs:53-63`).** It calls `candle_core::safetensors::load(path, &Device::Cpu)` — a full `fs::read` + deserialize, no mmap — **once per tensor**. `open` (`:44`) builds `names` from `map.keys()`, a `HashMap`, so consecutive tensors land in arbitrary shards. For an 18-shard / ~900-tensor checkpoint that is ~900 whole-shard reads, and `recombine.rs:32,42` drives the same call in a loop, paying it twice over.

**Half two, `open` (`read.rs:33-37`).** On a directory holding a single `model.safetensors` with **no** index:

```rust
} else if single.exists() {
    let st = candle_core::safetensors::load(&single, &Device::Cpu)?;
    for name in st.keys() {
        map.insert(name.clone(), single.clone());
    }
}
```

It loads **every tensor** in order to read `st.keys()`, then drops `st`. Against the 111 GB unsharded intermediate that `recombine` writes today, this OOMs before a single tensor is quantized. The one-slot cache in half one does **not** fix this — `open` runs before any `load_f32` call and does not go through it.

Fix both: parse the safetensors header for names in `open`, group `names` by shard path, and keep exactly one deserialized shard in `load_f32`. Grouping is what makes a one-slot cache sufficient; it bounds peak RSS to one shard instead of the whole checkpoint.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing tests**

Append to the existing `mod tests` in `crates/vox-quantize/src/read.rs` (it already defines `fn write_st(dir: &Path, name: &str, tensors: &[(&str, Tensor)])` at `:72`, which accepts multiple entries):

```rust
    /// Catches: reverting `load_f32` to a per-tensor
    /// `candle_core::safetensors::load` (the read-amplification bug — four
    /// tensors would cost four shard loads), and dropping the shard grouping
    /// in `open` (which makes the one-slot cache thrash back to one load per
    /// tensor). Both mutations are observable here; neither is observable
    /// from a cache driven by a fake loader.
    #[test]
    fn sharded_reads_group_by_shard_and_load_each_shard_once() {
        let dir = tempfile::tempdir().unwrap();
        let t = || Tensor::zeros((2, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        write_st(
            dir.path(),
            "model-00001-of-00002.safetensors",
            &[("a1", t()), ("a2", t())],
        );
        write_st(
            dir.path(),
            "model-00002-of-00002.safetensors",
            &[("b1", t()), ("b2", t())],
        );
        std::fs::write(
            dir.path().join("model.safetensors.index.json"),
            r#"{"weight_map":{
                "a1":"model-00001-of-00002.safetensors",
                "a2":"model-00001-of-00002.safetensors",
                "b1":"model-00002-of-00002.safetensors",
                "b2":"model-00002-of-00002.safetensors"}}"#,
        )
        .unwrap();

        let src = SafeTensorsSource::open(dir.path()).unwrap();
        assert_eq!(
            src.tensor_names(),
            &["a1".to_string(), "a2".to_string(), "b1".to_string(), "b2".to_string()],
            "names must be grouped by shard so a one-slot cache suffices"
        );

        for name in src.tensor_names().to_vec() {
            assert_eq!(src.load_f32(&name).unwrap().dims(), &[2, 256]);
        }
        assert_eq!(
            src.shard_loads(),
            2,
            "four tensors across two shards must cost two shard loads"
        );
    }

    /// Catches: reverting `open`'s no-index branch to
    /// `candle_core::safetensors::load(&single, ..)` just to read `st.keys()`.
    /// That loads the whole checkpoint to enumerate names and discards it —
    /// fatal on the ~111 GB unsharded intermediate `recombine` writes. The
    /// one-slot cache in `load_f32` does not cover `open`, so this mutation
    /// is invisible to the test above. `shard_loads() == 0` after `open` is
    /// the only observable that distinguishes a header scan from a full load.
    #[test]
    fn open_enumerates_a_single_file_model_without_loading_tensor_data() {
        let dir = tempfile::tempdir().unwrap();
        let t = || Tensor::zeros((2, 256), candle_core::DType::F32, &Device::Cpu).unwrap();
        write_st(dir.path(), "model.safetensors", &[("w1", t()), ("w2", t())]);

        let src = SafeTensorsSource::open(dir.path()).unwrap();
        assert_eq!(
            src.shard_loads(),
            0,
            "open must read the header only; it loaded tensor data instead"
        );
        let mut names = src.tensor_names().to_vec();
        names.sort();
        assert_eq!(names, vec!["w1".to_string(), "w2".to_string()]);
        assert_eq!(src.load_f32("w1").unwrap().dims(), &[2, 256]);
        assert_eq!(src.shard_loads(), 1, "the first load_f32 reads the shard once");
    }
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-quantize --lib sharded_reads_group_by_shard
```

Expected: **compile error**, not an assertion failure — `error[E0599]: no method named 'shard_loads' found for struct 'SafeTensorsSource'`. `write_st` already takes `&[(&str, Tensor)]` and accepts multiple entries, so those calls are fine; only `shard_loads` is missing.

- [ ] **Step 3: Implement**

Replace the struct:

```rust
pub struct SafeTensorsSource {
    map: HashMap<String, PathBuf>,
    names: Vec<String>,
    /// One-slot shard cache. `names` is grouped by shard, so a single slot
    /// yields exactly one load per shard and bounds peak RSS to one shard.
    /// Previously `load_f32` called `candle_core::safetensors::load` (fs::read
    /// + full deserialize, no mmap) once per tensor.
    cached: std::cell::RefCell<Option<(PathBuf, HashMap<String, Tensor>)>>,
    loads: std::cell::Cell<usize>,
}
```

Add a header-only name reader. The safetensors container is `[u8; 8]` little-endian header length, then that many bytes of JSON whose top-level keys are tensor names plus the reserved `__metadata__`. No new dependency: `serde_json` is already a direct dep of `vox-quantize`.

```rust
/// Tensor names from a safetensors file's header, reading only the 8-byte
/// length prefix and the JSON header itself — never tensor data.
///
/// `open` previously called `candle_core::safetensors::load` here purely to
/// enumerate `st.keys()`, which materializes the whole checkpoint and then
/// drops it. On the unsharded intermediate `recombine` writes, that is the
/// entire model in RAM before any tensor has been quantized.
fn header_tensor_names(path: &Path) -> Result<Vec<String>, QuantizeError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut len_buf = [0u8; 8];
    f.read_exact(&mut len_buf)?;
    let header_len = u64::from_le_bytes(len_buf);
    let header_len = usize::try_from(header_len).map_err(|_| {
        QuantizeError::ReadModel(format!("header length overflows usize in {}", path.display()))
    })?;
    let mut header = vec![0u8; header_len];
    f.read_exact(&mut header)?;
    let json: serde_json::Value = serde_json::from_slice(&header)
        .map_err(|e| QuantizeError::ReadModel(format!("{}: {e}", path.display())))?;
    let obj = json.as_object().ok_or_else(|| {
        QuantizeError::ReadModel(format!("header is not an object in {}", path.display()))
    })?;
    Ok(obj
        .keys()
        .filter(|k| k.as_str() != "__metadata__")
        .cloned()
        .collect())
}
```

Replace the no-index branch at `:33-37`:

```rust
        } else if single.exists() {
            for name in header_tensor_names(&single)? {
                map.insert(name, single.clone());
            }
        } else {
```

At the end of `open`, replace the `names` construction and the `Ok(Self { .. })`:

```rust
        let mut names: Vec<String> = map.keys().cloned().collect();
        // Group by shard (then by name for determinism) so the one-slot cache
        // in `load_f32` sees each shard exactly once.
        names.sort_by(|a, b| (&map[a], a).cmp(&(&map[b], b)));
        Ok(Self {
            map,
            names,
            cached: std::cell::RefCell::new(None),
            loads: std::cell::Cell::new(0),
        })
```

Replace `load_f32` and add the test accessor:

```rust
    /// Load a tensor and cast to f32 on CPU.
    pub fn load_f32(&self, name: &str) -> Result<Tensor, QuantizeError> {
        let path = self
            .map
            .get(name)
            .ok_or_else(|| QuantizeError::ReadModel(format!("tensor `{name}` not found")))?;

        let mut slot = self.cached.borrow_mut();
        let hit = slot.as_ref().is_some_and(|(p, _)| p == path);
        if !hit {
            // Drop the previous shard before reading the next one so peak RSS
            // stays near one shard rather than the whole checkpoint.
            *slot = None;
            let tensors = candle_core::safetensors::load(path, &Device::Cpu)?;
            self.loads.set(self.loads.get() + 1);
            *slot = Some((path.clone(), tensors));
        }
        let (_, tensors) = slot.as_ref().expect("just populated");
        let t = tensors.get(name).ok_or_else(|| {
            QuantizeError::ReadModel(format!("tensor `{name}` missing from shard"))
        })?;
        Ok(t.to_dtype(candle_core::DType::F32)?)
    }

    /// Number of shard files actually deserialized by `load_f32`. Test-only
    /// observability for the read-amplification guard; not part of the API.
    #[cfg(test)]
    fn shard_loads(&self) -> usize {
        self.loads.get()
    }
```

- [ ] **Step 4: Run the tests**

```bash
timeout 900s cargo test -p vox-quantize --lib
```

Expected: all green, including the pre-existing `reads_single_file_model`, `reads_sharded_model_via_index`, `quantizes_sharded_model` and `merged_subset_overrides_base_keys` — `load_f32`'s signature and semantics are unchanged.

Note `SafeTensorsSource` is now `!Sync`. Verified safe: every construction site is single-threaded and local (`engine.rs:27`, `recombine.rs:15`, plus tests) — there are no other uses in the workspace.

- [ ] **Step 5: Mutation check**

Revert the no-index branch to `candle_core::safetensors::load(&single, ..)` + `st.keys()`, re-run `open_enumerates_a_single_file_model_without_loading_tensor_data`, confirm **red** (`shard_loads() == 1`, expected 0). Restore, confirm green, and confirm the restore actually landed (`grep -c header_tensor_names crates/vox-quantize/src/read.rs` ≥ 2).

- [ ] **Step 6: Commit**

```bash
cargo fmt -p vox-quantize
git add crates/vox-quantize/src/read.rs
git commit -m "$(cat <<'EOF'
perf(quantize): read headers to enumerate, and each shard once to load

Two unbounded reads in one file. open()'s no-index branch called
candle_core::safetensors::load purely to read st.keys(), materializing
the whole checkpoint to enumerate names and then dropping it -- fatal on
the ~111 GB unsharded intermediate recombine writes. And load_f32 called
the same function per tensor (fs::read plus a full deserialize, no mmap)
while open() built the tensor list from HashMap::keys(), so consecutive
tensors landed in arbitrary shards: an 18-shard checkpoint paid a
whole-shard read for every one of its ~900 tensors, twice over, because
recombine drives the same loop again.

open() now parses the safetensors header for names, and groups them by
shard so load_f32 can keep a single deserialized shard: one load per
shard, peak RSS near one shard instead of the whole checkpoint.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Stop the machine-readable handoff naming a binary and routes that do not exist

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/external_serving_handoff.rs:28,43,74`
- Modify: `crates/vox-plugin-mens-candle-cuda/src/external_serving_handoff.rs:43`
- Modify: `crates/vox-plugin-mens-candle-metal/src/external_serving_handoff.rs:43`
- Modify: `docs/src/reference/mens-serving-ssot.md:16-21,32,36,52`
- Modify: `docs/src/how-to/how-to-model-routing.md:71,226`

**Interfaces:**
- Consumes: the **only** two constructors that exist —
  `ExternalServingHandoffV1::schola_training_run(run_dir: &Path, base_model: &str, adapter_filename: &str) -> Self` (`:30`) and
  `ExternalServingHandoffV1::merged_qlora_subset(merged_shard_path: &Path, base_model: &str, tokenizer_hint: Option<&str>) -> Self` (`:51`).
  There is no `for_base` and no `for_run`; both were fiction in the superseded plans.
- Produces: no API change — only the `notes` string content.

### The defect

Three copies of `external_serving_handoff.rs:43` write `"Local: vox-schola serve (OpenAI /v1/chat/completions + Ollama-shaped /api/generate, /api/chat)…"` into every training run's machine-readable handoff. `mens-serving-ssot.md:14` states in the same repo that **"there is no standalone `vox-schola` binary in this workspace."** The real router (`crates/vox-ml-cli/src/commands/ai/serve/mod.rs:116-123`) serves `/health`, `/ready`, `/v1/models`, `/v1/generate`, `/generate`, `/v1/completions` — none of the Ollama routes the handoff advertises.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing test**

Append to `crates/vox-populi/src/mens/tensor/external_serving_handoff.rs`:

```rust
#[cfg(test)]
mod handoff_honesty_tests {
    use super::*;

    /// Catches: restoring the `vox-schola serve` / Ollama-route notes string.
    /// This file is machine-readable output consumed by automation, so a
    /// nonexistent binary name in it is a broken contract, not a typo.
    #[test]
    fn handoff_names_no_binary_and_no_route_that_does_not_exist() {
        let cases = [
            serde_json::to_string(&ExternalServingHandoffV1::schola_training_run(
                Path::new("/tmp/run"),
                "Qwen/Qwen3.8-27B",
                "candle_qlora_adapter.safetensors",
            ))
            .unwrap(),
            serde_json::to_string(&ExternalServingHandoffV1::merged_qlora_subset(
                Path::new("/tmp/run/merged.safetensors"),
                "Qwen/Qwen3.8-27B",
                None,
            ))
            .unwrap(),
        ];
        for json in cases {
            assert!(
                !json.contains("vox-schola"),
                "handoff names a binary this workspace does not build: {json}"
            );
            for phantom in [
                "/api/generate",
                "/api/chat",
                "/api/tags",
                "/api/version",
                "/api/embeddings",
                "/v1/chat/completions",
            ] {
                assert!(
                    !json.contains(phantom),
                    "handoff advertises route {phantom}, which ai/serve/mod.rs does not register: {json}"
                );
            }
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib handoff_names_no_binary
```

Expected: FAIL on an assertion (not a compile error — both constructors exist): the first case's JSON contains `vox-schola` and `/api/generate`.

- [ ] **Step 3: Fix all three Rust copies**

In each of the three files, replace the `:43` notes string with:

```rust
                "Local: vox mens serve --model <artifact_dir> (requires a vox-ml-cli build with --features execution-api). Routes: POST /generate, /v1/generate, /v1/completions; GET /health, /ready, /v1/models. This server does NOT speak the Ollama HTTP API, so POPULI_URL/OLLAMA_URL clients will not interoperate with it."
                    .to_string(),
```

In `crates/vox-populi/src/mens/tensor/external_serving_handoff.rs` also fix the two remaining `vox-schola` mentions: the doc comment at `:28` and the `merged_qlora_subset` notes at `:74` (`"…not loaded by vox-schola serve…"` → `"…not loaded by vox mens serve…"`).

- [ ] **Step 4: Fix the docs**

`docs/src/reference/mens-serving-ssot.md`:
- Replace the six-route bullet list at `:16-21` with the six routes that are actually registered (`/health`, `/ready`, `/v1/models` GET; `/generate`, `/v1/generate`, `/v1/completions` POST), and state plainly that this server does not implement the Ollama HTTP API.
- Correct `:32` (`POPULI_MODEL` "must match the name returned by **`GET /api/tags`**") — there is no `/api/tags`; point at `GET /v1/models`.
- Delete the claim at `:36` ("This server implements `/api/generate`, so orchestrator streaming works when **`POPULI_URL`** targets it") — it does not.
- Delete the claim at `:52` ("MCP's Ollama bridge uses **`POST /api/chat`**, which this server already supports") — it does not.

`docs/src/how-to/how-to-model-routing.md` — both `:71` and `:226` state the precedence as `OLLAMA_URL → POPULI_URL → default`. The code (`crates/vox-config/src/inference.rs:222-244`) is `VOX_POPULI_LOCAL_OLLAMA_URL → POPULI_URL → OLLAMA_URL → http://localhost:11434`: reversed, and missing the first key entirely. Correct both.

- [ ] **Step 5: Verify the two plugin copies and the docs**

```bash
rg -n "vox-schola" crates/ docs/src/reference/mens-serving-ssot.md
timeout 900s cargo check -p vox-plugin-mens-candle-cuda
timeout 900s cargo check -p vox-plugin-mens-candle-metal
timeout 900s cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/reference/mens-serving-ssot.md docs/src/how-to/how-to-model-routing.md
```

Expected: zero `vox-schola` hits outside `docs/src/archive/`; both crates check clean; doc lint clean. If the CUDA plugin cannot build on this host for lack of a CUDA toolkit, **say so in your report** and do not mark the task complete.

- [ ] **Step 6: Run and commit**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib handoff_names_no_binary   # 1 passed
cargo fmt -p vox-populi -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal
git add crates/vox-populi/src/mens/tensor/external_serving_handoff.rs \
        crates/vox-plugin-mens-candle-cuda/src/external_serving_handoff.rs \
        crates/vox-plugin-mens-candle-metal/src/external_serving_handoff.rs \
        docs/src/reference/mens-serving-ssot.md docs/src/how-to/how-to-model-routing.md
git commit -m "$(cat <<'EOF'
fix(mens): stop the serving handoff naming a binary and routes that do not exist

Three copies of external_serving_handoff.rs wrote "vox-schola serve"
plus /api/generate and /api/chat into every training run's
MACHINE-READABLE handoff. mens-serving-ssot.md states in the same repo
that no vox-schola binary is built here, and ai/serve/mod.rs:116-123
registers /health /ready /v1/models /generate /v1/generate
/v1/completions and nothing else.

Also corrects the local base-URL precedence in how-to-model-routing.md,
which had it reversed and omitted VOX_POPULI_LOCAL_OLLAMA_URL entirely.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Route Ollama clients through the config SSOT

**Files:**
- Modify: `crates/vox-code-audit/src/review/providers.rs:87-89`
- Modify: `crates/vox-code-audit/src/ai_analyze.rs:58-60`
- Modify: `crates/vox-gamify/src/ai/constants.rs:6`
- Modify: `crates/vox-gamify/src/ai/client/ctor.rs:49,51,275`
- Modify: `crates/vox-gamify/src/ai/provider.rs:43`

**Interfaces:**
- Consumes: `vox_config::inference::local_ollama_populi_base_url() -> String` (`crates/vox-config/src/inference.rs:225`) and `vox_config::snapshot::bump(changed_keys: &[&str])` (`crates/vox-config/src/snapshot.rs:92`). Both crates already depend on `vox-config` (`vox-code-audit/Cargo.toml:30`, `vox-gamify/Cargo.toml:28`) — no new crate edge, so no `contracts/ci/crate-edges.allow.v1.json` change.
- Produces: `vox_code_audit::review::providers::default_ollama_url` keeps its signature `pub fn default_ollama_url() -> String`; only its body changes. `vox-gamify`'s `OLLAMA_DEFAULT_URL` const is replaced by `pub(crate) fn ollama_default_url() -> String`.

### The defect

`vox-code-audit` returns the literal `"http://localhost:11434"` from two separate serde-default functions; `vox-gamify` uses a `const` (`constants.rs:6`, aliasing `vox_config::LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT`). A `const` cannot read `POPULI_URL`/`OLLAMA_URL`, so none of these can be redirected at a self-hosted server — which is exactly the interop story `mens-serving-ssot.md` advertises.

**Exact `OLLAMA_DEFAULT_URL` sites** (verified — an earlier draft said "two uses in `provider.rs`", which was wrong):

```
crates/vox-gamify/src/ai/constants.rs:6      the const itself
crates/vox-gamify/src/ai/provider.rs:43      ONE use, not two
crates/vox-gamify/src/ai/client/ctor.rs:49   probe_ollama(OLLAMA_DEFAULT_URL)
crates/vox-gamify/src/ai/client/ctor.rs:51   url: OLLAMA_DEFAULT_URL.to_string()
crates/vox-gamify/src/ai/client/ctor.rs:275  a third site the earlier count missed
```

Deleting the const is compile-enforced, so a missed site surfaces as a build error rather than a silent skip — but get the count right so the diff is reviewable.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing test**

Append to `crates/vox-code-audit/src/review/providers.rs`:

```rust
#[cfg(test)]
mod ollama_ssot_tests {
    use super::*;

    /// Catches: reverting the body to a hardcoded "http://localhost:11434".
    /// With the literal, OLLAMA_URL cannot redirect the client, so
    /// mens-serving-ssot.md's "point POPULI_URL at your server" story is
    /// false for every vox-code-audit review.
    #[test]
    fn ollama_default_url_resolves_through_the_config_ssot() {
        // SAFETY: single-threaded scope; the var is restored below and no
        // other test in this crate reads OLLAMA_URL.
        unsafe { std::env::set_var("OLLAMA_URL", "http://ssot-probe:1234") };
        vox_config::snapshot::bump(&["OLLAMA_URL"]);
        let got = default_ollama_url();
        unsafe { std::env::remove_var("OLLAMA_URL") };
        vox_config::snapshot::bump(&["OLLAMA_URL"]);
        assert_eq!(got, "http://ssot-probe:1234");
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-code-audit --lib ollama_default_url_resolves
```

Expected: FAIL on the assertion — `assertion `left == right` failed: left: "http://localhost:11434", right: "http://ssot-probe:1234"`. It compiles: `vox_config::snapshot` and `vox_config::inference` are both `pub mod` in `crates/vox-config/src/lib.rs:17,31`.

- [ ] **Step 3: Implement**

`crates/vox-code-audit/src/review/providers.rs:87-89`:

```rust
/// Default Ollama listen URL. Resolves through the config SSOT
/// (`VOX_POPULI_LOCAL_OLLAMA_URL` -> `POPULI_URL` -> `OLLAMA_URL` -> default)
/// so a self-hosted server can be targeted without a code change.
pub fn default_ollama_url() -> String {
    vox_config::inference::local_ollama_populi_base_url()
}
```

`crates/vox-code-audit/src/ai_analyze.rs:58-60` — delegate rather than duplicate, so one test covers both call sites:

```rust
fn default_ollama_url() -> String {
    crate::review::providers::default_ollama_url()
}
```

`crates/vox-gamify/src/ai/constants.rs:6` — **delete** the `OLLAMA_DEFAULT_URL` const and replace it with:

```rust
/// Ollama base URL, resolved through the config SSOT at call time. Was a
/// `const` aliasing `vox_config::LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT`, which
/// by construction could not observe `POPULI_URL` / `OLLAMA_URL`.
pub(crate) fn ollama_default_url() -> String {
    vox_config::inference::local_ollama_populi_base_url()
}
```

`crates/vox-gamify/src/ai/client/ctor.rs:49-52` — bind once, use twice:

```rust
        let ollama_url = ollama_default_url();
        if Self::probe_ollama(&ollama_url).await {
            providers.push(FreeAiProvider::Ollama {
                url: ollama_url,
                model: OLLAMA_DEFAULT_MODEL.to_string(),
            });
        }
```

Then update `ctor.rs:275` and `provider.rs:43` to call `ollama_default_url()`.

**No test is written for the vox-gamify change, deliberately.** Deleting the `const` is compile-enforced: nothing can reference it afterwards, and the compiler names every site (all four). `assert_eq!(ollama_default_url(), local_ollama_populi_base_url())` is a tautology that passes against both the fixed and unfixed code once the const is gone, so it would be worse than nothing.

- [ ] **Step 4: Verify**

```bash
timeout 900s cargo test -p vox-code-audit --lib ollama_default_url_resolves   # 1 passed
timeout 900s cargo check -p vox-gamify                                        # const gone; no stale refs
rg -n "OLLAMA_DEFAULT_URL" crates/vox-gamify/src/                             # zero hits
rg -n "http://localhost:11434" crates/vox-code-audit/src/ crates/vox-gamify/src/
```

Expected: the last `rg` shows only doc-comment text and test fixtures (`ai_analyze.rs:30,320,325,365`, `providers.rs:48`). Update the two doc comments at `ai_analyze.rs:30` and `providers.rs:48` to say "resolved via `local_ollama_populi_base_url()`"; the three in `ai_analyze.rs`'s own tests are fixture values and stay.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-code-audit -p vox-gamify
git add crates/vox-code-audit/src crates/vox-gamify/src
git commit -m "$(cat <<'EOF'
fix(ollama): resolve the client base URL through the config SSOT

vox-code-audit returned a hardcoded http://localhost:11434 from two
serde-default fns, and vox-gamify used a const. A const cannot observe
POPULI_URL or OLLAMA_URL, so neither client could be pointed at a
self-hosted server -- which is precisely the interop story
mens-serving-ssot.md advertises.

vox-gamify's const is deleted rather than re-pointed: the compiler then
names every one of its four call sites, which a tautological equality
test would not.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Stop an empty Ollama URL silently disabling tool-calling

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:54-76`

**Interfaces:**
- Consumes: `vox_config::inference::local_ollama_populi_base_url()`. `vox-orchestrator-mcp` already depends on `vox-config` (`Cargo.toml:87`) — no new crate edge.
- Produces: `model_spec_to_llm_config` (`pub(crate)`, `agent_loop.rs:68`) keeps its signature; the `ProviderType::Ollama` arm stops returning `None`.

### The defect

`agent_loop.rs:72-75` resolves `SecretId::OllamaUrl`, filters out empty, and ends the chain with `?` — so an unset `OLLAMA_URL` makes the whole function return `None`. The caller then falls back to `mcp_infer_completion`, which is the **non-tool** path: an agentic turn loses its tools with no error. The default that `local_ollama_populi_base_url()` already supplies makes the `None` unnecessary.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing test**

Append inside the existing `mod tests` in `agent_loop.rs` (it already has the `model_spec(...)` helper at `:1375`):

```rust
    /// Catches: restoring the `?` on the OllamaUrl secret lookup. With it,
    /// an unset OLLAMA_URL makes this return None and the caller falls back
    /// to the NON-TOOL mcp_infer_completion path -- an agentic turn silently
    /// loses its tools with no error anywhere.
    #[test]
    fn ollama_keeps_tool_calling_when_the_url_secret_is_unset() {
        // SAFETY: restored below; no other test here reads these keys.
        unsafe {
            std::env::remove_var("OLLAMA_URL");
            std::env::remove_var("POPULI_URL");
            std::env::remove_var("VOX_POPULI_LOCAL_OLLAMA_URL");
        }
        vox_config::snapshot::bump(&[
            "OLLAMA_URL",
            "POPULI_URL",
            "VOX_POPULI_LOCAL_OLLAMA_URL",
        ]);
        let spec = model_spec(ProviderType::Ollama, "qwen3:8b");
        let cfg = model_spec_to_llm_config(&spec)
            .expect("Ollama must map to a config, not None -- None disables tools");
        assert_eq!(cfg.provider, "ollama");
        assert_eq!(
            cfg.base_url.as_deref(),
            Some("http://localhost:11434/v1/chat/completions"),
            "must fall back to the config SSOT default"
        );
    }
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-orchestrator-mcp --lib ollama_keeps_tool_calling
```

Expected: FAIL on the `expect` — `Ollama must map to a config, not None -- None disables tools`.

If it instead **passes** before the fix, the environment had `OLLAMA_URL` set at process start and `resolve_secret` cached it past the `remove_var`; in that case re-run in a clean env and confirm the failure before proceeding:

```bash
env -u OLLAMA_URL -u POPULI_URL -u VOX_POPULI_LOCAL_OLLAMA_URL \
  timeout 900s cargo test -p vox-orchestrator-mcp --lib ollama_keeps_tool_calling
```

Do not implement against a test you have not seen fail.

- [ ] **Step 3: Implement**

Replace `agent_loop.rs:72-76`:

```rust
        ProviderType::Ollama => {
            // An empty/unset OLLAMA_URL must not disable tool-calling: the
            // config SSOT already supplies a working default, and returning
            // None here routes the turn to the non-tool
            // mcp_infer_completion path with no error.
            let base = vox_secrets::resolve_secret(vox_secrets::SecretId::OllamaUrl)
                .expose()
                .filter(|s: &&str| !s.trim().is_empty())
                .map(std::string::ToString::to_string)
                .unwrap_or_else(vox_config::inference::local_ollama_populi_base_url);
            let base_url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
            Some(LlmConfig {
```

Update the doc comment at `:54-66` — it currently lists the `None`-returning providers; `Ollama` is no longer among the cases that can return `None`.

- [ ] **Step 4: Run and commit**

```bash
timeout 900s cargo test -p vox-orchestrator-mcp --lib model_spec_to_llm_config
```

Expected: green, including the pre-existing `model_spec_to_llm_config_maps_openrouter` and `..._returns_none_for_google_direct`, which are untouched.

```bash
cargo fmt -p vox-orchestrator-mcp
git add crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
git commit -m "$(cat <<'EOF'
fix(chat): an unset OLLAMA_URL no longer silently disables tool-calling

model_spec_to_llm_config ended the OllamaUrl secret lookup with `?`, so
an unset or empty OLLAMA_URL returned None and the caller fell back to
mcp_infer_completion -- the non-tool path. The turn lost its tools with
no error, on the single most common local configuration.

local_ollama_populi_base_url() already supplies a working default, so
the None was never needed.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Move embeddings off the deprecated Ollama endpoint

**Files:**
- Modify: `crates/vox-actor-runtime/src/mens.rs:116-128,213-230`

**Interfaces:**
- Produces: `EmbedResponse` changes from `pub embedding: Vec<f64>` to `pub embeddings: Vec<Vec<f64>>`. `MensClient::embed(&self, text: &str) -> Result<Vec<f64>, MensError>` keeps its signature.

### The defect

`/api/embeddings` is deprecated upstream in favour of `/api/embed`. The two are not drop-in: the request field is `input`, not `prompt`, and the response is `{"embeddings": [[…]]}`, not `{"embedding": […]}`.

**`MensError` has exactly four variants** (`mens.rs:17-33`, verified): `Http(#[from] reqwest::Error)`, `ModelNotAvailable(String)`, `RateLimited { retry_after_ms: u64 }`, `MalformedResponse(String)`. The empty-batch case is a malformed body, so the placeholder is `MensError::MalformedResponse("/api/embed returned no embeddings".into())`. **Do not add a variant.**

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing test**

Append to `crates/vox-actor-runtime/src/mens.rs` (create `#[cfg(test)] mod embed_wire_tests` if none exists in-file):

```rust
#[cfg(test)]
mod embed_wire_tests {
    use super::*;

    /// Catches: leaving the request field as `prompt`. /api/embed reads
    /// `input`; a request carrying `prompt` is accepted with an empty input
    /// and yields a useless embedding rather than an error.
    #[test]
    fn embed_request_uses_the_api_embed_field_name() {
        let json = serde_json::to_string(&EmbedRequest {
            model: "nomic-embed-text",
            input: "hello",
        })
        .unwrap();
        assert!(json.contains("\"input\""), "got {json}");
        assert!(!json.contains("\"prompt\""), "got {json}");
    }

    /// Catches: leaving `EmbedResponse { embedding: Vec<f64> }`. /api/embed
    /// returns a batch (`embeddings: [[..]]`); the old shape fails to
    /// deserialize against it, so this fails if the struct is reverted.
    #[test]
    fn embed_response_parses_the_api_embed_batch_shape() {
        let r: EmbedResponse =
            serde_json::from_str(r#"{"embeddings":[[0.1,0.2,0.3]]}"#).unwrap();
        assert_eq!(r.embeddings.len(), 1);
        assert_eq!(r.embeddings[0], vec![0.1, 0.2, 0.3]);
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-actor-runtime --lib embed_request_uses_the_api_embed_field_name
```

Expected: **compile error**, not an assertion failure — `error[E0560]: struct 'EmbedRequest' has no field named 'input'` (the field is currently `prompt`, `mens.rs:119`), plus `error[E0609]: no field 'embeddings' on type 'EmbedResponse'` in the second test.

- [ ] **Step 3: Implement**

```rust
/// An embedding request. `/api/embed` reads `input`; the deprecated
/// `/api/embeddings` read `prompt`.
#[derive(Debug, Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

/// An embedding response. `/api/embed` returns a batch, one vector per input.
#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    /// One embedding vector per input; this client sends exactly one input.
    pub embeddings: Vec<Vec<f64>>,
}
```

and in `embed` (`mens.rs:214-229`):

```rust
        let req = EmbedRequest {
            model: &self.config.model,
            input: text,
        };

        let resp = self
            .http
            .post(format!("{}/api/embed", self.config.base_url))
            .json(&req)
            .send()
            .await?;

        let body = resp.json::<EmbedResponse>().await?;
        body.embeddings.into_iter().next().ok_or_else(|| {
            MensError::MalformedResponse("/api/embed returned no embeddings".into())
        })
```

- [ ] **Step 4: Run and commit**

```bash
timeout 900s cargo test -p vox-actor-runtime --lib embed_   # 2 passed
rg -n "api/embeddings" crates/
```

Expected: the `rg` shows no remaining live call site (docs text and the `mens-serving-ssot.md` line Task 2 already rewrote are fine).

```bash
cargo fmt -p vox-actor-runtime
git add crates/vox-actor-runtime/src/mens.rs
git commit -m "$(cat <<'EOF'
fix(mens): use /api/embed instead of the deprecated /api/embeddings

The two endpoints are not drop-in: the request field is `input`, not
`prompt`, and the response is a batch (`embeddings: [[..]]`) rather than
a single `embedding`. Sending the old request shape to the new endpoint
is accepted with an empty input and returns a useless vector rather than
an error, so both halves change together.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Publish a merged checkpoint to Ollama — for the architectures Ollama supports

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs:75,219`
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs:404-421` (the `MergeQlora` variant)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:455-473`

**Interfaces:**
- Consumes: `vox_quantize::recombine::recombine(base_dir: &Path, merged_subset: &Path, out_dir: &Path) -> Result<(), QuantizeError>` — already writes `<out_dir>/model.safetensors` and copies `config.json` (`recombine.rs:47-52`).
- Produces: `pub fn render_ollama_modelfile(num_ctx: usize, license: &str) -> String` in `merge_qlora.rs`; `--keep-merged` on `vox mens merge-qlora`; `run_merge_qlora` gains a `keep_merged: bool` parameter.

### Scope — read this before the steps

`merge-qlora --quantize` already builds `recombined_full/model.safetensors` + `config.json` — exactly what `ollama create` consumes — and then deletes it at `merge_qlora.rs:219`. This task keeps it, copies three tokenizer files in, and writes a Modelfile.

**This lane works for `LlamaForCausalLM` and `Gemma2ForCausalLM` bases and does not work for `Qwen3ForCausalLM`** — see §The architecture split. MENS's default base is Qwen3, so **this task's acceptance criterion must not be run against a Qwen3 checkpoint**; it would fail with `Error: unsupported architecture "Qwen3ForCausalLM"` and that failure would be correct behaviour, not a bug in this task. Qwen3 is Task 12's lane. The Modelfile this task emits is architecture-agnostic; the *converter* is not.

**DO NOT TOUCH `merge_qlora.rs:200`.** That `remove_dir_all` is a **pre-run clear** whose comment states why it exists: without it, a prior sharded run's `model.safetensors.index.json` survives and misleads `SafeTensorsSource::open`, which prefers the index over `model.safetensors` (`read.rs:23-26`). Gating it behind `!keep_merged` resurrects that bug. **Only `:219` — the post-quantize delete of the artifact we now want to keep — is gated.**

No `ADAPTER` directive is emitted: Ollama's safetensors adapter converter covers `llama` and `gemma2` base architectures only. Merged weights are the supported route in both lanes.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`:

```rust
#[cfg(test)]
mod ollama_publish_tests {
    use super::*;

    /// Catches: emitting `FROM model.safetensors` or `FROM <abs path>`.
    /// `ollama create` converts a *directory*; a file path is rejected, and
    /// an absolute path breaks the moment the run dir is copied anywhere.
    #[test]
    fn modelfile_imports_the_directory_relatively() {
        let m = render_ollama_modelfile(8192, "apache-2.0");
        assert!(m.starts_with("FROM .\n"), "got: {m}");
        assert!(m.contains("PARAMETER num_ctx 8192"), "got: {m}");
    }

    /// Catches: dropping the TEMPLATE. MENS trains with prompt_format
    /// "qwen_chatml_im_start" (external_serving_handoff.rs:40); a Modelfile
    /// without the matching ChatML template serves the model
    /// off-distribution, which looks like a bad checkpoint rather than a
    /// bad Modelfile.
    #[test]
    fn modelfile_template_matches_the_training_prompt_format() {
        let m = render_ollama_modelfile(8192, "apache-2.0");
        assert!(m.contains("<|im_start|>"), "got: {m}");
        assert!(m.contains("<|im_end|>"), "got: {m}");
    }

    /// Catches: emitting an ADAPTER directive. Ollama's safetensors adapter
    /// converter handles base architectures llama and gemma2 only, and it
    /// would fail on a LoRA adapter regardless of the merged-weight route
    /// this Modelfile describes.
    #[test]
    fn modelfile_emits_no_adapter_directive() {
        let m = render_ollama_modelfile(8192, "apache-2.0");
        assert!(!m.contains("ADAPTER"), "got: {m}");
    }

    /// Catches: reverting `if keep_merged { .. } else { remove_dir_all }` at
    /// :219 back to an unconditional delete -- i.e. undoing this entire
    /// feature. The three renderer tests above are all pure string checks
    /// and stay green against that revert, so without this one the task has
    /// no regression guard at all.
    ///
    /// Drives the gated block directly rather than through run_merge_qlora,
    /// which needs a real base checkpoint and an adapter.
    #[test]
    fn keep_merged_leaves_the_recombined_directory_and_its_modelfile_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let recombined = dir.path().join("recombined_full");
        std::fs::create_dir_all(&recombined).unwrap();
        std::fs::write(recombined.join("model.safetensors"), b"stub").unwrap();

        finish_recombined(&recombined, dir.path(), true).unwrap();
        assert!(
            recombined.join("model.safetensors").is_file(),
            "--keep-merged must keep the merged artifact; it was deleted"
        );
        assert!(
            recombined.join("Modelfile").is_file(),
            "--keep-merged must write a Modelfile next to the weights"
        );

        finish_recombined(&recombined, dir.path(), false).unwrap();
        assert!(
            !recombined.exists(),
            "without --keep-merged the recombined dir is still deleted"
        );
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-ml-cli --features gpu --lib modelfile_imports_the_directory
```

Expected: **compile error**, not an assertion failure — `error[E0425]: cannot find function 'render_ollama_modelfile' in this scope` (and `finish_recombined` likewise).

- [ ] **Step 3: Implement the renderer and the gated finisher**

In `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`:

```rust
/// Modelfile for a merged checkpoint. `FROM .` imports the SafeTensors
/// directory itself, so `ollama create` runs its own conversion.
///
/// Applies only to base architectures Ollama's converter supports (llama,
/// gemma2). Qwen3 bases are rejected by `ollama create` with
/// `unsupported architecture "Qwen3ForCausalLM"` and go through the
/// llama.cpp lane instead. This renderer does not gate on architecture:
/// Ollama's enumeration is upstream and a hardcoded copy would rot.
///
/// No `ADAPTER` directive: Ollama's safetensors adapter converter covers
/// llama and gemma2 bases only — merged weights are the supported route.
#[must_use]
pub fn render_ollama_modelfile(num_ctx: usize, license: &str) -> String {
    let mut s = String::from("FROM .\n");
    s.push_str(&format!("PARAMETER num_ctx {num_ctx}\n"));
    // Must match training's prompt_format "qwen_chatml_im_start"
    // (vox_populi::mens::tensor::external_serving_handoff); a mismatch serves
    // the model off-distribution.
    s.push_str(
        "TEMPLATE \"\"\"{{ if .System }}<|im_start|>system\n{{ .System }}<|im_end|>\n{{ end }}\
         <|im_start|>user\n{{ .Prompt }}<|im_end|>\n<|im_start|>assistant\n\"\"\"\n",
    );
    s.push_str(&format!("LICENSE \"\"\"{license}\"\"\"\n"));
    s
}

/// Post-quantize disposition of `recombined_full/`. With `keep_merged`, copy
/// the tokenizer files Ollama's converter needs and write a Modelfile beside
/// the weights; without it, delete the directory as before.
///
/// Extracted from the body of `run_merge_qlora` so the keep-vs-delete
/// decision is testable without a real base checkpoint and adapter.
fn finish_recombined(
    recombined: &std::path::Path,
    base_dir: &std::path::Path,
    keep_merged: bool,
) -> anyhow::Result<()> {
    if !keep_merged {
        let _ = std::fs::remove_dir_all(recombined);
        return Ok(());
    }
    for f in ["tokenizer.json", "tokenizer_config.json", "generation_config.json"] {
        let src = base_dir.join(f);
        if src.exists() {
            std::fs::copy(&src, recombined.join(f))
                .with_context(|| format!("copy {f} into recombined_full"))?;
        }
    }
    std::fs::write(
        recombined.join("Modelfile"),
        render_ollama_modelfile(8192, "apache-2.0"),
    )
    .context("write Modelfile")?;
    println!(
        "Merged model kept at {} — for a llama/gemma2 base, publish with:\n  cd {} && ollama create -q q4_K_M <name> -f Modelfile\nFor a Qwen3 base, `ollama create` rejects the architecture; use `vox mens merge-qlora --gguf-out <file> --llama-cpp <dir>` instead.",
        recombined.display(),
        recombined.display()
    );
    Ok(())
}
```

- [ ] **Step 4: Thread `--keep-merged`**

The **real** `MergeQlora` variant (`action_populi_enum.rs:404-421`, verified) has fields `base_shard: Vec<PathBuf>` (`#[arg(long = "base-shard", required = true)]`), `adapter`, `meta`, `output`, `quantize: Option<String>`. `run_merge_qlora`'s signature (`merge_qlora.rs:75`) is `(base_shards: Vec<PathBuf>, adapter: PathBuf, meta: PathBuf, output: PathBuf, quantize: Option<String>) -> anyhow::Result<()>`. **There is no `vox schola merge-qlora` command** — `schola` is a module path, the command is `vox mens merge-qlora`. Use that spelling everywhere.

Add to the variant:

```rust
        /// Keep `recombined_full/` (merged SafeTensors + config + tokenizer +
        /// Modelfile) so `ollama create -q q4_K_M <name> -f Modelfile` can
        /// consume it. Without this the directory is deleted after quantizing.
        #[arg(long, default_value_t = false)]
        keep_merged: bool,
```

In `dispatch.rs:455-466`, add `keep_merged` to the destructured pattern and pass it through. Add `keep_merged: bool` to `run_merge_qlora` (`:75`), and change **only line 219** to `finish_recombined(&recombined, &base_dir, keep_merged)?;`. Line 200's unconditional `remove_dir_all(&recombined)` stays exactly as it is.

- [ ] **Step 5: Point `export-gguf` at the two working routes**

In `dispatch.rs:463-473`, replace the `NOT_IMPLEMENTED` message body (the `anyhow::bail!` stays; **no clap arg changes**, so no registered surface moves):

```rust
        PopuliAction::ExportGguf { input, output } => {
            anyhow::bail!(
                "`vox mens export-gguf` is not a separate step. Two routes, by base architecture:\n  \
                 llama / gemma2 base — let Ollama convert:\n    \
                 vox mens merge-qlora --base-shard <base> --adapter <adapter> \\\n      \
                   --meta <meta.json> --output <merged.safetensors> \\\n      \
                   --quantize q4_k_m --keep-merged\n    \
                 cd <merged-parent>/recombined_full && ollama create -q q4_K_M <name> -f Modelfile\n  \
                 Qwen3 base (MENS default) — ollama create rejects the architecture; use llama.cpp:\n    \
                 vox mens merge-qlora --base-shard <base> --adapter <adapter> \\\n      \
                   --meta <meta.json> --output <merged.safetensors> \\\n      \
                   --keep-merged --gguf-out <out.gguf> --llama-cpp <llama.cpp checkout>\n\
                 Requested: input={} output={}",
                input.display(),
                output.display()
            );
        }
```

This task is the **sole owner** of that arm. Task 12 adds `--gguf-out` / `--llama-cpp` and does not re-edit this message; write it in full here so the two tasks do not collide. (If Task 12 has not landed when a user reads this message, the second route is aspirational — that is acceptable for an error string and cheaper than two conflicting edits.)

- [ ] **Step 6: Run the gates**

```bash
timeout 900s cargo test -p vox-ml-cli --features gpu --lib ollama_publish_tests   # 4 passed
timeout 900s cargo run -p vox-cli -- ci command-sync
git status --short contracts/ docs/src/reference/
```

If `command-sync` regenerated `contracts/cli/command-registry.yaml`, `contracts/operations/catalog.v1.yaml`, `docs/src/reference/cli-command-surface.generated.md` or `mens-train-defaults.generated.md`, **stage them in this commit**. An unregistered surface change fails `vox ci ssot-drift`, which runs in the fast pre-push tier, with an error that reads as unrelated to this work.

- [ ] **Step 7: Mutation check**

Revert `finish_recombined`'s body to an unconditional `let _ = std::fs::remove_dir_all(recombined); Ok(())`. Re-run `keep_merged_leaves_the_recombined_directory_and_its_modelfile_on_disk` and confirm **red**; the other three tests stay green, which is the point of adding the fourth. Restore, confirm green, confirm the restore landed.

- [ ] **Step 8: Acceptance criterion (manual) — a llama or gemma2 base ONLY**

```bash
timeout 2700s cargo run -p vox-ml-cli --features gpu --release -- mens merge-qlora \
  --base-shard <base shard> --adapter <adapter> --meta <meta.json> \
  --output /tmp/merged/merged.safetensors --quantize q4_k_m --keep-merged
grep -o '"architectures":[^]]*]' /tmp/merged/recombined_full/config.json
cd /tmp/merged/recombined_full
ollama create -q q4_K_M vox-mens-test -f Modelfile
ollama run vox-mens-test "write a vox component"
```

**Before the `ollama create`, read the `architectures` value the `grep` prints.** If it is `Qwen3ForCausalLM`, **stop**: this criterion does not apply to that checkpoint and `ollama create` will correctly refuse it. Use a llama- or gemma2-family base here, and verify Qwen3 through Task 12 instead. Record the architecture you used in your report.

Expected on a supported base: Ollama converts, quantizes and serves the artifact. A file Vox can read is not the goal; a file *Ollama* can read is. Use a small checkpoint — until Tasks 8 and 9 land, `recombine` holds the whole model as f32 in RAM and `ArtifactWriter` holds the whole quantized output, so a 27B needs ~130 GB here.

- [ ] **Step 9: Commit**

```bash
cargo fmt -p vox-ml-cli
git add crates/vox-ml-cli/src contracts/ docs/src/reference/
git commit -m "$(cat <<'EOF'
feat(mens): publish a merged checkpoint to Ollama where Ollama can take it

merge-qlora --quantize already wrote recombined_full/model.safetensors
plus config.json -- exactly what `ollama create` consumes -- and then
deleted it. --keep-merged keeps it, copies the three tokenizer files in,
and writes a Modelfile beside them.

Scoped deliberately: ollama 0.33.3's converter accepts llama and gemma2
bases and rejects Qwen3ForCausalLM outright, and its --experimental
converter cannot emit k-quants at all. MENS's default base is Qwen3, so
this lane is the cheap route where it applies and the llama.cpp lane
covers the rest. No architecture gate is hardcoded -- Ollama's
enumeration is upstream and `ollama create` reports it itself.

Only the post-quantize delete is gated. The pre-run clear stays
unconditional on purpose: it exists so a prior sharded run's
model.safetensors.index.json cannot mislead SafeTensorsSource::open,
which prefers the index over model.safetensors.

export-gguf now names both routes instead of a bare NOT_IMPLEMENTED.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Delete the ollama_subprocess stub

**Files:**
- Delete: `crates/vox-populi/src/inference/backends/ollama_subprocess.rs`
- Modify: `crates/vox-populi/src/inference/backends/mod.rs:15`
- Modify: `crates/vox-populi/src/inference/mod.rs:20`
- Modify: `crates/vox-populi/src/inference/backend.rs:38`

`OllamaSubprocessBackend::predict` returns the literal `format!("[ollama stub] {}", prompt.text)` (`ollama_subprocess.rs:50`) — it echoes its own input — next to a fully working HTTP path. A stub that returns its input satisfies a type check and can pass a smoke test.

**No test: this is a deletion, and the compiler enforces that `BackendId::OllamaSubprocess` has no remaining referents.** A test asserting the backend is absent would be a tautology.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Delete and unwire**

```bash
git rm crates/vox-populi/src/inference/backends/ollama_subprocess.rs
```

Remove `pub use ollama_subprocess::OllamaSubprocessBackend;` (`backends/mod.rs:15`) and its `mod ollama_subprocess;` line, the `OllamaSubprocessBackend` re-export in `inference/mod.rs:20`, and the `OllamaSubprocess` variant in `backend.rs:38`.

- [ ] **Step 2: Verify nothing referenced it**

```bash
timeout 900s cargo check -p vox-populi --features mens
rg -n "OllamaSubprocess" crates/
```

Expected: clean check; zero hits. If `BackendId` is matched exhaustively anywhere, the compiler names the site — fix it there, do not re-add the variant.

- [ ] **Step 3: Run the crate suite and commit**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib
git add crates/vox-populi/src/inference
git commit -m "$(cat <<'EOF'
chore(populi): delete the ollama_subprocess stub

predict() returned "[ollama stub] {prompt}" verbatim, next to a working
HTTP client. A stub that echoes its own input is worse than an absent
one: it satisfies the trait, type-checks, and can pass a smoke test.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Stream `recombine` so a large model can be recombined at all

Without this task **and Task 9**, the answer to "can this machine quantize what it just trained" is **no** — including for the 27B already trained on it.

**Files:**
- Modify: `crates/vox-quantize/src/recombine.rs:28-53`
- Test: same file, existing `mod tests`

**Interfaces:**
- Produces: `pub fn recombine_with_shard_budget(base_dir: &Path, merged: &HashMap<String, Tensor>, out_dir: &Path, shard_bytes: u64) -> Result<(), QuantizeError>`. `recombine`'s own signature — `(base_dir: &Path, merged_subset: &Path, out_dir: &Path)` — is unchanged; it loads `merged_subset` as it does today (`recombine.rs:16`) and delegates with a default budget.

### The defect

`recombine.rs:28` builds

```rust
let mut complete: HashMap<String, candle_core::Tensor> = HashMap::new();
```

and fills it with **every tensor upcast to F32** (`m.to_dtype(candle_core::DType::F32)?` at `:40`, `base.load_f32(name)?` at `:32,:42`), then calls `candle_core::safetensors::save(&complete, out_dir.join("model.safetensors"))` (`:48`) — one unsharded file, written only after the whole model is resident.

At 4 bytes per parameter:

| model | F32 resident | on a 128 GiB host |
|---|---|---|
| Qwen3-0.6B | 2.4 GB | fine |
| Qwen3-8B | 32 GB | fine |
| **Qwen3.8-27B** | **111 GB** | **fails** — the OS and window server need the rest |
| a 70B cloud-sized run | 280 GB | fails everywhere |

Two independent costs are stacked: the F32 upcast (the base is 8-bit on disk, so this is a 4× expansion of an already-quantized artifact) and full residency. Task 1 fixes read amplification; this fixes the recombine write side; Task 9 fixes the quantized-output write side.

### Step 0 fixture — `write_fake_base` does not exist

The tests below call `write_fake_base`, which **is not in the tree**. The only fixture helper in `vox-quantize` is `write_st(dir: &Path, name: &str, tensors: &[(&str, Tensor)])` at `read.rs:72`, and it is private to `read.rs`'s test module. Define this in `recombine.rs`'s `mod tests` **before** writing the tests, or Step 2's promised `cannot find function 'recombine_with_shard_budget'` is preceded by a different missing-symbol error and you will not know which to fix:

```rust
    /// Write a single-file base checkpoint of F32 tensors with the given
    /// element counts, plus a minimal config.json, and return its directory.
    fn write_fake_base(root: &std::path::Path, tensors: &[(&str, usize)]) -> std::path::PathBuf {
        let dir = root.join("base");
        std::fs::create_dir_all(&dir).unwrap();
        let mut map: HashMap<String, Tensor> = HashMap::new();
        for (name, elems) in tensors {
            map.insert(
                (*name).to_string(),
                Tensor::zeros((*elems,), candle_core::DType::F32, &Device::Cpu).unwrap(),
            );
        }
        candle_core::safetensors::save(&map, dir.join("model.safetensors")).unwrap();
        std::fs::write(dir.join("config.json"), r#"{"model_type":"test"}"#).unwrap();
        dir
    }
```

Each `("a", 4096)` tensor is 4096 × 4 bytes = 16 KiB, so a `shard_bytes` budget of 8192 forces one tensor per shard and four shards — comfortably above the `>= 2` the first test asserts.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file, then add the `write_fake_base` helper above to `recombine.rs`'s `mod tests`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn recombine_does_not_hold_the_whole_model_resident() {
        let dir = tempfile::tempdir().unwrap();
        let base = write_fake_base(dir.path(), &[("a", 4096), ("b", 4096), ("c", 4096), ("d", 4096)]);
        let out = dir.path().join("out");
        recombine_with_shard_budget(&base, &HashMap::new(), &out, 8192).unwrap();

        let shards = std::fs::read_dir(&out).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".safetensors"))
            .count();
        assert!(shards >= 2, "a model larger than the shard budget must be written in shards, got {shards}");
        assert!(out.join("model.safetensors.index.json").exists(),
            "sharded output needs an index or no reader can load it");
    }

    #[test]
    fn recombine_preserves_every_tensor_across_shards() {
        let dir = tempfile::tempdir().unwrap();
        let base = write_fake_base(dir.path(), &[("a", 4096), ("b", 4096), ("c", 4096), ("d", 4096)]);
        let out = dir.path().join("out");
        recombine_with_shard_budget(&base, &HashMap::new(), &out, 8192).unwrap();
        let index: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(out.join("model.safetensors.index.json")).unwrap()).unwrap();
        let map = index["weight_map"].as_object().expect("weight_map");
        for name in ["a", "b", "c", "d"] {
            assert!(map.contains_key(name), "sharding dropped tensor `{name}`");
        }
    }
```

*Mutations caught:* (1) reverting to a single `save(&complete, …)` — one shard, no index, first test fails; (2) a sharding loop that drops a tensor at a boundary or writes an index disagreeing with the files — second test fails. Neither assertion is about mere existence, because an "output exists" check passes against both bugs.

- [ ] **Step 2: Run them and confirm they fail**

```bash
timeout 900s cargo test -p vox-quantize --lib recombine_does_not_hold_the_whole_model_resident
```

Expected: **FAIL to compile** — `cannot find function 'recombine_with_shard_budget'`. If you instead see `cannot find function 'write_fake_base'`, Step 0 was skipped.

- [ ] **Step 3: Implement streaming**

1. Walk `base.tensor_names()` accumulating names until the running byte total exceeds the budget. Sizes come from the safetensors header (Task 1 already made header reading the norm in this file's sibling), so this pass loads no tensor data — two passes are cheap, which is how the shard count becomes known before the first write.
2. Per group: load only that group's tensors (reusing Task 1's shard-grouped `load_f32`), apply the merged override where present, `save` to `model-{i:05}-of-{n:05}.safetensors`, and **drop the map before the next group**.
3. Write `model.safetensors.index.json` with `metadata.total_size` and a `weight_map` of tensor name → shard filename.
4. Copy `config.json` as today (`:49-52`).
5. Keep the two existing validations: the merged-key-absent-from-base check (`:20-26`) and the per-tensor dim equality check (`:35-39`). Three existing tests cover them.

Do **not** upcast to F32 when the merged tensor and the base already agree in dtype. The upcast exists to make tensors comparable, but the comparison is on `dims()`, which needs no dtype change. Preserving the base dtype halves peak RAM again.

- [ ] **Step 4: Run and confirm they pass**

```bash
timeout 900s cargo test -p vox-quantize --lib recombine
```

Expected: the two new tests pass and all three pre-existing ones (`merged_subset_overrides_base_keys`, `merged_key_absent_from_base_errors`, `merged_key_shape_mismatch_errors`) stay green.

- [ ] **Step 5: Prove it on a real model**

Use the **real** command spelling (verified: `vox mens merge-qlora`, `--base-shard` repeatable, `--adapter`, `--meta`, `--output`; there is no `schola merge-qlora` command and no `--base` / `--out`):

```bash
/usr/bin/time -l timeout 2700s cargo run -p vox-ml-cli --features gpu --release -- \
  mens merge-qlora --base-shard <8B shard 1> --base-shard <8B shard 2> \
  --adapter <adapter.safetensors> --meta <meta.json> \
  --output /tmp/merged8b/merged.safetensors --quantize q4_k_m --keep-merged 2>&1 | tail -30
```

**`/usr/bin/time -l` reports `maximum resident set size` in BYTES on macOS**, not kilobytes as on Linux. Record the literal number and assert `< 10_000_000_000` (10 GB), against the ~32 GB the 8B needs today. Paste the raw line into your report and the commit message — it is the measurement that decides whether the 27B and a cloud-sized run are in reach. A timeout exits 124; that is not a failure of the fix.

- [ ] **Step 6: Commit**

```bash
cargo fmt -p vox-quantize
git add crates/vox-quantize/src/recombine.rs
git commit -m "$(cat <<'EOF'
fix(quantize): stream recombine instead of holding the model in RAM

recombine built a HashMap of every tensor upcast to F32 and wrote one
unsharded file. At 4 bytes/param that is 111 GB for the 27B this machine
just trained and 280 GB for a 70B -- so the machine could train a model
it could not then recombine.

Shard-at-a-time writing drops peak residency to roughly one shard
regardless of model size, and the base dtype is preserved where the
merged tensor agrees, removing the 4x upcast of an 8-bit artifact.

Measured max RSS on the 8B: <paste the /usr/bin/time -l bytes here>

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Stream `ArtifactWriter` so the quantized output is not held in RAM

**Task 8 alone does not make the 27B quantizable.** It fixes the recombine stage; this fixes the stage after it.

**Files:**
- Modify: `crates/vox-quantize/src/write.rs:29-33,87-122`
- Test: same file, existing `mod tests`

**Interfaces:**
- Produces: `ArtifactWriter::with_shard_budget(out_dir: &Path, mixture: &str, shard_bytes: u64) -> Result<Self, QuantizeError>`. `ArtifactWriter::new()`, `add_quantized`, `add_f32` and `finish` keep their current signatures and behaviour from the engine's point of view; `finish` stops being the only thing that writes.

### The defect

```rust
// write.rs:29-33
#[derive(Default)]
pub struct ArtifactWriter {
    raw: HashMap<String, (Vec<u8>, candle_core::DType, Vec<usize>)>,
    meta: HashMap<String, TensorMeta>,
}
```

`add_quantized` (`:61`) and `add_f32` (`:82`) each `insert` the tensor's full byte buffer into `raw`, and nothing reaches disk until `finish` (`:87`) rebuilds every buffer into a `Tensor` and calls one `safetensors::save` at `:111`. **The entire quantized output is resident before a single byte is written.**

For a 27B at Q4_K_M that is ~18.4 GB — survivable alone, fatal stacked on the quantization working set, and unbounded in model size: an 8-bit 70B would be ~40 GB of `raw` on top of whatever else is live. Streaming `recombine` (Task 8) does not touch this path; `engine.rs:28` constructs the writer and `:44,:63,:80` feed it tensor by tensor while `raw` only grows.

Same shape of fix as Task 8: a byte budget, flush a shard when it is exceeded, write an index at the end.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Write the failing test**

Append to `write.rs`'s `mod tests`:

```rust
    /// Catches: reverting to accumulate-everything-then-save-once. The
    /// existing roundtrip test passes against both shapes because it only
    /// checks the metadata sidecar and that *a* model file exists, so it
    /// cannot see residency. Here the budget is smaller than the tensors, so
    /// an accumulating writer produces exactly one file and no index.
    #[test]
    fn the_writer_flushes_shards_instead_of_holding_the_whole_output() {
        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut artifact = ArtifactWriter::with_shard_budget(dir.path(), "Q4_K_M", 4096).unwrap();
        for name in ["w1", "w2", "w3", "w4"] {
            let t = Tensor::randn(0f32, 1f32, (8, 256), &dev).unwrap();
            let q = QTensor::quantize(&t, GgmlDType::Q4K).unwrap();
            artifact.add_quantized(name, &q, &[8, 256]).unwrap();
        }
        artifact.finish(dir.path(), "Q4_K_M").unwrap();

        let shards = std::fs::read_dir(dir.path()).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".safetensors"))
            .count();
        assert!(shards >= 2, "output above the shard budget must be flushed in shards, got {shards}");

        let index: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(dir.path().join("model.safetensors.index.json")).unwrap()).unwrap();
        let map = index["weight_map"].as_object().expect("weight_map");
        for name in ["w1", "w2", "w3", "w4"] {
            assert!(map.contains_key(name), "flushing dropped tensor `{name}`");
        }
    }
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-quantize --lib the_writer_flushes_shards_instead_of_holding
```

Expected: **FAIL to compile** — `cannot find function 'with_shard_budget' for struct 'ArtifactWriter'`.

- [ ] **Step 3: Implement**

- Give `ArtifactWriter` an output dir, a `shard_bytes` budget, a running `pending_bytes`, a `shard_index: usize`, and a `weight_map: HashMap<String, String>`. `new()` keeps working by using a default budget and deferring the directory until `finish` — or, simpler and preferred, make `new()` delegate to `with_shard_budget` with a temp-less default and have `finish` assert the dir matches; pick the shape that keeps `engine.rs` unchanged and say which you chose.
- In `add_quantized` / `add_f32`, after inserting into `raw`, if `pending_bytes > shard_bytes`, flush: convert the pending buffers to `Tensor`s exactly as `finish` does today (`:92-109`), `save` to `model-{i:05}-of-*.safetensors`, record each name in `weight_map`, clear `raw` and `pending_bytes`, bump `shard_index`.
- `finish` flushes the remainder, renames the shards to their final `-of-{n:05}` count (or writes them with a placeholder and renames — say which), writes `model.safetensors.index.json` with `metadata.total_size` and the `weight_map`, and writes `quant-metadata.json` unchanged from `:113-120`.
- **Single-shard case:** when only one shard is produced, write it as `model.safetensors` with no index, preserving today's layout. The existing `writes_blocks_and_metadata_that_roundtrip` test asserts `model.safetensors` exists and must stay green.
- Errors stay `QuantizeError`; use the existing `QuantizeError::Write(String)` variant for anything new. **`vox-quantize` has no `anyhow`.**

- [ ] **Step 4: Run and confirm**

```bash
timeout 900s cargo test -p vox-quantize --lib
```

Expected: the new test passes and `writes_blocks_and_metadata_that_roundtrip` stays green — that pairing is what proves the single-shard layout is preserved.

- [ ] **Step 5: Mutation check**

Raise the default budget to `u64::MAX` so nothing ever flushes early. Confirm `the_writer_flushes_shards_instead_of_holding` goes **red** and `writes_blocks_and_metadata_that_roundtrip` stays green. Restore, confirm both green.

- [ ] **Step 6: Commit**

```bash
cargo fmt -p vox-quantize
git add crates/vox-quantize/src/write.rs
git commit -m "$(cat <<'EOF'
fix(quantize): flush the artifact in shards instead of buffering it all

ArtifactWriter accumulated every tensor's bytes in a HashMap and
serialized only in finish(), so the entire quantized output was resident
before anything reached disk -- 18.4 GB for a 27B at Q4_K_M, unbounded
in model size, and stacked on top of the quantization working set.

Streaming recombine does not help this: it is the stage after it. Same
fix shape -- a byte budget, flush on exceed, index at the end -- with
the single-shard case still written as a bare model.safetensors so the
existing on-disk layout is unchanged for small models.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Replace three size estimators with one header walk that has no fitted parameters

**Files:**
- Modify: `crates/vox-quantize/src/policy.rs:93-152`
- Modify: `crates/vox-quantize/src/lib.rs:18`
- Modify: `crates/vox-ml-cli/src/commands/quantize.rs:34-48,80-100,159-170`

**Interfaces:**
- Produces: `pub fn plan_quantize(input_dir: &Path, mixture: &QuantMixture) -> Result<QuantizePlan, QuantizeError>` in `vox_quantize::policy`, returning `pub struct QuantizePlan { pub output_bytes: u64, pub peak_bytes: u64, pub bits_per_weight: f64, pub params: u64 }`.
- Deletes: `QWEN3_27B_BOOSTED_ROLE_FRACTION` (`policy.rs:109`), `mixture_bpw` (`:116`), `fits_target_tier` (`:147`) and its `lib.rs:18` re-export, and `estimate_params_b` (`quantize.rs:39`) with its test at `:159`.

### The defect — three estimators, all guessing, none needed

1. `estimate_params_b` (`quantize.rs:39-48`) divides the total `*.safetensors` byte size by 2, **assuming bf16**. Its own doc comment admits the assumption. Against an 8-bit checkpoint it doubles the parameter count.
2. `mixture_bpw` (`policy.rs:116`) weights two dtypes by a "boosted role fraction" — a property of the *architecture*, passed in as a scalar.
3. `QWEN3_27B_BOOSTED_ROLE_FRACTION = 0.303` (`policy.rs:109`) is that scalar, **measured from one checkpoint** and applied to all. `fits_target_tier`'s own doc comment (`:137-146`) documents that this makes it wrong for models with different embedding shares, and tells callers to "use `mixture_bpw` directly with a measured fraction when the architecture is known" — which no caller does.

**The measurement they approximate is available exactly, from the headers, for free.** Walking each shard's safetensors header and classifying every tensor through the code that already exists — `TensorRole::from_key` (`policy.rs:18`) → `QuantMixture::target_for` (`:60`) → `resolve_dtype` (`:155`) → candle's real `GgmlDType::{block_size, type_size}` — gives the true output size. On `Qwen/Qwen3.8-27B` the header walk yields **5.303 bpw / 18.42 GB**, while `mixture_bpw(0.303)` yields **5.125 bpw / 17.8 GB** — 3.4 % optimistic **on the very checkpoint the constant was fitted to**. On any other architecture the error is unbounded.

So: do not add a fourth estimator alongside three. Add the exact one and delete the three.

### The peak formula — every term is a file stat or block arithmetic

```
peak = 2·S_max + O + c·4·p_max
```

- `S_max` — largest shard's size on disk, from `fs::metadata().len()`. The `2·` covers Task 1's one-slot cache holding one shard while the next is read.
- `O` — total output bytes, summed exactly: for each tensor, `resolve_dtype(mixture.target_for(TensorRole::from_key(name)), *shape.last())`, then `elems / dtype.block_size() * dtype.type_size()`, with `KeepF32` / `GgmlDType::F32` tensors costing `elems * 4`. **Task 9 makes this a bound on output-side RAM rather than a requirement**, so carry the term and note it shrinks to one shard once Task 9 lands.
- `p_max` — largest tensor's element count, from the header `shape`.
- `c = 4` — live F32 temporaries of the largest tensor inside `verify`. `round_trip_mse` (`verify.rs:29-35`) holds `src`, `deq`, `diff` and `sq` simultaneously; `round_trip_max_abs` (`:38-42`) holds three. Four is the maximum, **counted from the source**, not fitted.

**No constant in this function was tuned against a measurement.** If a reviewer asks "where did this number come from", every answer is a `stat` call, a header field, or a line of `verify.rs`.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file. Read `policy.rs:89-152`, `verify.rs:28-42` and `quantize.rs:34-100` before editing.

- [ ] **Step 1: Write the failing tests**

Append to `policy.rs`'s test module:

```rust
    /// Catches: reintroducing a bpw estimate weighted by a fitted
    /// architecture constant. This fixture's boosted-role share is nothing
    /// like Qwen3-27B's 0.303, so a role-fraction estimator and an exact
    /// header walk disagree here; only the walk matches the size the engine
    /// actually writes.
    #[test]
    fn plan_quantize_sizes_the_output_from_real_ggml_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut map: HashMap<String, Tensor> = HashMap::new();
        // Matrix role -> Q4K; down_proj -> Q6K; a norm -> KeepF32.
        map.insert("model.layers.0.self_attn.q_proj.weight".into(),
                   Tensor::zeros((256, 256), candle_core::DType::F32, &dev).unwrap());
        map.insert("model.layers.0.mlp.down_proj.weight".into(),
                   Tensor::zeros((256, 256), candle_core::DType::F32, &dev).unwrap());
        map.insert("model.layers.0.input_layernorm.weight".into(),
                   Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap());
        candle_core::safetensors::save(&map, dir.path().join("model.safetensors")).unwrap();

        let plan = plan_quantize(dir.path(), &QuantMixture::Q4KM).unwrap();

        let q4k = GgmlDType::Q4K;
        let q6k = GgmlDType::Q6K;
        let expected = (65536 / q4k.block_size() * q4k.type_size()
            + 65536 / q6k.block_size() * q6k.type_size()
            + 256 * 4) as u64;
        assert_eq!(plan.output_bytes, expected,
            "output size must come from candle's real block layout, not a bpw constant");
        assert_eq!(plan.params, 65536 + 65536 + 256);
    }

    /// Catches: dropping any term of the peak formula. Every term is a file
    /// stat or block arithmetic; a peak below the output size or below the
    /// largest-tensor working set would let a run be scheduled that cannot
    /// complete, which is the failure this function exists to prevent.
    #[test]
    fn plan_quantize_peak_covers_the_shard_cache_the_output_and_the_verify_temporaries() {
        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut map: HashMap<String, Tensor> = HashMap::new();
        map.insert("model.layers.0.self_attn.q_proj.weight".into(),
                   Tensor::zeros((512, 256), candle_core::DType::F32, &dev).unwrap());
        candle_core::safetensors::save(&map, dir.path().join("model.safetensors")).unwrap();

        let plan = plan_quantize(dir.path(), &QuantMixture::Q4KM).unwrap();
        let shard_bytes = std::fs::metadata(dir.path().join("model.safetensors")).unwrap().len();
        let p_max = 512u64 * 256;
        assert!(plan.peak_bytes >= 2 * shard_bytes + plan.output_bytes + 4 * 4 * p_max,
            "peak {} omits a term: 2*S_max={} O={} 4*4*p_max={}",
            plan.peak_bytes, 2 * shard_bytes, plan.output_bytes, 4 * 4 * p_max);
    }
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-quantize --lib plan_quantize_sizes_the_output
```

Expected: **FAIL to compile** — `cannot find function 'plan_quantize' in this scope`.

- [ ] **Step 3: Implement `plan_quantize`**

Reuse Task 1's header reader if it has landed; otherwise read the header the same way (8-byte LE length, then that many bytes of JSON). For each `*.safetensors` file in `input_dir`, for each header entry other than `__metadata__`, take `shape` and accumulate as described in §The peak formula. Return `QuantizePlan`. `bits_per_weight` (`policy.rs:89`) already derives bpw from candle's block layout and **stays** — it is the honest primitive the deleted functions misused; `QuantizePlan.bits_per_weight` is `output_bytes * 8 / params`.

- [ ] **Step 4: Delete the three estimators and rewire the caller**

- `policy.rs`: delete `QWEN3_27B_BOOSTED_ROLE_FRACTION`, `mixture_bpw`, `fits_target_tier`. Keep `bits_per_weight` and `needed_gib`.
- `lib.rs:18`: drop `fits_target_tier` from the re-export list; add `plan_quantize` and `QuantizePlan`.
- `quantize.rs`: delete `estimate_params_b` (`:39-48`) and its test `estimate_params_b_halves_safetensors_bytes_for_bf16` (`:159-170`). Replace the `--target-vram-gib` block (`:80-100`) with a `plan_quantize` call and an error message quoting `plan.output_bytes` and `plan.peak_bytes` as exact figures — drop the "estimate is weights-only and assumes the Qwen3-27B boosted-role split; it is approximate for other architectures" hedge, which no longer applies.

The compiler enumerates every remaining referent of the deleted symbols. Fix them at the call site; do not reinstate a shim.

- [ ] **Step 5: Run and confirm**

```bash
timeout 900s cargo test -p vox-quantize --lib
timeout 900s cargo test -p vox-ml-cli --features gpu --lib quantize
rg -n "QWEN3_27B_BOOSTED_ROLE_FRACTION|mixture_bpw|fits_target_tier|estimate_params_b" crates/
```

Expected: both suites green; the `rg` returns **zero hits**.

- [ ] **Step 6: Cross-check against a real checkpoint (optional but recorded)**

If a `Qwen/Qwen3.8-27B` bf16 checkpoint is on disk, run `plan_quantize` against it and record `bits_per_weight` and `output_bytes`. The expected values are **5.303 bpw / 18.42 GB**; the deleted `mixture_bpw(0.303)` gave 5.125 bpw / 17.8 GB. If your walk reproduces 5.303, note it; if it does not, **stop and report the discrepancy** rather than adjusting the code to hit the number.

- [ ] **Step 7: Commit**

```bash
cargo fmt -p vox-quantize -p vox-ml-cli
git add crates/vox-quantize/src/policy.rs crates/vox-quantize/src/lib.rs \
        crates/vox-ml-cli/src/commands/quantize.rs
git commit -m "$(cat <<'EOF'
refactor(quantize): size a run from the headers, delete three estimators

Three approximations stood between a checkpoint and a yes/no on whether
quantizing it fits: estimate_params_b divided file bytes by 2 assuming
bf16, mixture_bpw weighted two dtypes by an architecture constant, and
QWEN3_27B_BOOSTED_ROLE_FRACTION was that constant, measured from one
model and applied to all. fits_target_tier's own doc comment documented
the resulting error and told callers to pass a measured fraction, which
no caller did.

plan_quantize walks the safetensors headers and classifies every tensor
through the shipped path -- TensorRole::from_key, QuantMixture::
target_for, resolve_dtype, candle's real GgmlDType block layout -- so
the output size is computed, not estimated. Peak is 2*S_max + O +
4*4*p_max: a file stat, the computed output, and the four live F32
temporaries round_trip_mse holds. No fitted parameter anywhere.

On Qwen3.8-27B the walk gives 5.303 bpw / 18.42 GB; the constant it
replaces gave 5.125 / 17.8 GB -- 3.4% optimistic on its own source
checkpoint, and unbounded elsewhere.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Refuse an MLX checkpoint up front instead of quantizing nonsense

**Files:**
- Modify: `crates/vox-quantize/src/read.rs` (the `open` path from Task 1)
- Test: same file

**Interfaces:**
- Produces: `SafeTensorsSource::open` returns `Err(QuantizeError::ReadModel(..))` for an MLX-packed checkpoint. No new public function.

### The defect

`vox-quantize` **cannot consume an MLX quantized checkpoint**, and nothing in the pipeline says so. Verified by header inspection of the local `models--mlx-community--Qwen3.8-27B-8bit`: **102 of 378 tensors per shard are `U32`** — packed integer weights — each with separate `.scales` and `.biases` siblings. `load_f32` would upcast those packed integers to F32 and hand them to `QTensor::quantize`, which succeeds, produces a **finite MSE**, and writes a plausible-looking artifact of noise. Nothing errors. The bf16 original is the only valid input.

Two further incompatibilities make a silent partial merge impossible to rescue anyway:

- **No shared key namespace.** bf16 uses `model.language_model.layers.…`; MLX uses `language_model.model.layers.…`. `recombine.rs:20-26` requires exact name membership and errors on a merged key absent from the base — so an MLX base against a bf16 merged subset fails there, but an MLX base *alone* does not.
- **Transposed `conv1d.weight`:** `[10240,1,4]` (bf16) vs `[10240,4,1]` (MLX). `recombine.rs:35-39` requires exact dim equality.

A finite MSE on garbage is the worst available failure mode — it passes `verify`. Catch it at `open`, where the header is already being read.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file. **Depends on Task 1**, which introduces the header reader this check hooks into.

- [ ] **Step 1: Write the failing test**

```rust
    /// Catches: accepting an MLX-packed checkpoint. MLX stores quantized
    /// weights as U32 with sibling `.scales`/`.biases`; load_f32 would upcast
    /// the packed integers and QTensor::quantize would return a FINITE mse on
    /// noise, so `verify` passes and a nonsense artifact ships. There is no
    /// later stage that catches this -- refusing at open is the only guard.
    #[test]
    fn open_refuses_an_mlx_packed_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let dev = Device::Cpu;
        let mut map: HashMap<String, Tensor> = HashMap::new();
        map.insert(
            "language_model.model.layers.0.self_attn.q_proj.weight".into(),
            Tensor::zeros((256, 32), candle_core::DType::U32, &dev).unwrap(),
        );
        map.insert(
            "language_model.model.layers.0.self_attn.q_proj.scales".into(),
            Tensor::zeros((256, 4), candle_core::DType::F32, &dev).unwrap(),
        );
        map.insert(
            "language_model.model.layers.0.self_attn.q_proj.biases".into(),
            Tensor::zeros((256, 4), candle_core::DType::F32, &dev).unwrap(),
        );
        candle_core::safetensors::save(&map, dir.path().join("model.safetensors")).unwrap();

        let err = SafeTensorsSource::open(dir.path())
            .expect_err("an MLX-packed checkpoint must be refused, not quantized");
        let msg = err.to_string();
        assert!(msg.contains("MLX"), "the error must name MLX so the fix is obvious: {msg}");
        assert!(msg.contains("bf16"), "the error must name the valid input: {msg}");
    }
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-quantize --lib open_refuses_an_mlx_packed_checkpoint
```

Expected: FAIL on the `expect_err` — `open` succeeds today.

- [ ] **Step 3: Implement**

In `open`, after the name/dtype map is built from the headers (Task 1's `header_tensor_names` must also surface each entry's `dtype` for this — widen it to return `Vec<(String, String)>` of name and dtype string, and update Task 1's two call sites and tests accordingly, or add a sibling `header_tensor_dtypes`; say which you chose):

```rust
        // MLX quantized checkpoints store packed weights as U32 with sibling
        // `.scales` / `.biases`. load_f32 would upcast the packed integers and
        // QTensor::quantize would return a finite mse on noise, so verify()
        // passes and a nonsense artifact ships. Refuse here — there is no
        // later stage that can tell the difference.
        if let Some(name) = packed_u32_weight(&dtypes) {
            return Err(QuantizeError::ReadModel(format!(
                "`{name}` is a U32 packed weight with .scales/.biases siblings — this is an MLX \
                 quantized checkpoint, which this engine cannot consume (it would upcast the \
                 packed integers and quantize noise with a finite error). Use the bf16 original."
            )));
        }
```

where `packed_u32_weight` finds the first `*.weight` entry whose dtype is `U32` **and** whose sibling `*.scales` exists in the same map. Both conditions matter: `U32` alone could be a legitimate index tensor, and a `.scales` sibling alone is not proof of packing.

- [ ] **Step 4: Run and confirm**

```bash
timeout 900s cargo test -p vox-quantize --lib
```

Expected: the new test passes and every pre-existing `read`/`recombine`/`engine` test stays green — a plain F32 or bf16 checkpoint must not trip the guard. That pairing is the whole assertion.

- [ ] **Step 5: Verify against the real checkpoint, if present**

```bash
ls ~/.cache/huggingface/hub/ | grep -i mlx
```

If `models--mlx-community--Qwen3.8-27B-8bit` is present, point `vox quantize --input <its snapshot dir>` at it and confirm the new error text appears instead of a running quantization. Record the output. If it is absent, say so — the unit test stands on its own.

- [ ] **Step 6: Commit**

```bash
cargo fmt -p vox-quantize
git add crates/vox-quantize/src/read.rs
git commit -m "$(cat <<'EOF'
fix(quantize): refuse an MLX checkpoint instead of quantizing noise

MLX 8-bit checkpoints store 102 of 378 tensors per shard as U32 packed
weights with separate .scales/.biases siblings. load_f32 upcasts those
packed integers to F32 and QTensor::quantize accepts them, returning a
FINITE mse -- so verify() passes and a plausible artifact of noise
ships. No later stage can tell the difference.

The two checkpoints also share no key namespace (model.language_model.*
vs language_model.model.*) and disagree on conv1d.weight's layout
([10240,1,4] vs [10240,4,1]), so recombine's membership and dim checks
would reject a mixed merge -- but an MLX base alone sails straight
through. The bf16 original is the only valid input, and open() is where
the header is already in hand to say so.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: The llama.cpp GGUF lane — restored, because Qwen3 has no other route

**This task was rejected in an earlier draft of this plan on the grounds that `ollama create -q q4_K_M` made it unnecessary. That premise was tested and is false** (see §The architecture split). For `Qwen3ForCausalLM` — MENS's default base — there is no `ollama create` invocation that produces a k-quantized model. This lane is the only one.

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs` (the `MergeQlora` variant)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:455-466`

**Interfaces:**
- Produces: `--gguf-out <PATH>` and `--llama-cpp <DIR>` on `vox mens merge-qlora`; `pub fn llama_cpp_commands(llama_cpp: &Path, merged_dir: &Path, gguf_out: &Path, quant: &str) -> (Vec<String>, Vec<String>)` returning the two argv vectors, in `merge_qlora.rs`.

### Locating llama.cpp: a CLI flag, not an env var — and why

The lane needs a cloned and built llama.cpp (`convert_hf_to_gguf.py` + the `llama-quantize` binary) and a way to find it. Two options, and they are not close in cost:

**A new env var `VOX_LLAMA_CPP_ROOT` costs four gated artifacts.** Verified: the name is absent from `contracts/config/registry.v1.yaml`, `contracts/config/env-vars.v1.yaml` **and** `contracts/config/config-registry-baseline.txt`. Adding it to the baseline breaks the **exact-count assertion** in `crates/vox-cli/tests/config_hygiene_baseline_counts.rs` and the **monotonic cap** in `crates/vox-cli/tests/config_gates_monotonic.rs` — both of which must then be bumped, in a PR whose subject is GGUF export. That is four files of config-governance churn to pass one path.

**A CLI flag on an already-registered command costs nothing.** `merge-qlora` already has rows in the command registry, the operations catalog and the generated surface doc, and those rows record `path` / `status` / `feature_gate` — **not** individual flags. A flag addition is expected to regenerate nothing; the §CLI-surface gate step confirms rather than predicts.

**Use the flag.** It is also the better interface: the llama.cpp checkout is a per-invocation input like `--base-shard`, not an ambient machine property, and a flag makes an unset value a clear clap error instead of a silent fallback.

### The house rule on glue scripts does not apply here — state this in review

`AGENTS.md` §VoxScript-First Glue Code bans new `.py` / `.sh` / `.ps1` **glue authored by us**. This task authors none. It *invokes* llama.cpp's own `convert_hf_to_gguf.py` as a subprocess, exactly as the rest of the workspace invokes `ollama`, `cargo`, `pnpm` and `rg`. Rewriting a third-party GGUF converter in VoxScript to satisfy a rule about our own automation would be a straightforward misreading. **Say this in the PR description** so a reviewer does not flag it and so the next agent does not "fix" it.

Subprocess calls go through the workspace's existing process primitives where available; `std::process::Command` is acceptable here and is what the surrounding `vox-ml-cli` code already uses.

- [ ] **Step 0:** Read the Executor Preamble at the top of this file. **Depends on Task 6**, which introduces `--keep-merged` and `finish_recombined`; this lane consumes the directory that flag preserves.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod gguf_lane_tests {
    use super::*;

    /// Catches: pointing the converter at the merged .safetensors FILE
    /// instead of the directory. convert_hf_to_gguf.py takes a model
    /// directory (it reads config.json and the tokenizer beside the
    /// weights); handed a file it exits with a usage error that reads like
    /// a missing dependency.
    #[test]
    fn the_converter_is_given_the_directory_and_the_quantizer_the_file() {
        let (convert, quantize) = llama_cpp_commands(
            Path::new("/opt/llama.cpp"),
            Path::new("/tmp/merged/recombined_full"),
            Path::new("/tmp/out/model-q4_k_m.gguf"),
            "Q4_K_M",
        );
        assert!(convert.iter().any(|a| a == "/tmp/merged/recombined_full"),
            "converter must take the model directory: {convert:?}");
        assert!(convert.iter().any(|a| a.ends_with("convert_hf_to_gguf.py")),
            "{convert:?}");
        assert!(!convert.iter().any(|a| a.ends_with(".safetensors")),
            "the converter takes a directory, never a weights file: {convert:?}");

        assert!(quantize.iter().any(|a| a.ends_with("llama-quantize")), "{quantize:?}");
        assert!(quantize.last().is_some_and(|a| a == "Q4_K_M"),
            "llama-quantize takes the type as its LAST argument: {quantize:?}");
    }

    /// Catches: emitting the f16 intermediate at the final --gguf-out path.
    /// llama-quantize reads one gguf and writes another; if both are the
    /// same path it truncates its own input and produces a corrupt file
    /// with a zero exit status on some builds.
    #[test]
    fn the_intermediate_and_the_final_gguf_are_different_paths() {
        let (convert, quantize) = llama_cpp_commands(
            Path::new("/opt/llama.cpp"),
            Path::new("/tmp/merged/recombined_full"),
            Path::new("/tmp/out/model-q4_k_m.gguf"),
            "Q4_K_M",
        );
        let intermediate = convert.iter().find(|a| a.ends_with(".gguf"))
            .expect("converter writes a gguf: {convert:?}");
        assert_ne!(intermediate.as_str(), "/tmp/out/model-q4_k_m.gguf",
            "the f16 intermediate must not be the final output path");
        assert!(quantize.iter().any(|a| a == intermediate),
            "llama-quantize must read the intermediate the converter wrote: {quantize:?}");
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
timeout 900s cargo test -p vox-ml-cli --features gpu --lib the_converter_is_given_the_directory
```

Expected: **compile error** — `cannot find function 'llama_cpp_commands' in this scope`.

- [ ] **Step 3: Implement `llama_cpp_commands`**

Pure argv construction, no process spawning — that is what makes it testable:

```rust
/// The two argv vectors for the llama.cpp GGUF lane: convert the merged
/// SafeTensors *directory* to an f16 gguf, then quantize that intermediate
/// to `quant`.
///
/// This lane exists because `ollama create` refuses Qwen3ForCausalLM (MENS's
/// default base) and its --experimental converter cannot emit k-quants. For
/// llama/gemma2 bases the Ollama lane is cheaper; see `--keep-merged`.
///
/// `convert_hf_to_gguf.py` is llama.cpp's own script, invoked as a
/// third-party tool. AGENTS.md's VoxScript-first rule bans glue *we author*;
/// it does not require reimplementing someone else's converter.
#[must_use]
pub fn llama_cpp_commands(
    llama_cpp: &Path,
    merged_dir: &Path,
    gguf_out: &Path,
    quant: &str,
) -> (Vec<String>, Vec<String>) {
    let intermediate = gguf_out.with_extension("f16.gguf");
    let convert = vec![
        "python3".to_string(),
        llama_cpp.join("convert_hf_to_gguf.py").display().to_string(),
        merged_dir.display().to_string(),
        "--outfile".to_string(),
        intermediate.display().to_string(),
        "--outtype".to_string(),
        "f16".to_string(),
    ];
    let quantize = vec![
        llama_cpp.join("build/bin/llama-quantize").display().to_string(),
        intermediate.display().to_string(),
        gguf_out.display().to_string(),
        quant.to_string(),
    ];
    (convert, quantize)
}
```

**Verify the `llama-quantize` path layout against the checkout you test with** before hardcoding `build/bin/` — llama.cpp's build output location has moved between releases. If it differs, fix the constructor and say so in your report; do not add a search path list.

- [ ] **Step 4: Thread the flags and run the subprocesses**

Add to the `MergeQlora` variant:

```rust
        /// Write a quantized GGUF here via llama.cpp. Required for Qwen3
        /// bases, which `ollama create` refuses. Implies --keep-merged.
        #[arg(long)]
        gguf_out: Option<PathBuf>,
        /// Path to a cloned and built llama.cpp checkout (provides
        /// convert_hf_to_gguf.py and llama-quantize). Required with
        /// --gguf-out. A flag rather than an env var: the checkout is a
        /// per-invocation input, and a new VOX_* var would need rows in
        /// registry.v1.yaml, env-vars.v1.yaml and config-registry-baseline.txt
        /// plus bumps to two exact-count config gates.
        #[arg(long, requires = "gguf_out")]
        llama_cpp: Option<PathBuf>,
```

In `dispatch.rs:455-466`, destructure and pass both through. In `run_merge_qlora`, after `finish_recombined` (Task 6) and only when `gguf_out.is_some()`:

1. `anyhow::bail!` if `llama_cpp` is `None` — clap's `requires` covers the reverse direction only.
2. `anyhow::bail!` with a clear message if `llama_cpp.join("convert_hf_to_gguf.py")` or the `llama-quantize` binary is missing, naming the path checked. A missing checkout must not surface as a `python3: can't open file` deep in stderr.
3. Force `keep_merged = true` when `gguf_out.is_some()` — the converter needs the directory that flag preserves.
4. Run both commands with `std::process::Command`, inheriting stdio, and `bail!` on a non-zero status naming which of the two failed.
5. Delete the f16 intermediate on success; leave it on failure so the user can retry the quantize step alone.

- [ ] **Step 5: Run the gates**

```bash
timeout 900s cargo test -p vox-ml-cli --features gpu --lib gguf_lane_tests   # 2 passed
timeout 900s cargo run -p vox-cli -- ci command-sync
git status --short contracts/ docs/src/reference/
```

Stage anything `command-sync` regenerates, per §CLI-surface gate.

- [ ] **Step 6: Acceptance criterion (manual) — this is the Qwen3 route**

```bash
timeout 2700s cargo run -p vox-ml-cli --features gpu --release -- mens merge-qlora \
  --base-shard <Qwen3 base shard 1> --base-shard <...> \
  --adapter <adapter.safetensors> --meta <meta.json> \
  --output /tmp/merged/merged.safetensors \
  --keep-merged --gguf-out /tmp/out/mens-q4_k_m.gguf --llama-cpp <llama.cpp checkout>
ls -l /tmp/out/mens-q4_k_m.gguf
<llama.cpp checkout>/build/bin/llama-cli -m /tmp/out/mens-q4_k_m.gguf -p "write a vox component" -n 64
```

Expected: a GGUF file exists and a non-Vox runtime generates from it. That is the whole point of this plan — **a file Vox can read is not the goal**. Use a small Qwen3 checkpoint until Tasks 8, 9 and 10 have landed; Task 10's `plan_quantize` tells you in advance whether a given one fits.

If llama.cpp is not available on this host, **report BLOCKED with that reason** — do not mark the task done on unit tests alone. The argv tests prove the command shape; only a real run proves the lane.

- [ ] **Step 7: Commit**

```bash
cargo fmt -p vox-ml-cli
git add crates/vox-ml-cli/src contracts/ docs/src/reference/
git commit -m "$(cat <<'EOF'
feat(mens): GGUF export via llama.cpp, for the bases Ollama refuses

Restores a lane an earlier plan dropped on a false premise. ollama
0.33.3 rejects Qwen3ForCausalLM outright ("unsupported architecture")
and its --experimental converter supports only int4/int8/nvfp4/mxfp4/
mxfp8 -- no k-quants. MENS's default base is Qwen/Qwen3-8B, so for the
model this workspace actually trains there was no publish route at all.

llama.cpp is located by --llama-cpp rather than a new VOX_* env var: the
checkout is a per-invocation input, and a new var would need rows in
registry.v1.yaml, env-vars.v1.yaml and config-registry-baseline.txt plus
bumps to the exact-count assertion in config_hygiene_baseline_counts.rs
and the monotonic cap in config_gates_monotonic.rs -- four gated
artifacts to pass one path. The flag lands on an already-registered
command and touches none of them.

convert_hf_to_gguf.py is llama.cpp's own script, invoked as a
third-party tool. AGENTS.md bans new glue scripts *we author*; it does
not ask us to reimplement someone else's converter.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: Verify the published artifact on the machines that will run it

The premise of this plan is that a MENS checkpoint runs somewhere other than the Mac that trained it. That is unproven until it does.

**Files:**
- Create: `docs/src/architecture/mens-cross-machine-verification-2026.md`

> **Do not add a `research-index.md` row.** That index was retired in `0155372b2` and archived; `AGENTS.md` on main says explicitly not to create or edit it. Valid frontmatter is sufficient — Starlight's `docs-astro/src/utils/sidebar.mjs` lists the page automatically. (An earlier draft of this task said otherwise, copied from an `AGENTS.md` that was 175 commits stale.)

**Hardware preconditions — check before scheduling this task:**

| host | role | status as of 2026-09-11 |
|---|---|---|
| `blaptop04` | Windows, Tailscale, Ollama target | **online**; SSH key auth rejected |
| `bdesktop` | 4080 Super, CUDA lane | **offline, last seen 67 days ago** |
| `bdesktop2` | 4080 Super, CUDA lane | **offline, last seen 28 days ago** |

- [ ] **Step 0:** Read the Executor Preamble at the top of this file.

- [ ] **Step 1: Unblock SSH access — OPERATOR ACTION ON ANOTHER MACHINE**

> ### STOP HERE AND REPORT
>
> **An agent cannot run this step and must not attempt to work around it.** It requires an elevated PowerShell session on the physical console of `blaptop04`, a different machine. Do not attempt SSH workarounds, do not try alternate credentials, do not probe other ports or hosts, and do not mark this task complete without it. Report `STATUS: BLOCKED — Step 1 is an operator action on blaptop04` and hand the block back.

SSH to `blaptop04` is blocked and the fix is on the Windows side. The key is offered and refused (`SHA256:cNnY8WySWpBo9OqyedgG7wGd9VL6+PKbO+jYuM7l/7A` → `Permission denied (publickey,password,keyboard-interactive)`). On Windows OpenSSH an account in the Administrators group does **not** read `~/.ssh/authorized_keys`; the stock `sshd_config`'s `Match Group administrators` block redirects to `C:\ProgramData\ssh\administrators_authorized_keys`, which must additionally be ACL'd to SYSTEM and Administrators only or sshd ignores it silently.

For the operator, on `blaptop04`, in an elevated PowerShell:

```powershell
$k = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPICgyyW9Mv59rlFcWnKZCBPqxOnsHD+RAUNJlQA0KJd brbrainerd@gmail.com'
$f = "$env:ProgramData\ssh\administrators_authorized_keys"
Add-Content -Path $f -Value $k
icacls $f /inheritance:r /grant 'SYSTEM:F' /grant 'BUILTIN\Administrators:F'
Restart-Service sshd
```

Confirm from the Mac with `ssh blaptop04 "hostname"`. If it still refuses, `Get-Content $env:ProgramData\ssh\logs\sshd.log -Tail 20` names the reason — usually the ACL, not the key.

- [ ] **Step 2: Establish what `blaptop04` can actually run**

Over SSH, record GPU model, VRAM, driver version, and whether Ollama is installed. **Do not assume a size.** The quant tier that fits follows from measured VRAM through Task 10's `plan_quantize` and the memory model in `2026-09-11-1-memory-ssot-and-fit-benchmark.md`, not from a guess.

- [ ] **Step 3: Publish, transfer, and run — through the lane the base architecture dictates**

Read the base's `architectures` value first and pick the lane:

- **llama / gemma2 base:** the Task 6 artifact, `ollama create` on the target.
- **Qwen3 base (MENS default):** the Task 12 GGUF. `ollama create -f` with a `FROM <file>.gguf` Modelfile accepts a prebuilt GGUF without invoking the architecture-gated converter — confirm that on the target and record the exact command that worked.

Then assert on behavior rather than exit status:

```bash
ssh blaptop04 "ollama run vox-mens '<a prompt whose correct answer is Vox-specific>'"
```

*What this catches:* a base model wearing the adapter's name. A generic prompt cannot distinguish "the adapter loaded" from "the adapter was silently skipped" — the most common failure in a hub-and-spoke setup, and one no size or memory check can see. Use a prompt the base model demonstrably gets wrong.

- [ ] **Step 4: Record, including what failed**

Write the doc with valid frontmatter (`title`, `description`, `category`, `status`). Record measured VRAM, the quant tier and why, the lane used and the base architecture that dictated it, tokens/sec, and the prompt-level result. **Record unreachable hosts as unverified** — an untested CUDA lane must never be described as working.

- [ ] **Step 5: Commit**

```bash
git add docs/src/architecture/mens-cross-machine-verification-2026.md
git commit -m "docs(mens): cross-machine verification of the published artifact

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

> **The CUDA lane cannot be verified while both 4080 Super hosts are offline.** Do not mark it working on the strength of Metal results — different hardware, different allocator, different kernels. The memory model returns `Uncalibrated` for CUDA by design, and that is the honest state until a machine is available.

---

## Sequencing

```
Task 1  (read.rs: header enumerate + shard cache)  — FIRST. Tasks 8, 10, 11 build on it.
Task 2  (handoff honesty)                          — independent
Task 3  (Ollama config SSOT)                       — independent
Task 4  (tool-calling on unset OLLAMA_URL)         — independent
Task 5  (/api/embed)                               — independent
Task 6  (Ollama publish lane)                      — independent; owns dispatch.rs:463 alone
Task 7  (delete ollama_subprocess)                 — independent
Task 8  (stream recombine)                         — after 1
Task 9  (stream ArtifactWriter)                    — independent of 8; BOTH needed for the 27B
Task 10 (plan_quantize; delete 3 estimators)       — after 1
Task 11 (refuse MLX input)                         — after 1
Task 12 (llama.cpp GGUF lane)                      — after 6 (consumes --keep-merged)
Task 13 (cross-machine verification)               — after 6 and 12; Step 1 is BLOCKED on an operator
```

**Parallel width:** seven immediately (2, 3, 4, 5, 6, 7, 9 are file-disjoint and independent of Task 1). After Task 1, add 8, 10 and 11 — but **10 and 11 both touch `read.rs`'s header reader**, so serialize those two, Task 10 first (it is the larger change to that function's shape).

**The capacity gate.** Tasks 8 **and** 9 must both land before any acceptance criterion is run against the 27B. Task 10 tells you in advance whether a given checkpoint fits, which is how you decide whether to try.

**Deliberate coupling, stated so it cannot rot silently.** Task 6 Step 5 writes the Task 12 route (`--gguf-out` / `--llama-cpp`) into `export-gguf`'s error message pre-emptively, so that Tasks 6 and 12 never edit the same match arm. That is the right trade — a file collision between two tasks is worse than a forward reference — but it has one failure mode: **if Task 12 is dropped or deferred, that error message names a flag that does not exist**, which is exactly the class of lying-documentation defect Task 2 exists to fix.

So: **Task 12 is not optional while Task 6 is landed.** If Task 12 is cut, the executor must return to Task 6 Step 5 and replace the flag names with a plain "not implemented" before shipping. Add a guard so this cannot pass unnoticed:

```rust
#[test]
fn the_export_gguf_error_only_names_flags_that_exist() {
    let msg = export_gguf_not_implemented_message();
    for flag in ["--gguf-out", "--llama-cpp"] {
        if msg.contains(flag) {
            assert!(
                MergeQloraArgs::command().get_arguments().any(|a| a.get_long() == Some(&flag[2..])),
                "the error message advertises {flag}, which no longer exists — Task 12 was cut \
                 or renamed its flags, and this message is now the same kind of lie Task 2 fixed"
            );
        }
    }
}
```

*Mutation caught:* cutting Task 12, or renaming its flags, while leaving Task 6's message in place. Use the real clap introspection API for this crate — read `action_populi_enum.rs` and adapt; do not assume `MergeQloraArgs::command()` is the right handle if the variant is inline on a larger enum.

## Self-review

**What the two superseded plans got right and this one keeps.** Plan B's read-amplification finding (Task 1) and its correct refusal to write a GGUF *container by hand*. Plan C's handoff-honesty finding (Task 2), its four routing defects (Tasks 3–5), its merge-qlora publish insight (Task 6), its stub deletion (Task 7), and two of its three rejection arguments (port collision, existing provider adapter).

**What this revision reversed, and why.** The previous draft's central claim — "Ollama's own converter reads a merged SafeTensors directory and does its own GGUF conversion and quantization, so Vox ships no GGUF writer and no third-party subprocess" — was tested against ollama 0.33.3 and is false for `Qwen3ForCausalLM`, which is MENS's default base. The llama.cpp lane is restored as Task 12; the Ollama lane survives, scoped in Task 6 to llama and gemma2 bases; and both acceptance criteria that previously ran `ollama create -q q4_K_M` against an unspecified "small checkpoint" now state which architecture they require and stop if the checkpoint is Qwen3.

**What was added.** Task 1 grew a second half — `open`'s no-index branch loaded the whole checkpoint to read `st.keys()`, which the one-slot cache does not fix. Task 9 (stream `ArtifactWriter`) — the quantized output was fully resident before anything reached disk, a cost Task 8 does not touch. Task 10 (`plan_quantize`) — replaces three estimators, one of which is 3.4 % optimistic on its own source checkpoint, with a header walk containing no fitted parameter. Task 11 (refuse MLX input) — the only failure mode in this plan that produces a *finite* verification error on garbage. Task 6 grew a fourth test, because its three renderer tests were all pure string checks that stay green against reverting the entire `--keep-merged` feature.

**What was corrected.** The `OLLAMA_DEFAULT_URL` count (one use in `provider.rs`, not two; a third site at `ctor.rs:275`; the const at `constants.rs:6`). The `merge-qlora` flag spelling — one spelling throughout, `vox mens merge-qlora --base-shard/--adapter/--meta/--output/--quantize`; there is no `vox schola merge-qlora` command and no `--base` / `--out`. Task 5's placeholder `MensError` variant is now named: `MalformedResponse`, one of exactly four. Task 8's `write_fake_base` is defined inline in a Step 0 instead of being called into existence. Every long command carries a `timeout`. Task 13 Step 1 carries an explicit STOP.

**What was deleted and why.** Plan B Task 0's `AGENTS.md` rewrite (governance change, human-authorized only). Plan B Task 4 (handoff quant recipes — a JSON-schema contract change to ship three command strings). Plan B Task 5 (MLX as a *lane* — Candle-Metal already serves Macs; Task 11 handles MLX as an *input*, which is a correctness bug, not a lane). Plan B Task 6 (**entirely stale**: `0.303` is already in the tree, `VoxMixture` and `.label()` never existed — and Task 10 deletes the constant outright, which is better than correcting it). Plan C Task 3 Step 4's duplicate ownership of `dispatch.rs:463` (Task 6 owns it alone, and writes the Task 12 route into that message pre-emptively so the two never collide). Plan C's `include_str!` cross-crate assertions (paths did not resolve). Plan C's `ollama_can_convert` architecture gate — still rejected, and the reason is now stronger, not weaker: the split is real and documented above, but Ollama's enumeration is upstream and `ollama create` reports it itself.

**Mutation coverage.** Task 1: revert `load_f32` to per-tensor loads → count 4 ≠ 2; revert `open` to a full load → `shard_loads()` 1 ≠ 0. Task 2: restore the notes string → substring asserts fail. Task 3: restore the hardcoded URL → env-driven assert fails. Task 4: restore the `?` → `expect` panics. Task 5: revert either the request field or the response shape → compile/deserialize failure. Task 6: `FROM model.safetensors`, drop the TEMPLATE, emit `ADAPTER`, **or revert the whole `--keep-merged` feature** → each asserted separately. Task 7: none — deletion, compiler-enforced, stated. Task 8: single `save` → no index; a dropped boundary tensor → weight_map gap. Task 9: infinite budget → one shard, no index. Task 10: a role-fraction estimator → disagrees with the block arithmetic on a non-Qwen3 fixture; a dropped peak term → inequality fails. Task 11: accept the U32 checkpoint → `expect_err` panics. Task 12: converter given a file → argv assert; intermediate == output → path assert. Task 13: behavioral prompt, not exit status.

**Placeholder scan.** Two, both deliberate and both bounded by a verification step: Task 9 Step 3 leaves the `new()`-vs-`with_shard_budget` shape to the executor ("pick the shape that keeps `engine.rs` unchanged and say which you chose"), because the right answer depends on the final `engine.rs` call sequence; and Task 12 Step 3's `build/bin/llama-quantize` path is flagged for verification against the actual checkout rather than asserted. Every other code step is complete.

---

## Verification log

All commands run in `/Users/brbrainerd/dev/vox-audit2` at `7295b4470` (`git log --oneline -1`), **read-only**. Rows above the rule are carried from the previous revision and were independently spot-checked ~25 deep with zero false entries; rows below the rule are new to this revision.

| Cited symbol / path / flag | Command | Result |
|---|---|---|
| `ExternalServingHandoffV1::schola_training_run(&Path, &str, &str)` | `rg -n "pub fn (schola_training_run\|merged_qlora_subset\|for_base\|for_run)" crates/vox-populi/src/mens/tensor/external_serving_handoff.rs` | `:30` and `:51` only; **no** `for_base`, **no** `for_run` |
| `ExternalServingHandoffV1::merged_qlora_subset(&Path, &str, Option<&str>)` | same | `:51` |
| handoff file lives in `mens/tensor/`, not `mens/serving/` | `find . -name external_serving_handoff.rs -not -path "./target/*"` | `vox-populi/src/mens/tensor/`, `vox-plugin-mens-candle-cuda/src/`, `vox-plugin-mens-candle-metal/src/` |
| `vox-schola` in three handoff copies | `rg -n "vox-schola serve" crates/*/src/**/external_serving_handoff.rs` | `vox-populi …:43,:74` (+ doc comment `:28`); cuda `:43`; metal `:43` |
| `pub mod external_serving_handoff` under `mens` | `rg -n external_serving_handoff crates/vox-populi/src/mens/tensor/mod.rs` | `:59` |
| serve routes (`/health`,`/ready`,`/v1/models`,`/v1/generate`,`/generate`,`/v1/completions`) | `rg -n "route\(" crates/vox-ml-cli/src/commands/ai/serve/mod.rs` | `:116-123`; **no** `/api/*` route anywhere |
| `mens-serving-ssot.md` route list `:16-21`, claims `:32,:36,:52` | `grep -n "api/chat\|api/generate\|api/tags\|api/version\|api/embeddings\|v1/chat/completions" docs/src/reference/mens-serving-ssot.md` | `:16,17,18,19,20,21,32,36,52` |
| `how-to-model-routing.md:71,226` precedence reversed | `grep -n local_ollama_populi_base_url docs/src/how-to/how-to-model-routing.md` | `:66,:71,:226`; `:71`/`:226` state `OLLAMA_URL → POPULI_URL → default` |
| real precedence `VOX_POPULI_LOCAL_OLLAMA_URL → POPULI_URL → OLLAMA_URL → default` | `sed -n '220,244p' crates/vox-config/src/inference.rs` | `:222` doc + `:225-244` body |
| `policy.rs:109` is `0.303`, not `0.22` | `rg -n "QWEN3_27B_BOOSTED_ROLE_FRACTION\|0\.303\|0\.22" crates/vox-quantize/src/` | `:109 = 0.303`; **zero** hits for `0.22` |
| type is `QuantMixture`, no `.label()`, `fits_target_tier(&QuantMixture)` | `rg -n "fits_target_tier" crates/vox-quantize/src/` | `lib.rs:18`, `policy.rs:147` — `mixture: &QuantMixture` |
| `quantize` CLI flags are `--input` / `--output` / `--to` | `sed -n '1,35p' crates/vox-ml-cli/src/commands/quantize.rs` | `input`, `output`, `to`, `no_verify`, `device`, `json`, `target_vram_gib` — **no** `--in`/`--out` |
| `ExportGguf { input, output }`, both `required = true`, **no** `--quant` | `rg -n -A 14 ExportGguf crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs` | `:427-434` |
| `export-gguf` unregistered | `rg "export-gguf\|ExportGguf" contracts/ docs/src/reference/` | zero matches |
| `merge-qlora` registered | `rg -n "merge-qlora" contracts/cli/command-registry.yaml contracts/operations/catalog.v1.yaml docs/src/reference/cli-command-surface.generated.md` | `:2193`, `:7726/:7745`, `:211` |
| `merge_qlora.rs:200` is the **pre-run** clear; `:219` is the artifact delete | `grep -n 'remove_dir_all\|pub fn run_merge_qlora\|let base_dir' crates/vox-ml-cli/src/commands/schola/merge_qlora.rs` + `sed -n '185,235p'` | `:200` clear before `recombine()`; `:205` clears `quantized/`; `:219` after quantize; `run_merge_qlora` at `:75`; `base_dir` bound at `:123` |
| `recombine` writes `model.safetensors` + `config.json` | `cat -n crates/vox-quantize/src/recombine.rs` | `:47-52` |
| `recombine` builds whole model in RAM (known ceiling) | same | `:28-45` `HashMap<String, Tensor>` then `save` at `:48` |
| `recombine` signature is `(&Path, &Path, &Path)` — `merged_subset` is a PATH | same | `:10-14`; `merged` loaded at `:16` |
| `recombine` name-membership and dim checks | same | `:20-26` (absent key), `:35-39` (dim mismatch) |
| `load_f32` per-tensor `safetensors::load`; `names` from `HashMap::keys()` | `cat -n crates/vox-quantize/src/read.rs` | `:53-63` and `:44` |
| only `SafeTensorsSource` construction sites | `rg -n SafeTensorsSource crates/` | `engine.rs:27`, `recombine.rs:15`, tests only |
| `vox-quantize` has no `anyhow`, no `mens` feature | `cat crates/vox-quantize/Cargo.toml` | deps: candle-core, safetensors, serde, serde_json, thiserror, tracing; features: `cuda`, `metal` |
| `QuantizeError` variants | `cat -n crates/vox-quantize/src/error.rs` | `ReadModel`, `UnsupportedDtype`, `ShardIndex`, `Quantize`, `Write`, `VerifyFailed`, `Io` |
| `agent_loop.rs:71-76` returns `None` on empty secret | `sed -n '55,90p' crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | `:72-75`, `?` on the `.map(...)` |
| `model_spec(...)` test helper exists | `sed -n '1370,1400p'` same file | `:1375` |
| `SecretId::OllamaUrl`, `SecretId::VoxPopuliLocalOllamaUrl` | `rg -n OllamaUrl crates/vox-secrets/src/spec/registry/llm.rs` | `:381`, `:392` |
| `vox_config::snapshot::bump`, `pub mod inference`/`snapshot` | `rg -n "pub mod inference\|pub mod snapshot" crates/vox-config/src/lib.rs` ; `rg -n "pub fn bump" crates/vox-config/src/snapshot.rs` | `lib.rs:17,31`; `snapshot.rs:92` |
| `vox-config` dep present in the 4 consumer crates | `rg -n "^vox-config" crates/{vox-orchestrator-mcp,vox-gamify,vox-code-audit,vox-actor-runtime}/Cargo.toml` | `:87`, `:28`, `:30`, `:23` — **no** new crate edge |
| `vox-code-audit` hardcoded URLs | `rg -n "localhost:11434" crates/vox-code-audit/src/` | `ai_analyze.rs:30,59,320,325,365`; `review/providers.rs:48,88` |
| `mens.rs` uses `/api/embeddings`, `prompt`, `embedding` | `sed -n '110,135p;210,235p' crates/vox-actor-runtime/src/mens.rs` | `EmbedRequest.prompt` `:119`; `EmbedResponse.embedding` `:127`; `/api/embeddings` `:222` |
| `ollama_subprocess` stub echoes its input | `cat crates/vox-populi/src/inference/backends/ollama_subprocess.rs` | `predict` → `format!("[ollama stub] {}", prompt.text)` |
| `OllamaSubprocess` referents | `rg -n OllamaSubprocess crates/vox-populi/src/` | `inference/mod.rs:20`, `inference/backend.rs:38`, `backends/mod.rs:15`, the file itself |
| `vox-ml-cli` features (`gpu` implies `quantize`) | `sed -n '/^\[features\]/,/^\[/p' crates/vox-ml-cli/Cargo.toml` | `gpu = [… "quantize"]`; `quantize = ["dep:vox-quantize"]` |
| **— new to this revision —** | | |
| `ollama create` rejects `Qwen3ForCausalLM` | `ollama create probe -q q4_K_M -f Modelfile` against a fixture whose `config.json` declares `architectures:["Qwen3ForCausalLM"]`, ollama 0.33.3 | `Error: unsupported architecture "Qwen3ForCausalLM"` |
| the experimental converter cannot emit k-quants | `ollama create probe --experimental -q q4_K_M -f Modelfile` | `Error: unsupported --quantize "q4_K_M": supported types are int4, int8, nvfp4, mxfp4, mxfp8` |
| `LlamaForCausalLM` / `Gemma2ForCausalLM` pass the arch check | same command, arch swapped in `config.json` | proceeds past the architecture gate; fails later on the fixture's stub tokenizer (expected) |
| **the real `MergeQlora` flags** — `--base-shard` (Vec), `--adapter`, `--meta`, `--output`, `--quantize`; **no** `--base`, **no** `--out` | `rg -n -A 30 'MergeQlora' crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs` | `:404-421` |
| **there is no `vox schola merge-qlora` command** — `schola` is a module path only | `rg -n 'merge-qlora\|MergeQlora' crates/vox-ml-cli/src/commands/schola/ crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` | `dispatch.rs:455` dispatches `PopuliAction::MergeQlora`; `schola/merge_qlora.rs:1` doc comment is the only "schola merge-qlora" string, and `dispatch.rs:467` already spells the user-facing command `vox mens merge-qlora` |
| `run_merge_qlora` signature (5 params, no `keep_merged`) | `sed -n '75,82p' crates/vox-ml-cli/src/commands/schola/merge_qlora.rs` | `(base_shards: Vec<PathBuf>, adapter, meta, output, quantize: Option<String>) -> anyhow::Result<()>` |
| `MensError` has **exactly four** variants | `sed -n '15,35p' crates/vox-actor-runtime/src/mens.rs` | `Http(#[from] reqwest::Error)`, `ModelNotAvailable(String)`, `RateLimited { retry_after_ms: u64 }`, `MalformedResponse(String)` |
| `OLLAMA_DEFAULT_URL` — **five** sites, not three | `rg -n 'OLLAMA_DEFAULT_URL' crates/vox-gamify/src/` | `constants.rs:6` (the const), `provider.rs:43` (**one**, not two), `client/ctor.rs:49`, `:51`, `:275` |
| `read.rs:33-37` loads the whole file to enumerate names | `cat -n crates/vox-quantize/src/read.rs` | `:34 candle_core::safetensors::load(&single, ..)` then `:35 for name in st.keys()` |
| `write.rs` buffers the entire output before `finish` | `cat -n crates/vox-quantize/src/write.rs` | `:31 raw: HashMap<String,(Vec<u8>,DType,Vec<usize>)>`; inserts at `:61`, `:82`; single `save` at `:111` inside `finish` (`:87`) |
| `write_fake_base` **does not exist**; the only fixture helper is `write_st` | `rg -n 'fn write_fake_base\|fn write_st' crates/` | zero hits for `write_fake_base`; `write_st(dir, name, &[(&str, Tensor)])` at `read.rs:72`, private to `read.rs`'s test mod |
| `verify` holds 4 live F32 tensors at peak (the `c` in the peak formula) | `sed -n '28,42p' crates/vox-quantize/src/verify.rs` | `round_trip_mse`: `src`, `deq`, `diff`, `sq`; `round_trip_max_abs`: `src`, `deq`, `diff` |
| `estimate_params_b` assumes bf16 | `sed -n '34,48p' crates/vox-ml-cli/src/commands/quantize.rs` | `:47 total_bytes as f64 / 2.0 / 1e9`, doc comment says "assuming a BF16 source" |
| `fits_target_tier` documents its own inaccuracy | `sed -n '131,152p' crates/vox-quantize/src/policy.rs` | `:137-146` "the estimate drifts… can pass a model that does not actually fit… use `mixture_bpw` directly with a measured fraction" |
| `VOX_LLAMA_CPP_ROOT` absent from all three config artifacts | `rg -in 'llama_cpp\|LLAMA_CPP' contracts/config/ crates/vox-cli/tests/` | **zero hits** across `registry.v1.yaml`, `env-vars.v1.yaml`, `config-registry-baseline.txt` |
| the two config gates a new env var would break | `ls crates/vox-cli/tests/ \| grep -i config` | `config_gates_monotonic.rs`, `config_hygiene_baseline_counts.rs` present (plus five more hygiene gates) |
| the MLX checkpoint is on this host | `ls ~/.cache/huggingface/hub/ \| grep -i mlx` | `models--mlx-community--Qwen3.8-27B-8bit`, `models--mlx-community--Qwen3-0.6B-4bit` |
| `serde_json` is a direct dep of `vox-quantize` (header parsing needs no new dep) | `cat crates/vox-quantize/Cargo.toml` | `serde_json = { workspace = true }` |
| plans under `docs/superpowers/` are not frontmatter-linted | `rg -n 'docs/src' crates/vox-doc-pipeline/src/ \| head` + `head -12 docs/superpowers/plans/2026-05-23-durable-functions-completion.md` | the linter walks `docs/src/` only; existing plans start with `# …`. Frontmatter kept here anyway — harmless. |

**Could not verify (stated rather than asserted in the plan):**
- Whether `vox_secrets::resolve_secret(SecretId::OllamaUrl)` caches past a `remove_var`. Task 4 Step 2 carries an explicit contingency: if the test passes before the fix, re-run under `env -u …` and confirm the failure first.
- Whether `cargo run -p vox-cli -- ci command-sync` regenerates anything for a flag-only addition. Tasks 6 and 12 run it and stage whatever appears, rather than predicting.
- Ollama's upstream converter architecture enumeration (`convert/convert.go`) — not in this tree. The four rows above record what the **binary** does, which is what a user hits; no `ollama_can_convert` gate is planned.
- llama.cpp's current build-output path for `llama-quantize` (assumed `build/bin/`). Task 12 Step 3 flags this for verification against the actual checkout rather than asserting it.
- Whether `ollama create -f` with a `FROM <file>.gguf` Modelfile bypasses the architecture gate on a prebuilt GGUF. Task 13 Step 3 requires the executor to confirm and record the exact command that worked, rather than assuming it.
