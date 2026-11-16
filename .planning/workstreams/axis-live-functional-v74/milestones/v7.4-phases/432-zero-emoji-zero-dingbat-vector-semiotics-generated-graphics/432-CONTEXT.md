# Phase 432: Zero-Emoji & Zero-Dingbat Vector Semiotics & Generated Graphics Sweep - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Purge every raw pictographic emoji and decorative BMP Unicode dingbat/arrow/shape across all 50+ JSX/TSX files, string formatters, and both Canvas 2D renderers (`worldRenderer.ts`, `hydraulicNetwork.ts`), replacing them with stroke-normalized `<VoxIcon strokeWidth={1.5} />` (`lucide-react@^1.72.0`), bespoke Romanesque SVGs in `crates/vox-gui/ui/src/components/ui/Icons.tsx`, Canvas 2D `Path2D` vector badges, and generated Romanesque/Slate graphical crests, medallions, and empty-state plates (`crates/vox-gui/ui/public/assets/romanesque/v74-*.jpg`), while resolving touched-surface `themeContract.test.ts` findings and locking enforcement via `zeroEmojiAndDingbat.test.ts` and an un-exempted full-DOM + portal glyph scanner in `crates/vox-gui/ui/e2e/review/audits.ts`.

</domain>

<decisions>
## Implementation Decisions

