---
license: apache-2.0
base_model: Qwen/Qwen3.5-0.8B
base_model_relation: adapter
base_model_revision: 2fc06364715b967f1860aea9cf38778875588b17
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
- gate-failed
model-index:
- name: vox-mens-qwen3.5-0.8b
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
widget:
- example_title: "Workflow: fetch save"
  text: "Write a Vox program that: define a workflow named `fetch_and_save` that fetches data from a URL stored in a variable `url` and saves the result to a variable `data`"
  output:
    text: "fn fetch_data(url: str) to Result[str] {\n    return Ok(\"mock_data\")\n}\n\nfn save_data(data: str) to Result[str] {\n    return Ok(\"saved:\" + data)\n}\n\nworkflow fetch_and_save(url: str) to Result[str] {\n    let data = fetch_data(url)?\n    let result = save_data(data)?\n    return Ok(result)\n}"
- example_title: "Binary search"
  text: "Write a Vox program that: Search for target in a sorted list xs using binary search.\nReturn the index of target if found, or -1 if not found."
  output:
    text: "fn binary_search(xs: list[int], target: int) to int {\n    let mut low = 0\n    let mut high = len(xs) - 1\n    while low <= high {\n        let mid = (low + high) // 2\n        let val = xs[mid]\n        if val == target {\n            return mid\n        } else if val < target {\n            low = mid + 1\n        } else {\n            high = mid - 1\n        }\n    }\n    return -1\n}"
---
# Vox Mens 0.8B — Smallest Open Model for Trying & Writing Vox Locally

