/** Shared chat / Loquela model-picker helpers.
 *
 * Catalog cards use `id` (not `model_id`). OpenRouter rows store the org
 * prefix in `provider` (`aion-labs`, `anthropic`) while
 * `inference_provider_status` names the backend (`OpenRouter`). Match
 * `provider_type` first; fall back to treating unmatched cloud prefixes as
 * OpenRouter when that key is present.
 */

export interface ProviderStatus {
  provider: string;
  key_present: boolean;
  is_local: boolean;
  local_reachable: boolean | null;
  local_models?: string[];
}

export interface ModelCardLike {
  id?: string;
  model_id?: string;
  display_name?: string;
  provider?: string;
  provider_type?: string;
}

export interface PickerModel {
  id: string;
  label: string;
  provider: string;
  providerType: string;
}

const LOCAL_ALIASES = new Set([
  'voxlocal',
  'populilocal',
  'populi',
  'mens',
  'ollama',
  'populimesh',
  'local',
]);

export function canonicalizeProvider(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]/g, '');
}

export function providerKey(name: string): string {
  const c = canonicalizeProvider(name);
  if (c === 'populilocal' || c === 'mens' || c === 'vox_local' || c === 'voxlocal') {
    return 'voxlocal';
  }
  return c;
}

export function normalizeModelCard(m: ModelCardLike): PickerModel | null {
  const id = (m.id ?? m.model_id ?? '').trim();
  if (!id) return null;
  const provider = (m.provider ?? '').trim();
  const providerType = (m.provider_type ?? '').trim();
  return {
    id,
    label: (m.display_name ?? id).trim() || id,
    provider,
    providerType,
  };
}

export function findProviderStatus(
  name: string | undefined,
  statuses: ProviderStatus[],
): ProviderStatus | undefined {
  if (!name) return undefined;
  const key = providerKey(name);
  return statuses.find(s => providerKey(s.provider) === key);
}

function statusForModel(model: PickerModel, statuses: ProviderStatus[]): ProviderStatus | undefined {
  return (
    findProviderStatus(model.providerType, statuses) ??
    findProviderStatus(model.provider, statuses)
  );
}

export function isLocalProviderName(name: string | undefined): boolean {
  if (!name) return false;
  return LOCAL_ALIASES.has(providerKey(name));
}

export function modelIdStem(id: string): string {
  const trimmed = id.trim().replace(/\/+$/, '');
  const withoutMens = trimmed.replace(/^mens\//, '');
  const slash = withoutMens.lastIndexOf('/');
  return slash >= 0 ? withoutMens.slice(slash + 1) : withoutMens;
}

function localModelListed(modelId: string, localModels: string[]): boolean {
  if (localModels.length === 0) return true;
  const stem = modelIdStem(modelId);
  return localModels.some(m => modelIdStem(m) === stem || m === modelId);
}

/** True when this card should appear in a "models we can actually call" list. */
export function isModelSelectable(model: PickerModel, statuses: ProviderStatus[]): boolean {
  if (statuses.length === 0) {
    return !isLocalProviderName(model.provider) && !isLocalProviderName(model.providerType);
  }
  const s = statusForModel(model, statuses);
  if (!s) {
    if (isLocalProviderName(model.provider) || isLocalProviderName(model.providerType)) {
      return false;
    }
    const openrouter = findProviderStatus('OpenRouter', statuses);
    if (openrouter) return openrouter.key_present;
    return true;
  }
  if (s.is_local) {
    if (s.local_reachable !== true) return false;
    if (!s.local_models || s.local_models.length === 0) return false;
    return localModelListed(model.id, s.local_models);
  }
  return s.key_present;
}

export function modelMatchesQuery(model: PickerModel, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    model.id.toLowerCase().includes(q) ||
    model.label.toLowerCase().includes(q) ||
    model.provider.toLowerCase().includes(q) ||
    model.providerType.toLowerCase().includes(q)
  );
}

export function filterPickerModels(
  models: PickerModel[],
  statuses: ProviderStatus[],
  query = '',
): PickerModel[] {
  return models.filter(m => isModelSelectable(m, statuses) && modelMatchesQuery(m, query));
}

export function shortModelLabel(id: string): string {
  const parts = id.split('/').filter(Boolean);
  if (parts.length >= 2) return parts.slice(-2).join('/');
  return id;
}

export const ROUTING_TIER_IDS = new Set(['auto', 'local', 'mesh', 'cloud']);

export function isRoutingTierId(id: string): boolean {
  return ROUTING_TIER_IDS.has(id);
}
