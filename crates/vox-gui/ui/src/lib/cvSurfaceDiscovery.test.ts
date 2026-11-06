import { describe, it, expect } from 'vitest';
import { SURFACE_REGISTRY } from '../generated/surfaceRegistry.generated';
import {
  CHAT_DOCK_PANEL_IDS,
  discoverAllGuiSurfacesAndPanels,
} from './cvSurfaceDiscovery';

describe('discoverAllGuiSurfacesAndPanels (CV-AUTO-DISCOVER-01)', () => {
  it('dynamically discovers 100% of non-null viewKey entries from SURFACE_REGISTRY without hardcoded lists', () => {
    const manifest = discoverAllGuiSurfacesAndPanels();
    const expectedFromRegistry = SURFACE_REGISTRY.filter((e) => e.viewKey != null)
      .map((e) => e.viewKey as string)
      .sort();

    expect(manifest.registeredViewKeys).toEqual(expectedFromRegistry);
    expect(manifest.registeredViewKeys.length).toBeGreaterThanOrEqual(26);
    expect(manifest.surfaces.map((s) => s.viewKey)).toEqual(expectedFromRegistry);
  });

  it('includes all Chat dock panel IDs and maps parent/child navigation domains', () => {
    const manifest = discoverAllGuiSurfacesAndPanels();
    expect(manifest.chatDockPanelIds).toEqual(CHAT_DOCK_PANEL_IDS);
    expect(manifest.chatDockPanelIds).toContain('transcript');
    expect(manifest.chatDockPanelIds).toContain('executionRail');
    expect(manifest.chatDockPanelIds).toContain('todos');
    expect(manifest.chatDockPanelIds).toContain('terminal');
    expect(manifest.chatDockPanelIds).toContain('urbs');

    for (const entry of manifest.surfaces) {
      expect(entry.parentDomain.length).toBeGreaterThan(0);
      expect(entry.childTab.length).toBeGreaterThan(0);
    }
  });
});
