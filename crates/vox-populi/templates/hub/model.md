{% include "partials/frontmatter.md" %}
# Vox Mens {{ f.size }} — {{ f.role_headline() }}

{% include "partials/status_line.md" %}

![Vox Mens {{ f.size }} architectural schematic](assets/banner.jpg)

*{{ f.figure_caption() }}*

{% if f.trained && f.heldout.is_some() && f.adapter_evidence == AdapterEvidence::Current -%}
| Model | Held-out pass@1 ({{ f.size }}) | Weights ({{ f.size }}) | License | Live Demo |
|---|---|---|---|---|
| **Vox Mens {{ f.size }}** | {{ f.base_cell() }} → **{{ f.adapter_cell() }}** | {% if f.is_complete_model() %}Standalone Safetensors + PEFT{% else %}PEFT + Candle Adapter{% endif %} | `{{ f.license }}` | [Open {{ f.size }} Playground](https://huggingface.co/spaces/{{ f.playground_space_id }}?model={{ f.repo_id }}) |

{% else -%}
Try **Vox Mens {{ f.size }}** in the [Vox Mens Playground Space](https://huggingface.co/spaces/{{ f.playground_space_id }}?model={{ f.repo_id }}).

{% endif -%}
## What it does

**Vox Mens {{ f.size }}** writes and fixes [Vox](https://voxlang.org) (`.vox`), a full-stack language where one file defines database tables, backend queries and mutations, web UI components, and workflows that compile to Rust and TypeScript. Because Vox is newer than mainstream languages, general-purpose AI models often guess Rust or TypeScript syntax that fails to compile; **Vox Mens {{ f.size }}** is fine-tuned on verified `.vox` examples so you can generate and repair Vox locally on Windows, Linux, or Mac (`Ollama`, `vLLM`, Python, or `vox mens serve`) or through Hugging Face Inference Providers without paying large cloud API rates on every prompt.

## Why this size

{% if f.why_is_error %}<!-- hub card config error: {{ f.why.join(" ") }} -->{% else %}{{ f.why.join(" ") }}{% endif %}
{%- if let Some(p) = f.peak %} Measured QLoRA peak memory during the {{ f.size }} run was {{ p.0 }} MiB (`{{ p.1 }}`).{% endif %}
{% if f.trained %}
{% include "partials/use_it.md" %}
{% include "partials/onboarding.md" %}
{% include "partials/examples.md" %}
## Results

| Result for {{ f.size }} | [Held-out pass@1]({{ f.docs }}#held-out-pass-rate) | [Collateral accuracy]({{ f.docs }}#collateral-damage) | [McNemar p-value]({{ f.docs }}#why-mcnemar) |
|---|---|---|---|
| Base {{ f.size }} | {{ f.base_cell() }} | — | — |
| Vox Mens {{ f.size }} | {{ f.adapter_cell() }} | — | — |
{%- for r in f.collateral %}
| {{ r.bench }}, {{ r.n }} items, {{ f.size }} (base then Vox Mens) | — | {{ "{:.3}"|format(r.base_acc) }} then {{ "{:.3}"|format(r.adapter_acc) }} | {{ "{:.4}"|format(r.p_value) }} |
{%- endfor %}

The {{ f.size }} rows compare the unadapted base checkpoint with Vox Mens {{ f.size }} on identical items{% if let Some(a) = f.alpha %}, and a slice p-value below {{ a }} would mean the model lost ground on that slice{% endif %}.
{%- if f.heldout.is_none() %} No held-out report under the current prompt protocol `{{ f.protocol }}` exists for {{ f.size }}, so no held-out score is shown for it.{% endif %}
{%- if f.adapter_report %}
{%- match f.adapter_evidence %}
{%- when AdapterEvidence::Current %}
{%- when AdapterEvidence::NoCheckpoint %} The adapter score for {{ f.size }} is not shown because no adapter checkpoint records its serving path, which the [serving-path rule]({{ f.docs }}#held-out-pass-rate) requires, so {{ f.size }} needs a fresh evaluation.
{%- when AdapterEvidence::Stale { recorded } %} The adapter score for {{ f.size }} is not shown because its checkpoint
{%- if let Some(r) = recorded %} records the serving path `{{ r }}`
{%- else %} has no serving field
{%- endif %}, so it does not count under the [serving-path rule]({{ f.docs }}#held-out-pass-rate) and {{ f.size }} needs a fresh evaluation.
{%- endmatch %}
{%- endif %}
{%- if f.has_receipt %}

The release gate record for {{ f.size }} is [`gate_receipt.json`](https://huggingface.co/{{ f.repo_id }}/blob/main/gate_receipt.json).
{%- endif %}

## Training

Vox Mens {{ f.size }} fine-tunes base pin `{{ f.base_pin }}` over {{ f.training.optimizer_steps.as_deref().unwrap_or("unrecorded") }} optimizer steps ({{ f.training.total_tokens.as_deref().unwrap_or("unrecorded") }} tokens), ended with {% if let Some(ema) = f.training.final_loss_ema %}a final loss (EMA) of {{ ema }}, {% endif %}a mean train loss of {{ f.training.avg_train_loss.as_deref().unwrap_or("unrecorded") }} and a last-epoch validation loss of {{ f.training.avg_val_loss_last_epoch.as_deref().unwrap_or("unrecorded") }}; the {{ f.size }} run {% if let Some(w) = f.training.wall %}{% if f.peak.is_some() %}took {{ w }} and peaked at {{ f.peak_text() }}{% else %}took {{ w }} with peak memory unrecorded{% endif %}{% else %}{% if f.peak.is_some() %}peaked at {{ f.peak_text() }} with wall time unrecorded{% else %}had wall time and peak memory unrecorded{% endif %}{% endif %} from Vox commit `{{ f.training.git_sha.as_deref().unwrap_or("unrecorded") }}` (shared settings: [docs page]({{ f.docs }}#shared-training-settings)).

## Files

{% if f.is_complete_model() -%}
The {{ f.size }} repository holds the standalone merged checkpoint (`{{ f.merged_files_text() }}`) built by `vox mens merge-qlora`, exported PEFT adapter files under `peft/`, raw `candle_qlora_adapter.safetensors`, tokenizer and `chat_template.jinja`, `.eval_results/vox_mens_eval.yaml`, `external_serving_handoff_v1.json`, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.
{%- else -%}
The {{ f.size }} repository holds `peft/` Hugging Face PEFT adapter files alongside `candle_qlora_adapter.safetensors`, tokenizer and `chat_template.jinja`, `.eval_results/vox_mens_eval.yaml`, `external_serving_handoff_v1.json`, training and gate manifests and records, `evidence/`, and `hf_sync_manifest.json` with the BLAKE3 hash of every file.
{%- endif %}
{% endif %}
This card is generated by `vox mens hub sync`; the [ladder index](https://huggingface.co/{{ f.namespace }}/vox-mens-index) lists the other sizes and [how the cards are made]({{ f.docs }}#how-the-cards-are-made) is on the docs page.

