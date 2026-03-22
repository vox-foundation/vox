# Deprecation policy — Populi native fine-tuning

## Stable

- **`vox populi train`** with `--backend lora` and `--backend qlora`.
- **`vox populi merge-qlora`** (alias **`merge-adapter`**).
- **`vox populi merge-weights`** for Burn `*.bin` LoRA checkpoints.

## Deprecated / transitional

- **`vox train --native-lora`**: use **`vox populi train --backend lora`** (stderr deprecation already emitted from dispatch).
- **Backend-only mental model**: prefer the **contract** fields (tokenizer mode, quant mode, adapter method) when scripting; CLI flags remain the user-facing surface until a preset/JSON contract ships.

## Timeline

- No CLI flags removed in this iteration; aliases added (`merge-adapter`).
- Future removal of legacy paths will be announced in this doc + `populi-training-ssot.md` with one release notice.
