---
license: apache-2.0
library_name: vox
pipeline_tag: text-generation
language:
- en
spaces:
- {{ f.playground_space_id }}
tags:
- vox
- vox-mens
- qwen3-5
- qwen3-8
- qlora
- code
- conversational
- index
---
# Vox Mens — Open Models Trained to Write & Fix Vox Code

[Vox](https://voxlang.org) (`.vox`) is a full-stack language where one file defines your database tables, backend queries and mutations, web UI components, and workflows, then compiles to Rust and TypeScript. Because Vox is new, general-purpose models often hallucinate Rust, Python, or TypeScript syntax that fails `vox check`. **Vox Mens** (`0.8B` through `27B`) provides open-weights models fine-tuned specifically on valid `.vox` code so anyone on Windows, Linux, Mac, or Hugging Face Inference Providers can write and repair Vox locally or at low cost. Try them side by side in the [Vox Mens Playground Space](https://huggingface.co/spaces/{{ f.playground_space_id }}), or read more on [what MENS is]({{ f.docs }}#what-mens-is), [capability lanes]({{ f.docs }}#hub-and-spoke), and [release checks]({{ f.docs }}#gating).
{%- if let Some(e) = f.config_error %}

<!-- hub card config error: {{ e }} -->
{%- endif %}

![Vox Mens routing and release check overview](assets/overview.jpg)

*Overview diagram: large cloud models handle broad app architecture and research on the left, Vox Mens models (`0.8B` to `27B`) write `.vox` code locally or over hosted APIs in the center, and every published checkpoint must clear syntax, type, and math/logic checks on the right.*

## Available models

{% if f.passed_rows().is_empty() -%}
No Vox Mens checkpoint has completed release verification in `{{ f.namespace }}` yet.
{% else -%}
| Model | Held-out pass@1 | Weights | Peak memory | Hub | Repo | Live Demo |
|---|---|---|---|---|---|---|
{%- for r in f.passed_rows() %}
| **Vox Mens {{ r.size }}** | **{{ r.adapter_cell() }}** | {% if r.merged %}Standalone Safetensors + PEFT{% else %}PEFT + Candle Adapter{% endif %} | {{ r.peak_text() }} | {% match r.hub %}{%- when HubCell::Published { version } %}{% if let Some(v) = version %}published `{{ v }}`{% else %}published{% endif %}{%- when HubCell::Diverged { version } %}{% if let Some(v) = version %}published `{{ v }}`{% else %}published{% endif %}{%- when HubCell::HubOnly { version } %}published{% if let Some(v) = version %} `{{ v }}`{% endif %}{%- when HubCell::NotPublished %}verified{%- when HubCell::NoWeights %}verified{%- endmatch %} | [{{ r.repo_id }}](https://huggingface.co/{{ r.repo_id }}) | [Open {{ r.size }}](https://huggingface.co/spaces/{{ f.playground_space_id }}?model={{ r.repo_id }}) |
{%- endfor %}

{{ f.spoke_source.as_deref().unwrap_or("") }}

{% if !f.ready_merged_rows().is_empty() %}Standalone merged checkpoints ready for `Ollama`, `vLLM`, Hugging Face Inference Providers, and `vox mens hub download`:{% for r in f.ready_merged_rows() %}{% if !loop.first %},{% endif %} **Vox Mens {{ r.size }}** (`vox mens hub download --repo {{ r.repo_id }} --output-dir {{ r.local_run_dir }}`){% endfor %}.{% endif %}
{%- endif %}

## Which model should I use

Even `27B` is a compact model compared to massive commercial cloud models (Claude, Gemini, GPT), so pick based on your hardware and what each tier actually does best:

| Job | Recommended Choice | Real Tradeoff |
|---|---|---|
| **Trying Vox in a browser or on any basic laptop** | **Vox Mens `0.8B` / `2B`** | Smallest download; runs on CPU or entry-level GPUs across Windows, Linux, and Mac for short functions, tables, and tests, with lower accuracy on multi-step logic. |
| **Everyday single-file `.vox` coding on a dev PC** | **Vox Mens `4B` / `9B`** | Practical sweet spot for laptops and desktops with a modest GPU (or hosted HF endpoints) when combining tables, queries, mutations, UI views, and workflows. |
| **Private local ceiling or zero-per-token team server** | **Vox Mens `27B`** | Strongest `.vox` accuracy in this open-weights family for high-VRAM GPUs or hosted routers when code cannot leave your infrastructure. |
| **Open-ended app design, hard algorithms & cross-repo planning** | **Large Cloud Models (`vox llm`)** | Much larger reasoning capacity for ambiguous specs and architecture — pair them to plan the feature while Vox Mens writes and fixes the `.vox` files. |
{%- match f.pick %}
{%- when Some with (p) %}

Among the verified published models, **Vox Mens {{ p.size }}** currently leads held-out pass@1 at {{ p.rate_text() }} (ties favor the smaller model):

```bash
vox mens hub pull --size {{ p.size }} --namespace {{ f.namespace }}
```
{%- when None %}

No verified model with a current-serving benchmark score is available to recommend yet.
{%- endmatch %}

## Held-out pass@1 by size
{%- if f.comparison.is_empty() %}

No release-verified model has a held-out report under `{{ f.protocol }}` yet.
{%- else %}

| Model | Base pass@1 | Vox Mens pass@1 |
|---|---|---|
{%- for c in f.comparison %}
| **Vox Mens {{ c.0 }}** | {{ c.1.base_cell() }} | **{{ c.1.adapter_cell() }}** |
{%- endfor %}

Every row compares the unadapted base checkpoint against the compiler-verified **Vox Mens** checkpoint on the same 52-task held-out suite ([methodology]({{ f.docs }}#held-out-pass-rate)) under `{{ f.protocol }}`.
{%- endif %}

## Links

- Docs page: [MENS models]({{ f.docs }})
- Playground: [Vox MENS Playground Space](https://huggingface.co/spaces/{{ f.playground_space_id }})
- Training: [MENS training](https://voxlang.org/reference/mens-training/)
- Training data: [training data contract](https://voxlang.org/reference/mens-training-data-contract/)
- Serving: [MENS serving](https://voxlang.org/reference/mens-serving-ssot/)
- Source: [vox-foundation/vox](https://github.com/vox-foundation/vox)
- Machine-readable: `ladder_index.json` in this repository

This index is generated by `vox mens hub sync` from each size's records, and [how the cards are made]({{ f.docs }}#how-the-cards-are-made) is on the docs page.
