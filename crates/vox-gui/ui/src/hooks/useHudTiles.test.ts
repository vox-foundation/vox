import { describe, it, expect } from 'vitest';
import {
  HUD_TILE_KINDS,
  HUD_TILE_LABELS,
  defaultHudTiles,
  validateHudTilesConfig,
  filterKpisByTiles,
  resolveVisibleHudTiles,
  toggleHudTile,
  reorderHudTile,
} from './useHudTiles';

describe('pending_approvals HUD tile', () => {
  it('is part of the HUD tile SSOT with a label', () => {
    expect(HUD_TILE_KINDS).toContain('pending_approvals');
    expect(HUD_TILE_LABELS.pending_approvals).toBe('Needs you');
  });

  it('appears in the strip by default and DROPS when disabled', () => {
    const cfg = defaultHudTiles();
    expect(resolveVisibleHudTiles(cfg)).toContain('pending_approvals');
    const disabled = toggleHudTile(cfg, 'pending_approvals', false);
    expect(resolveVisibleHudTiles(disabled)).not.toContain('pending_approvals');
  });
});

describe('useHudTiles', () => {
  it('defaultHudTiles() returns the 5 card kinds in order', () => {
    const config = defaultHudTiles();
    expect(config.tiles.map((t) => t.kind)).toEqual([
      'active_agents',
      'budget_burn',
      'mesh_peers',
      'active_model',
      'pending_approvals',
    ]);
  });

  it('validateHudTilesConfig rejects unknown tile id', () => {
    expect(() =>
      validateHudTilesConfig({
        version: 1,
        tiles: [{ id: 'not-a-real-tile', kind: 'active_agents', enabled: true }],
      }),
    ).toThrow(/unknown tile id/i);
  });

  it('filterKpisByTiles only renders enabled tiles', () => {
    const config = {
      version: 1 as const,
      tiles: [
        { id: 'active_agents', kind: 'active_agents' as const, enabled: true },
        { id: 'queue_depth', kind: 'queue_depth' as const, enabled: false },
        { id: 'budget_burn', kind: 'budget_burn' as const, enabled: true },
        { id: 'mesh_peers', kind: 'mesh_peers' as const, enabled: false },
        { id: 'active_model', kind: 'active_model' as const, enabled: true },
        { id: 'openrouter_spend', kind: 'openrouter_spend' as const, enabled: false },
      ],
    };
    expect(filterKpisByTiles(config)).toEqual([
      'active_agents',
      'budget_burn',
      'active_model',
    ]);
    expect(resolveVisibleHudTiles(config)).toEqual(filterKpisByTiles(config));
  });

  it('toggleHudTile updates enabled flag for matching id', () => {
    const config = defaultHudTiles();
    const next = toggleHudTile(config, 'mesh_peers', false);
    expect(next.tiles.find((t) => t.id === 'mesh_peers')?.enabled).toBe(false);
    expect(next.tiles.find((t) => t.id === 'active_agents')?.enabled).toBe(true);
  });

  it('reorderHudTile moves tile from fromIndex to toIndex', () => {
    const config = defaultHudTiles();
    const next = reorderHudTile(config, 0, 2);
    expect(next.tiles.map((t) => t.kind)).toEqual([
      'budget_burn',
      'mesh_peers',
      'active_agents',
      'active_model',
      'pending_approvals',
    ]);
  });
});

describe('retired HUD tiles (merged into the Engine and Spend cards)', () => {
  it('drops queue_depth and openrouter_spend from a stored config and keeps the other choices', () => {
    const cfg = validateHudTilesConfig({
      version: 1,
      tiles: [
        { id: 'active_agents', kind: 'active_agents', enabled: false },
        { id: 'queue_depth', kind: 'queue_depth', enabled: true },
        { id: 'openrouter_spend', kind: 'openrouter_spend', enabled: false },
        { id: 'mesh_peers', kind: 'mesh_peers', enabled: true },
      ],
    });
    expect(cfg.tiles).toEqual([
      { id: 'active_agents', kind: 'active_agents', enabled: false },
      { id: 'mesh_peers', kind: 'mesh_peers', enabled: true },
    ]);
  });

  it('still rejects an id that was never a tile', () => {
    expect(() =>
      validateHudTilesConfig({ version: 1, tiles: [{ id: 'queue_depthx', kind: 'active_agents', enabled: true }] }),
    ).toThrow(/unknown tile id/i);
  });

  it('labels are the card names', () => {
    expect(HUD_TILE_LABELS).toEqual({
      active_agents: 'Engine',
      budget_burn: 'Spend',
      mesh_peers: 'Mesh',
      active_model: 'Routing',
      pending_approvals: 'Needs you',
    });
  });
});
