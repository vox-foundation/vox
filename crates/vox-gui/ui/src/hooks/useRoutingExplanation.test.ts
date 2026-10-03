// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';

const explainRouting = vi.fn();
const getRoutingHealth = vi.fn();
vi.mock('../transport', () => ({ voxTransport: { explainRouting: (...a: unknown[]) => explainRouting(...a), getRoutingHealth: () => getRoutingHealth() } }));

import { useRoutingExplanation } from './useRoutingExplanation';

beforeEach(() => {
  explainRouting.mockReset();
  getRoutingHealth.mockReset();
});

describe('useRoutingExplanation', () => {
  it('asks for the explanation and the health for the given mode, task and complexity', async () => {
    explainRouting.mockResolvedValue({ chosen: 'acme/widget-5.5', candidates: [], excluded: [] });
    getRoutingHealth.mockResolvedValue({ violations: [] });
    const { result } = renderHook(() => useRoutingExplanation('balanced', 'codegen', 7));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(explainRouting).toHaveBeenCalledWith('balanced', 'codegen', 7);
    expect(getRoutingHealth).toHaveBeenCalledTimes(1);
    expect(result.current.explanation?.chosen).toBe('acme/widget-5.5');
    expect(result.current.error).toBeNull();
  });

  it('treats a null or failed response as unavailable, not as a crash', async () => {
    explainRouting.mockRejectedValue(new Error('x'));
    getRoutingHealth.mockResolvedValue(null);
    const { result } = renderHook(() => useRoutingExplanation('efficiency', 'general', 5));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.explanation).toBeNull();
    expect(result.current.health).toBeNull();
    expect(result.current.error).toBe('Routing explanation unavailable');
  });
});
