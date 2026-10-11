// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { LanguageProvider } from '../../../hooks/useLanguage';
import type { CommandCatalogEntry } from '../../../types/catalog';

const exec = vi.hoisted(() => vi.fn());
vi.mock('../../../transport', () => ({ voxTransport: { executeCommand: exec } }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(null) }));

import { Catalog } from './Catalog';

const CHECK: CommandCatalogEntry = {
  path: ['check'],
  command: 'vox check',
  about: 'Run vox check',
  aliases: [],
  has_subcommands: false,
  compiled_in: true,
  source_group: 'core',
  feature_gate: null,
  tier: 'recommended',
  arguments: [],
};

const arg = (over: Record<string, unknown>) => ({
  name: 'x', short: null, long: null, help: null, required: false, takes_value: true, value_kind: 'value' as const, ...over,
});
const PLUGIN_REMOVE: CommandCatalogEntry = {
  ...CHECK,
  path: ['plugin', 'remove'],
  command: 'vox plugin remove',
  arguments: [arg({ name: 'name', required: true })],
};
const FEATURES: CommandCatalogEntry = {
  ...CHECK,
  path: ['features'],
  command: 'vox features',
  arguments: [arg({ name: 'domain', long: 'domain' })],
};

describe('Catalog', () => {
  it('renders the Command Center heading', () => {
    render(<LanguageProvider><Catalog catalog={[]} /></LanguageProvider>);
    expect(screen.getByText(/command center/i)).toBeDefined();
  });

  it('renders without crashing when no catalog is provided', () => {
    expect(() => render(<LanguageProvider><Catalog /></LanguageProvider>)).not.toThrow();
  });

  it('a prefill selects that command and shows its form heading', () => {
    render(
      <LanguageProvider>
        <Catalog catalog={[CHECK]} prefill={{ path: ['check'], nonce: 1 }} />
      </LanguageProvider>,
    );
    expect(screen.getByRole('heading', { name: 'vox check' })).toBeDefined();
  });

  describe('Run (D-03, D-16)', () => {
    beforeEach(() => exec.mockReset());

    it('sends positionals, calls execute_command directly, and retries with consent after a confirmation refusal', async () => {
      exec.mockRejectedValueOnce('confirmation_required: cli.plugin.remove');
      exec.mockResolvedValueOnce({ exit_code: 0, stdout: 'removed', stderr: '' });
      render(
        <LanguageProvider>
          <Catalog catalog={[PLUGIN_REMOVE]} prefill={{ path: ['plugin', 'remove'], nonce: 1 }} />
        </LanguageProvider>,
      );
      await userEvent.type(screen.getByLabelText('name'), 'demo');
      await userEvent.click(screen.getByRole('button', { name: 'Run' }));
      await waitFor(() => expect(exec).toHaveBeenCalledTimes(1));
      expect(exec).toHaveBeenNthCalledWith(1, ['plugin', 'remove'], { __positionals: ['demo'], __argv: [] }, { confirmed: undefined });

      const dlg = await screen.findByRole('dialog', { name: 'Run a destructive command?' });
      expect(dlg.textContent).toContain('cli.plugin.remove');
      // Nothing was re-sent while the dialog was open.
      expect(exec).toHaveBeenCalledTimes(1);
      await userEvent.click(screen.getByRole('button', { name: 'Run anyway' }));
      await waitFor(() => expect(exec).toHaveBeenCalledTimes(2));
      expect(exec).toHaveBeenNthCalledWith(2, ['plugin', 'remove'], { __positionals: ['demo'], __argv: [] }, { confirmed: true });
      expect(await screen.findByText(/Exit Code: 0/)).toBeDefined();
    });

    it('Cancel in the confirm dialog never retries', async () => {
      exec.mockRejectedValueOnce('confirmation_required: cli.plugin.remove');
      render(
        <LanguageProvider>
          <Catalog catalog={[PLUGIN_REMOVE]} prefill={{ path: ['plugin', 'remove'], nonce: 1 }} />
        </LanguageProvider>,
      );
      await userEvent.type(screen.getByLabelText('name'), 'demo');
      await userEvent.click(screen.getByRole('button', { name: 'Run' }));
      await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
      expect(exec).toHaveBeenCalledTimes(1);
      expect(screen.queryByRole('dialog')).toBeNull();
    });

    it('sends an option argument under its long name and labels the button Run', async () => {
      exec.mockResolvedValueOnce({ exit_code: 0, stdout: 'ok', stderr: '' });
      render(
        <LanguageProvider>
          <Catalog catalog={[FEATURES]} prefill={{ path: ['features'], nonce: 1 }} />
        </LanguageProvider>,
      );
      expect(screen.queryByRole('button', { name: 'Execute' })).toBeNull();
      await userEvent.type(screen.getByLabelText('--domain'), 'ml');
      await userEvent.click(screen.getByRole('button', { name: 'Run' }));
      await waitFor(() => expect(exec).toHaveBeenCalledTimes(1));
      expect(exec).toHaveBeenCalledWith(['features'], { domain: 'ml', __argv: [] }, { confirmed: undefined });
    });
  });

});
