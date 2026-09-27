// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useLocalStorage } from './useLocalStorage';

// Replace `window.localStorage` wholesale: depending on the Node version it is
// jsdom's Storage or Node's own global, and neither an instance spy nor a
// `Storage.prototype` spy intercepts both.
const real = Object.getOwnPropertyDescriptor(window, 'localStorage');
function stubStorage(overrides: Partial<Storage>) {
  const store: Partial<Storage> = { getItem: () => null, setItem: () => {}, removeItem: () => {}, ...overrides };
  Object.defineProperty(window, 'localStorage', { value: store, configurable: true });
}

describe('useLocalStorage error reporting', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    if (real) Object.defineProperty(window, 'localStorage', real);
    else delete (window as { localStorage?: Storage }).localStorage;
  });

  it('warns (not console.log) and falls back when reading throws', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});
    stubStorage({ getItem: () => { throw new Error('storage disabled'); } });
    const { result } = renderHook(() => useLocalStorage('lk-read', 'fallback'));
    expect(result.current[0]).toBe('fallback');
    expect(warn).toHaveBeenCalled();
    expect(log).not.toHaveBeenCalled();
  });

  it('warns (not console.log) when writing throws', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});
    stubStorage({ setItem: () => { throw new Error('quota exceeded'); } });
    const { result } = renderHook(() => useLocalStorage('lk-write', 'v'));
    act(() => { result.current[1]('next'); });
    expect(warn).toHaveBeenCalled();
    expect(log).not.toHaveBeenCalled();
  });
});
