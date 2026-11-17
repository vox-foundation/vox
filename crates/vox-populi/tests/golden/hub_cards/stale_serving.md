---
license: apache-2.0
base_model: Qwen/Qwen3.5-2B
base_model_relation: adapter
base_model_revision: 15852e8c16360a2fea060d615a32b45270f8a8fc
library_name: peft
pipeline_tag: text-generation
language:
- en
datasets:
- ns/vox-mens-eval-vox-lang-bench
- ns/vox-mens-data-vox-lang
spaces:
- ns/vox-mens-playground
tags:
- vox
- vox-mens
- qwen3-5
- qwen3_5
- gated-delta-networks
- qlora
- rslora
- candle
- code
- conversational
- spoke-vox-lang
- gate-needs-re-gate
---
# Vox Mens 2B — Lightweight Laptop Model for Vox Functions, Tables & Tests

**Vox Mens 2B** is [trained](https://voxlang.org/reference/mens-models/#state-legend) for [`vox-lang`](https://voxlang.org/reference/mens-models/#hub-and-spoke) (release check: [needs re-gate](https://voxlang.org/reference/mens-models/#state-legend)): it passed the retired 32-item collateral rule and the 128-item paired re-run for 2B has not been done.

![Vox Mens 2B architectural schematic](assets/banner.jpg)

*Diagram (2B): a task prompt runs locally on Windows, Linux, or Mac with low memory overhead to produce a working `.vox` function and `@test` block.*

Try **Vox Mens 2B** in the [Vox Mens Playground Space](https://huggingface.co/spaces/ns/vox-mens-playground?model=ns/vox-mens-qwen3.5-2b).

## What it does

**Vox Mens 2B** writes and fixes [Vox](https://voxlang.org) (`.vox`), a full-stack language where one file defines database tables, backend queries and mutations, web UI components, and workflows that compile to Rust and TypeScript. Because Vox is newer than mainstream languages, general-purpose AI models often guess Rust or TypeScript syntax that fails to compile; **Vox Mens 2B** is fine-tuned on verified `.vox` examples so you can generate and repair Vox locally on Windows, Linux, or Mac (`Ollama`, `vLLM`, Python, or `vox mens serve`) or through Hugging Face Inference Providers without paying large cloud API rates on every prompt.

## Why this size

Vox Mens 2B fits everyday laptops and entry-level GPUs across Windows, Linux, and Mac while keeping memory use light alongside your editor and browser. Use 2B for writing single Vox functions, database table definitions, and small self-contained unit tests. Drop to 0.8B when disk or RAM is tight, or move up to 4B and 9B when a file combines tables, queries, mutations, and UI views. Measured QLoRA peak memory during the 2B run was 27007 MiB (`run_summary`).

## Use it

This page currently distributes the **Vox Mens 2B** trained add-on while release review is unfinished, and the files are available from the Files tab or with this download command:

```bash
vox mens hub download --repo ns/vox-mens-qwen3.5-2b --output-dir mens/runs/qwen3.5-2b
```

`vox mens serve` loads the 2B add-on only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so no serving example is given while review is pending.
## Vox and Python

Beginners can write and run their first full-stack `.vox` file alongside **Vox Mens 2B** in under a minute, or call `ns/vox-mens-qwen3.5-2b` from Python and Rust.

**Try Vox in 60 seconds (2B)**

[Vox](https://voxlang.org) keeps your schema, backend queries, mutations, and helpers in one compiler-checked file so **Vox Mens 2B** can edit an entire feature coherently:

```vox
table Note {
    title: str
    done: bool
}

query list_notes() to Result[list[Note], str] {
    return db.Note.all()
}

mutation add_note(title: str) to Result[int, str] {
    return db.Note.insert({ title: title, done: false })
}

fn format_status(done: bool) to str {
    if done { return "done" }
    return "open"
}
```

Type-check and execute your program locally while **Vox Mens 2B** assists in the same workspace:

```bash
vox check main.vox
vox run --interp main.vox
vox llm prompt "Add an archive mutation to main.vox"
```

**Use 2B from Vox and Rust**

Because even `27B` is smaller than massive commercial cloud models, Vox projects commonly use a large cloud model for high-level planning while routing `.vox` file generation and `vox check` fixes to `ns/vox-mens-qwen3.5-2b` (locally or via Hugging Face Router) through `vox_actor_runtime::llm::llm_chat`:

```bash
export HF_CHAT_MODEL="ns/vox-mens-qwen3.5-2b"
export HF_TOKEN="hf_your_token_here"
export VOX_POPULI_LOCAL_OLLAMA_URL="http://127.0.0.1:11435"
```

```rust
use vox_actor_runtime::inference_env::resolve_huggingface_router;
use vox_actor_runtime::model_resolution::{RouteResolutionInput, resolve_chat_provider_route};

// Keep ambiguous cross-repo planning on your frontier cloud route, and delegate
// .vox handlers, tests, and `vox check` repair loops to ns/vox-mens-qwen3.5-2b:
let mens_specialist = resolve_huggingface_router("ns/vox-mens-qwen3.5-2b");
```

**Use 2B from Python**

Call `ns/vox-mens-qwen3.5-2b` with the standard `openai` Python SDK through Hugging Face Inference Providers or a local `vox mens serve` endpoint:

```python
import os
from openai import OpenAI

# Cloud inference via Hugging Face Router (:auto selects the fastest available host)
client = OpenAI(
    base_url="https://router.huggingface.co/v1",
    api_key=os.environ["HF_TOKEN"],
)
# Or target your local machine after running `vox mens serve`:
# client = OpenAI(base_url="http://127.0.0.1:11435/v1", api_key="local")

reply = client.chat.completions.create(
    model="ns/vox-mens-qwen3.5-2b:auto",
    messages=[{"role": "user", "content": "Write a Vox query that returns open Note rows."}],
)
print(reply.choices[0].message.content)
```

Manage Python dependencies inside a Vox project for **Vox Mens 2B** using `[python]` in `Vox.toml` and `vox py`:

```toml
# Vox.toml
[python]
version = "3.12"
packages = ["openai", "transformers", "peft"]
```

```bash
vox py sync
vox py doctor
vox py bind openai
```

Generate a typed Python client from a Vox service API built with **Vox Mens 2B** using OpenAPI:

```bash
vox emit openapi src/main.vox --out dist/openapi.json
openapi-python-client generate --path dist/openapi.json
```
## Examples

No 2B example is shown because the adapter checkpoint has no serving field, so its outputs do not count under the [serving-path rule](https://voxlang.org/reference/mens-models/#held-out-pass-rate).

```text
no recorded output
```

## Results

| Result for 2B | [Held-out pass@1](https://voxlang.org/reference/mens-models/#held-out-pass-rate) | [Collateral accuracy](https://voxlang.org/reference/mens-models/#collateral-damage) | [McNemar p-value](https://voxlang.org/reference/mens-models/#why-mcnemar) |
|---|---|---|---|
| Base 2B | 2/8 (25.0%) | — | — |
| Vox Mens 2B | — | — | — |

The 2B rows compare the unadapted base checkpoint with Vox Mens 2B on identical items. The adapter score for 2B is not shown because its checkpoint has no serving field, so it does not count under the [serving-path rule](https://voxlang.org/reference/mens-models/#held-out-pass-rate) and 2B needs a fresh evaluation.

The release gate record for 2B is [`gate_receipt.json`](https://huggingface.co/ns/vox-mens-qwen3.5-2b/blob/main/gate_receipt.json).

## Training

Vox Mens 2B fine-tunes base pin `Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` over 138 optimizer steps (2711427 tokens), ended with a final loss (EMA) of 0.6170, a mean train loss of 0.8323 and a last-epoch validation loss of 0.7721; the 2B run took 4h 10m and peaked at 27007 MiB from Vox commit `c56e13a33` (shared settings: [docs page](https://voxlang.org/reference/mens-models/#shared-training-settings)).

## Files

The 2B repository holds `peft/` Hugging Face PEFT adapter files alongside `candle_qlora_adapter.safetensors`, tokenizer and `chat_template.jinja`, `.eval_results/vox_mens_eval.yaml`, `external_serving_handoff_v1.json`, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.

This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/ns/vox-mens-index) lists the other sizes and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.
