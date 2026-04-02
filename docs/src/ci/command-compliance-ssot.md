---
title: "Command compliance SSOT"
description: "CI-side pointer to reference/command-compliance.md for vox ci command-compliance registry and docs parity; exists so check-docs-ssot file lists stay aligned with the canonical doc."
category: "ci"
---

# Command compliance SSOT

Registry + docs parity expectations for **`vox ci command-compliance`** live in **[`reference/command-compliance.md`](../reference/command-compliance.md)**.

CI-facing checklist copies should link there; this `docs/src/ci/` path exists so `check-docs-ssot` file lists stay consistent.

**Mandatory on PRs (`.github/workflows/ci.yml`):** `vox ci data-ssot-guards` runs as its own step after `vox ci command-compliance`. Broader local aggregate `vox ci ssot-drift` still includes additional nested guards (docs/SQL/contracts) for maintainers.

**Telemetry / trust SSOT (release hygiene):** When a change affects metric contracts, optional remote upload, or Clavis telemetry secrets, verify links from [telemetry-trust-ssot](../architecture/telemetry-trust-ssot.md) still resolve and add a **Telemetry** bullet under `CHANGELOG.md` [Unreleased] (see project changelog convention).
