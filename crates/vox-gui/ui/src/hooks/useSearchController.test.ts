import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSearchController } from './useSearchController';
import { BackendStatusProvider } from './useBackendStatus';

vi.mock('../transport', () => ({
  voxTransport: {
    voxSearchQuery: vi.fn().mockResolvedValue({ hits: [{ title: 'hit-1' }] }),
  },
  axisBackendStatus: vi.fn(),
}));

import { axisBackendStatus, voxTransport } from '../transport';

const withBackend = ({ children }: { children: React.ReactNode }) =>
  React.createElement(BackendStatusProvider, null, children);

async function typeThreeKeys(setQuery: (q: string) => void) {
  for (const q of ['v', 'vo', 'vox']) {
    act(() => setQuery(q));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
  }
}

describe('useSearchController', () => {
  beforeEach(() => {
    vi.mocked(voxTransport.voxSearchQuery).mockClear();
    vi.mocked(axisBackendStatus).mockReset();
  });

  it('debounces search and returns hits', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSearchController({ debounceMs: 100 }));

    act(() => {
      result.current.setQuery('vox check');
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });

    expect(voxTransport.voxSearchQuery).toHaveBeenCalledWith('vox check', 30, expect.any(Array));
    expect(result.current.state.hits.length).toBe(1);
    vi.useRealTimers();
  });

  it('clears hits when query empty', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSearchController({ debounceMs: 50 }));

    act(() => {
      result.current.setQuery('   ');
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(50);
    });

    expect(result.current.state.hits).toEqual([]);
    expect(voxTransport.voxSearchQuery).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it('waits 350 ms after the last keystroke against a remote server', async () => {
    vi.useFakeTimers();
    vi.mocked(axisBackendStatus).mockResolvedValue({
      mode: 'remote',
      url: 'https://vox.lan:3921/',
      profile: null,
      surfaces: [],
    });
    const { result } = renderHook(() => useSearchController(), { wrapper: withBackend });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    await typeThreeKeys(result.current.setQuery);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(249);
    });
    expect(voxTransport.voxSearchQuery).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(voxTransport.voxSearchQuery).toHaveBeenCalledTimes(1);
    expect(voxTransport.voxSearchQuery).toHaveBeenCalledWith('vox', 30, expect.any(Array));
    vi.useRealTimers();
  });

  it('keeps the 200 ms default in local mode', async () => {
    vi.useFakeTimers();
    vi.mocked(axisBackendStatus).mockResolvedValue({ mode: 'local', url: null, profile: null, surfaces: [] });
    const { result } = renderHook(() => useSearchController(), { wrapper: withBackend });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    act(() => result.current.setQuery('vox'));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(199);
    });
    expect(voxTransport.voxSearchQuery).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(voxTransport.voxSearchQuery).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });
});
