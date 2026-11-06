import { SURFACE_REGISTRY, type RepresentationTier } from '../generated/surfaceRegistry.generated';
import {
  LEGACY_VIEW_ALIASES,
  PARENT_CHILD_MAP,
  TOP_LEVEL_VIEWS,
  resolveNavigation,
} from './navigation';

export const ALWAYS_CORE_DOCK_PANEL_IDS = ['transcript', 'executionRail'] as const;
export const CORE_DOCK_PANEL_IDS = [...ALWAYS_CORE_DOCK_PANEL_IDS, 'todos'] as const;
export type CoreDockPanelId = (typeof CORE_DOCK_PANEL_IDS)[number];

export const OPT_IN_DOCK_PANEL_IDS = [
  'needs-you',
  'voxgraph',
  'activity',
  'repository',
  'mercatus',
  'harness',
  'approvals',
  'research',
  'terminal',
  'urbs',
  'browser',
] as const;
export type OptInDockPanelId = (typeof OPT_IN_DOCK_PANEL_IDS)[number];

export const CHAT_DOCK_PANEL_IDS = [
  ...CORE_DOCK_PANEL_IDS,
  ...OPT_IN_DOCK_PANEL_IDS,
] as const;
export type ChatDockPanelId = (typeof CHAT_DOCK_PANEL_IDS)[number];

export interface DiscoveredSurfaceEntry {
  viewKey: string;
  navLabel: string;
  tier: RepresentationTier;
  parentDomain: string;
  childTab: string;
  isDockableInChat: boolean;
}

export interface GuiSurfaceDiscoveryManifest {
  registeredViewKeys: string[];
  routedViewKeys: string[];
  topLevelDomains: readonly string[];
  chatDockPanelIds: readonly ChatDockPanelId[];
  legacyAliases: Record<string, string>;
  surfaces: DiscoveredSurfaceEntry[];
}

/**
 * Dynamically introspects `SURFACE_REGISTRY`, `navigation.ts` (`PARENT_CHILD_MAP`,
 * `TOP_LEVEL_VIEWS`, `LEGACY_VIEW_ALIASES`), and `CHAT_DOCK_PANEL_IDS` to produce
 * a complete, zero-hardcoded manifest of all GUI surfaces and dock panels.
 * Any new surface registered in `surface-registry.v1.yaml` or `navigation.ts`
 * is automatically discovered by the CV Flywheel scanner on the next run.
 */
export function discoverAllGuiSurfacesAndPanels(): GuiSurfaceDiscoveryManifest {
  const dockSet = new Set<string>(CHAT_DOCK_PANEL_IDS);
  const registryByKey = new Map(
    SURFACE_REGISTRY.filter((e) => e.viewKey != null).map((e) => [e.viewKey as string, e]),
  );

  const registeredViewKeys = Array.from(registryByKey.keys()).sort();
  const routedSet = new Set<string>([
    ...registeredViewKeys,
    ...Object.keys(PARENT_CHILD_MAP),
    ...TOP_LEVEL_VIEWS,
  ]);
  const routedViewKeys = Array.from(routedSet)
    .filter((k) => !(k in LEGACY_VIEW_ALIASES))
    .sort();

  const surfaces: DiscoveredSurfaceEntry[] = registeredViewKeys.map((viewKey) => {
    const reg = registryByKey.get(viewKey);
    const resolved = resolveNavigation(viewKey);
    return {
      viewKey,
      navLabel: reg?.navLabel ?? viewKey,
      tier: reg?.tier ?? 'live_backend',
      parentDomain: resolved.parent,
      childTab: resolved.child,
      isDockableInChat:
        dockSet.has(viewKey) ||
        (viewKey === 'chat' && dockSet.has('transcript')) ||
        (viewKey === 'vox-search' && dockSet.has('voxgraph')) ||
        (viewKey === 'gamify' && dockSet.has('urbs')),
    };
  });

  return {
    registeredViewKeys,
    routedViewKeys,
    topLevelDomains: TOP_LEVEL_VIEWS,
    chatDockPanelIds: CHAT_DOCK_PANEL_IDS,
    legacyAliases: { ...LEGACY_VIEW_ALIASES },
    surfaces,
  };
}
