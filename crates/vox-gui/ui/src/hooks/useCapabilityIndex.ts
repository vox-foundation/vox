import { useEffect, useMemo, useState } from 'react';
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';
import { getLoadedCapabilityIndex, loadCapabilityIndex, type CapabilityIndexModule } from '../lib/capabilityIndexLoader';
import { toRankEntries, type RankEntry } from '../lib/capabilityRank';
import { LEXICON } from '../lib/lexicon';

export interface UseCapabilityIndexResult {
  rows: CapabilityIndexEntry[];
  /** Rank entries (aliases + tokens), computed once per load. */
  entries: RankEntry[];
  domains: { id: string; label: string }[];
  loading: boolean;
  error: string | null;
}

const EMPTY: CapabilityIndexEntry[] = [];
const NO_DOMAINS: { id: string; label: string }[] = [];

/** Loads the capability index lazily, only once `enabled` (palette open) is true. */
export function useCapabilityIndex(enabled: boolean): UseCapabilityIndexResult {
  const [mod, setMod] = useState<CapabilityIndexModule | null>(getLoadedCapabilityIndex);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled || mod) return;
    let cancelled = false;
    setError(null); // a retry starts clean: the earlier failure must not outlive a later success
    loadCapabilityIndex()
      .then((m) => {
        if (!cancelled) setMod(m);
      })
      .catch(() => {
        if (!cancelled) setError('capability index failed to load');
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, mod]);

  const rows = mod?.CAPABILITY_INDEX ?? EMPTY;
  // Surfaces stay findable by their English/Latin lexicon and nav labels (parity with the
  // federated surface source the index replaces).
  const entries = useMemo(
    () =>
      toRankEntries(rows, (r) =>
        r.kind === 'surface' && r.viewKey
          ? [LEXICON[r.viewKey]?.en, LEXICON[r.viewKey]?.la, r.navLabel].filter((s): s is string => !!s)
          : [],
      ),
    [rows],
  );

  return {
    rows,
    entries,
    domains: mod?.CAPABILITY_DOMAINS ?? NO_DOMAINS,
    loading: enabled && !mod && !error,
    error,
  };
}
