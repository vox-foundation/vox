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
# Vox MENS — AI coding models for the Vox language

MENS is the Vox project's family of open-weights coding models (`0.8B` through `27B`), fine-tuned to write, check, and explain [Vox](https://voxlang.org) — a full-stack programming language where database tables, backend endpoints, reactive web UI components, and AI agents live in one compiler-checked `.vox` file. Pick a size that fits your machine, run locally or through Python and Hugging Face Inference Providers, and try live checkpoints in the [Vox MENS Playground Space](https://huggingface.co/spaces/{{ f.playground_space_id }}). The docs explain [what MENS is]({{ f.docs }}#what-mens-is), how [models are organized]({{ f.docs }}#hub-and-spoke), and how automated [quality gates]({{ f.docs }}#gating) verify every release; this index lists every size side by side.
{%- if let Some(e) = f.config_error %}

<!-- hub card config error: {{ e }} -->
{%- endif %}

![Vox MENS overview](assets/overview.jpg)

## Sizes and spokes

| Size |{% for s in f.spokes %} {{ s.label }} |{% endfor %} Hub | Repo |
|---|{% for s in f.spokes %}---|{% endfor %}---|---|
{%- for r in f.rows %}
| {{ r.size }} |
{%- for c in r.cells %} {% match c %}
{%- when GridCell::Gate with (g) %}{{ g.label() }}
{%- when GridCell::NotTrained %}not trained
{%- when GridCell::Planned { link } %}[planned]({{ link }})
{%- endmatch %} |
{%- endfor %} {% match r.hub %}
{%- when HubCell::Published { version } %}{% if let Some(v) = version %}published `{{ v }}`{% else %}published (untagged){% endif %}
{%- when HubCell::Diverged { version } %}{% if let Some(v) = version %}`{{ v }}` on Hub; local run differs{% else %}on Hub; local run differs{% endif %}
{%- when HubCell::HubOnly { version } %}published{% if let Some(v) = version %} `{{ v }}`{% endif %} (no local run)
{%- when HubCell::NotPublished %}not published
{%- when HubCell::NoWeights %}no weights
{%- endmatch %} | [{{ r.repo_id }}](https://huggingface.co/{{ r.repo_id }}) |
{%- endfor %}

Each trained cell holds the size's gate, defined in the [state legend]({{ f.docs }}#state-legend), and `planned` marks a spoke with no adapter yet.
{%- if let Some(src) = f.spoke_source %} {{ src }}{% endif %}

{% if f.ready_merged_rows().is_empty() %}Sizes without a complete merged checkpoint currently distribute the trained add-on while their release review is unfinished, and merged models are held until collateral and gate checks pass.{% else %}Ready sizes distributing a complete merged checkpoint via `vox mens hub download` or the Files tab:{% for r in f.ready_merged_rows() %}{% if !loop.first %},{% endif %} {{ r.model_label }} (`vox mens hub download --repo {{ r.repo_id }} --output-dir {{ r.local_run_dir }}`){% endfor %}.{% endif %}

## Which model should I use

Pick the largest current model that fits comfortably in your memory (choosing a compact laptop checkpoint for quick edits or a larger studio checkpoint for multi-file changes), and step down to a smaller size whenever you want lower latency or lighter RAM use. Each card's Why this size, What it does, and Vox and Python sections explain what tasks it suits and how to wire it up, while the training peak reflects measured training memory rather than serving memory.
{%- match f.pick %}
{%- when Some with (p) %}

Among the sizes with a current gate and an adapter score from the current serving path, {{ p.size }} has the highest held-out adapter pass@1, {{ p.rate_text() }}, and a tie goes to the smaller size.

| Size | Adapter pass@1 | Training peak |
|---|---|---|
{%- for r in f.passed_rows() %}
| {{ r.size }} | {{ r.adapter_cell() }} | {{ r.peak_text() }} |
{%- endfor %}

```bash
vox mens hub pull --size {{ p.size }} --namespace {{ f.namespace }}
```
{%- when None %}

{% if f.passed_rows().is_empty() %}No size has a current gate, so this index recommends none and gives no pull command.{% else %}No size with a current gate has an adapter score from the current serving path, so this index recommends none and gives no pull command.{% endif %}
{%- let np = f.not_passed() %}
{%- if !np.is_empty() %} Sizes whose gate is not passed:
{%- for p in np %}{% if !loop.first %},{% endif %} {{ p.size }}{% endfor %}.{% endif %}
{%- let re = f.needs_reevaluation() %}
{%- if !re.is_empty() %} Sizes that need a held-out re-evaluation under the current serving path:
{%- for p in re %}{% if !loop.first %},{% endif %} {{ p.size }}{% endfor %}.{% endif %}
{%- endmatch %}

## What's next
{%- if f.pending.is_empty() && f.planned_spokes.is_empty() %}

Every trained size has a current gate and an adapter score from the current serving path, and every spoke has an adapter.
{%- endif %}
{%- for p in f.pending %}

- **{{ p.size }}**:
{%- if let Some(b) = p.blocker %}
{%- match b %}
{%- when Blocker::Stale { refusal, recorded_slice_n, current_slice_n } %} re-run `vox mens eval-collateral-damage` for {{ p.size }} on the current {{ current_slice_n }}-item slices.
{%- when Blocker::Missing(names) %} produce the records that are missing for {{ p.size }}:
{%- for n in names %}{% if !loop.first %},{% endif %} `{{ n }}`{% endfor %}.
{%- when Blocker::Failed { refusal, gates } %} review the gate that stopped {{ p.size }}, because the current evaluation measured its adapter and repeating that evaluation would measure the same adapter.
{%- when Blocker::NoAdapter { record_reason } %} complete the training record for {{ p.size }}.
{%- when Blocker::NotPublished %} publish the {{ p.size }} adapter weights.
{%- endmatch %}
{%- endif %}
{%- if p.reevaluate %} Re-evaluate the {{ p.size }} adapter on the held-out bench under the current serving path with `vox mens eval-local`.{% endif %}
{%- if let Some(e) = p.eta %} This is an estimate, not a measurement: the {{ p.size }} re-gate is projected at about {{ "{:.1}"|format(e.hours_expected) }} hours, or {{ "{:.1}"|format(e.hours_conservative) }} hours if slower, with {{ e.confidence }} confidence. Basis: {{ e.basis }}
{%- else %} There is no estimate for {{ p.size }}.{% endif %}
{%- endfor %}
{%- for s in f.planned_spokes %}

- **{{ s.label }}**: {{ s.purpose }} No adapter has been trained for {{ s.label }}, and there is no estimate for it.
{%- if let Some(l) = s.link %} The [plan for {{ s.label }}]({{ l }}) is on the docs page.{% endif %}
{%- endfor %}

## Held-out pass@1 by size
{%- if f.comparison.is_empty() %}

No size has a held-out report under the current prompt protocol `{{ f.protocol }}`, so there is nothing to compare.
{%- else %}

| Size | Base pass@1 | Adapter pass@1 |
|---|---|---|
{%- for c in f.comparison %}
| {{ c.0 }} | {{ c.1.base_cell() }} | {{ c.1.adapter_cell() }} |
{%- endfor %}

The table lists held-out pass@1 ([defined here]({{ f.docs }}#held-out-pass-rate)) only for sizes with a report under the current prompt protocol `{{ f.protocol }}`, and an adapter score recorded under an earlier serving path is shown as a dash because it does not count.
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
