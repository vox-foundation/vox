# App Phase 4 — Clinician-grade export completion — Implementation Plan

**Goal:** Produce deterministic, clinician-shareable exports: full CSV (rows, not just header), JSON bundle with metadata + content hash over the full row set, and a real HTML clinical summary using the materializer's per-day timeline + weekly aggregates.

**Why:** Today `export_health_csv` returns only the header line, `export_health_json_bundle` hashes only the header (`content_sha256_header_only`), and `export_clinical_html` is a stub. The TS materializer is the SSOT for derived state — pull it in via the TS-source FFI extern (now landed) so the export pipeline emits real materialized output rather than re-implementing logic in Vox.

**Architecture:**
- New TS module `apps/vox-mental-tracker/src/ts/export_pipeline.ts` aggregates: rows → materializer → CSV via `export_contract` → JSON bundle with full content hash → HTML clinical view.
- Vox-side endpoints become thin proxies that call into TS via `extern fn` (plan #6 / PR #77 surface).
- `export_health_json_bundle` returns the bundle as a JSON string (computed by TS), keeping the Vox endpoint signature stable.
- `export_clinical_html` returns the HTML body string, computed in TS using the materializer's daily groups + weekly aggregate.

**Tech Stack:** TS (extending `materializer.ts` + `export_contract.ts`), Vox (extern fn), `WebCrypto` for SHA-256.

**Out of scope:**
- Per-event PDF rendering (browser-side print-to-PDF works; native PDF generation deferred).
- Encrypted bundle export (deferred).
- Localization of clinical headings.

---

## Tasks

### Task A — TS export pipeline

- [ ] **A1.** Create `apps/vox-mental-tracker/src/ts/export_pipeline.ts`:
  - `buildExportBundle(rows: HealthEventRow[], generatedMs: number): Promise<{ csv, json, html, contentSha256 }>`.
  - Internally: `resolveCorrections` → `buildHealthCsv(materialized)` → `sha256Hex(csv)` → return all four artifacts.
- [ ] **A2.** Update `buildJsonBundle` in `export_contract.ts` to accept the `contentSha256` and include it as `content_sha256` (replacing the existing "note" placeholder).
- [ ] **A3.** Add `renderClinicalHtml(materialized, weekly, daily, generatedMs, hash): string` — a minimal HTML page with: title, generated time, hash, weekly per-kind chart-as-table, daily timeline buckets with event payload preview.
- [ ] **A4.** Tests: `tests/export_pipeline.test.ts` — given a fixture row set, assert deterministic CSV (sorted), correct hash, and HTML containing all expected sections. Hash determinism is critical.

### Task B — Vox extern bridges

- [ ] **B1.** In `apps/vox-mental-tracker/src/main.vox`, declare:
  ```
  extern fn buildExportBundleSync(rows_json: str, generated_ms: int) to str = "./ts/export_pipeline_sync"
  ```
  (Sync wrapper around the async pipeline, returning a JSON-encoded `{csv,json,html,content_sha256}` string. The async-to-sync bridge runs on the browser side; Capacitor surface treats it as immediate.)
- [ ] **B2.** Add `apps/vox-mental-tracker/src/ts/export_pipeline_sync.ts` exporting `buildExportBundleSync`. For browser/web targets, use `await` inside an IIFE pattern that the codegen TS surface tolerates; for plain string return, blocking via `crypto.subtle.digest` is fine because emitted code can `await` the promise inside an `async` event handler.
  - **Note:** if the TS-FFI codegen requires sync extern signatures, refactor B1/B2 to take an already-hashed-elsewhere shape, or expose `buildExportBundleAsync` as a TS function callable from a generated React effect. Inspect `examples/golden/ts_source_ffi.vox` and `crates/vox-compiler/src/codegen_ts/` to confirm the calling convention.
- [ ] **B3.** Refactor `export_health_json_bundle()`, `export_health_csv()`, and `export_clinical_html()` to fetch all rows (already wired via `timeline_events_json`), pass to `buildExportBundleSync`, and return the relevant slice.

### Task C — UI integration

- [ ] **C1.** In `ExportPage`, add three buttons: **Copy CSV**, **Copy JSON bundle**, **Print HTML**. Each pulls from the appropriate endpoint and uses `clipboard.writeText` (web) or `Clipboard.write` (Capacitor).
- [ ] **C2.** Display the bundle's `content_sha256` so a clinician can verify integrity.
- [ ] **C3.** Add a "Download bundle (.zip)" button (deferred OK; document the gap).

### Task D — Verification

- [ ] **D1.** `pnpm test` — `export_pipeline.test.ts` passes; existing tests still pass.
- [ ] **D2.** `pnpm e2e` — Playwright covers ExportPage button clicks + clipboard read assertion.
- [ ] **D3.** Manual: open `/export`, save a few events on `/voice`, confirm CSV row count matches saved count, confirm hash is stable across reloads of the same data.
