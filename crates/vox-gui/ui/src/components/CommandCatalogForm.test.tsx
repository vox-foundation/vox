// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import React from 'react';
import type { CommandCatalog, CommandCatalogEntry } from '../types/catalog';

// A plain function, not vi.fn(): spy bookkeeping attaches a handler-less `.then` to the returned
// promise, which turns a deliberate rejection into a spurious unhandled-rejection failure.
const calls: { path: string[]; args: unknown; opts?: { confirmed?: boolean } }[] = [];
let next: (() => Promise<unknown>)[] = [];
vi.mock('../transport', () => ({
  voxTransport: {
    executeCommand: (path: string[], args: unknown, opts?: { confirmed?: boolean }) => {
      calls.push({ path, args, opts });
      return (next.shift() ?? (() => Promise.resolve({ exit_code: 0, stdout: '', stderr: '' })))();
    },
  },
}));

import { CommandCatalogForm } from './CommandCatalogForm';

function cat(path: string[]): CommandCatalogEntry {
  return {
    path,
    command: path.join(' '),
    about: `about ${path.join(' ')}`,
    aliases: [],
    has_subcommands: false,
    compiled_in: true,
    source_group: 'g',
    feature_gate: null,
    tier: 'advanced',
    arguments: [],
  };
}
const catalog = { generated_from: 't', entries: [cat(['plugin', 'remove']), cat(['features'])] } as CommandCatalog;

const select = (p: string) => fireEvent.click(document.querySelector(`[data-catalog-path="${p}"]`) as HTMLElement);
const refuse = (id: string) => () => Promise.reject(new Error(`confirmation_required: ${id}`));
const ok = (stdout = 'ok') => () => Promise.resolve({ exit_code: 0, stdout, stderr: '' });

describe('CommandCatalogForm run safety (WR-08)', () => {
  beforeEach(() => {
    calls.length = 0;
    next = [];
  });

  it('Confirm sends confirmed:true to the refused path and args', async () => {
    next = [refuse('cli.plugin.remove'), ok()];
    render(<CommandCatalogForm catalog={catalog} />);
    select('plugin remove');
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    const dialog = await screen.findByRole('dialog', { name: 'Run a destructive command?' });
    expect(dialog.textContent).toContain('vox plugin remove');
    fireEvent.click(screen.getByRole('button', { name: 'Run anyway' }));
    await waitFor(() => expect(calls).toHaveLength(2));
    expect(calls[1].path).toEqual(['plugin', 'remove']);
    expect(calls[1].opts).toEqual({ confirmed: true });
  });

  it('changing the selection closes a pending confirmation, which can never confirm another command', async () => {
    next = [refuse('cli.plugin.remove')];
    render(<CommandCatalogForm catalog={catalog} />);
    select('plugin remove');
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    await screen.findByRole('dialog');
    act(() => select('features'));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(calls.filter((c) => c.opts?.confirmed)).toHaveLength(0);
  });

  it('a late refusal for a command that is no longer selected opens nothing', async () => {
    let reject!: (e: Error) => void;
    next = [() => new Promise((_r, rej) => { reject = rej; })];
    render(<CommandCatalogForm catalog={catalog} />);
    select('plugin remove');
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    select('features'); // the user moves on while the first run is pending
    await act(async () => reject(new Error('confirmation_required: cli.plugin.remove')));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText(/Execution Result/)).toBeNull();
  });

  it('Run is guarded while a run is in flight (no double execution)', async () => {
    let resolve!: (v: unknown) => void;
    next = [() => new Promise((r) => { resolve = r; })];
    render(<CommandCatalogForm catalog={catalog} />);
    select('features');
    const run = screen.getByRole('button', { name: 'Run' });
    fireEvent.click(run);
    fireEvent.click(run);
    expect(calls).toHaveLength(1);
    expect((screen.getByRole('button', { name: /Running/ }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => resolve({ exit_code: 0, stdout: 'done', stderr: '' }));
    await screen.findByText('done');
    expect((screen.getByRole('button', { name: 'Run' }) as HTMLButtonElement).disabled).toBe(false);
  });
});
