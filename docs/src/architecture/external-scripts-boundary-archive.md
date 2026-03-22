---
title: "Out-of-scope external script boundary (archive)"
category: architecture
last_updated: 2026-03-21
---

# Explicitly out of scope for Rust migration

- **Third-party GitHub Actions** (checkout, cache, toolchain installers) — remain YAML-native.
- **GPU / CUDA host setup** on self-hosted runners — may use shell bootstrap outside `vox ci`.
- **Hugging Face / cloud publish** flows in ML workflows — optional `uv`/`curl` steps where no stable Rust API exists yet.

Record new long-lived shell **guard** logic in [`docs/agents/script-registry.json`](../../agents/script-registry.json) and prefer a `vox ci` subcommand if the check must be reproducible on developer laptops.
