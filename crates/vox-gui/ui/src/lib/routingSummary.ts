import type { RoutingHealth, RoutingSummary } from '../types/tauri';
import type { TurnEventDto } from '../types/dashboard';
import { familyKey } from './modelFamily';
import { routingModelLabel } from './turnEvents';

/**
 * The global routing pick as a label, through the trace plan's `routingModelLabel` (one rule for the turn trace,
 * the status bar and the rail). Null without a decision. A daemon that predates `family` / `resolved_from` is
 * treated as bootstrap, so a version it sends is never shown.
 */
function summaryModelLabel(summary: RoutingSummary | null | undefined): string | null {
  const selected = summary?.decision_preview?.selected_model?.trim();
  if (!summary || !selected) return null;
  const event: TurnEventDto = {
    kind: 'routing_decision',
    family: summary.family?.trim() || familyKey(selected),
    resolved_id: selected,
    resolved_from: summary.resolved_from ?? 'bootstrap',
  };
  return routingModelLabel(event);
}

/** Status-bar Routing card value: `Auto → <label>`, or `Auto` when the engine reported no decision. */
export function routingCardValue(summary: RoutingSummary | null | undefined): string {
  const label = summaryModelLabel(summary);
  return label ? `Auto → ${label}` : 'Auto';
}

/** How many things the engine says are wrong with routing: each violation, and each scale that fell back. */
export function routingHealthProblems(health: RoutingHealth | null | undefined): number {
  if (!health) return 0;
  return (
    health.violations.length +
    (health.quality_scale === 'fallback' ? 1 : 0) +
    (health.price_bands === 'fallback' ? 1 : 0)
  );
}

/** The chat rail's Routing section (the engine's pick for the next turn). */
export interface RailRouting {
  /** `routingModelLabel` of the pick. */
  model: string;
  reason: string | null;
  /** `decision_preview.discovery_state` exactly as the server sent it. */
  state: string | null;
  /** At most 3: catalog ids when catalog-resolved, else family keys (deduped, the chosen family excluded). */
  alternatives: string[];
}

export function railRoutingFromSummary(summary: RoutingSummary | null | undefined): RailRouting | null {
  const model = summaryModelLabel(summary);
  const preview = summary?.decision_preview;
  if (!summary || !preview || !model) return null;
  const fromCatalog = summary.resolved_from === 'catalog';
  const chosen = fromCatalog ? preview.selected_model : summary.family?.trim() || familyKey(preview.selected_model);
  const alternatives: string[] = [];
  for (const alt of preview.alternatives ?? []) {
    const shown = fromCatalog ? alt : familyKey(alt);
    if (shown && shown !== chosen && !alternatives.includes(shown)) alternatives.push(shown);
    if (alternatives.length === 3) break;
  }
  return {
    model,
    reason: summary.reason?.trim() || null,
    state: preview.discovery_state?.trim() || null,
    alternatives,
  };
}

/** Tooltip text for `decision_preview.discovery_state` (the server sends `ModelConfidence` values). */
const MODEL_STATE_HINTS = new Map([
  ['confirmed', 'Confirmed: measured and eligible for routing.'],
  ['provisional', 'Provisional: newly discovered, not yet measured.'],
  ['shadowed', 'Shadowed: evaluated in the background, not routed yet.'],
  ['deprecated', 'Deprecated: being retired from routing.'],
]);

export function modelStateHint(state: string | null | undefined): string | null {
  return state ? MODEL_STATE_HINTS.get(state.toLowerCase()) ?? null : null;
}
