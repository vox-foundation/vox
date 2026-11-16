## Use it

{% if f.is_complete_model() -%}
The {{ f.size }} complete merged checkpoint folds the {{ f.model_label }} trained add-on into the base pin `{{ f.base_pin }}` via `vox mens merge-qlora`, and this command downloads it (or take the files directly from the Files tab) after checking the BLAKE3 hash of every file in `hf_sync_manifest.json`.

```bash
vox mens hub download --repo {{ f.repo_id }} --output-dir {{ f.local_run_dir }}
```

Serve the {{ f.model_label }} (`{{ f.size }}`) checkpoint locally from `{{ f.local_run_dir }}` via `vox mens serve`:

```bash
vox mens serve --model {{ f.local_run_dir }} --port 11435
```

Query {{ f.model_label }} (`{{ f.size }}`) over Hugging Face Inference Providers (`:auto` or `:featherless-ai`) with an OpenAI-compatible client:

```python
from openai import OpenAI

client = OpenAI(base_url="https://router.huggingface.co/v1", api_key="$HF_TOKEN")
resp = client.chat.completions.create(
    model="{{ f.repo_id }}:auto",  # or "{{ f.repo_id }}:featherless-ai"
    messages=[{"role": "user", "content": "Write a Vox hello-world function."}],
)
print(resp.choices[0].message.content)
```

Load {{ f.model_label }} (`{{ f.size }}`) directly with `transformers`{% if f.uses_no_think_switch %} using the `/no_think` prompt suffix{% else %} using `enable_thinking=False`{% endif %}:

```python
from transformers import AutoModelForCausalLM, AutoTokenizer

tok = AutoTokenizer.from_pretrained("{{ f.repo_id }}")
model = AutoModelForCausalLM.from_pretrained("{{ f.repo_id }}", dtype="auto", device_map="auto")
{% if f.uses_no_think_switch -%}
prompt = tok.apply_chat_template(
    [{"role": "user", "content": "Write a Vox hello-world function. /no_think"}],
    tokenize=False,
    add_generation_prompt=True,
)
{% else -%}
prompt = tok.apply_chat_template(
    [{"role": "user", "content": "Write a Vox hello-world function."}],
    tokenize=False,
    add_generation_prompt=True,
    enable_thinking=False,
)
{% endif -%}
inputs = tok(prompt, return_tensors="pt").to(model.device)
out = model.generate(**inputs, max_new_tokens=256)
print(tok.decode(out[0], skip_special_tokens=True))
```

Host {{ f.model_label }} (`{{ f.size }}`) with `vLLM` or run it locally via `Ollama`:

```bash
vllm serve {{ f.repo_id }}
ollama run hf.co/{{ f.repo_id }}
```

Attach the {{ f.model_label }} (`{{ f.size }}`) LoRA adapter via `peft` (`subfolder="peft"`) onto `{{ f.base_repo }}`:

```python
from peft import PeftModel
from transformers import AutoModelForCausalLM, AutoTokenizer

base = AutoModelForCausalLM.from_pretrained("{{ f.base_repo }}", dtype="auto", device_map="auto")
model = PeftModel.from_pretrained(base, "{{ f.repo_id }}", subfolder="peft")
tok = AutoTokenizer.from_pretrained("{{ f.base_repo }}")
```
{%- else -%}
{% if !f.is_passed() -%}
This page currently distributes the {{ f.size }} trained add-on on `{{ f.base_pin }}` while the release review is unfinished, and the files are available from the Files tab or with this download command.

```bash
vox mens hub download --repo {{ f.repo_id }} --output-dir {{ f.local_run_dir }}
```

`vox mens serve` loads the {{ f.size }} add-on only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so no serving example is given while review is pending.
{%- else -%}
The {{ f.size }} adapter sits on the base pin `{{ f.base_pin }}`, and this command downloads it after checking the BLAKE3 hash of every file in `hf_sync_manifest.json`.

```bash
vox mens hub pull --size {{ f.size }} --namespace {{ f.namespace }}
```

The pull writes the {{ f.size }} run directory `{{ f.local_run_dir }}` that this command serves.

```bash
vox mens serve --model {{ f.local_run_dir }} --port 11435
```

Or load the {{ f.model_label }} (`{{ f.size }}`) PEFT adapter onto `{{ f.base_repo }}` with `transformers` and `peft`:

```python
from peft import PeftModel
from transformers import AutoModelForCausalLM, AutoTokenizer

base = AutoModelForCausalLM.from_pretrained("{{ f.base_repo }}", dtype="auto", device_map="auto")
model = PeftModel.from_pretrained(base, "{{ f.repo_id }}")
tok = AutoTokenizer.from_pretrained("{{ f.base_repo }}")
```
{%- endif %}{%- endif %}{%- if f.is_passed() %}
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
