---
title: "Knip GUI deepen backlog"
description: "v5.5: dual knip passes and local pin shipped; Effect schema preprocessor remains optional backlog."
category: "CI & Quality"
status: "current"
topic: "ci-guards"
domain: "platform"
authorship: "llm-generated"
authorship_model: "unknown"
---

# Knip GUI deepen (T3-HOST-SCHEMA-03)

<!-- kb:summary -->
Status of the knip unused-code checks for the vox-gui UI after milestone v5.5. Shipped: a pinned
knip version in the UI package, a dual pass that checks unused files and dependencies first
(errors) and unused exports second (warnings, because the transport surface is wide), a
`vox ci knip-gui --check` wrapper, and both passes running in UI CI after vitest. The only
remaining backlog item is an optional preprocessor for unused Effect Schema exports, which is
not needed for the dual-pass setup to count as shipped.
<!-- /kb:summary -->

**Shipped in v5.5:**

- `knip@5.50.0` pinned in `crates/vox-gui/ui/package.json`
- Dual pass: `pnpm run knip:files` then `pnpm run knip:exports` (also `vox ci knip-gui --check`)
- `exports` / `nsExports` are **warn** (wide transport surface); `files` / `dependencies` are **error**
- UI CI runs both scripts after vitest

**Still optional backlog:**

- Effect Schema unused-export preprocessor (t3code `scripts/knip-schemas.ts`) — no Vox Effect-TS analog required for dual-pass ship.
