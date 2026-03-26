---
title: "Clavis SSOT"
description: "Canonical secret-management source of truth for Vox Clavis"
category: "reference"
last_updated: 2026-03-25
training_eligible: true
---

## Clavis SSOT

`vox-clavis` is the canonical source of truth for managed secret metadata and resolution precedence.

## Naming Convention

- `VOX_*`: Vox-owned platform contracts (mesh, runtime auth, DB, cloud orchestration, internal boundaries).
- Provider-native names (for example `OPENROUTER_API_KEY`, `OPENAI_API_KEY`): upstream ecosystem names kept for compatibility.
- Optional `VOX_*` provider aliases are accepted as migration aids; canonical names remain stable.

## Secret Inventory (Phase 0)

| Secret | Scope | Tier | Primary consumer surfaces |
| --- | --- | --- | --- |
| `OPENROUTER_API_KEY` / `GEMINI_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` | LLM inference | Minimal cloud LLM | `vox-mcp`, `vox-runtime`, `vox-cli doctor/status` |
| `HF_TOKEN` | LLM retrieval / HF router | Optional | `vox-config`, HF routes |
| `GROQ_API_KEY`, `CEREBRAS_API_KEY`, `MISTRAL_API_KEY`, `DEEPSEEK_API_KEY`, `SAMBANOVA_API_KEY`, `CUSTOM_OPENAI_API_KEY` | Alternative LLM providers | Optional power-user | provider-specific runtime/mcp paths |
| `VOX_RUNPOD_API_KEY`, `VOX_VAST_API_KEY` | Cloud GPU infra | Optional cloud GPU | `vox-populi` cloud providers |
| `TOGETHER_API_KEY` | Remote fine-tune API | Optional cloud training | `vox-cli train --provider together` |
| `GITHUB_TOKEN` | Publishing/review automation | Workflow-specific required | `vox-cli review/publish` |
| `VOX_NEWS_TWITTER_TOKEN`, `VOX_NEWS_OPENCOLLECTIVE_TOKEN`, `VOX_SOCIAL_REDDIT_*`, `VOX_SOCIAL_YOUTUBE_*` | Scientia/news syndication | Optional (per channel) | `vox-publisher` resolves via Clavis `SecretId` specs; GitHub syndication also accepts `VOX_NEWS_GITHUB_TOKEN` as an alias of `GITHUB_TOKEN` |
| `VOX_DB_URL`, `VOX_DB_TOKEN` | Remote DB | Workflow-specific required | DB remote flows |
| `VOX_MESH_TOKEN` | Populi control-plane auth | Workflow-specific required | Mesh transport/auth |
| `VOX_API_KEY`, `VOX_BEARER_TOKEN` | Runtime ingress auth | Optional hardening | `vox-runtime` auth gate |
| `V0_API_KEY`, `VOX_OPENCLAW_TOKEN` | Auxiliary tooling | Optional | island generation / OpenClaw |

## Managed Secret Env Names

- `GEMINI_API_KEY`
- `VOX_GEMINI_API_KEY`
- `GOOGLE_AI_STUDIO_KEY`
- `OPENROUTER_API_KEY`
- `VOX_OPENROUTER_API_KEY`
- `OPENAI_API_KEY`
- `VOX_OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `VOX_ANTHROPIC_API_KEY`
- `HF_TOKEN`
- `VOX_HF_TOKEN`
- `HUGGING_FACE_HUB_TOKEN`
- `GITHUB_TOKEN`
- `VOX_GITHUB_TOKEN`
- `VOX_NEWS_GITHUB_TOKEN`
- `GH_TOKEN`
- `VOX_NEWS_TWITTER_TOKEN`
- `VOX_NEWS_OPENCOLLECTIVE_TOKEN`
- `VOX_SOCIAL_REDDIT_CLIENT_ID`
- `VOX_SOCIAL_REDDIT_CLIENT_SECRET`
- `VOX_SOCIAL_REDDIT_REFRESH_TOKEN`
- `VOX_SOCIAL_REDDIT_USER_AGENT`
- `VOX_SOCIAL_YOUTUBE_CLIENT_ID`
- `VOX_SOCIAL_YOUTUBE_CLIENT_SECRET`
- `VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN`
- `GROQ_API_KEY`
- `VOX_GROQ_API_KEY`
- `CEREBRAS_API_KEY`
- `VOX_CEREBRAS_API_KEY`
- `MISTRAL_API_KEY`
- `VOX_MISTRAL_API_KEY`
- `DEEPSEEK_API_KEY`
- `VOX_DEEPSEEK_API_KEY`
- `SAMBANOVA_API_KEY`
- `VOX_SAMBANOVA_API_KEY`
- `CUSTOM_OPENAI_API_KEY`
- `VOX_CUSTOM_OPENAI_API_KEY`
- `V0_API_KEY`
- `VOX_V0_API_KEY`
- `VOX_OPENCLAW_TOKEN`
- `TOGETHER_API_KEY`
- `VOX_TOGETHER_API_KEY`
- `VOX_RUNPOD_API_KEY`
- `VOX_VAST_API_KEY`
- `VOX_API_KEY`
- `VOX_BEARER_TOKEN`
- `VOX_DB_URL`
- `VOX_TURSO_URL`
- `TURSO_URL`
- `VOX_DB_TOKEN`
- `VOX_TURSO_TOKEN`
- `TURSO_AUTH_TOKEN`
- `VOX_MESH_TOKEN`

## Resolution Precedence

For each managed secret ID:

1. canonical env name
2. non-deprecated aliases (including opt-in `VOX_*` aliases)
3. deprecated aliases (returns `DeprecatedAliasUsed` status)
4. configured external backend (`infisical` or `vault`, when enabled)
5. secure local store
6. compatibility file stores (`~/.vox/auth.json`, legacy `~/.vox/auth_token`, `.vox/populi/mesh.env` where applicable)

## Required vs Optional Model

- `vox clavis doctor` evaluates **blocking requirement groups** (`AnyOf`/`AllOf`) per workflow/profile.
- `Chat`/`Mcp` blocking model in cloud mode is **OpenRouter-first** (`OPENROUTER_API_KEY` / `VOX_OPENROUTER_API_KEY`); alternate providers are optional capability keys.
- `local` mode requires no cloud key; `auto` resolves from `VOX_INFERENCE_PROFILE`.
- Optional keys are reported separately as capability unlocks (not startup blockers).
- OpenRouter does not replace RunPod/Vast keys: LLM gateway credentials and cloud GPU credentials are distinct domains.

## Canonical Bundles

- `minimal_local_dev`: zero required cloud keys.
- `minimal_cloud_dev`: OpenRouter only.
- `gpu_cloud`: RunPod or Vast key (plus Together optional).
- `publish_review`: GitHub token only.

## Transition and Deprecation Window Policy

1. Add alias support first (no breakage).
2. Emit `DeprecatedAliasUsed` in doctor for legacy aliases.
3. Keep legacy aliases for at least two release trains after warning lands.
4. Remove legacy aliases from docs examples first; remove runtime support only after explicit release note and CI parity update.

## Command Surfaces

- `vox clavis doctor --workflow <...> --profile <dev|ci|mobile|prod> --mode <auto|local|cloud> [--bundle <minimal-local-dev|minimal-cloud-dev|gpu-cloud|publish-review>]`
- `vox clavis set <registry> <token> [--username <name>]`
- `vox clavis get <registry>`
- `vox clavis backend-status`
- `vox clavis migrate-auth-store`
