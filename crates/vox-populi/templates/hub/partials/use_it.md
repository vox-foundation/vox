## Use it

The {{ f.size }} adapter sits on the base pin `{{ f.base_pin }}`, and this command downloads it after checking the BLAKE3 hash of every file in `hf_sync_manifest.json`.

```bash
vox mens hub pull --size {{ f.size }} --namespace {{ f.namespace }}{% if !f.is_passed() %} --allow-ungated{% endif %}
```
{% if !f.is_passed() %}
`vox mens serve` loads the {{ f.size }} adapter only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so the pull needs `--allow-ungated` and no serving example is given.

The docs page explains [why the {{ f.size }} adapter cannot be loaded with transformers or PEFT]({{ f.docs }}#why-peft-cannot-load-the-adapters).
{% else %}
The pull writes the {{ f.size }} run directory that this command serves.

```bash
vox mens serve --model mens/runs/qwen3-{{ f.size|lower }} --port 11435
```
{% if let Some(u) = f.use_it() %}{% if let Some(req) = f.request_json %}
This request sends the user turn of held-out item `{{ u.id }}` to the {{ f.size }} server, which applies its `codegen` lane to it.

{{ f.request_fence() }}bash
curl -s http://127.0.0.1:11435/v1/chat/completions -H 'Content-Type: application/json' -d @- <<'JSON'
{{ req }}
JSON
{{ f.request_fence() }}
{% endif %}{% if let Some(code) = u.adapter.code %}
The code below is the {{ f.size }} adapter's recorded output for item `{{ u.id }}` in the held-out evaluation, taken from `evidence/heldout_adapter_nothink.ckpt.jsonl`, where the `codegen` lane's prompt protocol `{{ f.protocol }}` wrapped the same user turn and every stored check passed; it is not a transcript of `vox mens serve`.
{%- if !f.decode_recorded %} The {{ f.size }} report does not record the evaluation's decode settings, so the request above leaves them out.{% endif %}

{{ u.adapter.fence() }}vox
{{ code }}
{{ u.adapter.fence() }}
{% endif %}{% else %}{% match f.adapter_evidence %}
{%- when AdapterEvidence::Current %}
No recorded passing {{ f.size }} output is available for a request beyond the examples, so none is shown for {{ f.size }}.
{%- when AdapterEvidence::NoCheckpoint %}
The {{ f.size }} adapter output for a request is not shown because no adapter checkpoint records its serving path.

```text
no recorded output
```
{%- when AdapterEvidence::Stale { recorded } %}
The {{ f.size }} adapter output for a request is not shown because its checkpoint
{%- if let Some(r) = recorded %} records the serving path `{{ r }}`
{%- else %} has no serving field
{%- endif %}, so it does not count under the [serving-path rule]({{ f.docs }}#held-out-pass-rate).

```text
no recorded output
```
{%- endmatch %}
{% endif %}{% endif %}
