// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, waitFor, cleanup } from '@testing-library/react';
import React from 'react';
import type { BackendStatus } from '../types/tauri';

const axisBackendStatus = vi.fn();
vi.mock('../transport', () => ({ axisBackendStatus: () => axisBackendStatus() }));

import { BackendStatusProvider, LOCAL_BACKEND_STATUS, surfaceAvailability, useBackendStatus } from './useBackendStatus';

const REMOTE: BackendStatus = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: null,
  surfaces: [
    { surface: 'mesh', available: true, reason: null },
    { surface: 'console', available: false, reason: 'needs_local_machine' },
  ],
};

function Probe() {
  const { status } = useBackendStatus();
  return <span data-testid="mode">{status.mode}</span>;
}

afterEach(() => {
  cleanup();
  axisBackendStatus.mockReset();
});

describe('useBackendStatus', () => {
  it('defaults to local without a provider', () => {
    render(<Probe />);
    expect(screen.getByTestId('mode').textContent).toBe('local');
  });

  it('loads the backend status on mount', async () => {
    axisBackendStatus.mockResolvedValue(REMOTE);
    render(<BackendStatusProvider><Probe /></BackendStatusProvider>);
    await waitFor(() => expect(screen.getByTestId('mode').textContent).toBe('remote'));
  });

  it('falls back to local when the command is missing or returns garbage', async () => {
    axisBackendStatus.mockResolvedValue(null);
    render(<BackendStatusProvider><Probe /></BackendStatusProvider>);
    await waitFor(() => expect(axisBackendStatus).toHaveBeenCalled());
    expect(screen.getByTestId('mode').textContent).toBe('local');
    cleanup();
    axisBackendStatus.mockRejectedValue(new Error('unknown command'));
    render(<BackendStatusProvider><Probe /></BackendStatusProvider>);
    await waitFor(() => expect(axisBackendStatus).toHaveBeenCalledTimes(2));
    expect(screen.getByTestId('mode').textContent).toBe('local');
  });
});

describe('surfaceAvailability', () => {
  it('is always available in local mode', () => {
    expect(surfaceAvailability(LOCAL_BACKEND_STATUS, 'console')).toEqual({ surface: 'console', available: true, reason: null });
  });

  it('uses the server-computed entry in remote mode', () => {
    expect(surfaceAvailability(REMOTE, 'console').reason).toBe('needs_local_machine');
    expect(surfaceAvailability(REMOTE, 'mesh').available).toBe(true);
  });

  it('treats an unknown view as not in profile when remote', () => {
    expect(surfaceAvailability(REMOTE, 'mystery').reason).toBe('not_in_profile');
  });
});
