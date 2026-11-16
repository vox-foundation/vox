import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import type { TermRunbookDto } from '../../../transport';

const termRunbooksListMock = vi.fn<() => Promise<TermRunbookDto[] | null>>();

vi.mock('../../../transport', () => ({
  termRunbooksList: (...args: unknown[]) => termRunbooksListMock(...(args as [])),
}));

import {
  RunbookPanel,
  interpolateRunbookCommand,
  quoteShellParam,
} from './RunbookPanel';

const SAMPLE_RUNBOOKS: TermRunbookDto[] = [
  {
    id: 'rb-deploy',
    title: 'Deploy Service',
    description: 'Deploy target environment with tag',
    path: 'scripts/deploy.vox',
    kind: 'vox',
    command_template: 'vox run scripts/deploy.vox --env {{env}} --tag {{tag}}',
    params: [
      {
        name: 'env',
        default_value: 'staging',
        description: 'Target environment',
        options: ['staging', 'prod'],
      },
      {
        name: 'tag',
        default_value: 'v1.0.0',
        description: 'Release tag',
        options: [],
      },
    ],
    tags: ['ops', 'release'],
  },
  {
    id: 'rb-smoke',
    title: 'Smoke Check Guide',
    description: 'Run health checks from markdown runbook',
    path: 'docs/src/how-to/smoke.md',
    kind: 'markdown',
    command_template: 'vox doctor',
    params: [],
    tags: ['qa'],
  },
];

describe('RunbookPanel', () => {
  beforeEach(() => {
    cleanup();
    termRunbooksListMock.mockReset();
  });

  it('quotes shell-unsafe parameter values in quoteShellParam and interpolates {{ident=default}} templates (CR-03)', () => {
    expect(quoteShellParam('')).toBe("''");
    expect(quoteShellParam('staging-1')).toBe('staging-1');
    expect(quoteShellParam('user@host.local')).toBe('user@host.local');
    expect(quoteShellParam('prod; rm -rf /')).toBe("'prod; rm -rf /'");
    expect(quoteShellParam("a'b")).toBe("'a'\\''b'");

    const cmd = interpolateRunbookCommand(
      'vox run scripts/deploy.vox --env {{env=staging}} --note {{ note }}',
      { env: 'prod; rm -rf /', note: '$(whoami)' },
      [],
    );
    expect(cmd).toBe("vox run scripts/deploy.vox --env 'prod; rm -rf /' --note '$(whoami)'");

    const defaulted = interpolateRunbookCommand(
      'curl http://{{host=localhost}}:8080/healthz',
      {},
      [{ name: 'host', default_value: 'localhost', description: null, options: [] }],
    );
    expect(defaulted).toBe('curl http://localhost:8080/healthz');
  });

  it('loads runbooks, filters via searchbox, renders VoxSelect + text params, and dispatches Insert & Run with shell escaping', async () => {
    termRunbooksListMock.mockResolvedValue(SAMPLE_RUNBOOKS);
    const onInsert = vi.fn();
    const onRun = vi.fn();

    render(<RunbookPanel cwd="/workspace" onInsert={onInsert} onRun={onRun} />);

    await waitFor(() => expect(screen.getByText('Deploy Service')).toBeTruthy());
    expect(screen.getByTestId('term-runbook-panel')).toBeTruthy();

    const filterBox = screen.getByTestId('term-runbook-filter') as HTMLInputElement;
    expect(filterBox.type).toBe('search');
    expect(screen.getByRole('searchbox', { name: /Filter runbooks/i })).toBe(filterBox);

    // Enum param uses VoxSelect (combobox trigger, never native <select>)
    const envTrigger = screen.getByTestId('term-runbook-param-env');
    expect(envTrigger.getAttribute('data-slot')).toBe('select-trigger');

    // Free-text param input
    const tagInput = screen.getByTestId('term-runbook-param-tag') as HTMLInputElement;
    fireEvent.change(tagInput, { target: { value: 'prod; rm -rf /' } });

    fireEvent.click(screen.getByTestId('term-runbook-insert'));
    expect(onInsert).toHaveBeenCalledWith(
      "vox run scripts/deploy.vox --env staging --tag 'prod; rm -rf /'",
    );

    fireEvent.click(screen.getByTestId('term-runbook-run'));
    expect(onRun).toHaveBeenCalledWith(
      "vox run scripts/deploy.vox --env staging --tag 'prod; rm -rf /'",
    );

    // Filter by tag/title
    fireEvent.change(filterBox, { target: { value: 'qa' } });
    expect(screen.queryByText('Deploy Service')).toBeNull();
    expect(screen.getByText('Smoke Check Guide')).toBeTruthy();
  });

  it('renders No workspace runbooks discovered empty state, enforces required params (CR-03), and sanitizes scan errors', async () => {
    termRunbooksListMock.mockResolvedValueOnce([]);

    const { unmount } = render(<RunbookPanel onInsert={vi.fn()} onRun={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('No workspace runbooks discovered')).toBeTruthy());
    unmount();

    // Required param without default disables Run & Insert until filled
    termRunbooksListMock.mockResolvedValueOnce([
      {
        id: 'rb-req',
        title: 'Rotate Token',
        description: 'Requires profile',
        path: 'scripts/rotate.vox',
        kind: 'vox',
        command_template: 'vox run scripts/rotate.vox -- --profile {{profile}}',
        params: [{ name: 'profile', default_value: null, description: 'Profile', options: [] }],
        tags: [],
      },
    ]);
    const onRun = vi.fn();
    const { unmount: unmountReq } = render(<RunbookPanel onInsert={vi.fn()} onRun={onRun} />);
    await waitFor(() => expect(screen.getByTestId('term-runbook-param-error')).toBeTruthy());
    expect((screen.getByTestId('term-runbook-run') as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByTestId('term-runbook-param-profile'), { target: { value: 'prod' } });
    expect((screen.getByTestId('term-runbook-run') as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByTestId('term-runbook-run'));
    expect(onRun).toHaveBeenCalledWith('vox run scripts/rotate.vox -- --profile prod');
    unmountReq();

    // Error state sanitizes internal Tauri errors
    termRunbooksListMock.mockRejectedValueOnce(new Error('window.__TAURI_INTERNALS__ broken'));
    render(<RunbookPanel onInsert={vi.fn()} onRun={vi.fn()} />);
    const errAlert = await screen.findByTestId('term-runbook-error');
    expect(errAlert.textContent).not.toContain('__TAURI_INTERNALS__');
    expect(screen.getByTestId('term-runbook-reload')).toBeTruthy();
  });
});
