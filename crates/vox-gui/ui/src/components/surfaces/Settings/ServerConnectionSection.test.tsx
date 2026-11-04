import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import React from 'react';
import type { BackendStatus } from '../../../types/tauri';

const axisConnectRemote = vi.fn();
const axisDisconnectRemote = vi.fn();
vi.mock('../../../transport', () => ({
  axisConnectRemote: (...a: unknown[]) => axisConnectRemote(...a),
  axisDisconnectRemote: () => axisDisconnectRemote(),
  axisBackendStatus: () => Promise.resolve({ mode: 'local', url: null, profile: null, surfaces: [] }),
}));

import { ServerConnectionSection, formatFingerprint } from './ServerConnectionSection';
import { BackendStatusProvider } from '../../../hooks/useBackendStatus';

const FP = 'ab'.repeat(32);
const REMOTE: BackendStatus = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: { profiles: ['mesh'], max_exposure: 'operator', subsystems: { booted: ['mesh'], failed: [] }, tools: [], axis_surfaces: ['mesh'], server_version: '0.6.0' },
  surfaces: [],
};

afterEach(() => {
  cleanup();
  axisConnectRemote.mockReset();
  axisDisconnectRemote.mockReset();
});

function renderSection() {
  const pushToast = vi.fn();
  render(<BackendStatusProvider><ServerConnectionSection pushToast={pushToast} /></BackendStatusProvider>);
  fireEvent.change(screen.getByLabelText(/server url/i), { target: { value: 'https://vox.lan:3921' } });
  fireEvent.change(screen.getByLabelText(/bearer token/i), { target: { value: 'tok' } });
  return pushToast;
}

describe('ServerConnectionSection', () => {
  it('groups fingerprints in fours', () => {
    expect(formatFingerprint('abcdef012345')).toBe('abcd ef01 2345');
  });

  it('asks to confirm the fingerprint on first use, then connects with it', async () => {
    axisConnectRemote
      .mockRejectedValueOnce(JSON.stringify({ error: { code: 'tls_pin_required', message: 'first', fingerprint: FP } }))
      .mockResolvedValueOnce(REMOTE);
    const pushToast = renderSection();
    fireEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    const fp = await screen.findByTestId('server-fingerprint');
    expect(fp.textContent).toContain('abab abab');
    expect(screen.getByText(/first connection to this server/i)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /trust and connect/i }));
    await waitFor(() => expect(screen.getByTestId('server-connected')).toBeTruthy());
    expect(axisConnectRemote).toHaveBeenLastCalledWith('https://vox.lan:3921', 'tok', FP);
    expect(pushToast).toHaveBeenCalledWith(expect.objectContaining({ tone: 'ok', cause: 'backend-ok' }));
  });

  it('warns when the pinned certificate changed', async () => {
    axisConnectRemote.mockRejectedValueOnce(
      JSON.stringify({ error: { code: 'tls_pin_mismatch', message: 'changed', pinned: 'cd'.repeat(32), fingerprint: FP } }),
    );
    renderSection();
    fireEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    expect(await screen.findByText(/certificate changed since you last connected/i)).toBeTruthy();
    expect(screen.getByText(/previously pinned/i).textContent).toContain('cdcd cdcd');
  });

  it('shows the version mismatch message', async () => {
    axisConnectRemote.mockRejectedValueOnce(
      JSON.stringify({ error: { code: 'version_mismatch', message: 'server runs Vox 0.7.0, Axis is 0.6.0; major.minor must match' } }),
    );
    renderSection();
    fireEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    expect((await screen.findByRole('alert')).textContent).toContain('major.minor must match');
  });
});
