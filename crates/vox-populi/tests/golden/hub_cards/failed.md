---
license: apache-2.0
base_model: Qwen/Qwen3-0.6B
base_model_relation: adapter
base_model_revision: c1899de289a04d12100db370d81485cdf75e47ca
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
- gate-failed
model-index:
- name: vox-mens-qwen3-0.6b
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
# Vox MENS — Qwen3-0.6B adapter

0.6B is [trained](https://voxlang.org/reference/mens-models/#state-legend), its spoke is [vox-lang](https://voxlang.org/reference/mens-models/#hub-and-spoke) and its gate is [failed](https://voxlang.org/reference/mens-models/#state-legend): the collateral check refused 0.6B: “the collateral report status is `fail` on `gsm8k_think`”.

![Qwen3-0.6B scorecard](assets/scorecard.svg)

## Why this size

The smallest rung, aimed at machines with very little memory, including CPU-only boxes. It suits quick completions and short single-function edits where reply time matters more than depth. Step up when it loses the thread of a multi-step task; there is no smaller size to step down to. The measured training peak for 0.6B is 15319 MiB (`peak_source`: run_summary), and no serving memory figure has been measured for it.

## Examples

For 0.6B, the examples are [the 2 shortest items the adapter fixes and the shortest it still fails](https://voxlang.org/reference/mens-models/#examples-on-the-cards); of 8 items it fixes 4 and breaks 1.

### Fixed `humaneval_vox_069_binary_search`

```text
Search for target in a sorted list xs using binary search.
Return the index of target if found, or -1 if not found.
```

Qwen3-0.6B base on `humaneval_vox_069_binary_search`: type errors (count 7).

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

0.6B adapter on `humaneval_vox_069_binary_search`: passes.

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

Qwen3-0.6B base on `mutation_update`: does not parse (count 3).

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

0.6B adapter on `mutation_update`: passes.

```vox
mutation update_item(id: int, title: str) to Result[str] {
    return Ok("updated")
}
```

### Still fails `humaneval_vox_042_reverse_list`

```text
Return a new list with the elements in reverse order.
```

Qwen3-0.6B base on `humaneval_vox_042_reverse_list`: type errors (count 2).

```text
fn reverse_list(list: list) to list {
    return list.reverse()
}
```

0.6B adapter on `humaneval_vox_042_reverse_list`: type errors (count 1).

```text
fn reverse_list(items: list[int]) to list[int] {
    return items.reverse()
}
```

## Use it

The 0.6B adapter sits on the base pin `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca`, and this command downloads it after checking the BLAKE3 hash of every file in `hf_sync_manifest.json`.

```bash
vox mens hub pull --size 0.6B --namespace ns --allow-ungated
```

`vox mens serve` loads the 0.6B adapter only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so the pull needs `--allow-ungated` and no serving example is given.

The docs page explains [why the 0.6B adapter cannot be loaded with transformers or PEFT](https://voxlang.org/reference/mens-models/#why-peft-cannot-load-the-adapters).

## Results

| Result for 0.6B | [Held-out pass@1](https://voxlang.org/reference/mens-models/#held-out-pass-rate) | [Collateral accuracy](https://voxlang.org/reference/mens-models/#collateral-damage) | [McNemar p-value](https://voxlang.org/reference/mens-models/#why-mcnemar) |
|---|---|---|---|
| Base 0.6B | 2/8 (25.0%) | — | — |
| Adapter 0.6B | 5/8 (62.5%) | — | — |

The 0.6B rows compare the base model with the adapter on identical items.

The release gate record for 0.6B is [`gate_receipt.json`](https://huggingface.co/ns/vox-mens-qwen3-0.6b/blob/main/gate_receipt.json).

## Training

0.6B ran 1108 optimizer steps over 2711427 tokens, ended with a final loss (EMA) of 0.6898, a mean train loss of 0.8427 and a last-epoch validation loss of 0.8324, took 2h 49m and peaked at 15319 MiB, from Vox commit `c56e13a33`; the settings every size shares are on the [docs page](https://voxlang.org/reference/mens-models/#shared-training-settings).

## Files

The 0.6B repository holds `candle_qlora_adapter.safetensors`, the adapter, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.

This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/ns/vox-mens-index) lists the other sizes and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.
