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
- gate-passed
model-index:
- name: vox-mens-qwen3.5-2b
  results:
  - task:
      type: text-generation
      name: Vox code generation
    dataset:
      type: vox-mens-heldout-bench
      name: Vox held-out bench (8 items)
    metrics:
    - type: pass@1
      name: pass@1 (qwen3_non_thinking_codegen_v1)
      value: 0.6250
  - task:
      type: text-generation
      name: Reasoning regression check (thinking mode)
    dataset:
      type: vox-mens-gsm8k_think
      name: gsm8k_think (128-item slice)
    metrics:
    - type: accuracy
      name: gsm8k_think accuracy
      value: 0.1172
  - task:
      type: text-generation
      name: Reasoning regression check (thinking mode)
    dataset:
      type: vox-mens-math500_think
      name: math500_think (128-item slice)
    metrics:
    - type: accuracy
      name: math500_think accuracy
      value: 0.1172
  - task:
      type: text-generation
      name: Reasoning regression check (thinking mode)
    dataset:
      type: vox-mens-vox_think
      name: vox_think (128-item slice)
    metrics:
    - type: accuracy
      name: vox_think accuracy
      value: 0.1172
widget:
- example_title: "Workflow: fetch save"
  text: "Write a Vox program that: define a workflow named `fetch_and_save` that fetches data from a URL stored in a variable `url` and saves the result to a variable `data`"
  output:
    text: "fn fetch_data(url: str) to Result[str] {\n    return Ok(\"mock_data\")\n}\n\nfn save_data(data: str) to Result[str] {\n    return Ok(\"saved:\" + data)\n}\n\nworkflow fetch_and_save(url: str) to Result[str] {\n    let data = fetch_data(url)?\n    let result = save_data(data)?\n    return Ok(result)\n}"
- example_title: "Binary search"
  text: "Write a Vox program that: Search for target in a sorted list xs using binary search.\nReturn the index of target if found, or -1 if not found."
  output:
    text: "fn binary_search(xs: list[int], target: int) to int {\n    let mut low = 0\n    let mut high = len(xs) - 1\n    while low <= high {\n        let mid = (low + high) // 2\n        let val = xs[mid]\n        if val == target {\n            return mid\n        } else if val < target {\n            low = mid + 1\n        } else {\n            high = mid - 1\n        }\n    }\n    return -1\n}"
- example_title: "Matrix row sums"
  text: "Write a Vox program that: define a function named `matrix_row_sums` that takes a list of lists of integers `matrix` and returns a list of the sum of each row"
  output:
    text: "fn matrix_row_sums(matrix: list[list[int]]) to list[int] {\n    return matrix.map(fn(row: list[int]) to int { row.sum() })\n}"
---
# Vox Mens 2B — Lightweight Laptop Model for Vox Functions, Tables & Tests

