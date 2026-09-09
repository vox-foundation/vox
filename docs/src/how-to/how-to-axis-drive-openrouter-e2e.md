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
- `reply_ok` is intentional: bare `reply` means only that a turn settled and
  can also match an error response.
- Stop an orphaned session with `vox gui drive stop`.
