import {
  applySet,
  emptyLiveState,
  executionToComposerMode,
  snapshotCatalog,
  type DriveExecution,
  type DriveSet,
  type DriveState,
} from './axisDrive';
import type { PickerModel, ProviderStatus } from './modelPicker';

export interface DriveSetters {
  setChatModelOverride: (id: string | null) => void;
  setGroundingCheckEnabled?: (enabled: boolean) => void;
  setActiveSkill?: (id: string | null) => void;
  setSkillExclusions?: (ids: string[]) => void;
  setTier?: (tier: string) => void;
  setClutch?: (clutch: string) => void;
  setRisk?: (risk: string) => void;
  setMode?: (mode: string) => void;
  setDryRun?: (dryRun: boolean) => void;
  setExecution?: (execution: DriveExecution) => void;
  setContextFiles?: (files: string[]) => void;
}

export interface DriveSubmitPayload {
  description: string;
  execution_mode?: 'chat' | 'task' | 'plan';
  model_override?: string | null;
  tier?: string | null;
  clutch?: string | null;
  risk?: string | null;
  dry_run?: boolean | null;
  mode?: string | null;
  files?: string[];
  priority?: string | null;
  active_skill?: string | null;
}

export interface DriveRequest {
  id: string;
  verb: 'set' | 'send' | 'state' | 'show';
  body: unknown;
}

export interface DriveHttpLike {
  status: number;
  plane: 'live';
  error?: string;
  reason?: string | null;
  state: DriveState;
}

export interface HandleDriveRequestArgs {
  state: DriveState;
  models: PickerModel[];
  statuses: ProviderStatus[];
  setters?: DriveSetters;
  submit: (payload: DriveSubmitPayload) => Promise<unknown> | unknown;
  req: DriveRequest;
}

/** Drive-only: click path does not 409 an unselectable pin. */
export function assertPinSelectable(
  state: DriveState,
  models: PickerModel[],
  statuses: ProviderStatus[],
): DriveHttpLike | null {
  const id = state.knobs.model_override ?? state.pin;
  const policy = state.knobs.pin_policy ?? 'fail';
  if (!id || policy !== 'fail') return null;
  const catalog = snapshotCatalog(models, statuses);
  const row = catalog.find(r => r.id === id);
  if (row && row.selectable) return null;
  return {
    status: 409,
    plane: 'live',
    error: 'model_not_selectable',
    reason: row?.reason ?? 'not_listed',
    state: { ...state, catalog },
  };
}

export async function handleDriveRequest(args: HandleDriveRequestArgs): Promise<DriveHttpLike> {
  let state = args.state;
  const catalog = snapshotCatalog(args.models, args.statuses);
  if (args.req.verb === 'state') {
    return { status: 200, plane: 'live', state: { ...state, catalog } };
  }
  if (args.req.verb === 'show') {
    return { status: 200, plane: 'live', state: { ...state, catalog } };
  }
  if (args.req.verb === 'set') {
    const body = (args.req.body ?? {}) as DriveSet;
    try {
      state = applySet(state, body);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      return {
        status: 400,
        plane: 'live',
        error: message,
        state: { ...state, catalog },
      };
    }
    const blocked = assertPinSelectable(state, args.models, args.statuses);
    if (blocked) return blocked;
    applySetters(state, args.setters);
    return { status: 200, plane: 'live', state: { ...state, catalog: snapshotCatalog(args.models, args.statuses) } };
  }
  if (args.req.verb === 'send') {
    const text = String((args.req.body as { text?: unknown } | null)?.text ?? '').trim();
    if (!text) {
      return { status: 400, plane: 'live', error: 'empty_text', state: { ...state, catalog } };
    }
    const blocked = assertPinSelectable(state, args.models, args.statuses);
    if (blocked) return blocked;
    const payload: DriveSubmitPayload = {
      description: text,
      execution_mode: executionToComposerMode(state.knobs.execution),
      model_override: state.knobs.model_override ?? state.pin,
      tier: state.knobs.tier ?? null,
      clutch: state.knobs.clutch ?? null,
      risk: state.knobs.risk ?? null,
      dry_run: state.knobs.dry_run ?? null,
      mode: state.knobs.mode ?? null,
      files: state.knobs.context_files,
      priority: state.knobs.priority ?? null,
      active_skill: state.knobs.active_skill ?? null,
    };
    await args.submit(payload);
    return {
      status: 200,
      plane: 'live',
      state: {
        ...state,
        catalog,
        bubbles: [...state.bubbles, { role: 'user', content: text }],
      },
    };
  }
  return { status: 404, plane: 'live', error: 'not_found', state: { ...state, catalog } };
}

function applySetters(state: DriveState, setters?: DriveSetters): void {
  if (!setters) return;
  if (state.knobs.model_override !== undefined) {
    setters.setChatModelOverride(state.knobs.model_override);
  }
  if (state.knobs.grounding_check_enabled !== undefined) {
    setters.setGroundingCheckEnabled?.(state.knobs.grounding_check_enabled);
  }
  if (state.knobs.active_skill !== undefined) {
    setters.setActiveSkill?.(state.knobs.active_skill);
  }
  if (state.knobs.skill_exclusions.length > 0 || state.knobs.skill_exclusions) {
    setters.setSkillExclusions?.(state.knobs.skill_exclusions);
  }
  if (state.knobs.tier !== undefined) setters.setTier?.(state.knobs.tier);
  if (state.knobs.clutch !== undefined) setters.setClutch?.(state.knobs.clutch);
  if (state.knobs.risk !== undefined) setters.setRisk?.(state.knobs.risk);
  if (state.knobs.mode !== undefined) setters.setMode?.(state.knobs.mode);
  if (state.knobs.dry_run !== undefined) setters.setDryRun?.(state.knobs.dry_run);
  if (state.knobs.execution !== undefined) setters.setExecution?.(state.knobs.execution);
  if (state.knobs.context_files.length > 0) setters.setContextFiles?.(state.knobs.context_files);
}

let loquelaApi: DriveSetters | null = null;

export function registerLoquelaDriveApi(api: DriveSetters | null): void {
  loquelaApi = api;
}

export function loquelaDriveApi(): DriveSetters | null {
  return loquelaApi;
}

export { emptyLiveState };
