// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { SurfaceGate } from './SurfaceUnavailable';
import { LOCAL_BACKEND_STATUS, StaticBackendStatusProvider } from '../../hooks/useBackendStatus';
import type { BackendStatus } from '../../types/tauri';

const REMOTE: BackendStatus = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: null,
  surfaces: [
    { surface: 'mesh', available: true, reason: null },
    { surface: 'console', available: false, reason: 'needs_local_machine' },
    { surface: 'models', available: false, reason: 'not_remote_capable' },
    { surface: 'tasks', available: false, reason: 'not_in_profile' },
  ],
};

afterEach(cleanup);

function gate(status: BackendStatus, viewKey: string) {
  return render(
    <StaticBackendStatusProvider status={status}>
      <SurfaceGate viewKey={viewKey}><div data-testid="surface-body">body</div></SurfaceGate>
    </StaticBackendStatusProvider>,
  );
}

describe('SurfaceGate', () => {
  it('renders the surface in local mode', () => {
    gate(LOCAL_BACKEND_STATUS, 'console');
    expect(screen.getByTestId('surface-body')).toBeTruthy();
  });

  it('renders an available remote surface', () => {
    gate(REMOTE, 'mesh');
    expect(screen.getByTestId('surface-body')).toBeTruthy();
  });

  it.each([
    ['console', 'needs_local_machine', /files, processes, or settings on this machine/i],
    ['models', 'not_remote_capable', /cannot run against a remote server yet/i],
    ['tasks', 'not_in_profile', /profile does not include/i],
  ])('names the reason for %s', (viewKey, reason, text) => {
    gate(REMOTE, viewKey);
    const node = screen.getByTestId('surface-unavailable');
    expect(node.getAttribute('data-reason')).toBe(reason);
    expect(screen.getByText(/not available on this server/i)).toBeTruthy();
    expect(screen.getByText(text)).toBeTruthy();
    expect(screen.queryByTestId('surface-body')).toBeNull();
  });
});
