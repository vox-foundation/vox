---
license: apache-2.0
library_name: vox
pipeline_tag: text-generation
language:
- en
tags:
- vox
- vox-mens
- qwen3
- qlora
- index
---
# Vox MENS — Qwen3 adapter ladder

MENS is the Vox project's own adapter ladder: one LoRA adapter per size of a pinned Qwen3 base model, trained on Vox source and gated before it is published. The docs page explains [what MENS is]({{ f.docs }}#what-mens-is), how [hubs and spokes]({{ f.docs }}#hub-and-spoke) relate and how the [gates]({{ f.docs }}#gating) decide; this index is the one place that lists every size.
{%- if let Some(e) = f.config_error %}

<!-- hub card config error: {{ e }} -->
{%- endif %}

![Vox MENS overview](assets/overview.svg)

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

## Which model should I use

Pick among the sizes whose gate is current by held-out adapter pass@1, and step down to the next smaller current size when the pick does not fit your memory. Each card's Why this size section says what work that size suits, and the training peak is measured while training rather than during serving.
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
- Training: [MENS training](https://voxlang.org/reference/mens-training/)
- Training data: [training data contract](https://voxlang.org/reference/mens-training-data-contract/)
- Serving: [MENS serving](https://voxlang.org/reference/mens-serving-ssot/)
- Source: [vox-foundation/vox](https://github.com/vox-foundation/vox)
- Machine-readable: `ladder_index.json` in this repository

This index is generated by `vox mens hub sync` from each size's records, and [how the cards are made]({{ f.docs }}#how-the-cards-are-made) is on the docs page.
