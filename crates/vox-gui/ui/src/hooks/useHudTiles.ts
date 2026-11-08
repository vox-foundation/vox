/**
 * HUD tile config — SSOT kinds from contracts/gui/hud-tiles.v1.yaml
 * Persisted under gui.hud.tiles.v1 (shell-persistence.v1.yaml).
 */

export const HUD_TILE_KINDS = [
  'repo_version',
  'active_agents',
  'budget_burn',
  'mesh_peers',
  'routing_decision',
  'attention_debt',
] as const;

export type HudTileKind =
  | (typeof HUD_TILE_KINDS)[number]
  | 'active_model'
  | 'pending_approvals';

/** Card names — the Configure menu shows exactly these, so menu and cards cannot drift. */
export const HUD_TILE_LABELS: Record<HudTileKind, string> = {
  repo_version: 'Repo',
  active_agents: 'Engine',
  budget_burn: 'Spend',
  mesh_peers: 'Mesh',
  routing_decision: 'Routing',
  attention_debt: 'Needs you',
  active_model: 'Routing',
  pending_approvals: 'Needs you',
};

/**
 * Tile ids retired 2026-09-28: queue depth merged into the Engine card, OpenRouter spend into the Spend
 * card's popover. A stored config that still lists them loads with those entries dropped (other choices kept).
 */
export const RETIRED_HUD_TILE_IDS: ReadonlySet<string> = new Set(['queue_depth', 'openrouter_spend']);

export interface HudTileEntry {
  id: string;
  kind: HudTileKind;
  enabled: boolean;
}

export interface HudTilesConfig {
  version: 1;
  tiles: HudTileEntry[];
}

const KIND_SET = new Set<string>([...HUD_TILE_KINDS, 'active_model', 'pending_approvals']);

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

export function defaultHudTiles(): HudTilesConfig {
  return {
    version: 1,
    tiles: HUD_TILE_KINDS.map((kind) => ({
      id: kind,
      kind,
      enabled: true,
    })),
  };
}

/**
 * Normalizes a HUD tiles config (e.g. from localStorage), ensuring all default HUD_TILE_KINDS
 * (such as repo_version) are present while preserving existing ordering and explicit enabled/visible toggles.
 */
export function normalizeHudTiles(raw: unknown): HudTilesConfig {
  if (!isRecord(raw) || !Array.isArray(raw.tiles)) {
    return defaultHudTiles();
  }

  const existingEntries = raw.tiles.filter((entry): entry is Record<string, unknown> => {
    return isRecord(entry) && typeof entry.id === 'string' && !RETIRED_HUD_TILE_IDS.has(entry.id);
  });

  const tileMap = new Map<string, HudTileEntry>();
  const order: string[] = [];

  for (const entry of existingEntries) {
    let id = entry.id as string;
    let kind = entry.kind as string;

    // Normalize legacy aliases
    if (id === 'active_model') id = 'routing_decision';
    if (id === 'pending_approvals') id = 'attention_debt';
    if (kind === 'active_model') kind = 'routing_decision';
    if (kind === 'pending_approvals') kind = 'attention_debt';

    const enabled =
      typeof entry.enabled === 'boolean'
        ? entry.enabled
        : typeof (entry as Record<string, unknown>).visible === 'boolean'
        ? (entry as Record<string, unknown>).visible as boolean
        : true;

    if (KIND_SET.has(id) && KIND_SET.has(kind)) {
      if (!tileMap.has(id)) {
        order.push(id);
      }
      tileMap.set(id, { id, kind: kind as HudTileKind, enabled });
    }
  }

  // Ensure every HUD_TILE_KINDS entry is included
  for (const defaultKind of HUD_TILE_KINDS) {
    if (!tileMap.has(defaultKind)) {
      tileMap.set(defaultKind, { id: defaultKind, kind: defaultKind, enabled: true });
      if (defaultKind === 'repo_version') {
        order.unshift(defaultKind);
      } else {
        order.push(defaultKind);
      }
    }
  }

  const tiles = order.map((id) => tileMap.get(id)!).filter(Boolean);

  return {
    version: 1,
    tiles,
  };
}

export function validateHudTilesConfig(raw: unknown): HudTilesConfig {
  if (!isRecord(raw)) {
    throw new Error('hud tiles config must be an object');
  }
  if (raw.version !== 1) {
    throw new Error('hud tiles config version must be 1');
  }
  if (!Array.isArray(raw.tiles)) {
    throw new Error('hud tiles config tiles must be an array');
  }

  const tiles: HudTileEntry[] = raw.tiles
    .filter((entry) => !(isRecord(entry) && typeof entry.id === 'string' && RETIRED_HUD_TILE_IDS.has(entry.id)))
    .map((entry, index) => {
    const path = `tiles[${index}]`;
    if (!isRecord(entry)) {
      throw new Error(`${path}: tile must be an object`);
    }
    const id = entry.id;
    const kind = entry.kind;
    const enabled = entry.enabled;
    if (typeof id !== 'string' || id.length === 0) {
      throw new Error(`${path}: id must be a non-empty string`);
    }
    if (!KIND_SET.has(id)) {
      throw new Error(`unknown tile id: ${id}`);
    }
    if (typeof kind !== 'string' || !KIND_SET.has(kind)) {
      throw new Error(`unknown tile kind: ${String(kind)}`);
    }
    if (typeof enabled !== 'boolean') {
      throw new Error(`${path}: enabled must be a boolean`);
    }
    return { id, kind: kind as HudTileKind, enabled };
  });

  return { version: 1, tiles };
}

export function resolveVisibleHudTiles(config: HudTilesConfig): HudTileKind[] {
  return config.tiles.filter((t) => t.enabled).map((t) => t.kind);
}

/** Returns enabled tile kinds in config order (alias for HUD KPI filtering). */
export function filterKpisByTiles(config: HudTilesConfig): HudTileKind[] {
  return resolveVisibleHudTiles(config);
}

export function toggleHudTile(
  config: HudTilesConfig,
  id: string,
  enabled: boolean,
): HudTilesConfig {
  return {
    ...config,
    tiles: config.tiles.map((t) => (t.id === id ? { ...t, enabled } : t)),
  };
}

export function reorderHudTile(
  config: HudTilesConfig,
  fromIndex: number,
  toIndex: number,
): HudTilesConfig {
  if (fromIndex === toIndex) return config;
  if (fromIndex < 0 || fromIndex >= config.tiles.length) return config;
  if (toIndex < 0 || toIndex >= config.tiles.length) return config;
  const tiles = config.tiles.slice();
  const [moved] = tiles.splice(fromIndex, 1);
  tiles.splice(toIndex, 0, moved);
  return { ...config, tiles };
}
