---
license: apache-2.0
base_model: Qwen/Qwen3-1.7B
base_model_relation: adapter
base_model_revision: 70d244cc86ccca08cf5af4e1e306ecf908b1ad5e
library_name: vox
pipeline_tag: text-generation
language:
- en
tags:
- vox
- vox-mens
- qwen3
- qlora
- candle
- code
- spoke-vox-lang
- gate-passed
model-index:
- name: vox-mens-qwen3-1.7b
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
- example_title: humaneval_vox_069_binary_search
  text: "Write a Vox program that: Search for target in a sorted list xs using binary search.\nReturn the index of target if found, or -1 if not found."
  output:
    text: "fn binary_search(xs: list[int], target: int) to int {\n    let mut low = 0\n    let mut high = len(xs) - 1\n    while low <= high {\n        let mid = (low + high) // 2\n        let val = xs[mid]\n        if val == target {\n            return mid\n        } else if val < target {\n            low = mid + 1\n        } else {\n            high = mid - 1\n        }\n    }\n    return -1\n}"
- example_title: mutation_update
  text: "Write a Vox program that: define a mutation named `update_item` that takes an integer `id` and a string `title` and updates the item with the matching id"
  output:
    text: "mutation update_item(id: int, title: str) to Result[str] {\n    return Ok(\"updated\")\n}"
- example_title: page_index
  text: "Write a Vox program that: define a page named `IndexPage` with a title 'Home' that displays a heading with the text 'Welcome'"
  output:
    text: "component IndexPage() {\n    view: heading(level=1) { \"Welcome\" }\n}"
---
# Vox MENS — Qwen3-1.7B adapter

1.7B is [trained](https://voxlang.org/reference/mens-models/#state-legend), its spoke is [vox-lang](https://voxlang.org/reference/mens-models/#hub-and-spoke) and its gate is [passed](https://voxlang.org/reference/mens-models/#state-legend).

![Qwen3-1.7B scorecard](assets/scorecard.svg)

## Why this size

A small rung for ordinary laptops and desktop GPUs with a modest memory budget. It handles single-file generation and one-pass fixes for compiler diagnostics. Drop to the smallest size when latency is the limit, or climb to the middle size when an answer needs reasoning across functions. The measured training peak for 1.7B is 27007 MiB (`peak_source`: run_summary), and no serving memory figure has been measured for it.

## Examples

For 1.7B, the examples are [the 2 shortest items the adapter fixes and the shortest it still fails](https://voxlang.org/reference/mens-models/#examples-on-the-cards); of 8 items it fixes 4 and breaks 1.

### Fixed `humaneval_vox_069_binary_search`

```text
Search for target in a sorted list xs using binary search.
Return the index of target if found, or -1 if not found.
```

Qwen3-1.7B base on `humaneval_vox_069_binary_search`: type errors (count 7).

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

1.7B adapter on `humaneval_vox_069_binary_search`: passes.

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

Qwen3-1.7B base on `mutation_update`: does not parse (count 3).

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

1.7B adapter on `mutation_update`: passes.

```vox
mutation update_item(id: int, title: str) to Result[str] {
    return Ok("updated")
}
```

### Still fails `humaneval_vox_042_reverse_list`

```text
Return a new list with the elements in reverse order.
```

Qwen3-1.7B base on `humaneval_vox_042_reverse_list`: type errors (count 2).

```text
fn reverse_list(list: list) to list {
    return list.reverse()
}
```

1.7B adapter on `humaneval_vox_042_reverse_list`: type errors (count 1).

```text
fn reverse_list(items: list[int]) to list[int] {
    return items.reverse()
}
```

## Use it

The 1.7B adapter sits on the base pin `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`, and this command downloads it after checking the BLAKE3 hash of every file in `hf_sync_manifest.json`.

```bash
vox mens hub pull --size 1.7B --namespace ns
```

The pull writes the 1.7B run directory that this command serves.

```bash
vox mens serve --model mens/runs/qwen3-1.7b --port 11435
```

This request sends the user turn of held-out item `page_index` to the 1.7B server, which applies its `codegen` lane to it.

```bash
curl -s http://127.0.0.1:11435/v1/chat/completions -H 'Content-Type: application/json' -d @- <<'JSON'
{
  "lane": "codegen",
  "messages": [
    {
      "role": "user",
      "content": "Write a Vox program that: define a page named `IndexPage` with a title 'Home' that displays a heading with the text 'Welcome'"
    }
  ],
  "max_tokens": 1024,
  "temperature": 0.0
}
JSON
```

The code below is the 1.7B adapter's recorded output for item `page_index` in the held-out evaluation, taken from `evidence/heldout_adapter_nothink.ckpt.jsonl`, where the `codegen` lane's prompt protocol `qwen3_non_thinking_codegen_v1` wrapped the same user turn and every stored check passed; it is not a transcript of `vox mens serve`.

```vox
component IndexPage() {
    view: heading(level=1) { "Welcome" }
}
```

## Results

| Result for 1.7B | [Held-out pass@1](https://voxlang.org/reference/mens-models/#held-out-pass-rate) | [Collateral accuracy](https://voxlang.org/reference/mens-models/#collateral-damage) | [McNemar p-value](https://voxlang.org/reference/mens-models/#why-mcnemar) |
|---|---|---|---|
| Base 1.7B | 2/8 (25.0%) | — | — |
| Adapter 1.7B | 5/8 (62.5%) | — | — |
| gsm8k_think, 128 items, 1.7B (base then adapter) | — | 0.125 then 0.117 | 0.5000 |
| math500_think, 128 items, 1.7B (base then adapter) | — | 0.125 then 0.117 | 0.5000 |
| vox_think, 128 items, 1.7B (base then adapter) | — | 0.125 then 0.117 | 0.5000 |

The 1.7B rows compare the base model with the adapter on identical items, and a slice p-value below 0.05 would mean the adapter lost ground on that slice.

The release gate record for 1.7B is [`gate_receipt.json`](https://huggingface.co/ns/vox-mens-qwen3-1.7b/blob/main/gate_receipt.json).

## Training

1.7B ran 138 optimizer steps over 2711427 tokens, ended with a final loss (EMA) of 0.6170, a mean train loss of 0.8323 and a last-epoch validation loss of 0.7721, took 4h 10m and peaked at 27007 MiB, from Vox commit `c56e13a33`; the settings every size shares are on the [docs page](https://voxlang.org/reference/mens-models/#shared-training-settings).

## Files

The 1.7B repository holds `candle_qlora_adapter.safetensors`, the adapter, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.

This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/ns/vox-mens-index) lists the other sizes and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.
