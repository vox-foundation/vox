// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';
import { LanguageProvider } from '../../../hooks/useLanguage';
import type { CommandCatalogEntry } from '../../../types/catalog';

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
});
