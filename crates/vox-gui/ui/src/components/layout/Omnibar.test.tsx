// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { clearSearchableRegistry, registerSearchable } from '../../lib/searchableRegistry';

const invokeMcpTool = vi.fn();
const indexState = vi.hoisted(() => ({ loading: false }));
vi.mock('../../transport', () => ({
  voxTransport: {
    voxDocsIndex: vi.fn().mockResolvedValue([]),
    voxContentManifest: vi.fn().mockResolvedValue([]),
    listPolicies: vi.fn().mockResolvedValue([]),
    openLocator: vi.fn().mockResolvedValue(undefined),
    invokeMcpTool: (...a: unknown[]) => invokeMcpTool(...a),
  },
}));

vi.mock('../../hooks/useSearchController', () => ({
  useSearchController: vi.fn(() => ({
    state: { query: 'pending', hits: [], loading: false, scopes: ['code'], requestToken: 1 },
    setQuery: vi.fn(),
    setScopes: vi.fn(),
  })),
}));

// finding #8: the mock must HONOR the query and the { kinds } filter, mirroring
// the real searchFederatedIndex(entries, query, { kinds }) — otherwise the test
// proves nothing about query filtering or prefix-mode kind restriction (#4).
vi.mock('../../hooks/useFederatedSearchIndex', () => ({
  useFederatedSearchIndex: () => ({
    entries: [],
    search: (query: string, options?: { kinds?: string[] }) => {
      const q = (query ?? '').toLowerCase();
      const all = [
        {
          kind: 'surface',
          id: 'surface:approvals',
          label: 'Approvals',
          detail: 'Runs',
          payload: { type: 'surface', viewKey: 'approvals' },
        },
      ];
      const byQuery = q ? all.filter((e) => e.label.toLowerCase().includes(q)) : [];
      const kinds = options?.kinds;
      return kinds && kinds.length > 0
        ? byQuery.filter((e) => kinds.includes(e.kind))
        : byQuery;
    },
  }),
}));

vi.mock('../../hooks/useContentManifest', () => ({
  useContentManifest: () => [
    {
      viewKey: 'activity',
      label: 'Activity',
      route: '#view=activity',
      headings: ['3 pending approvals'],
      copy: ['3 pending approvals'],
      commands: [],
      docs: [],
    },
  ],
}));

// Never import the generated 784 KB index in unit tests: mock the hook with the shared fixture.
vi.mock('../../hooks/useCapabilityIndex', async () => {
  const { CAPABILITY_FIXTURE } = await import('../../test-fixtures/capabilityIndexFixture');
  const { toRankEntries } = await import('../../lib/capabilityRank');
  const value = {
    rows: CAPABILITY_FIXTURE,
    entries: toRankEntries(CAPABILITY_FIXTURE),
    domains: [{ id: 'compiler', label: 'Compiler' }],
    error: null,
  };
  return { useCapabilityIndex: () => ({ ...value, loading: indexState.loading }) };
});

import { Omnibar } from './Omnibar';
import { Toasts } from '../ui/Toasts';
import type { CommandCatalogEntry } from '../../types/catalog';

const noop = () => {};

function cat(path: string[], over: Partial<CommandCatalogEntry> = {}): CommandCatalogEntry {
  return {
    path,
    command: path.join(' '),
    about: 'x',
    aliases: [],
    has_subcommands: false,
    compiled_in: true,
    source_group: 'g',
    feature_gate: null,
    tier: 'recommended',
    arguments: [],
    ...over,
  };
}

function renderOmnibar(overrides: Partial<React.ComponentProps<typeof Omnibar>> = {}) {
  return render(
    <Omnibar
      open
      onClose={noop}
      onNavigate={overrides.onNavigate ?? vi.fn()}
      onRunCommand={overrides.onRunCommand ?? vi.fn()}
      onSendToChat={overrides.onSendToChat ?? vi.fn()}
      onOpenDoc={overrides.onOpenDoc ?? vi.fn()}
      agents={[]}
      skills={[]}
      catalog={[]}
      onRunCli={vi.fn()}
      {...overrides}
    />,
  );
}

