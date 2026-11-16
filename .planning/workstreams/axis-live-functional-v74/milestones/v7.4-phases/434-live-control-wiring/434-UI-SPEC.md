# Phase 434 UI Specification — Tactile Controls, Brass Gauges & Interactive Recharts

## Visual & Interaction Contract
1. **`<VoxSelect>` / Radix `<Select>`**:
   - Uses `data-slot="select-trigger"` and `data-slot="select-content"` (`z-(--z-popover)`), supporting `size="sm" | "xs"` for dense toolbars while maintaining WCAG 2.2 AA contrast across `arcane`, `glacier`, `slate`, and `high-contrast`.
   - Maps `""` option values through an internal sentinel (`"__vox_empty__"`) so empty-string filter options (`"All"`, `"None"`) work without Radix `Select.Item` errors.
2. **`<RomanRotaryKnob>` & `<Slider>`**:
   - Continuous bounded parameters (`FORVM` routing exploration weight, `Loquela` cadence/temperature, `PriorityChainEditor` weights) render with finite `aria-valuenow`, `aria-valuetext`, keyboard arrow stepping, and debounced `onValueCommit` IPC persistence.
3. **Interactive `<RomanBrassGauge>`**:
   - Zero `transition-all` classes (`transition-[border-color,transform]` on button, `transition-[stroke-dashoffset]` on SVG arc).
   - Supports `active?: boolean` (`aria-pressed={active}` + brass highlight ring) and `actionHint` tooltip guidance when `onClick` is wired to filter or focus surface content.
4. **Full-Bleed Interactive Recharts**:
   - `ChartWidgetFrame` uses `flex h-full min-h-[200px] flex-col` with `flex-1 min-h-[150px] min-w-0` body so `<ResponsiveContainer width="100%" height="100%">` fills >=85% of panel height (`chartPanelFillRatio >= 0.84`), with visible formatted axes, `<ReferenceLine>` budget/SLO marker when provided, `<Brush>` scrubber on multi-point timelines, and active datum badge on click.
