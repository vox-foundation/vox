import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { CAPABILITY_FIXTURE } from '../test-fixtures/capabilityIndexFixture';
import { rankCapabilities } from '../lib/capabilityRank';

vi.mock('../lib/capabilityIndexLoader', () => {
  const mod = { CAPABILITY_INDEX: CAPABILITY_FIXTURE, CAPABILITY_DOMAINS: [], CAPABILITY_COVERAGE_AS_OF: null };
  return { loadCapabilityIndex: () => Promise.resolve(mod), getLoadedCapabilityIndex: () => null };
});

import { useCapabilityIndex } from './useCapabilityIndex';

describe('useCapabilityIndex entries', () => {
  it('builds rank entries once per load, with LEXICON and nav labels as surface aliases', async () => {
    const { result, rerender } = renderHook(() => useCapabilityIndex(true));
    await waitFor(() => expect(result.current.entries.length).toBe(CAPABILITY_FIXTURE.length));
    const first = result.current.entries;
    rerender();
    expect(result.current.entries).toBe(first);
    const mesh = first.find((e) => e.row.id === 'surface.mesh')!;
    expect(mesh.aliases).toContain('rete'); // LEXICON.mesh.la
    expect(rankCapabilities(first, 'rete')[0]?.row.id).toBe('surface.mesh');
  });

  it('is empty until the palette is opened', () => {
    const { result } = renderHook(() => useCapabilityIndex(false));
    expect(result.current.entries).toEqual([]);
  });
});
