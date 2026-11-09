---
license: {{ f.license }}
base_model: {{ f.base_repo }}
base_model_relation: adapter
{%- if !f.base_rev.is_empty() %}
base_model_revision: {{ f.base_rev }}
{%- endif %}
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
