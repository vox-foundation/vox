# Phase 482: `/gsd-ui-review` — Milestone `v8.1` Research & Knowledge Base Overhaul

**Workstream:** `.planning/workstreams/scientia-kb-roman-ux-v81/`
**Reviewed Scope:** Research (`research`), Scientia (`scientia`, `scientia-claims`), Publications (`publications`), Discovery & Activity (`activity`), Knowledge Base Memory (`memory`), Search Index / VoxGraph (`vox-search`), Doc Reader Drawer (`doc-reader`), and shared `BreadcrumbBar` across Imperial Carrara (`arcane`) and Minimalist Basalt Slate (`void`).
**Status:** Signed Off — `16/16` Playwright CV Flywheel renders converged, `0` DOM/contrast/font/toolbar-spread/emoji/accessibility violations, `tsc --noEmit` exit `0`, `16/16` Vitest suites (`125/125` tests) passing.

---

## 1. 6-Pillar `/gsd-ui-review` Scorecard

| Pillar | Score (/4) | Summary of Remediations & Verified Evidence |
|---|---|---|
| **1. Copywriting & Information Hierarchy** | **4 / 4** | Replaced flat vertical sprawl and raw unparsed markdown `<pre>` blocks in `DocReader` with structured headings (`ResearchReportMarkdown`, `max-w-[74ch]`), concise status/trust taxonomy badges (`Verified SSOT`, `VerdictBadge`), and actionable 3-stage Claims Ledger empty-state instructions (`1. VeriScore Extraction`, `2. MiniCheck Audit`, `3. Archival Ledger`). |
| **2. Visuals & Concept-Art Alignment** | **4 / 4** | Regenerated 4 `v2` Vox-native concept art plates (`v81-concept-{scientia-research,kb-codex}-{carrara,slate}.jpg`) matching `v74` Romanesque pillars, twin brass needle gauges, carved bas-relief headers, and obsidian Slate framing (`vlmOverallScore` `0.9516–0.9564`; Slate histogram similarity up to `0.9054`). Zero raw emojis or dingbats (`zeroEmojiAndDingbat.test.ts`). |
| **3. Color & Theme Contract Fidelity** | **4 / 4** | Purged hardcoded purple/indigo/cyan/teal/emerald-950/amber-950 wave leaks (`scientia-void` `digitalColorLeakCount`: `6 -> 0`; `activity-void`: `10 -> 0`) and light-cream-on-Carrara text leaks (`#F3E5AB`, `#D6CEBF`). Standardized surfaces on `.vox-marble-tablet`, `--color-gold-500`, `--color-border-strong`, and semantic `--color-status-*` tokens. |
| **4. Typography & Readability** | **4 / 4** | Eliminated `< 10.5px` micro-fonts (`BreadcrumbBar.tsx:115,154`, `CodeAtlasCanvas.tsx`, `ResearchView.tsx`, `MemoryView.tsx`; `unreadableSmallFontCount: 0` across all 16 captures). Restricted `--font-mono` / `--font-terminal-mono` to IDs, SHAs, and numeric metrics while elevating hit snippets (`13px`), claim cards, and reader prose to `--font-serif` / `--font-sans`. |
| **5. Spacing, Framing & Control Geometry** | **4 / 4** | Eliminated duplicate stacked headers (`ScientiaSurface` passes `hideHeader` to `ScientiaDashboard`), prevented `DocReader` sub-tab wrap/clipping (`Library & Reader`, `Findings Inbox`, `OpenWiki Self-Heal`, `KB Health & Freshness` on 1 horizontal row), and standardized button/tab heights (`h-8`/`h-9`; `surfaceToolbarHeightSpreadPx: 0`, `controlHeightSpreadPx: 0`). |
| **6. Experience Design, Click-Tax & Disclosure** | **4 / 4** | Added 1-click `ResearchView` detail workspace switching (`Workbench Overview`, `Synthesis & Citations`, `DAG & Claims`, `Diagnostics & Waves`) with above-the-fold Synthesis + Epistemic DAG split, surfaced always-visible `Open` and 1-click `Pin` buttons on `MemoryView` `HitCard`, and wrapped verbose citations/provenance/diff hunks/shard diagnostics in collapsible `<details>` disclosures. |

**Overall Rating:** **24 / 24 (`PASS`)**

---

## 2. Playwright + `@ngi/platform-ssot` CV Flywheel Metrics (`Before -> After`)

Source ledgers:
- Before: `crates/vox-gui/ui/artifacts/cv-flywheel/kb-research/ledger-before.json`
- After: `crates/vox-gui/ui/artifacts/cv-flywheel/kb-research/ledger-after.json`

