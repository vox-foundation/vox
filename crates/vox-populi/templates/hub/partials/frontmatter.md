---
license: {{ f.license }}
base_model: {{ f.base_repo }}
base_model_relation: {% if f.is_complete_model() %}finetune{% else %}adapter{% endif %}
{%- if !f.base_rev.is_empty() %}
base_model_revision: {{ f.base_rev }}
{%- endif %}
library_name: {% if f.inference_provider_ready %}transformers{% else if f.has_peft_adapter %}peft{% else %}vox{% endif %}
pipeline_tag: text-generation
language:
- en
{%- if !f.datasets.is_empty() %}
datasets:
{%- for d in f.datasets %}
- {{ d }}
{%- endfor %}
{%- endif %}
spaces:
- {{ f.playground_space_id }}
tags:
- vox
- vox-mens
- {{ f.family_tag() }}
{%- if f.model_family == "qwen3.5" %}
- qwen3_5
- gated-delta-networks
{%- endif %}
- qlora
{%- if f.trained %}
- rslora
{%- endif %}
- candle
- code
- conversational
{%- if f.inference_provider_ready %}
- text-generation-inference
- vllm
{%- endif %}
{%- if f.trained %}
{%- if let Some(sp) = f.spoke %}
- spoke-{{ sp.id }}
{%- endif %}
- {{ f.gate.state.tag() }}
{%- else %}
- not-trained
{%- endif %}
{%- if f.has_model_index() %}
model-index:
- name: {{ f.repo }}
  results:
{%- if let Some(h) = f.heldout %}
{%- if let Some(rate) = h.adapter_rate %}
  - task:
      type: text-generation
      name: Vox code generation
    dataset:
      type: vox-mens-heldout-bench
      name: Vox held-out bench ({{ h.total }} items)
    metrics:
    - type: pass@1
      name: pass@1 ({{ f.protocol }})
      value: {{ "{:.4}"|format(rate) }}
{%- endif %}
{%- endif %}
{%- for r in f.collateral %}
  - task:
      type: text-generation
      name: Reasoning regression check (thinking mode)
    dataset:
      type: vox-mens-{{ r.bench }}
      name: {{ r.bench }} ({{ r.n }}-item slice)
    metrics:
    - type: accuracy
      name: {{ r.bench }} accuracy
      value: {{ "{:.4}"|format(r.adapter_acc) }}
{%- endfor %}
{%- endif %}
{%- if let Some(ex) = f.examples %}
{%- if ex.has_widget() %}
widget:
{%- for w in ex.wins %}
{%- if let Some(out) = w.widget_output() %}
- example_title: {{ w.id }}
  text: {{ w.widget_prompt() }}
  output:
    text: {{ out }}
{%- endif %}
{%- endfor %}
{%- if let Some(u) = f.use_it() %}
{%- if let Some(out) = u.widget_output() %}
- example_title: {{ u.id }}
  text: {{ u.widget_prompt() }}
  output:
    text: {{ out }}
{%- endif %}
{%- endif %}
{%- endif %}
{%- endif %}
---
