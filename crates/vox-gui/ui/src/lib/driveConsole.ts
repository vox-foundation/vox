// Mirror of contracts/gui/drive-console.v1.yaml (kept in sync by the BE parity gate).
import { modeLabel } from './turnEvents';
import { SHELL_PREFERENCE_KEYS } from './shellPersistence';
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

let latestControlState: ControlState = defaultControl();
const controlListeners = new Set<(state: ControlState) => void>();

export function getLatestControlState(): ControlState {
  return latestControlState;
}

export function publishControlState(next: ControlState): void {
  latestControlState = next;
  for (const listener of controlListeners) {
    listener(latestControlState);
  }
}

export function subscribeControlState(listener: (state: ControlState) => void): () => void {
  controlListeners.add(listener);
  return () => {
    controlListeners.delete(listener);
  };
}

function readInitialModelOverride(): string | null {
  try {
    if (typeof window === 'undefined' || !window.localStorage) return null;
    const raw = window.localStorage.getItem(SHELL_PREFERENCE_KEYS.chatModelOverride);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return typeof parsed === 'string' && parsed.trim() ? parsed.trim() : null;
  } catch {
    return null;
  }
}

let latestModelOverride: string | null = readInitialModelOverride();
const modelOverrideListeners = new Set<(modelId: string | null) => void>();

export function getLatestModelOverride(): string | null {
  return latestModelOverride;
}

export function publishModelOverride(next: string | null): void {
  latestModelOverride = next;
  for (const listener of modelOverrideListeners) {
    listener(latestModelOverride);
  }
}

export function subscribeModelOverride(listener: (modelId: string | null) => void): () => void {
  modelOverrideListeners.add(listener);
  return () => {
    modelOverrideListeners.delete(listener);
  };
}

