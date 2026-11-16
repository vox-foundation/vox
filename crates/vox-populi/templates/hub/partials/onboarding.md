## Vox and Python

Beginners can write and run their first `.vox` file alongside {{ f.model_label }} (`{{ f.size }}`) in under a minute, or call `{{ f.repo_id }}` from Python and Rust.

**Try Vox in 60 seconds ({{ f.size }})**

[Vox](https://voxlang.org) keeps your data model, backend queries, mutations, and helpers in one compiler-checked file so {{ f.model_label }} (`{{ f.size }}`) can edit your whole feature cleanly:

```vox
table Note {
    title: str
    done: bool
}

query list_notes() to Result[list[Note], str] {
    return db.Note.all()
}

mutation add_note(title: str) to Result[int, str] {
    return db.Note.insert({ title: title, done: false })
}

fn format_status(done: bool) to str {
    if done { return "done" }
    return "open"
}
```

Type-check and run your first program locally while {{ f.model_label }} (`{{ f.size }}`) assists in the same workspace:

```bash
vox check main.vox
vox run --interp main.vox
vox llm prompt "Add an archive mutation to main.vox"
```

**Use {{ f.size }} from Vox and Rust**

Inside a Vox workspace or Rust crate, route chat requests to `{{ f.repo_id }}` through `vox_actor_runtime::llm::llm_chat` instead of hardcoding provider URLs:

```bash
export HF_CHAT_MODEL="{{ f.repo_id }}"
export HF_TOKEN="hf_your_token_here"
export VOX_POPULI_LOCAL_OLLAMA_URL="http://127.0.0.1:11435"
```

```rust
use vox_actor_runtime::inference_env::resolve_huggingface_router;
use vox_actor_runtime::model_resolution::{RouteResolutionInput, resolve_chat_provider_route};

// Build a model-agnostic route to {{ f.repo_id }} via Hugging Face Router or local MENS:
let hf_endpoint = resolve_huggingface_router("{{ f.repo_id }}");
```

**Use {{ f.size }} from Python**

Call `{{ f.repo_id }}` with the standard `openai` Python client through Hugging Face Inference Providers or a local OpenAI-compatible `vox mens serve` endpoint:

```python
import os
from openai import OpenAI

# Cloud inference via Hugging Face Router (:auto selects the fastest available host)
client = OpenAI(
    base_url="https://router.huggingface.co/v1",
    api_key=os.environ["HF_TOKEN"],
)
# Or target your local machine after running `vox mens serve`:
# client = OpenAI(base_url="http://127.0.0.1:11435/v1", api_key="local")

reply = client.chat.completions.create(
    model="{{ f.repo_id }}:auto",
    messages=[{"role": "user", "content": "Write a Vox query that returns open Note rows."}],
)
print(reply.choices[0].message.content)
```

Manage Python dependencies cleanly inside a Vox project for {{ f.model_label }} (`{{ f.size }}`) using `[python]` in `Vox.toml` and `vox py`:

```toml
# Vox.toml
[python]
version = "3.12"
packages = ["openai", "transformers", "peft"]
```

```bash
vox py sync
vox py doctor
vox py bind openai
```

Generate a typed Python client from a Vox service API backed by {{ f.model_label }} (`{{ f.size }}`) using OpenAPI:

```bash
vox emit openapi src/main.vox --out dist/openapi.json
openapi-python-client generate --path dist/openapi.json
```
