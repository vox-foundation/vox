# Deferred items (phase 317)

- **WR-03 (316 review):** `snapshot_complete` requires every indexed shard while `vox mens download` skips vision/MTP-only shards. This affects non-dense checkpoints only. The six dense pins' indexes list only their own shards (checked 2026-10-27), so the dense ladder is unaffected.
- **`Qwen/Qwen3.8-27B@1d4bf0f2ff6012fd82039f2fa52739d0dd7c60c0`** in `strong_code_default` and `agentic_default` is not a real Qwen3 dense size and has no license entry. Removing or relicensing it is deferred. It is not part of `qwen3_code`.
