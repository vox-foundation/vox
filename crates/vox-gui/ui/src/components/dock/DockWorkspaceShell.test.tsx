// crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.test.tsx
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import React from 'react';
import { DockviewApi } from 'dockview';
import { __resetBackendAvailabilityForTests } from '../../lib/backendGuard';
import { voxTransport } from '../../transport';
import { DockWorkspaceShell, layoutStorageKeyFor } from './DockWorkspaceShell';

describe('DockWorkspaceShell', () => {
  beforeEach(() => {
    localStorage.clear();
    delete (globalThis as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetBackendAvailabilityForTests();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    delete (globalThis as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetBackendAvailabilityForTests();
  });

  it('mounts a dockview-theme-vox container and calls onReady with an api', () => {
    const onReady = vi.fn();
    const { container } = render(
      <DockWorkspaceShell storageKeyPrefix="test.host" onReady={onReady} components={{}} />,
    );
    expect(container.querySelector('.dockview-theme-vox')).not.toBeNull();
    expect(onReady).toHaveBeenCalledTimes(1);
    expect(onReady.mock.calls[0][0]).toHaveProperty('api');
  });

  it('restores a previously serialized layout via fromJSON on mount, keyed per storageKeyPrefix', () => {
    const savedLayout = { grid: {} };
    localStorage.setItem(layoutStorageKeyFor('test.host'), JSON.stringify(savedLayout));

    const fromJSONSpy = vi.spyOn(DockviewApi.prototype, 'fromJSON').mockImplementation(() => {});
    const onReady = vi.fn();
    render(<DockWorkspaceShell storageKeyPrefix="test.host" onReady={onReady} components={{}} />);

    expect(fromJSONSpy).toHaveBeenCalledTimes(1);
    expect(fromJSONSpy).toHaveBeenCalledWith(savedLayout);
    fromJSONSpy.mockRestore();
  });

  it('two different storageKeyPrefix values persist to two different localStorage keys', () => {
    expect(layoutStorageKeyFor('gui.chat')).not.toBe(layoutStorageKeyFor('gui.other-host'));
  });

  it('never persists panel params (live React nodes) — only geometry survives the round trip', async () => {
    function Probe(props: { params: { node: React.ReactNode } }) {
      return <div>{props.params.node}</div>;
    }
    const onReady = vi.fn((event) => {
      event.api.addPanel({ id: 'probe', component: 'probe', params: { node: <span>live content</span> } });
    });
    render(<DockWorkspaceShell storageKeyPrefix="test.host2" onReady={onReady} components={{ probe: Probe }} />);

    await new Promise((resolve) => setTimeout(resolve, 1200)); // past LAYOUT_PERSIST_DEBOUNCE_MS (1000ms)

    const persisted = localStorage.getItem(layoutStorageKeyFor('test.host2'));
    expect(persisted).not.toBeNull();
    expect(persisted).not.toContain('"params"');
    expect(persisted).toContain('probe');
  }, 10000);

  it('writes debounced layout changes through to voxTransport.setGuiPreference when backend is available', async () => {
    (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    const setPrefSpy = vi
      .spyOn(voxTransport, 'setGuiPreference')
      .mockResolvedValue(undefined);
    vi.spyOn(voxTransport, 'getGuiPreference').mockResolvedValue(null);

    function Probe() {
      return <div>probe</div>;
    }
    const onReady = vi.fn((event) => {
      event.api.addPanel({ id: 'p1', component: 'probe' });
    });

    render(
      <DockWorkspaceShell
        storageKeyPrefix="test.backend-write"
        onReady={onReady}
        components={{ probe: Probe }}
      />,
    );

    await new Promise((resolve) => setTimeout(resolve, 1200));

    const key = layoutStorageKeyFor('test.backend-write');
    const localVal = localStorage.getItem(key);
    expect(localVal).not.toBeNull();
    expect(setPrefSpy).toHaveBeenCalledWith(key, localVal);
  }, 10000);

  it('hydrates remote dock layout from voxTransport.getGuiPreference when localStorage is empty', async () => {
    (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    const remoteLayout = { grid: { root: { type: 'branch', data: [] }, width: 800, height: 600, orientation: 'HORIZONTAL' } };
    const remoteRaw = JSON.stringify(remoteLayout);
    const getPrefSpy = vi
      .spyOn(voxTransport, 'getGuiPreference')
      .mockResolvedValue(remoteRaw);
    const fromJSONSpy = vi
      .spyOn(DockviewApi.prototype, 'fromJSON')
      .mockImplementation(() => {});

    const key = layoutStorageKeyFor('test.backend-restore');
    const onReady = vi.fn();
    render(
      <DockWorkspaceShell
        storageKeyPrefix="test.backend-restore"
        onReady={onReady}
        components={{}}
      />,
    );

    await waitFor(() => {
      expect(getPrefSpy).toHaveBeenCalledWith(key);
      expect(fromJSONSpy).toHaveBeenCalledWith(remoteLayout);
    });
    expect(localStorage.getItem(key)).toBe(remoteRaw);
  });
});
