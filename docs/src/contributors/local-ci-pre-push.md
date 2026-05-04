---
title: "Local CI parity (pre-push)"
description: "Run the merge-blocking subset locally before every push via `vox ci pre-push`."
category: "contributors"
last_updated: "2026-05-03"
training_eligible: true
schema_type: "TechArticle"
---

# Local CI parity (pre-push)

`vox ci pre-push` runs the merge-blocking subset of `.github/workflows/ci.yml`
locally so failures show up before the GitHub round-trip.

## Modes

| Mode | Steps | Typical wall-clock |
|------|-------|--------------------|
| `--quick` | fmt-check, line-endings, ssot-drift | ~30 s |
| default | + doc-inventory verify, clippy (workspace), TOESTUB on changed `crates/<x>` | ~2–4 min |
| `--full` | + `cargo nextest run --workspace` | ~10–25 min |

## Install the git hook (one-time)

```bash
cargo run -q -p vox-cli -- ci install-hooks
```

This writes `.git/hooks/pre-push` as a one-line delegate to
`vox ci pre-push`. The generated stub honours
[AGENTS.md §VoxScript-First Glue Code](../../../AGENTS.md) — no business logic
in shell.

## Bypass

`git push --no-verify` skips the hook. Use sparingly; CI still runs.

## Tuning the diff base

The TOESTUB-scoped step looks at `git diff origin/main...HEAD` by default.
Override with `VOX_PREPUSH_BASE=<ref>` (e.g. `VOX_PREPUSH_BASE=HEAD~1`).