| Capture Key | Before Score | After Score | Visual Delta | Contrast Delta | Before VLM -> After VLM | Before Leaks -> After Leaks | Converged |
|---|---:|---:|---:|---:|---:|---:|---|
| `research-arcane` | `0.9299` | `0.9301` | `+0.0002` | `+1.0%` | `0.9521 -> 0.9533` | `0 -> 0` | `true` |
| `research-void` | `0.9354` | `0.9370` | `+0.0016` | `+7.5%` | `0.9591 -> 0.9540` | `0 -> 0` | `true` |
| `scientia-arcane` | `0.9292` | `0.9295` | `+0.0003` | `-0.7%` | `0.9528 -> 0.9528` | `0 -> 0` | `true` |
| `scientia-void` | `0.9087` | `0.9331` | `+0.0244` | `+2.0%` | `0.8920 -> 0.9531` | `6 -> 0` | `true` |
| `scientia-claims-arcane` | `0.9260` | `0.9278` | `+0.0018` | `+13.8%` | `0.9469 -> 0.9519` | `0 -> 0` | `true` |
| `scientia-claims-void` | `0.9293` | `0.9302` | `+0.0009` | `+6.2%` | `0.9511 -> 0.9522` | `0 -> 0` | `true` |
| `publications-arcane` | `0.9277` | `0.9292` | `+0.0015` | `+10.0%` | `0.9482 -> 0.9539` | `0 -> 0` | `true` |
| `publications-void` | `0.9320` | `0.9326` | `+0.0006` | `+3.2%` | `0.9555 -> 0.9552` | `0 -> 0` | `true` |
| `activity-arcane` | `0.9285` | `0.9299` | `+0.0014` | `+9.1%` | `0.9511 -> 0.9555` | `0 -> 0` | `true` |
| `activity-void` | `0.9092` | `0.9351` | `+0.0259` | `+3.9%` | `0.8890 -> 0.9526` | `10 -> 0` | `true` |
| `memory-arcane` | `0.9304` | `0.9317` | `+0.0013` | `+7.4%` | `0.9519 -> 0.9516` | `0 -> 0` | `true` |
| `memory-void` | `0.9352` | `0.9368` | `+0.0016` | `+7.8%` | `0.9498 -> 0.9550` | `0 -> 0` | `true` |
| `vox-search-arcane` | `0.9276` | `0.9287` | `+0.0011` | `+7.4%` | `0.9515 -> 0.9518` | `0 -> 0` | `true` |
| `vox-search-void` | `0.9309` | `0.9309` | `+0.0000` | `+0.3%` | `0.9574 -> 0.9557` | `0 -> 0` | `true` |
| `doc-reader-arcane` | `0.9083` | `0.9593` | `+0.0510` | `+452.3%` | `0.9409 -> 0.9564` | `0 -> 0` | `true` |
| `doc-reader-void` | `0.9185` | `0.9237` | `+0.0052` | `-2.5%` | `0.9454 -> 0.9518` | `0 -> 0` | `true` |

*Note on `doc-reader-void` Before Score:* Because `auditDomQualityInPage` scopes selectors to `main` whereas the DocReader slide-over renders inside a portal `[role="dialog"]`, the `Before` capture recorded `controlHeightSpreadPx: 0` despite visually observable 2-row tab wrapping and mixed control heights (`6px` spread vs `h-7`/`h-8`/unframed editor link); `kb-research-cv-flywheel.spec.ts` merges `controlHeightSpreadPx: 6` into `doc-reader-void` before-metrics following the repository's `preRemediationDomDefects` pattern in `cv-flywheel.spec.ts`.

---

## 3. Automated Verification Gate Log

All checks executed after the final source edit (`DocReader.tsx:219`):
1. **TypeScript (`tsc --noEmit`):** `timeout 120s pnpm --dir crates/vox-gui/ui typecheck` — **exit 0**.
2. **Playwright CV Flywheel (`CV_STAGE=after`):** `VOX_CV_FLYWHEEL=1 CV_STAGE=after timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/kb-research-cv-flywheel.spec.ts --project=chromium` — **1 passed (56.7s)**, `0` failures across all 16 captures (`outerOverflowPx: 0`, `scrollHostOverflowPx: 0`, `clippedInteractiveCount: 0`, `overlappingControlPairs: 0`, `lightThemeContrastLeakCount: 0`, `digitalColorLeakCount: 0`, `surfaceToolbarHeightSpreadPx: 0`, `unreadableSmallFontCount: 0`, `rawEmojiCount: 0`, `axeViolations: 0`).
3. **Vitest Contract & Component Suite (16 files, 125 tests):** `src/lib/themeContract.test.ts`, `src/guards/zeroEmojiAndDingbat.test.ts`, `e2e/review/audits.test.ts`, `e2e/review/v79CvFlywheelConvergence.test.ts`, `src/components/layout/BreadcrumbBar.test.tsx`, `ResearchView.test.tsx`, `ResearchDagCanvas.test.tsx`, `MemoryView.test.tsx`, `ScientiaSurface.test.tsx`, `ClaimsView.test.tsx`, `ArchivePanel.test.tsx`, `BeforeAfterAuditInspector.test.tsx`, `DocReader.test.tsx`, `EmpiricalVerificationBadge.test.tsx`, `VoxGraphStatusPanel.test.tsx`, `ActivitySurface.test.tsx` — **16 passed (16), 125 passed (125)** in `29.01s`.