**Vox Mens 0.8B** is [trained](https://voxlang.org/reference/mens-models/#state-legend) for [`vox-lang`](https://voxlang.org/reference/mens-models/#hub-and-spoke) (release check: [failed](https://voxlang.org/reference/mens-models/#state-legend)): the collateral check refused 0.8B: “the collateral report status is `fail` on `gsm8k_think`”.

![Vox Mens 0.8B architectural schematic](assets/banner.jpg)

*Diagram (0.8B): a short prompt runs in the browser or on an everyday laptop CPU or GPU offline and returns compiler-checked `.vox` code.*

| Model | Held-out pass@1 (0.8B) | Weights (0.8B) | License | Live Demo |
|---|---|---|---|---|
| **Vox Mens 0.8B** | 2/8 (25.0%) → **5/8 (62.5%)** | PEFT + Candle Adapter | `apache-2.0` | [Open 0.8B Playground](https://huggingface.co/spaces/ns/vox-mens-playground?model=ns/vox-mens-qwen3.5-0.8b) |

## What it does

**Vox Mens 0.8B** writes and fixes [Vox](https://voxlang.org) (`.vox`), a full-stack language where one file defines database tables, backend queries and mutations, web UI components, and workflows that compile to Rust and TypeScript. Because Vox is newer than mainstream languages, general-purpose AI models often guess Rust or TypeScript syntax that fails to compile; **Vox Mens 0.8B** is fine-tuned on verified `.vox` examples so you can generate and repair Vox locally on Windows, Linux, or Mac (`Ollama`, `vLLM`, Python, or `vox mens serve`) or through Hugging Face Inference Providers without paying large cloud API rates on every prompt.

## Why this size

Vox Mens 0.8B is the smallest download in the family, built to run on almost any Windows, Linux, or Mac laptop — even without a dedicated GPU — or in the browser Playground. Use 0.8B to try Vox locally, generate short functions or simple queries, and check basic Vox syntax with minimal memory and disk use. Because 0.8B has very limited capacity for multi-step logic, step up to 2B or 4B for full handlers and use a larger cloud model for app design. Measured QLoRA peak memory during the 0.8B run was 15319 MiB (`run_summary`).

## Use it

This page currently distributes the **Vox Mens 0.8B** trained add-on while release review is unfinished, and the files are available from the Files tab or with this download command:

```bash
vox mens hub download --repo ns/vox-mens-qwen3.5-0.8b --output-dir mens/runs/qwen3.5-0.8b
```

`vox mens serve` loads the 0.8B add-on only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so no serving example is given while review is pending.
## Vox and Python

Beginners can write and run their first full-stack `.vox` file alongside **Vox Mens 0.8B** in under a minute, or call `ns/vox-mens-qwen3.5-0.8b` from Python and Rust.

**Try Vox in 60 seconds (0.8B)**

[Vox](https://voxlang.org) keeps your schema, backend queries, mutations, and helpers in one compiler-checked file so **Vox Mens 0.8B** can edit an entire feature coherently:

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

Type-check and execute your program locally while **Vox Mens 0.8B** assists in the same workspace:

```bash
vox check main.vox
vox run --interp main.vox
vox llm prompt "Add an archive mutation to main.vox"
```

**Use 0.8B from Vox and Rust**

Because even `27B` is smaller than massive commercial cloud models, Vox projects commonly use a large cloud model for high-level planning while routing `.vox` file generation and `vox check` fixes to `ns/vox-mens-qwen3.5-0.8b` (locally or via Hugging Face Router) through `vox_actor_runtime::llm::llm_chat`:

```bash
export HF_CHAT_MODEL="ns/vox-mens-qwen3.5-0.8b"
export HF_TOKEN="hf_your_token_here"
export VOX_POPULI_LOCAL_OLLAMA_URL="http://127.0.0.1:11435"
```

```rust
use vox_actor_runtime::inference_env::resolve_huggingface_router;
use vox_actor_runtime::model_resolution::{RouteResolutionInput, resolve_chat_provider_route};

// Keep ambiguous cross-repo planning on your frontier cloud route, and delegate
// .vox handlers, tests, and `vox check` repair loops to ns/vox-mens-qwen3.5-0.8b:
let mens_specialist = resolve_huggingface_router("ns/vox-mens-qwen3.5-0.8b");
```

**Use 0.8B from Python**

Call `ns/vox-mens-qwen3.5-0.8b` with the standard `openai` Python SDK through Hugging Face Inference Providers or a local `vox mens serve` endpoint:

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
    model="ns/vox-mens-qwen3.5-0.8b:auto",
    messages=[{"role": "user", "content": "Write a Vox query that returns open Note rows."}],
)
print(reply.choices[0].message.content)
```

Manage Python dependencies inside a Vox project for **Vox Mens 0.8B** using `[python]` in `Vox.toml` and `vox py`:

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

Generate a typed Python client from a Vox service API built with **Vox Mens 0.8B** using OpenAPI:

```bash
vox emit openapi src/main.vox --out dist/openapi.json
openapi-python-client generate --path dist/openapi.json
```
## Examples

For 0.8B, the examples are [the 2 shortest items the adapter fixes and the shortest it still fails](https://voxlang.org/reference/mens-models/#examples-on-the-cards); of 8 items it fixes 4 and breaks 1.

### Fixed `workflow_fetch_save`

```text
define a workflow named `fetch_and_save` that fetches data from a URL stored in a variable `url` and saves the result to a variable `data`
```

Unadapted 0.8B base on `workflow_fetch_save`: does not parse (count 5).

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

Vox Mens 0.8B on `workflow_fetch_save`: passes.

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

Unadapted 0.8B base on `humaneval_vox_069_binary_search`: type errors (count 7).

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

Vox Mens 0.8B on `humaneval_vox_069_binary_search`: passes.

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

### Still fails `humaneval_vox_042_reverse_list`

```text
Return a new list with the elements in reverse order.
```

Unadapted 0.8B base on `humaneval_vox_042_reverse_list`: type errors (count 2).

```text
fn reverse_list(list: list) to list {
    return list.reverse()
}
```

Vox Mens 0.8B on `humaneval_vox_042_reverse_list`: type errors (count 1).

```text
fn reverse_list(items: list[int]) to list[int] {
    return items.reverse()
}
```

## Results

| Result for 0.8B | [Held-out pass@1](https://voxlang.org/reference/mens-models/#held-out-pass-rate) | [Collateral accuracy](https://voxlang.org/reference/mens-models/#collateral-damage) | [McNemar p-value](https://voxlang.org/reference/mens-models/#why-mcnemar) |
|---|---|---|---|
| Base 0.8B | 2/8 (25.0%) | — | — |
| Vox Mens 0.8B | 5/8 (62.5%) | — | — |

The 0.8B rows compare the unadapted base checkpoint with Vox Mens 0.8B on identical items.

The release gate record for 0.8B is [`gate_receipt.json`](https://huggingface.co/ns/vox-mens-qwen3.5-0.8b/blob/main/gate_receipt.json).

## Training

Vox Mens 0.8B fine-tunes base pin `Qwen/Qwen3.5-0.8B@2fc06364715b967f1860aea9cf38778875588b17` over 1108 optimizer steps (2711427 tokens), ended with a final loss (EMA) of 0.6898, a mean train loss of 0.8427 and a last-epoch validation loss of 0.8324; the 0.8B run took 2h 49m and peaked at 15319 MiB from Vox commit `c56e13a33` (shared settings: [docs page](https://voxlang.org/reference/mens-models/#shared-training-settings)).

## Files

The 0.8B repository holds `peft/` Hugging Face PEFT adapter files alongside `candle_qlora_adapter.safetensors`, tokenizer and `chat_template.jinja`, `.eval_results/vox_mens_eval.yaml`, `external_serving_handoff_v1.json`, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.

This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/ns/vox-mens-index) lists the other sizes and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.
