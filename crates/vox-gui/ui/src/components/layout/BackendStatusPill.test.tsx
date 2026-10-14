// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { BackendStatusPill } from './BackendStatusPill';
import { LOCAL_BACKEND_STATUS, StaticBackendStatusProvider } from '../../hooks/useBackendStatus';

afterEach(cleanup);

describe('BackendStatusPill', () => {
  it('renders nothing in local mode', () => {
    const { container } = render(
      <StaticBackendStatusProvider status={LOCAL_BACKEND_STATUS}><BackendStatusPill /></StaticBackendStatusProvider>,
    );
    expect(container.innerHTML).toBe('');
  });

  it('names the host and profile when remote', () => {
    render(
      <StaticBackendStatusProvider
        status={{
          mode: 'remote',
          url: 'https://vox.lan:3921/',
          profile: { profiles: ['mesh'], max_exposure: 'operator', subsystems: { booted: [], failed: [] }, tools: [], axis_surfaces: ['mesh'], server_version: '0.6.0' },
          surfaces: [],
        }}
      >
        <BackendStatusPill />
      </StaticBackendStatusProvider>,
    );
    const pill = screen.getByTestId('backend-status-pill');
    expect(pill.getAttribute('aria-label')).toBe('Connected to vox.lan:3921, profile mesh');
    expect(pill.textContent).toContain('vox.lan:3921');
  });
});
