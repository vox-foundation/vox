import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { CAPABILITY_FIXTURE } from '../test-fixtures/capabilityIndexFixture';

const mod = { CAPABILITY_INDEX: CAPABILITY_FIXTURE, CAPABILITY_DOMAINS: [], CAPABILITY_COVERAGE_AS_OF: null };
const load = vi.fn();
vi.mock('../lib/capabilityIndexLoader', () => ({
  loadCapabilityIndex: () => load(),
  getLoadedCapabilityIndex: () => null,
}));

import { useCapabilityIndex } from './useCapabilityIndex';

describe('useCapabilityIndex retry (WR-05)', () => {
  it('a successful retry clears the earlier load error', async () => {
    load.mockRejectedValueOnce(new Error('chunk 404')).mockResolvedValueOnce(mod);
    const { result, rerender } = renderHook(({ on }) => useCapabilityIndex(on), { initialProps: { on: true } });
    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.rows).toHaveLength(0);

    act(() => rerender({ on: false })); // palette closed
    act(() => rerender({ on: true })); // palette reopened: retry
    await waitFor(() => expect(result.current.rows.length).toBe(CAPABILITY_FIXTURE.length));
    expect(result.current.error).toBeNull();
    expect(result.current.loading).toBe(false);
  });

  it('shows loading (not a stale error) while the retry is in flight', async () => {
    let resolve!: (m: typeof mod) => void;
    load.mockReset().mockRejectedValueOnce(new Error('x')).mockReturnValueOnce(new Promise((r) => { resolve = r; }));
    const { result, rerender } = renderHook(({ on }) => useCapabilityIndex(on), { initialProps: { on: true } });
    await waitFor(() => expect(result.current.error).not.toBeNull());
    act(() => rerender({ on: false }));
    act(() => rerender({ on: true }));
    await waitFor(() => expect(result.current.loading).toBe(true));
    expect(result.current.error).toBeNull();
    await act(async () => resolve(mod));
    await waitFor(() => expect(result.current.loading).toBe(false));
  });

  it('retry() re-attempts a failed load without closing the palette (WR-06 Retry)', async () => {
    load.mockReset().mockRejectedValueOnce(new Error('x')).mockResolvedValueOnce(mod);
    const { result } = renderHook(() => useCapabilityIndex(true));
    await waitFor(() => expect(result.current.error).not.toBeNull());
    act(() => result.current.retry());
    await waitFor(() => expect(result.current.rows.length).toBe(CAPABILITY_FIXTURE.length));
    expect(result.current.error).toBeNull();
    expect(load).toHaveBeenCalledTimes(2);
  });
});
