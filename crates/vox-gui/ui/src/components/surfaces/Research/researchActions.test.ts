import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the Tauri core invoke bridge so the action can run outside Tauri.
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import {
  startResearchAsync,
  executeSandboxProbe,
  probeSearchProvider,
  probeAllSearchProviders,
} from './researchActions';

describe('startResearchAsync (A2)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('invokes the async daemon command, NOT the inline execute_command', async () => {
    invokeMock.mockResolvedValue({ session_id: 7, task_id: 't-1', status: 'running' });

    const handle = await startResearchAsync({ query: 'what is the latency trend?' });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [cmd, payload] = invokeMock.mock.calls[0];
    expect(cmd).toBe('start_research_async');
    // It must never fall back to the blocking inline CLI path.
    expect(cmd).not.toBe('execute_command');
    expect(payload).toMatchObject({ query: 'what is the latency trend?' });

    // Returns the fire-and-forget envelope immediately (no final result awaited).
    expect(handle).toEqual({ session_id: 7, task_id: 't-1', status: 'running' });
  });

  it('forwards optional scope/maxSources/verifyClaims as named args', async () => {
    invokeMock.mockResolvedValue({ session_id: 1, task_id: 't', status: 'running' });

    await startResearchAsync({ query: 'q', scope: 'web', maxSources: 5, verifyClaims: true });

    const [, payload] = invokeMock.mock.calls[0];
    expect(payload).toEqual({ query: 'q', scope: 'web', maxSources: 5, verifyClaims: true });
  });

  it('executeSandboxProbe invokes execute_sandbox_probe with code and language', async () => {
    invokeMock.mockResolvedValue({ passed: true, stdout: 'ok', stderr: '' });

    const res = await executeSandboxProbe('fn main() {}', 'rust');

    expect(invokeMock).toHaveBeenCalledWith('execute_sandbox_probe', {
      code: 'fn main() {}',
      language: 'rust',
    });
    expect(res).toEqual({ passed: true, stdout: 'ok', stderr: '' });
  });

  it('probeSearchProvider invokes probe_search_provider with provider and query', async () => {
    invokeMock.mockResolvedValue({ provider: 'duckduckgo', success: true });

    const res = await probeSearchProvider('duckduckgo', 'vox query');

    expect(invokeMock).toHaveBeenCalledWith('probe_search_provider', {
      provider: 'duckduckgo',
      query: 'vox query',
    });
    expect(res).toEqual({ provider: 'duckduckgo', success: true });
  });

  it('probeAllSearchProviders invokes probe_all_search_providers with query', async () => {
    invokeMock.mockResolvedValue([{ provider: 'all', success: true }]);

    const res = await probeAllSearchProviders('all query');

    expect(invokeMock).toHaveBeenCalledWith('probe_all_search_providers', {
      query: 'all query',
    });
    expect(res).toEqual([{ provider: 'all', success: true }]);
  });
});
