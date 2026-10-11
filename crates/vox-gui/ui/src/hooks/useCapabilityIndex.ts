import { useEffect, useState } from 'react';
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';
import { getLoadedCapabilityIndex, loadCapabilityIndex, type CapabilityIndexModule } from '../lib/capabilityIndexLoader';

export interface UseCapabilityIndexResult {
  rows: CapabilityIndexEntry[];
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

  return {
    rows: mod?.CAPABILITY_INDEX ?? EMPTY,
    domains: mod?.CAPABILITY_DOMAINS ?? NO_DOMAINS,
    loading: enabled && !mod && !error,
    error,
  };
}
