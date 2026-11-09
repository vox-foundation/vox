{{ f.size }} is
{%- if f.trained %} [trained]({{ f.docs }}#state-legend),
{%- if let Some(sp) = f.spoke %} its spoke is [{{ sp.id }}]({{ f.docs }}#hub-and-spoke) and
{%- endif %} its gate is [{{ f.gate.state.label() }}]({{ f.docs }}#state-legend)
{%- else %} [not trained]({{ f.docs }}#state-legend)
{%- endif %}
{%- if let Some(b) = f.gate.blocker %}:
{%- match b %}
{%- when Blocker::NoAdapter { record_reason } %}
{%- if let Some(r) = record_reason %} `train-record.json` says “{{ r }}”
{%- else %} no training run is on record for it
{%- endif %}
{%- when Blocker::NotPublished %} its training run is on record but the adapter weights are not in this repository
{%- when Blocker::Missing(names) %} the records that decide {{ f.size }} are missing:
{%- for n in names %}{% if !loop.first %},{% endif %} `{{ n }}`{% endfor %}
{%- when Blocker::Stale { refusal, recorded_slice_n, current_slice_n } %}
{%- if let Some(n) = recorded_slice_n %}
{%- if n != current_slice_n %} it passed the retired {{ n }}-item collateral rule and the {{ current_slice_n }}-item paired re-run for {{ f.size }} has not been done
{%- else %} its collateral report is out of date for the current rule: “{{ refusal }}”
{%- endif %}
{%- else %} its collateral report is out of date for the current rule: “{{ refusal }}”
{%- endif %}
{%- when Blocker::Failed { refusal, gates } %}
{%- if let Some(r) = refusal %} the collateral check refused {{ f.size }}: “{{ r }}”{% endif %}
{%- for g in gates %}{% if refusal.is_some() || !loop.first %};{% endif %} the `{{ g.0 }}` gate reported “{{ g.1 }}”{% endfor %}
{%- endmatch %}
{%- endif %}.