### Two-Tier Vector Icon Architecture (`lucide-react` + Bespoke `Icons.tsx`)
- **D-432-01 (`<VoxIcon>` Stroke & Box Normalization):** Create `crates/vox-gui/ui/src/components/ui/VoxIcon.tsx` wrapping `lucide-react@^1.72.0` icons with `strokeWidth={1.5}` and `className="shrink-0"` (`14px`–`16px` standard chrome, `10px`–`12px` dense badges/chevrons) so Lucide strokes match `Icons.tsx`'s `24x24` `1.5px` stroke weight.
- **D-432-02 (Domain vs Utility Split):** Use `lucide-react` (`LockKeyhole`, `Scissors`, `GitBranch`, `GripVertical`, `ExternalLink`, `RotateCcw`, `RefreshCw`, `Check`, `X`, `ChevronRight`, `ChevronDown`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowLeftRight`, `CornerDownLeft`, `Power`, `Ban`, `Pause`, `Star`, `Square`, `Target`) for technical IDE/UI controls, and expand `Icons.tsx` with bespoke Romanesque domain SVGs (`amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`, `homeVilla`, `penQuill`, `<PlanStepGlyph />`, `<RoleRankEmblem />`, `<KeycapIcon />`).
- **D-432-03 (Accessible Naming & Bounding-Box Contract):** Set `aria-hidden="true"` on decorative icons inside labeled buttons; require explicit `aria-label` + `shrink-0` non-zero bounding boxes on all icon-only triggers (`iconIssueCount === 0`, zero Axe `button-name` violations).
- **D-432-04 (SVG Keycap Legends):** Replace raw Unicode arrow/return/chevron characters (`↵`, `⏎`, `⇥`, `«`, `»`) in `Omnibar.tsx`, `Loquela.tsx`, `Sidebar.tsx`, `ChatSurface.tsx`, `CommandCardsView.tsx`, and `BottomStatusBar.tsx` with `<KeycapIcon />` / `CornerDownLeft` / `Command` / `ChevronsLeft` SVGs while preserving `lib/keybinds.ts` `formatChord` unit-tested string output.

### Canvas `Path2D` Badges & Generated Romanesque/Slate Graphics
- **D-432-05 (Canvas 2D `Path2D` Vector Badges):** Replace every `ctx.fillText` glyph hack (`◆`, `▲`, `⬆`, `✓`, `✕`, `❯`, `◎`, `◉`, `→`) in `crates/vox-gui/ui/src/components/gamify/urbs/worldRenderer.ts` and `hydraulicNetwork.ts` with cached `Path2D` vector helpers (`crates/vox-gui/ui/src/components/gamify/urbs/canvasVectorBadges.ts`) scaled cleanly by `devicePixelRatio`.
- **D-432-06 (Dual-Theme Generated Crests & Empty-State Plates):** Generate 4 sovereign local `.jpg` plates under `crates/vox-gui/ui/public/assets/romanesque/` (`v74-crest-empty-carrara.jpg`, `v74-crest-empty-slate.jpg`, `v74-medallion-imperium-carrara.jpg`, `v74-medallion-imperium-slate.jpg`) and wire theme-reactive `<RomanCrestPlate />` into `EmptyState.tsx`, `ImperiumInspector.tsx`, `LudusSandbox.tsx`, and `DiscoverySurface.tsx`.
- **D-432-07 (Touched-Surface `themeContract` Cleanup):** Replace hardcoded `#059669` and `#1c140c` tokens in `LudusSandbox.tsx` and `GamifyView.tsx` with semantic theme variables (`var(--color-success, ...)`, `bg-bg-elevated`, `text-text-primary`) so `src/lib/themeContract.test.ts` passes `7/7` (`0` errors).
- **D-432-08 (Offline Sovereignty & `<= 160 KB` Plate Budget):** Keep every generated plate `<= 160 KB` on disk with explicit `width`/`height` attributes, `decoding="async"`, `loading="lazy"`, and zero external runtime URLs.

### Full-DOM + Portal Glyph Gate & Vitest Suite Migration
- **D-432-09 (Un-Exempted Full-DOM + Portal Audit in `audits.ts`):** Upgrade `crates/vox-gui/ui/e2e/review/audits.ts` to scan visible text across `document.body` and open Radix portals (`[data-radix-popper-content-wrapper]`, `[role="dialog"]`, `[role="listbox"]`, `[role="menu"]`) for `\p{Extended_Pictographic}` AND decorative BMP ranges (`\u2600-\u27BF`, `\u23F0-\u23FE`, `\u25A0-\u25FF`, `\u2197`, `\u21BA-\u21C4`, `\u21E2`, `\u2B06`, `\u2302`, `\u2442`), and delete the `[data-testid="urbs-imperium-inspector"]` exemption (`rawEmojiCount === 0`).
- **D-432-10 (`zeroEmojiAndDingbat.test.ts` Static Guard):** Add `crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts` scanning non-comment source across `crates/vox-gui/ui/src/**/*.{ts,tsx}` (excluding `.test.*` and `generated/*`) for banned pictographic emojis and decorative BMP dingbats/shapes.
- **D-432-11 (5 Legacy Vitest Glyph Migrations):** Update `Kpi.test.tsx`, `CapabilityBadge.test.tsx`, `ExplorerView.test.tsx`, `SecretaryToast.test.tsx`, `TurnTrace.test.tsx` (plus `turnTrace.test.ts` if affected by `'✓'` -> `'verified'`) to assert semantic `data-icon`, `aria-label`, and accessible roles instead of literal glyph strings.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-gui/ui/src/components/ui/Icons.tsx` — 40+ 24x24 `strokeWidth="1.5"` icons including `romanColumn`, `laurelWreath`, `gladiusShield`, `denariusCoin`, `hourglassVia`, `brazierTorch`, `astrolabeRadar`, `aqueductArch`, `waxSeal`, `scrollTabula`, `romanGalley`, `anvilForge`.
- `lucide-react@^1.72.0` — installed in Phase 431 (`package.json`), tree-shakeable ESM icon library.
- `crates/vox-gui/ui/src/components/ui/EmptyState.tsx` — shared empty-state component across 15+ surfaces.

### Established Patterns
- `useTheme` / `document.documentElement.dataset.theme` (`arcane`, `glacier`, `slate`, `high-contrast`) selects Carrara vs Slate visual plates cleanly without runtime layout shift.
- `knip.jsonc` (`files: "error"`) requires every new helper/component (`VoxIcon.tsx`, `RomanCrestPlate.tsx`, `canvasVectorBadges.ts`) to be imported by active runtime views and tests.

### Integration Points
- Shared high-fanout UI primitives (`Kpi.tsx`, `StatusPill.tsx`, `CapabilityBadge.tsx`, `DataTable.tsx`, `Toasts.tsx`, `EmptyState.tsx`, `IntelliSensePopover.tsx`).
- Gamify & Canvas renderers (`ImperiumInspector.tsx`, `SpeculumLoupePopover.tsx`, `LudusSandbox.tsx`, `CitizenSprite.tsx`, `HudPanels.tsx`, `GamifyView.tsx`, `LudusHud.tsx`, `ViaTemporisScrubber.tsx`, `worldRenderer.ts`, `hydraulicNetwork.ts`).
- Chat, Scientia, Research, Policies, Models, Harness, Settings, Tasks, Mercatus, and Layout shell files.

</code_context>

<specifics>
## Specific Ideas

- Give `<PlanStepGlyph state={...} />` a genuine miniature chronometer arc fill (empty ring -> quarter -> half -> full check -> paused double-bar -> error cross) so plan steps in `PlanPanel.tsx` and `ChatExecutionRail.tsx` look engineered rather than font-hacked `○◔◑●`.
- Use subtle circular brass/slate framing around `<RomanCrestPlate />` in `EmptyState.tsx` and `ImperiumInspector.tsx` so generated relief medallions sit naturally inside both Carrara parchment and dark Slate surfaces.

</specifics>

<deferred>
## Deferred Ideas

- Full-viewport Bento concept-art plates (`v74-concept-*.jpg`) for redesigned master-detail surfaces (`oratio`, `populi`, `catalog`, `mercatus`, etc.) belong in **Phase 435–436** once Bento layouts land.

</deferred>
