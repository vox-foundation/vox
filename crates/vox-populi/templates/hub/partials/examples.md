## Examples

{% if f.heldout.is_none() -%}
No held-out report is on record for {{ f.size }}, so no example is shown for it.
{% else -%}
{%- if let Some(ex) = f.examples -%}
For {{ f.size }}, the examples are [
{%- if ex.wins.len() == 2 -%}the 2 shortest items the adapter fixes
{%- else if ex.wins.len() == 1 -%}the one item the adapter fixes
{%- else -%}none that the adapter fixes
{%- endif -%}
{%- if ex.loss.is_some() %} and the shortest it still fails{% else %} and no item it still fails{% endif %}]({{ f.docs }}#examples-on-the-cards); of {{ ex.total }} items it fixes {{ ex.fixes }} and breaks {{ ex.breaks }}.
{%- if ex.unmatched_ids > 0 %} The {{ f.size }} held-out reports do not share every item: the count of ids found in only one of them is {{ ex.unmatched_ids }}, and none is in the counts above.{% endif %}
{% for (win, e) in ex.blocks() %}
### {% if win %}Fixed{% else %}Still fails{% endif %} `{{ e.id }}`

{{ e.prompt_fence() }}text
{{ e.description }}
{{ e.prompt_fence() }}
{% for (is_base, s) in e.sides() %}
{% if is_base %}Qwen3-{{ f.size }} base{% else %}{{ f.size }} adapter{% endif %} on `{{ e.id }}`: {{ s.verdict.phrase() }}
{%- if let Some(n) = s.verdict.count() %} (count {{ n }}){% endif %}
{%- if let Some(c) = s.verdict.check() %} `{{ c }}`{% endif %}.

{% if let Some(code) = s.code -%}
{{ s.fence() }}{{ s.lang() }}
{{ code }}
{{ s.fence() }}
{%- else -%}
```text
no recorded output
```
{%- endif %}
{% endfor %}
{%- endfor %}
{%- else -%}
{%- match f.adapter_evidence %}
{%- when AdapterEvidence::Current -%}
The {{ f.size }} held-out reports hold no per-item results to pick from, so no example is shown for {{ f.size }}.
{%- when AdapterEvidence::NoCheckpoint -%}
No {{ f.size }} example is shown because no adapter checkpoint records its serving path, so the {{ f.size }} adapter output cannot be tied to the current serving path.
{%- when AdapterEvidence::Stale { recorded } -%}
No {{ f.size }} example is shown because the adapter checkpoint
{%- if let Some(r) = recorded %} records the serving path `{{ r }}`
{%- else %} has no serving field
{%- endif %}, so its outputs do not count under the [serving-path rule]({{ f.docs }}#held-out-pass-rate).
{%- endmatch %}

```text
no recorded output
```
{% endif %}
{%- endif %}
