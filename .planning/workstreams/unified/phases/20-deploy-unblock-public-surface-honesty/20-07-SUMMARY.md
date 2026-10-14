---
phase: 20-deploy-unblock-public-surface-honesty
plan: 07
status: complete
completed_at: "2026-10-12T21:19:30-07:00"
requirements:
  - HONEST-02
  - DEPLOY-03
measured:
  llms_full_bytes_before: 16228159
  llms_full_bytes_after: 12582650
  llms_full_bytes_reduced: 3645509
  internals_headings_excluded: 218
---

# Plan 20-07 Summary: LLMS Exclusion for Internals & Agent File Honesty

## Accomplishments

1. **Patched `starlight-llms-txt@0.10.0`**
   - Created reproducible patch `docs-astro/patches/starlight-llms-txt@0.10.0.patch` via `pnpm patch`.
   - Patched `llms-full.txt.ts` and `llms-custom.txt.ts` to forward `exclude: starlightLllmsTxtContext.exclude` to `generateLlmsTxt`.
   - Added `patchedDependencies` to `docs-astro/pnpm-workspace.yaml` and verified with `pnpm install --frozen-lockfile`.

2. **Configured Internals Exclusion in `astro.config.mjs`**
   - Wired `exclude: internalsDocIds(docPages)` to `starlightLlmsTxt` plugin options.
   - Removed dead configuration `llmsFullTxt: true`.

3. **Hand-Written `.well-known` Honesty**
   - Removed research link `front-facing-honesty-audit-2026` from `docs/src/.well-known/llms.txt`.
   - Removed legacy link `mcp-tool-reference` from `docs/src/.well-known/llms-full.txt`.

4. **Automated Verification (`tests/build-output/llms.spec.ts`)**
   - Verified red before patch: `Found internals page headings in llms-full.txt: Expected: 0, Received: 218`.
   - Verified green after patch & build: all 3 tests passing.
   - Measured `llms-full.txt` size reduction from 16,228,159 bytes down to 12,582,650 bytes (3.65 MB reduction).
