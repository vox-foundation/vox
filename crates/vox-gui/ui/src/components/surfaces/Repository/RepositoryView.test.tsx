import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';

const invokeMock = vi.fn((cmd: string, args?: { tool?: string }) => {
  if (cmd === 'get_vcs_isolation') {
    return Promise.resolve({ strategy_default: 'shared_branch', per_agent: {}, active_conflicts: [] });
  }
  if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_repo_status') {
    return Promise.resolve({
      is_error: false,
      result: {
        success: true,
        data: {
          repository_id: 'vox-main',
          root: '/workspace/vox',
          capabilities: { vox_project: true, cargo_workspace: true, git: true },
          active_context: { repository_id: 'vox-main', display_name: 'vox', root_path: '/workspace/vox', access_mode: 'local', read_only: false },
          openwiki: { initialized: true, page_count: 6, total_pages: 6, stale_pages: [] },
        },
      },
    });
  }
  if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_repo_catalog_list') {
    return Promise.resolve({
      is_error: false,
      result: {
        success: true,
        data: {
          primary_repository_id: 'vox-main',
          repositories: [
            { repository_id: 'vox-main', display_name: 'vox', resolved_root: '/workspace/vox', access_mode: 'local', read_only: false },
          ],
        },
      },
    });
  }
  if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_repo_catalog_refresh') {
    return Promise.resolve({ is_error: false, result: { success: true, data: {} } });
  }
  return Promise.resolve({ exit_code: 0, stdout: 'ok', stderr: '' });
});
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

const recordGamifyMock = vi.fn().mockResolvedValue(null);
vi.mock('../../../lib/gamifyGuiEvents', () => ({
  recordGamifyGuiEvent: (...args: unknown[]) => recordGamifyMock(...args),
}));

import { LanguageProvider } from '../../../hooks/useLanguage';
import { RepositoryView } from './RepositoryView';

describe('RepositoryView', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
    recordGamifyMock.mockClear();
  });

  it('renders the Repository Harness heading', () => {
    render(<LanguageProvider><RepositoryView pushToast={vi.fn()} /></LanguageProvider>);
    expect(screen.getByText('Repository Harness')).toBeTruthy();
  });

  it('condensed prop renders only an active-conflict count, not the action grid', async () => {
    render(<LanguageProvider><RepositoryView pushToast={vi.fn()} condensed /></LanguageProvider>);
    // Reuses the same conflictRows(status) count IsolationPanel's "Active
    // conflicts" section already computes, rendered on its own.
    await waitFor(() => expect(screen.getByText(/0 active conflicts/i)).toBeTruthy());
    expect(screen.queryByText('Workspace status')).toBeNull();
  });

  it('every action button carries an explicit type="button"', async () => {
    render(<LanguageProvider><RepositoryView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('Workspace status')).toBeTruthy());
    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('marks the command output region as a polite live region', () => {
    render(<LanguageProvider><RepositoryView pushToast={vi.fn()} /></LanguageProvider>);
    const out = screen.getByLabelText('Command output');
    expect(out.getAttribute('aria-live')).toBe('polite');
  });

  it('fires isolation_scan_complete when a repository action succeeds', async () => {
    render(<LanguageProvider><RepositoryView pushToast={vi.fn()} gamifyEnabled /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('Workspace status')).toBeTruthy());
    fireEvent.click(screen.getByText('Workspace status'));
    await vi.waitFor(() => {
      expect(recordGamifyMock).toHaveBeenCalledWith(
        'isolation_scan_complete',
        { label: 'Workspace status', path: 'status' },
        { enabled: true },
      );
    });
  });

  it('renders Multi-Repo Catalog & Onboarding Wizard and supports Read-Only onboard invocation', async () => {
    render(<LanguageProvider><RepositoryView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => {
      expect(screen.getByTestId('repo-catalog-panel')).toBeTruthy();
      expect(screen.getByTestId('repo-onboarding-wizard')).toBeTruthy();
      expect(screen.getByTestId('repo-active-pill')).toHaveTextContent('vox (hybrid)');
    });
    fireEvent.click(screen.getByTestId('repo-access-mode-toggle'));
    fireEvent.click(screen.getByTestId('repo-onboard-init-btn'));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'invoke_mcp_tool',
        expect.objectContaining({
          tool: 'vox_repo_catalog_refresh',
          args: expect.objectContaining({ action: 'init', read_only: true }),
        }),
      );
    });
  });
});
