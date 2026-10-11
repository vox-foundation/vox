/**
 * Explain card computed in TS from index rows (D-07). Mirrors
 * crates/vox-capability-registry/src/card.rs `render_card`; never shells out to `vox explain`.
 */
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';

// Must equal SURFACE_CRATES in crates/vox-capability-registry/src/index.rs (parity test reads it).
export const SURFACE_CRATES = ['vox-cli', 'vox-cli-ci', 'vox-orchestrator-mcp', 'vox-gui'] as const;

const DASH = '—';

export interface ExplainCard {
  title: string;
  kind: string;
  id: string;
  summary: string | null;
  equivalents: { cli: string[]; mcp: string[]; gui: string; catalog: string };
  location: { crate: string; source: string; docs: string };
  status: { coverage: string; tier: string; sideEffects: string; domain: string };
  related: { id: string; title: string }[];
  actions: {
    openForm: { path: string[]; unavailable?: string } | null;
    copyMcp: string | null;
    openDocs: string | null;
  };
}

export function buildExplainCard(
  row: CapabilityIndexEntry,
  _rows: readonly CapabilityIndexEntry[],
  _domains: readonly { id: string; label: string }[],
  _catalogPaths: ReadonlySet<string>,
): ExplainCard {
  return {
    title: row.title,
    kind: row.kind,
    id: row.id,
    summary: row.summary,
    equivalents: { cli: [DASH], mcp: [DASH], gui: DASH, catalog: DASH },
    location: { crate: DASH, source: DASH, docs: DASH },
    status: { coverage: DASH, tier: DASH, sideEffects: DASH, domain: DASH },
    related: [],
    actions: { openForm: null, copyMcp: null, openDocs: null },
  };
}
