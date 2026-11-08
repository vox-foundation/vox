---
phase: "345"
slug: "kiln-inspired-synthetic-topic-trees-vox-axis-mens-all-hubs-a"
status: approved
shadcn_initialized: false
preset: none
created: "2026-11-07"
---

# Phase 345 — UI Design Contract (Vox Axis MENS Crucible — Option A)

> Visual and interaction contract for Phase 345 (`MENS-STUDIO-01..04`): Option A — The MENS Ladder Matrix, AFK Series Crucible, Tactile Rotary Inspector, and Configurable Multi-Trace Hardware/Training Oscilloscope.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | Vox Axis Style-Dictionary Tokens (`tokens.generated.css` + `tokens.travertine.generated.css`) |
| Preset | Basalt (`[data-theme="basalt"]`) & Travertine (`[data-theme="travertine"]`) |
| Component library | Vox Axis UI + Radix/Shadcn Primitives (`Dialog`, `Tooltip`, `Segment`, `Button`, `Glass`) |
| Icon library | `lucide-react` + bespoke SVG precision glyphs (Roman Crucible Crest, Rotary Arcs, Gate Shields) |
| Font | `Cinzel` (`--font-display`), `EB Garamond Italic` (`--font-serif`), `Inter` (`--font-sans`), `JetBrains Mono` (`--font-mono`) |

---

## Component Inventory

Enumerated by `rg --files crates/vox-gui/ui/src/components/ui/` — 22 components — `vox-gui-ui@0.6.0` — `2026-11-08`.

| Component | Import path | Notes |
|-----------|-------------|-------|
| `Glass` | `../../ui/Glass` | Primary Basalt/Travertine surface container with `ring-inset` and Roman corner ticks (`vox-tick`) |
| `Dialog`, `DialogContent`, `DialogTitle`, `DialogDescription` | `../../ui/Dialog` | `@radix-ui/react-dialog` modal portals for `+ Define Spoke` & `❖ Kiln Topic Tree Studio` (eliminates inline layout shift) |
| `Tooltip` | `../../ui/Tooltip` | `@radix-ui/react-tooltip` portals for matrix cell SHA/CLI previews, rotary knob formulas, and route latency details |
| `Button` | `../../ui/Button` | `@radix-ui/react-slot` + `lucide-react` clustered action buttons |
| `Segment` | `../../ui/Segment` | Segmented control for Matrix Filter (`All / Stale / Gated`), Hardware Presets (`16G / 24G / 80G`), and Time Window (`60s / 5m / 15m / Full Run`) |
| `Pill` / `StatusPill` | `../../ui/Pill`, `../../ui/StatusPill` | Version provenance & gate badges |
| `MensServePanel` | `./MensServePanel` | Supervised `:8089` local inference control |

---

## Button Clustering & User Journey Convenience Architecture

1. **Cluster A — Hub & Ladder Command Header (Single Row):**
   - **Identity + Classical Motto:** Generated Roman Crucible Crest + `COMPUTE › MENS CRUCIBLE` (`font-display`) + *"Cursus Honorum · Deterministic Oracle"* (`font-serif italic`).
   - **Segmented Hub Trio Cluster:** `[◉ Hub I: Qwen3-0.6B | ◆ Hub II: Qwen3-8B | ⚡ Hub III: 4080S 16G]` grouped inside a unified pill frame.
   - **Primary Ladder Lifecycle Cluster:** `[⟳ Update All From Repo Master]` + `[☾ AFK Train All Missing in Series (4 Jobs)]` + `[⏸ Pause / ▶ Resume]` + `[Hide Sync ▾]`.
2. **Cluster B — Orchestrator Route & Clavis Contribution Ribbon (Single Row):**
   - **Left Cluster (Live Chat Route + Simulator):** Animated pulse dot + `ROUTE: lane:vox_codegen ⇢ Qwen3-8B ⇢ Spoke: vox_lang (18ms)` + `.vox | Rust | Review | Voice` quick-route buttons.
   - **Right Cluster (Hideable Clavis `HF_TOKEN` & Hub Sync):** `⚠ HF_TOKEN missing` + `[Settings → Keys & Secrets ↗]` + Repo target input + `[Auto-Sync: ON]` + `[⬆ Upload Train]`.
3. **Cluster C — Matrix Filter & Modal Authoring Cluster (Matrix Header):**
   - Filter pills (`All Rungs (6)` | `Action Needed (5)` | `Gated Only`) + Radix `<Dialog>` triggers for `[❖ Kiln Tree]` and `[+ Define Spoke]`.
4. **Cluster D — Inspector Hardware Presets & Single-Cell Quick-Actions (Right Pane):**
   - **Hardware Preset Cluster:** `[16G Local | 24G Pro | 80G H100]` one-click tuning for `LoRA Rank`, `Alpha`, `Seq Len`, and `VRAM Cap %`.
   - **Selected Cell Quick-Action Cluster:** `[⚡ Train Cell]`, `[🧪 Eval Gate]`, `[🚀 Hot-Swap :8089]`, and `[↺ Rollback SHA]` — all wired to real `execute_command` / `set_active_model` Tauri backend invocations.
