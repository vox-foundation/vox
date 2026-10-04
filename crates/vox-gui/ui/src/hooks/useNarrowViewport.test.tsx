// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useNarrowViewport } from './useNarrowViewport';

type Listener = () => void;

function fakeMatchMedia(width: { current: number }) {
  const listeners = new Set<Listener>();
  window.matchMedia = ((q: string) => {
    const max = Number(/max-width:\s*(\d+)px/.exec(q)?.[1]);
    return {
      get matches() { return width.current <= max; },
      addEventListener: (_: string, l: Listener) => listeners.add(l),
      removeEventListener: (_: string, l: Listener) => listeners.delete(l),
    } as unknown as MediaQueryList;
  }) as typeof window.matchMedia;
  return () => listeners.forEach((l) => l());
}

describe('useNarrowViewport', () => {
  const original = window.matchMedia;
  afterEach(() => { window.matchMedia = original; });

  it('is false where matchMedia does not exist', () => {
    // @ts-expect-error simulate an environment without matchMedia
    window.matchMedia = undefined;
    expect(renderHook(() => useNarrowViewport(640)).result.current).toBe(false);
  });

  it('is true below the breakpoint and false at or above it, following resizes', () => {
    const width = { current: 390 };
    const fire = fakeMatchMedia(width);
    const { result } = renderHook(() => useNarrowViewport(640));
    expect(result.current).toBe(true);
    width.current = 640;
    act(() => fire());
    expect(result.current).toBe(false);
    width.current = 500;
    act(() => fire());
    expect(result.current).toBe(true);
  });
});