describe('Omnibar', () => {
  beforeEach(() => {
    clearSearchableRegistry();
    indexState.loading = false;
    invokeMcpTool.mockReset();
    // master-spec discover shape: { result: { results: [...] } } (NOT `neighbors`).
    invokeMcpTool.mockResolvedValue({ result: { results: [] } });
  });

  it('renders the index surface row (Best match) and the ON-SCREEN facet from the manifest', async () => {
    // D-05: the Approvals surface row comes from the capability index, not the federated source
    // (the federated mock still offers one; it must not render a second row). The manifest
    // heading "3 pending approvals" still feeds On Screen.
    renderOmnibar();
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'approvals' } });
    await waitFor(() => expect(screen.getAllByText('Approvals')).toHaveLength(1));
    expect(screen.getByText('Best match')).toBeTruthy();
    expect(screen.getByText('3 pending approvals')).toBeTruthy();
    expect(screen.getByText(/On Screen/i)).toBeTruthy();
  });

  it('Enter activates the top hit via onNavigate', async () => {
    const onNavigate = vi.fn();
    renderOmnibar({ onNavigate });
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'approvals' } });
    await waitFor(() => expect(screen.getByText('Best match')).toBeTruthy());
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(onNavigate).toHaveBeenCalledWith('approvals', undefined);
  });

  it('Shift+Enter sends the raw query to chat', async () => {
    const onSendToChat = vi.fn();
    renderOmnibar({ onSendToChat });
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'why is queue stuck' } });
    fireEvent.keyDown(window, { key: 'Enter', shiftKey: true });
    expect(onSendToChat).toHaveBeenCalledWith('why is queue stuck');
  });

  it('runtime registry feeds the ON-SCREEN facet', async () => {
    registerSearchable('mesh', [{ label: 'mesh: 4 peers online', detail: 'Mesh', viewKey: 'mesh' }]);
    renderOmnibar();
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'peers' } });
    await waitFor(() => expect(screen.getByText('mesh: 4 peers online')).toBeTruthy());
  });

  // finding #4: a `/` (skills) prefix must restrict federated kinds so the
  // surface row is NOT returned — proves prefix modes aren't decorative.
  it('skills prefix (/) restricts kinds — surface row is filtered out', async () => {
    renderOmnibar();
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: '/approvals' } });
    // With kinds=['doc','skill'], the surface:approvals entry is excluded.
    await waitFor(() => expect(screen.queryByText('Approvals')).toBeNull());
  });

  it('debounces the GRAPH discover MCP call — one fire per burst, not per keystroke', async () => {
    renderOmnibar();
    const input = screen.getByPlaceholderText(/search/i);
    const discoverCalls = () =>
      invokeMcpTool.mock.calls.filter((c) => c[0] === 'vox_search_structural');
    // Type a 4-char burst fast (well within the 200ms debounce window).
    for (const v of ['z', 'zn', 'zno', 'znod']) {
      fireEvent.change(input, { target: { value: v } });
    }
    // Wait for the debounce to settle and a discover call to land.
    await waitFor(() => expect(discoverCalls().length).toBeGreaterThan(0));
    // Debounced: the burst collapses to a SINGLE discover call for the final
    // query — NOT one call per keystroke (which would be 4).
    expect(discoverCalls().length).toBe(1);
    expect(discoverCalls()[0][1]).toEqual({ query: 'znod', limit: 6 });
  });

  it('Alt+ArrowRight expands graph neighbors of the selected node', async () => {
    // First call (discover, query lane) seeds the graph facet; second call
    // (neighbors lane) returns the expansion — both in master-spec `results` shape.
    invokeMcpTool
      .mockResolvedValueOnce({ result: { results: [{ node_id: 'surface:chat' }] } })
      .mockResolvedValueOnce({ result: { results: [{ node_id: 'surface:approvals' }] } });
    renderOmnibar();
    // Use a token that matches ONLY the graph-discover lane (federated/manifest
    // do not match), so the GRAPH row is the top row — deterministic selection.
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'znode' } });
    await waitFor(() => expect(screen.getByText('chat')).toBeTruthy()); // label derived from surface:<vk>
    // Top row is the graph node; Alt+ArrowRight (idx -1 → list[0]) expands it.
    fireEvent.keyDown(window, { key: 'ArrowRight', altKey: true });
    await waitFor(() => expect(screen.getByText('approvals')).toBeTruthy());
    expect(screen.getByText('chat')).toBeTruthy(); // original neighbor retained
  });

  it('typing "vox check" shows an index-backed Commands row with CLI and MCP chips; Enter opens its Catalog form', async () => {
    const onOpenCatalogForm = vi.fn();
    const onRunCommand = vi.fn();
    renderOmnibar({ onOpenCatalogForm, onRunCommand, catalog: [cat(['check'])] });
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'vox check' } });
    const label = await screen.findByText('vox check');
    const rowButton = label.closest('button') as HTMLElement;
    expect(rowButton.textContent).toContain('CLI');
    expect(rowButton.textContent).toContain('MCP');
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(onOpenCatalogForm).toHaveBeenCalledWith(['check']);
    expect(onRunCommand).not.toHaveBeenCalled();
  });

  it('typing "capability" shows Best match plus a Tools & crates facet with MCP and CLI chips; Enter explains the best row', async () => {
    const onExplain = vi.fn();
    renderOmnibar({ onExplain });
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'capability' } });
    await screen.findByText('Best match');
    expect(screen.getByText('Tools & crates')).toBeTruthy();
    const best = screen.getByText('vox_capability_explain').closest('button') as HTMLElement;
    const other = screen.getByText('vox_capability_features').closest('button') as HTMLElement;
    expect(best.previousElementSibling?.textContent).toContain('Best match');
    expect(other.textContent).toContain('MCP');
    expect(other.textContent).toContain('CLI');
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(onExplain).toHaveBeenCalledWith('mcp.vox_capability_explain');
  });

  it('# mode shows only crate rows', async () => {
    renderOmnibar();
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: '#vox-actor' } });
    await screen.findByText('vox-actor-runtime');
    expect(screen.queryByText('Surfaces')).toBeNull();
    expect(screen.queryByText('Commands')).toBeNull();
    expect(screen.queryByText('Tools & crates')).toBeNull(); // the lone crate is the Best match
  });

  it('is an accessible dialog and marks the keyboard-selected row with aria-current', async () => {
    renderOmnibar();
    expect(screen.getByRole('dialog', { name: 'Command palette' }).getAttribute('aria-modal')).toBe('true');
    fireEvent.change(screen.getByLabelText('Search Vox'), { target: { value: 'approvals' } });
    await screen.findByText('Best match');
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    const rows = screen.getAllByRole('button').filter((b) => b.getAttribute('aria-current') === 'true');
    expect(rows).toHaveLength(1);
    expect(rows[0].textContent).toContain('Approvals');
  });

  it('announces loading in a polite status region and suppresses "No matches" meanwhile', async () => {
    indexState.loading = true;
    renderOmnibar();
    fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'zzzz-nothing' } });
    const status = await screen.findByRole('status');
    expect(status.getAttribute('aria-live')).toBe('polite');
    expect(status.textContent).toContain('Loading capability index…');
    expect(screen.queryByText(/No matches/)).toBeNull();
  });
  describe('run from the palette (D-01)', () => {
    async function type(q: string) {
      fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: q } });
      return (await screen.findByText(q)).closest('button') as HTMLElement;
    }

    it('Enter on a read_only compiled-in leaf runs it and does NOT open the form', async () => {
      const onRunCli = vi.fn();
      const onOpenCatalogForm = vi.fn();
      renderOmnibar({ onRunCli, onOpenCatalogForm, catalog: [cat(['features'])] });
      const row = await type('vox features');
      expect(row.textContent).toContain('↵ runs');
      fireEvent.keyDown(window, { key: 'Enter' });
      expect(onRunCli).toHaveBeenCalledWith(['features']);
      expect(onOpenCatalogForm).not.toHaveBeenCalled();
    });

    it('SAFETY: an unclassified verb opens the form and is never run', async () => {
      const onRunCli = vi.fn();
      const onOpenCatalogForm = vi.fn();
      renderOmnibar({ onRunCli, onOpenCatalogForm, catalog: [cat(['check'])] });
      const row = await type('vox check');
      expect(row.textContent).toContain('↵ opens form');
      fireEvent.keyDown(window, { key: 'Enter' });
      expect(onOpenCatalogForm).toHaveBeenCalledWith(['check']);
      expect(onRunCli).not.toHaveBeenCalled();
    });

    it('SAFETY: a destructive verb opens the form even with no required arguments', async () => {
      const onRunCli = vi.fn();
      const onOpenCatalogForm = vi.fn();
      renderOmnibar({ onRunCli, onOpenCatalogForm, catalog: [cat(['plugin', 'remove'])] });
      await type('vox plugin remove');
      fireEvent.keyDown(window, { key: 'Enter' });
      expect(onOpenCatalogForm).toHaveBeenCalledWith(['plugin', 'remove']);
      expect(onRunCli).not.toHaveBeenCalled();
    });

    it('SAFETY: a read_only verb with a required argument opens the form', async () => {
      const onRunCli = vi.fn();
      const onOpenCatalogForm = vi.fn();
      const req = { name: 'q', short: null, long: null, help: null, required: true, takes_value: true, value_kind: 'value' as const };
      renderOmnibar({ onRunCli, onOpenCatalogForm, catalog: [cat(['features'], { arguments: [req] })] });
      await type('vox features');
      fireEvent.keyDown(window, { key: 'Enter' });
      expect(onOpenCatalogForm).toHaveBeenCalledWith(['features']);
      expect(onRunCli).not.toHaveBeenCalled();
    });

    it('a cli row absent from the live catalog explains instead of running', async () => {
      const onRunCli = vi.fn();
      const onExplain = vi.fn();
      renderOmnibar({ onRunCli, onExplain, catalog: [] });
      const row = await type('vox features');
      expect(row.textContent).toContain('↵ explains');
      fireEvent.keyDown(window, { key: 'Enter' });
      expect(onExplain).toHaveBeenCalledWith('cli.features');
      expect(onRunCli).not.toHaveBeenCalled();
    });
  });

  describe('keybind action rows (D-13)', () => {
    const actions = [{ id: 'toggle-sidebar', label: 'Toggle sidebar width', chord: '⌘B' }];

    it('shows the live chord in a <kbd>; Enter closes the palette first, then runs the action', async () => {
      const onClose = vi.fn();
      const onRunAction = vi.fn();
      renderOmnibar({ actions, onRunAction, onClose });
      fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'Toggle sidebar width' } });
      const chord = await screen.findByText('⌘B');
      expect(chord.tagName).toBe('KBD');
      fireEvent.keyDown(window, { key: 'Enter' });
      expect(onRunAction).toHaveBeenCalledTimes(1);
      expect(onRunAction).toHaveBeenCalledWith('toggle-sidebar');
      expect(onClose).toHaveBeenCalled();
      expect(onClose.mock.invocationCallOrder[0]).toBeLessThan(onRunAction.mock.invocationCallOrder[0]);
    });

    it('clicking the row runs the action too', async () => {
      const onRunAction = vi.fn();
      renderOmnibar({ actions, onRunAction });
      fireEvent.change(screen.getByPlaceholderText(/search/i), { target: { value: 'sidebar' } });
      fireEvent.click((await screen.findByText('Toggle sidebar width')).closest('button') as HTMLElement);
      expect(onRunAction).toHaveBeenCalledWith('toggle-sidebar');
    });
  });

  it('a toast with an action renders its button and clicking it calls onClick', () => {
    const onClick = vi.fn();
    render(
      <Toasts
        items={[{ id: 't1', tone: 'ok', title: 'vox features — exit 0', cause: 'backend-ok', action: { label: 'View full output', onClick } }]}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'View full output' }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

});
