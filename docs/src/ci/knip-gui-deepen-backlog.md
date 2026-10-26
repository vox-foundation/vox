---
title: "Knip GUI deepen backlog"
description: "v5.5: dual knip passes and local pin shipped; Effect schema preprocessor remains optional backlog."
category: "CI & Quality"
status: "current"
---

# Knip GUI deepen (T3-HOST-SCHEMA-03)

**Shipped in v5.5:**

- `knip@5.50.0` pinned in `crates/vox-gui/ui/package.json`
- Dual pass: `pnpm run knip:files` then `pnpm run knip:exports` (also `vox ci knip-gui --check`)
- `exports` / `nsExports` are **warn** (wide transport surface); `files` / `dependencies` are **error**
- UI CI runs both scripts after vitest

**Still optional backlog:**

- Effect Schema unused-export preprocessor (t3code `scripts/knip-schemas.ts`) — no Vox Effect-TS analog required for dual-pass ship.
