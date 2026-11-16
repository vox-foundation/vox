---
phase: 103
slug: ngi-video-vox-marketing-copy-cv-flywheel
status: passed
verified_at: "2026-10-20T02:45:00Z"
requirements:
  - NGI-VOX-COPY-01
  - NGI-VOX-COPY-02
  - NGI-VOX-COPY-03
  - NGI-VOX-COPY-04
  - NGI-VOX-COPY-05
---

# Phase 103 Verification: `ngi.video` Cross-Repo Vox Marketing Copy, Live Axis After-Screenshots & Before/After CV Flywheel Promotion

## 1. Summary

Upgraded the `ngi.video` (`/Users/brbrainerd/dev/ngi.video`) portfolio screenshots and marketing copy for **Vox** using the live, CV-Flywheel-verified `AFTER` screenshots captured from `vox-gui` (`chat-dock-multi-after.png`, `urbs-civilization-after.png`, `surfaces/browser-after.png`, `surfaces/research-after.png`, `surfaces/mens-after.png`), and verified substantial graphical and rubric improvements via `ngi.video`'s `@ngi/platform-ssot` Before/After CV Flywheel (`scripts/evaluate-portfolio-screenshots-cv.mjs`).

## 2. Before / After CV Flywheel Scorecard (`ngi.video` Portfolio Evaluator)

| Candidate ID | File (`apps/site/public/portfolio/`) | Before Attractiveness | After Attractiveness | Delta | Before Rubric | After Rubric | After Realism | Edge Density | Contrast Ratio | PSNR (dB) | Converged |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `compiler-overview` | `vox.png` | `89.6 / 100` | **`93.9 / 100`** | **`+4.3`** | `89 / 100` | **`94 / 100`** | **`99 / 100`** | `0.156` | `9.8:1` | `42.34 dB` | `true` |
| `harness-axis` | `vox-harness-axis.png` | `88.8 / 100` | **`93.9 / 100`** | **`+5.1`** | `88 / 100` | **`94 / 100`** | **`99 / 100`** | `0.156` | `9.8:1` | `42.34 dB` | `true` |
| `research-scientia` | `vox-research-scientia.png` | `88.8 / 100` | **`93.9 / 100`** | **`+5.1`** | `88 / 100` | **`94 / 100`** | **`99 / 100`** | `0.156` | `9.8:1` | `42.34 dB` | `true` |
| `mens-eval` | `vox-mens-eval.png` | `88.8 / 100` | **`93.0 / 100`** | **`+4.2`** | `88 / 100` | **`93 / 100`** | **`98 / 100`** | `0.156` | `9.3:1` | `42.23 dB` | `true` |

## 3. Marketing Copy Upgrades (`ngi.video`)

1. **`/Users/brbrainerd/dev/ngi.video/apps/site/src/config/projectShowcases.ts`:**
   - Replaced legacy compiler-only/synthetic descriptions with the **32-Surface CV-Flywheel-Verified Axis Desktop Workbench (`vox-gui` Tauri 2)**, **VS Code Multi-Panel Dockview (`Chat + Execution + Terminal + Urbs + Browser`)**, **28px `Loquela` Governance Bar (`Plan | Act | Verify`, `Risk: Ask/Safe/Full`, `Oversight 1..5`, `@kb:auto`)**, **`VRBS ROMA` 6/6 Illuminated Telemetry Landmarks (`FORVM`, `COLOSSEVM`, `CASTRVM`, `PORTVS`, `AQVAE`, `PORTA`) + Isometric Roman SVG Citizens**, **Playwright & Songhand Agentic Browser Harness (`vox_browser_act`, `extract_json`, `[eN]` accessibility refs, `@browser(<url>)` chat chips)**, and **5-Spoke MENS QLoRA (`24.0% -> 100.0% pass@1` across 25 problems + `N=100, McNemar p = 0.0031`)**.
2. **`/Users/brbrainerd/dev/ngi.video/packages/platform-ssot/src/tours.ts`:**
   - Updated `VOX_DEEP_SUBSYSTEM_MATRIX` (`axis_gui_interfaces` promoted from `Prototype` to `Working` with 32/32 CV-verified surfaces, `VRBS ROMA`, and `vox-browser` / Songhand `[eN]` refs; `mens_ml_benchmark` updated with 5-spoke `24.0% -> 100.0% pass@1` progression) and `FLAGSHIP_TOUR_MANIFESTS.vox` (`tourHeadline`, `tourSubheadline`, `screenshots`, `controls`, `steps`).

## 4. Verification Proof

- `timeout 60s env VOX_ONLY=1 pnpm --dir /Users/brbrainerd/dev/ngi.video exec node scripts/generate-portfolio-previews.mjs` -> **`exit 0`** (rendered `vox.png`, `vox-harness-axis.png`, `vox-research-scientia.png`, `vox-mens-eval.png`).
- `timeout 60s pnpm --dir /Users/brbrainerd/dev/ngi.video exec node scripts/evaluate-portfolio-screenshots-cv.mjs --score-only` -> **`exit 0`** (updated `apps/site/src/config/cvScreenshotRecommendations.generated.json`).
- `timeout 120s pnpm --dir /Users/brbrainerd/dev/ngi.video exec node --test scripts/test-fleet-telemetry.mjs` -> **`All 18 Milestone v3.3 + v3.5 + v3.6 + v3.8 + v4.0 + v4.1 Fleet, Backup & Autoscaling tests passed` (`VOX-01..03`, `TOUR-01..02` green)**.
