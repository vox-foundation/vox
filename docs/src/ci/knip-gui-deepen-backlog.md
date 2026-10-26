---
title: "Knip GUI deepen backlog"
description: "v5.5 Partial: dual knip passes and schema unused-export preprocessor vs current knip-gui gate."
category: "CI & Quality"
status: "current"
---

# Knip GUI deepen backlog (T3-HOST-SCHEMA-03)

Shipped in v5.4 / retained in v5.5:

- `crates/vox-gui/ui/knip.jsonc`
- `vox ci knip-gui`

Still Partial (honest):

1. **Local knip pin** — prefer a workspace-locked knip version over ephemeral `pnpm dlx` when CI can afford the lock entry.
2. **Dual pass** — t3code runs files+dependencies then exports; Vox currently runs one knip invocation.
3. **Schema preprocessor** — t3code `scripts/knip-schemas.ts` Effect Schema unused-export awareness has no Vox analog yet (`vox-schema` / contract JSON are separate SSOT).

Do not mark T3-HOST-SCHEMA-03 Shipped until (1)+(2) land; (3) may remain backlog with owner.