5. **Cluster E — Full-Bleed Edge-to-Edge Oscilloscope & Queue Pipeline:**
   - **100% Width Edge-to-Edge SVG Chart:** Eliminates the left sidebar column gap; live metrics (`GPU 94%`, `VRAM 11.8/16.0 GB`, `CPU 42%`, `RAM 22.4/64 GB`, `Loss 0.142`) are embedded directly into the interactive channel toggle pills AND rendered as a translucent floating HUD inside the top-left of the full-width oscilloscope canvas.
   - **Multi-Resolution Verification:** Verified at `1280×800` (Compact Laptop), `1440×900` (Standard Desktop), and `1920×1080` (FHD Widescreen) with `outerOverflowPx === 0`, `scrollHostOverflowPx === 0`, and `axeViolations === 0`.

---

## Spacing Scale

Declared values (multiples of 4):

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4px | Badge & matrix cell micro-gaps |
| sm | 8px | Compact button & pill spacing |
| md | 16px | Panel inner padding & grid gaps |
| lg | 24px | Major workbench pane separation |
| xl | 32px | Section breaks |

Exceptions: `1px` crisp hairline borders (`border-border-subtle`) and `8px` Roman corner tick brackets (`vox-tick-tl`, `vox-tick-tr`, `vox-tick-bl`, `vox-tick-br`).

---

## Typography

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Body | 12px (`text-xs`) | 400 | 1.45 |
| Label / Mono | 11px–12px (`font-mono`) | 500 | 1.35 |
| Heading | 13px–14px (`font-display`) | 600 | 1.30 |
| Display | 16px (`font-display tracking-widest uppercase`) | 600 | 1.25 |

---

## Color

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | `#0a0c0e` (`--color-bg-base`) | Workbench background & oscilloscope void |
| Secondary (30%) | `#121619` (`--color-bg-surface`) | Matrix table, Rotary Inspector, Oscilloscope deck |
| Accent (10%) | `#d4af37` (`--color-accent-default` Brass) / `#79c8ba` (`--color-status-pass` Verdigris) | Active CTA triggers, gated check-shields, rotary arc gauges, GPU/VRAM traces |
| Destructive / Warn | `#d98b6a` (`--color-status-warn` Terracotta) / `#f0a59a` (`--color-status-fail`) | Stale SHA alerts, Turn-1 Oracle Failures, Abort Run |

Accent reserved for: Primary header CTAs (`Update All From Repo Master`, `AFK Train All Missing in Series`), active matrix selection ring, rotary arc progress strokes, and live telemetry traces.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Primary CTA 1 | `Update All From Repo Master` |
| Primary CTA 2 | `AFK Train All Missing in Series (4 Jobs)` |
| Empty state heading | `No run reports loaded for this directory` |
| Empty state body | `Select a Qwen3 ladder rung or click Load 5-Spoke Benchmark to inspect gate receipts and scorecards.` |
| Error state | `Telemetry or gate receipt load failed — verify run directory or run vox mens size-train.` |
| Destructive confirmation | `Abort AFK Series: Stop active training after the current checkpoint?` |

---

## UI Considerations

Applicable state considerations resolved: 6 covered, 1 backstop, 0 unresolved.

| Category | Element(s) | Status | Resolution / Reason |
|----------|------------|--------|---------------------|
| `overflow` | `qwen3-ladder-matrix`, `telemetry-oscilloscope` | ✅ covered | Responsive CSS grid + `overflow-x-auto` container with `outerOverflowPx === 0` and `scrollHostOverflowPx === 0` at `1440x900`, `1100x720`, and `900x600`. |
| `populated` | `qwen3-ladder-matrix` (6 rungs × 8 spokes) | ✅ covered | Displays all 6 pinned Qwen3 sizes (`0.6B @c1899de`..`32B @4f09d11`) and 8 Organic Spokes (`vox_lang`..`skill_creation`) with live version & gate status shields. |
| `interactive-knobs` | `rotary-arc-dials`, `curriculum-staircase` | ✅ covered | Clicking any rotary dial (`LoRA Rank r`, `Alpha α`, `Seq Len`, `VRAM Cap %`) or 3-Epoch Curriculum step dynamically updates the VRAM budget bar and active training command. |
| `time-series-monitor` | `mens-telemetry-oscilloscope` | ✅ covered | Multi-trace SVG/Canvas time-series graph over time for `GPU Compute %`, `VRAM GB`, `CPU %`, `RAM GB`, and `Training Loss` with interactive channel toggles and time-window pills (`60s`, `5m`, `15m`, `Full Run`). |
| `empty` | `mens-run-reports-panel` | ✅ covered | Clean fallback state before `Load 5-Spoke Benchmark` is clicked; preserves all existing `MensTrainingView.test.tsx` assertions. |
| `theme-parity` | `basalt` & `travertine` | ✅ covered | Zero light-theme pastel contrast leaks (`lightThemeContrastLeakCount === 0`) and zero unstyled native `<select>` elements (`unstyledNativeSelectCount === 0`). |
| `cv-flywheel` | `mens` surface before/after | 🧪 backstop | Verified by Playwright CV Flywheel (`computeRgbaFrameCvMetrics` + `evaluateGuiSurfaceRubric` + `runUniversalCvFlywheelPass` with `visualAfterScore >= 0.88` and `flywheelConverged === true`). |

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| Vox Axis First-Party UI | `Glass`, `MensServePanel`, `CommandCardsView` | not required |

---

## Checker Sign-Off

- [x] Dimension 1 Copywriting: PASS
- [x] Dimension 2 Visuals: PASS
- [x] Dimension 3 Color: PASS
- [x] Dimension 4 Typography: PASS
- [x] Dimension 5 Spacing: PASS
- [x] Dimension 6 Registry Safety: PASS
- [x] Dimension 7 Inventory Provenance: PASS

**Approval:** approved 2026-11-07
