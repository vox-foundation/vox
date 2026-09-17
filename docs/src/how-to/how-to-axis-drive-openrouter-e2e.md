---
title: "How To: Prove Axis Drive OpenRouter Chat"
description: "Run the live Axis Drive OpenRouter proof and diagnose honest failure states."
category: "How-To Guides"
status: "current"
---

# Axis Drive OpenRouter e2e

This proof exercises the live Axis Drive path: it starts a dedicated Drive
session, loads the model catalog, selects a selectable OpenRouter model, sends
a short prompt, and waits for `reply_ok`. It exits non-zero if the catalog is
missing, the reply is an error, or the Drive event trail lacks `submit_ok`.

## Run it

Build `vox` first, then run:

```bash
vox run scripts/axis-drive-openrouter-e2e.vox
```

The script prefers `openrouter/auto`, then other `openrouter/*` rows, and
finally a selectable row whose `provider_type` is `OpenRouter`. It rejects
`mens/` models and bare tier identifiers. It does not read an OpenRouter key
from the environment; configure the canonical
`SecretId::OpenRouterApiKey` and inspect it with:

```bash
vox secrets doctor
```

The script prints a summary JSON object only after `reply_ok` succeeds. A
successful result also requires a live plane, no `last_error`, a non-empty
non-error assistant bubble, and a `submit_ok` event for the final turn.

## Troubleshooting

- A missing selectable model means the OpenRouter secret is unavailable or
  provider status could not be loaded. Run `vox secrets doctor`.
- Step 0 stops a prior Drive session and best-effort kills a foreign
  `vox-orchestrator-d` so the GUI refuses adopt and spawns under
  `VOX_GUI_DRIVE=1` with the vault path + cloud keys. If `orch_fresh` is
  false after start, a keyless foreign daemon is still holding the port.
- `send` exits non-zero when the JSON body has `last_error` even if HTTP is
  200. Prefer `wait --until reply_ok` (requires `submit_ok` for
  `last_turn_id`) over bare `reply`.
- Drive HTTP hop timeout is `D_180S` (see
  `docs/superpowers/specs/2026-09-10-axis-chat-surface-audit-design.md` §9).
- Catalog fetch uses `MODEL_LIST_LIMIT` (2000), same as Loquela.
- `reply_ok` is intentional: bare `reply` means only that a turn settled and
  can also match an error response.
- Stop an orphaned session with `vox gui drive stop`.
