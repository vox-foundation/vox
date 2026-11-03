# 332-01 / 332-02 SUMMARY

**Completed:** 2026-11-02 (source + unit tests; live tagging is verified in 332-04)

- **332-01:** `PublishOutcome::Uploaded` carries `version`; publish creates the next `v<N>` tag at the uploaded commit; an unchanged but untagged repo gets a baseline `v1`; the index README and `ladder_index.json` list `latest_version`.
- **332-02:** `pull_adapter_revision` takes a tag, branch or commit (default: latest `v<N>`, else `main`); new `vox mens hub pull --size --revision --allow-ungated`.
- Default visibility is now public (auto-publish hooks and `hub sync`). The four existing repos were set public on 2026-11-02.
- Tests: `cargo test -p vox-populi --features mens-hf-hub --lib hub_` (18 passed, 1 ignored live); `vox-ml-cli` hub (3 passed).
