// Mirror of contracts/gui/drive-console.v1.yaml (kept in sync by the BE parity gate).
import { modeLabel } from './turnEvents';
export type ClutchId = 'free' | 'efficiency' | 'balanced' | 'genius';
export type RiskId = 'high' | 'moderate' | 'low';

/** Canonical mode name for a clutch id — one source: `MODE_NAMES` in `lib/turnEvents.ts` (the turn trace uses it too). */
const modeName = (id: ClutchId): string => modeLabel(id);

export const CLUTCH_DETENTS: { id: ClutchId; label: string; hint: string }[] = [
  { id: 'free',       label: modeName('free'),       hint: 'Free models only' },
  { id: 'efficiency', label: modeName('efficiency'), hint: 'Most out of the tokens you spend; delegates to free agents on simple tasks' },
  { id: 'balanced',   label: modeName('balanced'),   hint: 'Balanced cost/quality' },
  { id: 'genius',     label: modeName('genius'),     hint: 'Most intelligent solutions; budget relaxed' },
];

export const RISK_POSTURES: { id: RiskId; label: string; tone: 'rose' | 'amber' | 'emerald' }[] = [
  { id: 'high',     label: 'High',     tone: 'rose' },
  { id: 'moderate', label: 'Moderate', tone: 'amber' },
  { id: 'low',      label: 'Low',      tone: 'emerald' },
];

export interface ControlState {
  clutch: ClutchId;
  risk: RiskId;
  safetyTokenBudget?: number; // optional override surfaced in the risk popover
}

export function defaultControl(): ControlState {
  return { clutch: 'efficiency', risk: 'moderate' };
}
