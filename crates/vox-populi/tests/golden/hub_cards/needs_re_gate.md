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
widget:
- example_title: humaneval_vox_069_binary_search
  text: "Write a Vox program that: Search for target in a sorted list xs using binary search.\nReturn the index of target if found, or -1 if not found."
  output:
    text: "fn binary_search(xs: list[int], target: int) to int {\n    let mut low = 0\n    let mut high = len(xs) - 1\n    while low <= high {\n        let mid = (low + high) // 2\n        let val = xs[mid]\n        if val == target {\n            return mid\n        } else if val < target {\n            low = mid + 1\n        } else {\n            high = mid - 1\n        }\n    }\n    return -1\n}"
- example_title: mutation_update
  text: "Write a Vox program that: define a mutation named `update_item` that takes an integer `id` and a string `title` and updates the item with the matching id"
  output:
    text: "mutation update_item(id: int, title: str) to Result[str] {\n    return Ok(\"updated\")\n}"
---
# Vox MENS — Qwen3.5-2B adapter

2B is [trained](https://voxlang.org/reference/mens-models/#state-legend), its spoke is [vox-lang](https://voxlang.org/reference/mens-models/#hub-and-spoke) and its gate is [needs re-gate](https://voxlang.org/reference/mens-models/#state-legend): it passed the retired 32-item collateral rule and the 128-item paired re-run for 2B has not been done.

![Vox MENS Qwen3.5-2B banner](assets/banner.jpg)

Try the Qwen3.5-2B (`2B`) checkpoint live in the [Vox MENS Playground Space](https://huggingface.co/spaces/ns/vox-mens-playground?model=ns/vox-mens-qwen3.5-2b).

## What it does

Qwen3.5-2B (`2B`) is a specialized coding model for the [Vox programming language](https://voxlang.org) — a full-stack, AI-native language where database tables, backend endpoints, reactive web UI components, and typed AI agents live in one compiler-checked `.vox` file. Before publishing, Qwen3.5-2B (`2B`) must improve real Vox compiler checks on held-out prompts without losing general software engineering accuracy.

## Why this size

The compact 2B hybrid model fits comfortably on everyday developer laptops and modest desktop GPUs with limited unified memory. Pick 2B for single-file Vox endpoints, quick schema drafts, beginner Q&A, and one-pass compiler error fixes. Move down to 0.8B for tighter latency on small hardware, or step up to 4B when changes span several files. The measured training peak for 2B is 27007 MiB (`peak_source`: run_summary), and no serving memory figure has been measured for it.

## Examples

For 2B, the examples are [the 2 shortest items the adapter fixes and the shortest it still fails](https://voxlang.org/reference/mens-models/#examples-on-the-cards); of 8 items it fixes 4 and breaks 1.

### Fixed `humaneval_vox_069_binary_search`

```text
Search for target in a sorted list xs using binary search.
Return the index of target if found, or -1 if not found.
```

Qwen3.5-2B base on `humaneval_vox_069_binary_search`: type errors (count 7).

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

2B adapter on `humaneval_vox_069_binary_search`: passes.

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

### Fixed `mutation_update`

```text
define a mutation named `update_item` that takes an integer `id` and a string `title` and updates the item with the matching id
```

Qwen3.5-2B base on `mutation_update`: does not parse (count 3).

```text
mutation update_item(id: int, title: str) to Result[Unit] {
    // Assume we have a database table named 'items'
    // This is a simplified example; in a real application, you would use a database query
    let result = query update_item_query(id: id, title: title) with { retries: 1, timeout: "5s" }
    return result
}

query update_item_query(id: int, title: str) to Result[Unit] {
    // This is a placeholder for the actual database query
    // In a real application, you would use a database client to update the item
    return Ok(())
}
```

2B adapter on `mutation_update`: passes.

```vox
mutation update_item(id: int, title: str) to Result[str] {
    return Ok("updated")
}
```

### Still fails `humaneval_vox_042_reverse_list`

```text
Return a new list with the elements in reverse order.
```

Qwen3.5-2B base on `humaneval_vox_042_reverse_list`: type errors (count 2).

```text
fn reverse_list(list: list) to list {
    return list.reverse()
}
```

2B adapter on `humaneval_vox_042_reverse_list`: type errors (count 1).

```text
fn reverse_list(items: list[int]) to list[int] {
    return items.reverse()
}
```

## Use it

This page currently distributes the 2B trained add-on on `Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` while the release review is unfinished, and the files are available from the Files tab or with this download command.

```bash
vox mens hub download --repo ns/vox-mens-qwen3.5-2b --output-dir mens/runs/qwen3.5-2b
```

`vox mens serve` loads the 2B add-on only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so no serving example is given while review is pending.
## Vox and Python

Beginners can write and run their first `.vox` file alongside Qwen3.5-2B (`2B`) in under a minute, or call `ns/vox-mens-qwen3.5-2b` from Python and Rust.

**Try Vox in 60 seconds (2B)**

[Vox](https://voxlang.org) keeps your data model, backend queries, mutations, and helpers in one compiler-checked file so Qwen3.5-2B (`2B`) can edit your whole feature cleanly:

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

Type-check and run your first program locally while Qwen3.5-2B (`2B`) assists in the same workspace:

```bash
vox check main.vox
vox run --interp main.vox
vox llm prompt "Add an archive mutation to main.vox"
```

**Use 2B from Vox and Rust**

Inside a Vox workspace or Rust crate, route chat requests to `ns/vox-mens-qwen3.5-2b` through `vox_actor_runtime::llm::llm_chat` instead of hardcoding provider URLs:

```bash
export HF_CHAT_MODEL="ns/vox-mens-qwen3.5-2b"
export HF_TOKEN="hf_your_token_here"
export VOX_POPULI_LOCAL_OLLAMA_URL="http://127.0.0.1:11435"
```

```rust
use vox_actor_runtime::inference_env::resolve_huggingface_router;
use vox_actor_runtime::model_resolution::{RouteResolutionInput, resolve_chat_provider_route};

// Build a model-agnostic route to ns/vox-mens-qwen3.5-2b via Hugging Face Router or local MENS:
let hf_endpoint = resolve_huggingface_router("ns/vox-mens-qwen3.5-2b");
```

**Use 2B from Python**

Call `ns/vox-mens-qwen3.5-2b` with the standard `openai` Python client through Hugging Face Inference Providers or a local OpenAI-compatible `vox mens serve` endpoint:

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

Manage Python dependencies cleanly inside a Vox project for Qwen3.5-2B (`2B`) using `[python]` in `Vox.toml` and `vox py`:

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

Generate a typed Python client from a Vox service API backed by Qwen3.5-2B (`2B`) using OpenAPI:

```bash
vox emit openapi src/main.vox --out dist/openapi.json
openapi-python-client generate --path dist/openapi.json
```
## Results

| Result for 2B | [Held-out pass@1](https://voxlang.org/reference/mens-models/#held-out-pass-rate) | [Collateral accuracy](https://voxlang.org/reference/mens-models/#collateral-damage) | [McNemar p-value](https://voxlang.org/reference/mens-models/#why-mcnemar) |
|---|---|---|---|
| Base 2B | 2/8 (25.0%) | — | — |
| Adapter 2B | 5/8 (62.5%) | — | — |

The 2B rows compare the base model with the adapter on identical items.

The release gate record for 2B is [`gate_receipt.json`](https://huggingface.co/ns/vox-mens-qwen3.5-2b/blob/main/gate_receipt.json).

## Training

2B ran 138 optimizer steps over 2711427 tokens, ended with a final loss (EMA) of 0.6170, a mean train loss of 0.8323 and a last-epoch validation loss of 0.7721, took 4h 10m and peaked at 27007 MiB, from Vox commit `c56e13a33`; the settings every size shares are on the [docs page](https://voxlang.org/reference/mens-models/#shared-training-settings).

## Files

The 2B repository holds `peft/` Hugging Face PEFT adapter files alongside `candle_qlora_adapter.safetensors`, tokenizer and `chat_template.jinja`, `.eval_results/vox_mens_eval.yaml`, `external_serving_handoff_v1.json`, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.

This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/ns/vox-mens-index) lists the other sizes and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.
