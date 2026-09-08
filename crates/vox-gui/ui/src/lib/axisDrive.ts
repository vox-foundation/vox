import {
  isLocalProviderName,
  isModelSelectable,
  type PickerModel,
  type ProviderStatus,
} from './modelPicker';

export type DrivePlane = 'live' | 'headless';
export type DriveExecution = 'sync' | 'background' | 'plan';
export type DrivePinPolicy = 'fail' | 'coerce';

export const ALLOWED_SET_KEYS = [
  'model_override',
  'pin',
  'pin_policy',
  'execution',
  'tier',
  'clutch',
  'risk',
  'grounding_check_enabled',
  'active_skill',
  'skill_exclusions',
  'session_id',
  'chat_session_id',
  'priority',
  'dry_run',
  'allow_duplicate',
  'mode',
  'context_files',
  'refresh_catalog',
] as const;

export type DriveSetKey = (typeof ALLOWED_SET_KEYS)[number];

export interface DriveSet {
  model_override?: string;
  pin?: string;
  pin_policy?: DrivePinPolicy;
  execution?: DriveExecution;
  tier?: string;
  clutch?: string;
  risk?: string;
  grounding_check_enabled?: boolean;
  active_skill?: string | null;
  skill_exclusions?: string[];
  session_id?: string;
  chat_session_id?: string;
  priority?: string;
  dry_run?: boolean;
  allow_duplicate?: boolean;
  mode?: string;
  context_files?: string[];
  refresh_catalog?: boolean;
}

export interface DriveKnobs {
  model_override?: string;
  pin_policy?: DrivePinPolicy;
  execution?: DriveExecution;
  tier?: string;
  clutch?: string;
  risk?: string;
  grounding_check_enabled?: boolean;
  active_skill?: string | null;
  skill_exclusions: string[];
  session_id?: string;
  chat_session_id?: string;
  priority?: string;
  dry_run?: boolean;
  allow_duplicate?: boolean;
  mode?: string;
  context_files: string[];
}

export interface DriveCatalogRow {
  id: string;
  selectable: boolean;
  reason: string | null;
}

export interface DriveClaims {
  picker_ui: boolean;
  composer_knobs: boolean;
  bubbles: boolean;
}

export interface DriveState {
  plane: DrivePlane;
  knobs: DriveKnobs;
  pin: string | null;
  catalog: DriveCatalogRow[];
  probe: {
    reachable: boolean;
    base_url: string | null;
    service: string | null;
    models: string[];
  };
  bubbles: unknown[];
  last_error: string | null;
  orch_fresh: boolean;
  claims: DriveClaims;
}

export function emptyLiveState(): DriveState {
  return {
    plane: 'live',
    knobs: {
      skill_exclusions: [],
      context_files: [],
      pin_policy: 'fail',
    },
    pin: null,
    catalog: [],
    probe: { reachable: false, base_url: null, service: null, models: [] },
    bubbles: [],
    last_error: null,
    orch_fresh: false,
    claims: { picker_ui: true, composer_knobs: true, bubbles: true },
  };
}

export function applySet(state: DriveState, set: DriveSet): DriveState {
  for (const key of Object.keys(set)) {
    if (!ALLOWED_SET_KEYS.includes(key as DriveSetKey)) {
      throw new Error(`unknown_key:${key}`);
    }
  }
  const next: DriveState = {
    ...state,
    knobs: {
      ...state.knobs,
      skill_exclusions: [...state.knobs.skill_exclusions],
      context_files: [...state.knobs.context_files],
    },
  };
  const model = set.model_override ?? set.pin;
  if (model !== undefined) {
    next.knobs.model_override = model;
    next.pin = model;
  }
  if (set.pin_policy !== undefined) next.knobs.pin_policy = set.pin_policy;
  if (set.execution !== undefined) next.knobs.execution = set.execution;
  if (set.tier !== undefined) next.knobs.tier = set.tier;
  if (set.clutch !== undefined) next.knobs.clutch = set.clutch;
  if (set.risk !== undefined) next.knobs.risk = set.risk;
  if (set.grounding_check_enabled !== undefined) {
    next.knobs.grounding_check_enabled = set.grounding_check_enabled;
  }
  if (set.active_skill !== undefined) next.knobs.active_skill = set.active_skill;
  if (set.skill_exclusions !== undefined) next.knobs.skill_exclusions = set.skill_exclusions;
  if (set.session_id !== undefined) next.knobs.session_id = set.session_id;
  if (set.chat_session_id !== undefined) next.knobs.chat_session_id = set.chat_session_id;
  if (set.priority !== undefined) next.knobs.priority = set.priority;
  if (set.dry_run !== undefined) next.knobs.dry_run = set.dry_run;
  if (set.allow_duplicate !== undefined) next.knobs.allow_duplicate = set.allow_duplicate;
  if (set.mode !== undefined) next.knobs.mode = set.mode;
  if (set.context_files !== undefined) next.knobs.context_files = set.context_files;
  return next;
}

/** Drive fail-closed: an empty status list cannot mark anything selectable. */
export function snapshotCatalog(
  models: PickerModel[],
  statuses: ProviderStatus[],
): DriveCatalogRow[] {
  if (statuses.length === 0) {
    return models.map(m => ({
      id: m.id,
      selectable: false,
      reason: 'status_unavailable',
    }));
  }
  return models.map(m => {
    const selectable = isModelSelectable(m, statuses);
    return {
      id: m.id,
      selectable,
      reason: selectable ? null : catalogReason(m, statuses),
    };
  });
}

function catalogReason(model: PickerModel, statuses: ProviderStatus[]): string {
  const local = isLocalProviderName(model.provider) || isLocalProviderName(model.providerType);
  const status =
    statuses.find(s => s.provider.toLowerCase() === model.providerType.toLowerCase()) ??
    statuses.find(s => s.provider.toLowerCase() === model.provider.toLowerCase());
  if (status?.is_local && status.local_reachable !== true) return 'local_unreachable';
  if (local && !status) return 'local_unreachable';
  if (status && !status.is_local && !status.key_present) return 'key_missing';
  return 'not_listed';
}

export function parseKnobPairs(pairs: string[]): DriveSet {
  const set: DriveSet = {};
  for (const pair of pairs) {
    const eq = pair.indexOf('=');
    if (eq < 0) throw new Error(`unknown_key:${pair}`);
    const key = pair.slice(0, eq);
    const value = pair.slice(eq + 1);
    if (!ALLOWED_SET_KEYS.includes(key as DriveSetKey)) {
      throw new Error(`unknown_key:${key}`);
    }
    assignKnob(set, key as DriveSetKey, value);
  }
  return set;
}

function assignKnob(set: DriveSet, key: DriveSetKey, value: string): void {
  if (key === 'pin') {
    set.model_override = value;
    return;
  }
  if (key === 'grounding_check_enabled' || key === 'dry_run' || key === 'allow_duplicate' || key === 'refresh_catalog') {
    set[key] = value === 'true';
    return;
  }
  if (key === 'skill_exclusions' || key === 'context_files') {
    set[key] = value ? value.split(',') : [];
    return;
  }
  if (key === 'active_skill') {
    set.active_skill = value === 'null' ? null : value;
    return;
  }
  (set as Record<string, unknown>)[key] = value;
}

export function executionToComposerMode(execution?: DriveExecution): 'chat' | 'task' | 'plan' {
  if (execution === 'background') return 'task';
  if (execution === 'plan') return 'plan';
  return 'chat';
}
