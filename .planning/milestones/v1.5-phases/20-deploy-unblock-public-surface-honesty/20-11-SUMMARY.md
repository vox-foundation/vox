# Plan 20-11 Summary: Post-Deploy Smoke Verification & Final Deploy Evidence

## What was built
- **LLMS Link Extractor & Health Checker (`docs-astro/tests/lib/llms-links.mjs`)**:
  - `extractLlmsUrls(text, { origin, baseUrl })`: extracts, dedupes, and maps internal URLs in `llms.txt` and `.well-known` manifests, supporting base URL rewriting for testing against local preview servers.
  - `checkUrls(urls, fetcher, concurrency)`: asynchronously checks a batch of URLs with bounded concurrency (default 8), reporting non-2xx responses and connection exceptions.
- **Unit Test Coverage (`docs-astro/tests/unit/llms-links.test.mjs`)**:
  - 4/4 passing unit tests covering normal URL extraction/rewriting, 200 OK batch verification, 404 detection (negative proof), and thrown fetch error handling.
- **Extended Live & Preview Smoke Suite (`docs-astro/tests/smoke.spec.ts`)**:
  - `/voxup`: verified returns status 200 and begins with `#!` shebang.
  - `/voxup.ps1`: verified returns status 200.
  - `/llms.txt`, `/.well-known/llms.txt`, `/.well-known/llms-full.txt`: each manifest is fetched, all URLs extracted and verified to resolve (2xx), and verified that no URL path maps to an Internals-status document.
  - Retired syntax scan: samples 12 reader pages (home, 7 tutorials, installation, syntax reference, CLI, stability matrix), extracts main content, excludes historical/carveout lines, and verifies no line matches any pattern in `contracts/documentation/retired-symbols.v1.yaml`.
  - Archive redirect test: verifies `/archive/p20-redirect-probe/` returns 301 with `location` pointing to `/retired/` (skipped on localhost where Cloudflare Pages `_redirects` rules do not run).

## Local Verification & Mutation Proofs
- Unit tests: `node --test tests/unit/llms-links.test.mjs` passed (4/4).
- Local smoke suite: run against `npx pnpm preview` on `http://localhost:4321`:
  - 14 passed, 1 skipped (`archive redirect` cleanly skipped on preview server).
- Negative / Mutation proofs:
  - Unit test: fake fetcher returning 404 causes `checkUrls` to return `ok: false` and identify the failing URL.
  - Smoke test: injected `@component fn` into `<main>` of built `dist/index.html`; rerun failed immediately with `Retired syntax violations found: [/] component-fn-decorator`. Restored index; re-run returned green.

## Live Deployment & GitHub State (Task 3 Evidence)
- **Live Archive Redirect**:
  - `curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' https://voxlang.org/archive/p20-redirect-probe/`
  - Output: `301 https://voxlang.org/retired/` (verified live Cloudflare Pages redirect).
- **Issue #462 (Deploy Outage Root Cause & Resolution)**:
  - State: **CLOSED** (closed at `2026-10-09T16:56:12Z`).
  - Root cause resolution comment: [Comment #6085846292](https://github.com/vox-foundation/vox/issues/462#issuecomment-6085846292) detailing build break history (May–September), Cloudflare Pages token rotation (`Pages:Edit`), and safeguards instituted under #635.
- **Push Deploy Runs**:
  - Latest push deploy run: [Run 38286987185](https://github.com/vox-foundation/vox/actions/runs/38286987185) (`conclusion: success`).
  - Prior push runs: 38213799637 (`success`), 37982186858 (`success`), 37960645178 (`success`).
- **Failure & Liveness Drills**:
  - Simulated failure dispatch runs: 37975765942, 37975877478.
  - Active open `docs-deploy-broken` issues: **0** (issue closed automatically upon recovery run).
  - CI liveness runs: verified green on schedule and workflow dispatch.