**Vox Mens 2B** is [trained](https://voxlang.org/reference/mens-models/#state-legend) for [`vox-lang`](https://voxlang.org/reference/mens-models/#hub-and-spoke) (release check: [passed](https://voxlang.org/reference/mens-models/#state-legend)).

![Vox Mens 2B architectural schematic](assets/banner.jpg)

*Diagram (2B): a task prompt runs locally on Windows, Linux, or Mac with low memory overhead to produce a working `.vox` function and `@test` block.*

| Model | Held-out pass@1 (2B) | Weights (2B) | License | Live Demo |
|---|---|---|---|---|
| **Vox Mens 2B** | 2/8 (25.0%) → **5/8 (62.5%)** | PEFT + Candle Adapter | `apache-2.0` | [Open 2B Playground](https://huggingface.co/spaces/ns/vox-mens-playground?model=ns/vox-mens-qwen3.5-2b) |

## What it does

**Vox Mens 2B** writes and fixes [Vox](https://voxlang.org) (`.vox`), a full-stack language where one file defines database tables, backend queries and mutations, web UI components, and workflows that compile to Rust and TypeScript. Because Vox is newer than mainstream languages, general-purpose AI models often guess Rust or TypeScript syntax that fails to compile; **Vox Mens 2B** is fine-tuned on verified `.vox` examples so you can generate and repair Vox locally on Windows, Linux, or Mac (`Ollama`, `vLLM`, Python, or `vox mens serve`) or through Hugging Face Inference Providers without paying large cloud API rates on every prompt.

## Why this size

Vox Mens 2B fits everyday laptops and entry-level GPUs across Windows, Linux, and Mac while keeping memory use light alongside your editor and browser. Use 2B for writing single Vox functions, database table definitions, and small self-contained unit tests. Drop to 0.8B when disk or RAM is tight, or move up to 4B and 9B when a file combines tables, queries, mutations, and UI views. Measured QLoRA peak memory during the 2B run was 27007 MiB (`run_summary`).

## Use it

Download the BLAKE3-verified **Vox Mens 2B** adapter (`hf_sync_manifest.json`) with `vox mens hub pull`:

```bash
vox mens hub pull --size 2B --namespace ns
```

The pull writes the 2B run directory `mens/runs/qwen3.5-2b` that this command serves:

```bash
vox mens serve --model mens/runs/qwen3.5-2b --port 11435
```

Or load the **Vox Mens 2B** PEFT adapter onto `Qwen/Qwen3.5-2B` with `transformers` and `peft`:

```python
from peft import PeftModel
from transformers import AutoModelForCausalLM, AutoTokenizer

base = AutoModelForCausalLM.from_pretrained("Qwen/Qwen3.5-2B", dtype="auto", device_map="auto")
model = PeftModel.from_pretrained(base, "ns/vox-mens-qwen3.5-2b")
tok = AutoTokenizer.from_pretrained("Qwen/Qwen3.5-2B")
```

This request sends the user turn of held-out item `matrix_row_sums` to the 2B server, which applies its `codegen` lane to it.

```bash
curl -s http://127.0.0.1:11435/v1/chat/completions -H 'Content-Type: application/json' -d @- <<'JSON'
{
  "lane": "codegen",
  "messages": [
    {
      "role": "user",
      "content": "Write a Vox program that: define a function named `matrix_row_sums` that takes a list of lists of integers `matrix` and returns a list of the sum of each row"
    }
  ],
  "max_tokens": 1024,
  "temperature": 0.0
}
JSON
```

Compiler-verified **Vox Mens 2B** output recorded on held-out task `matrix_row_sums` under `qwen3_non_thinking_codegen_v1`:

```vox
fn matrix_row_sums(matrix: list[list[int]]) to list[int] {
    return matrix.map(fn(row: list[int]) to int { row.sum() })
}
```

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

For 2B, the examples are [2 held-out tasks **Vox Mens 2B** fixes over the base model](https://voxlang.org/reference/mens-models/#examples-on-the-cards); of 8 items it fixes 4 and breaks 1.

### Fixed `workflow_fetch_save`

```text
define a workflow named `fetch_and_save` that fetches data from a URL stored in a variable `url` and saves the result to a variable `data`
```

Unadapted 2B base on `workflow_fetch_save`: does not parse (count 5).

```text
import http

type FetchResult {
    data: str
    error: str
}

fn fetch_data(url: str) to Result[FetchResult] {
    let response = http.get(url)?
    let data = response.body?
    return Ok(FetchResult {
        data: data,
        error: ""
    })
}

workflow fetch_and_save(url: str) to Result[Unit] {
    let result = fetch_data(url) with { retries: 3, timeout: "30s" }?
    if result.error is not "" {
        return Error(result.error)
    }
    let data = result.data
    // Save data to a variable (in a real application, this would be persisted to a database or file)
    // For this example, we just assign it to a variable
    let saved_data = data
    return Ok(())
}
```

Vox Mens 2B on `workflow_fetch_save`: passes.

```vox
fn fetch_data(url: str) to Result[str] {
    return Ok("mock_data")
}

fn save_data(data: str) to Result[str] {
    return Ok("saved:" + data)
}

workflow fetch_and_save(url: str) to Result[str] {
    let data = fetch_data(url)?
    let result = save_data(data)?
    return Ok(result)
}
```

### Fixed `humaneval_vox_069_binary_search`

```text
Search for target in a sorted list xs using binary search.
Return the index of target if found, or -1 if not found.
```

Unadapted 2B base on `humaneval_vox_069_binary_search`: type errors (count 7).

```text
fn binary_search(xs: list[int], target: int) to Result[int] {
    let low = 0
    let high = xs.length - 1
    
    while low <= high {
        let mid = (low + high) / 2
        let mid_val = xs[mid]
        
        if mid_val == target {
            return Ok(mid)
        } else if mid_val < target {
            low = mid + 1
        } else {
            high = mid - 1
        }
    }
    
    return Error(-1)
}
```

Vox Mens 2B on `humaneval_vox_069_binary_search`: passes.

```vox
fn binary_search(xs: list[int], target: int) to int {
    let mut low = 0
    let mut high = len(xs) - 1
    while low <= high {
        let mid = (low + high) // 2
        let val = xs[mid]
        if val == target {
            return mid
        } else if val < target {
            low = mid + 1
        } else {
            high = mid - 1
        }
    }
    return -1
}
```

## Results

| Result for 2B | [Held-out pass@1](https://voxlang.org/reference/mens-models/#held-out-pass-rate) | [Collateral accuracy](https://voxlang.org/reference/mens-models/#collateral-damage) | [McNemar p-value](https://voxlang.org/reference/mens-models/#why-mcnemar) |
|---|---|---|---|
| Base 2B | 2/8 (25.0%) | — | — |
| Vox Mens 2B | 5/8 (62.5%) | — | — |
| gsm8k_think, 128 items, 2B (base then Vox Mens) | — | 0.125 then 0.117 | 0.5000 |
| math500_think, 128 items, 2B (base then Vox Mens) | — | 0.125 then 0.117 | 0.5000 |
| vox_think, 128 items, 2B (base then Vox Mens) | — | 0.125 then 0.117 | 0.5000 |

The 2B rows compare the unadapted base checkpoint with Vox Mens 2B on identical items, and a slice p-value below 0.05 would mean the model lost ground on that slice.

The release gate record for 2B is [`gate_receipt.json`](https://huggingface.co/ns/vox-mens-qwen3.5-2b/blob/main/gate_receipt.json).

## Training

Vox Mens 2B fine-tunes base pin `Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` over 138 optimizer steps (2711427 tokens), ended with a final loss (EMA) of 0.6170, a mean train loss of 0.8323 and a last-epoch validation loss of 0.7721; the 2B run took 4h 10m and peaked at 27007 MiB from Vox commit `c56e13a33` (shared settings: [docs page](https://voxlang.org/reference/mens-models/#shared-training-settings)).

## Files

The 2B repository holds `peft/` Hugging Face PEFT adapter files alongside `candle_qlora_adapter.safetensors`, tokenizer and `chat_template.jinja`, `.eval_results/vox_mens_eval.yaml`, `external_serving_handoff_v1.json`, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.

This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/ns/vox-mens-index) lists the other sizes and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.
