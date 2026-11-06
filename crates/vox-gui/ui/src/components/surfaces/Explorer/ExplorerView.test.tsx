import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import { ExplorerView } from './ExplorerView';
import {
  CAPABILITY_FIXTURE,
  DOMAIN_FIXTURE,
  GROUP_FIXTURE,
} from '../../../test-fixtures/capabilityIndexFixture';

const mockUseCapabilityIndex = vi.fn();

vi.mock('../../../hooks/useCapabilityIndex', () => ({
  useCapabilityIndex: (enabled: boolean) => mockUseCapabilityIndex(enabled),
}));

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(
    () =>
      ({
        height: 600,
        width: 900,
        top: 0,
        left: 0,
        right: 900,
        bottom: 600,
        x: 0,
        y: 0,
        toJSON: () => {},
      }) as DOMRect,
  );
  Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
    configurable: true,
    value: 600,
  });
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
    configurable: true,
    value: 600,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

function mockLoadedFixture() {
  mockUseCapabilityIndex.mockReturnValue({
    rows: CAPABILITY_FIXTURE,
    entries: CAPABILITY_FIXTURE,
    domains: DOMAIN_FIXTURE,
    groups: GROUP_FIXTURE,
    coverageAsOf: { sha: 'abcdef123456', date: '2026-10-15' },
    loading: false,
    error: null,
    retry: vi.fn(),
  });
}

describe('ExplorerView', () => {
  it('renders title, subtitle, snapshot, domain rail, filter bar, and treegrid', () => {
    mockLoadedFixture();
    render(<ExplorerView pushToast={vi.fn()} />);

    expect(screen.getByRole('heading', { name: /Capability Explorer/i })).toBeTruthy();
    expect(
      screen.getByText('Everything Vox can do, by domain, and how you can reach it.'),
    ).toBeTruthy();
    expect(screen.getByText(/Coverage as of abcdef123 \(2026-10-15\)/)).toBeTruthy();

    expect(screen.getByRole('navigation', { name: 'Domains' })).toBeTruthy();
    expect(screen.getByRole('searchbox', { name: 'Search capabilities' })).toBeTruthy();
    expect(screen.getByRole('group', { name: 'Filter by status' })).toBeTruthy();
    expect(screen.getByRole('checkbox', { name: /Show exempt groups \(1\)/i })).toBeTruthy();

    const treegrid = screen.getByRole('treegrid', { name: 'Capabilities' });
    expect(treegrid).toBeTruthy();
    expect(screen.getByText('vox mesh')).toBeTruthy();
    expect(screen.getByText('vox fmt')).toBeTruthy();
  });

  it('filters by search (Escape clears), status chips (toggle ✓, counts ignore status filter), and show-exempt checkbox', () => {
    mockLoadedFixture();
    render(<ExplorerView pushToast={vi.fn()} />);

    const search = screen.getByRole('searchbox', { name: 'Search capabilities' }) as HTMLInputElement;
    fireEvent.change(search, { target: { value: 'mesh status' } });
    expect(screen.getByRole('status').textContent).toBe('1 capability in 1 group');

    // Escape clears a non-empty query
    fireEvent.keyDown(search, { key: 'Escape' });
    expect(search.value).toBe('');
    expect(screen.getByRole('status').textContent).toContain('capabilities in');

    // Status chips toggle aria-pressed and show ✓ when pressed; chip counts ignore status filter
    const chipGroup = screen.getByRole('group', { name: 'Filter by status' });
    const dedicatedChip = within(chipGroup).getByRole('button', { name: /Dedicated surface/i });
    const genericChip = within(chipGroup).getByRole('button', { name: /Generic form/i });
    expect(dedicatedChip.getAttribute('aria-pressed')).toBe('false');
    const initialGenericText = genericChip.textContent;

    fireEvent.click(dedicatedChip);
    expect(dedicatedChip.getAttribute('aria-pressed')).toBe('true');
    expect(dedicatedChip.textContent).toContain('✓');
    // Chip count for Generic form stays unchanged even while Dedicated surface is pressed
    expect(genericChip.textContent).toBe(initialGenericText);
    // Singular result count when 1 capability matches
    expect(screen.getByRole('status').textContent).toBe('1 capability in 1 group');

    fireEvent.click(dedicatedChip);
    expect(dedicatedChip.getAttribute('aria-pressed')).toBe('false');

    // Show exempt groups reveals exempt group (vox attention)
    expect(screen.queryByText('vox attention')).toBeNull();
    const exemptCheckbox = screen.getByRole('checkbox', { name: /Show exempt groups \(1\)/i });
    fireEvent.click(exemptCheckbox);
    expect(screen.getByText('vox attention')).toBeTruthy();
  });

  it('renders loading state with 8 rail skeletons, 8 tree skeletons, polite status, and disabled filters', () => {
    mockUseCapabilityIndex.mockReturnValue({
      rows: [],
      entries: [],
      domains: [],
      groups: [],
      coverageAsOf: null,
      loading: true,
      error: null,
      retry: vi.fn(),
    });

    const { container } = render(<ExplorerView pushToast={vi.fn()} />);
    const status = screen.getByRole('status');
    expect(status.textContent).toBe('Loading the capability index…');

    const skeletons = container.querySelectorAll('[data-slot="skeleton"]');
    expect(skeletons.length).toBe(16);

    expect(
      (screen.getByRole('searchbox', { name: 'Search capabilities' }) as HTMLInputElement).disabled,
    ).toBe(true);
    expect(
      (screen.getByRole('checkbox', { name: /Show exempt groups/i }) as HTMLInputElement).disabled,
    ).toBe(true);
  });

  it('renders degraded error state with SURFACE_REGISTRY fallback and focuses search after retry', () => {
    const retryMock = vi.fn();
    mockUseCapabilityIndex.mockReturnValue({
      rows: [],
      entries: [],
      domains: [],
      groups: [],
      coverageAsOf: null,
      loading: false,
      error: 'network offline',
      retry: retryMock,
    });

    const { rerender } = render(<ExplorerView pushToast={vi.fn()} onNavigate={vi.fn()} />);

    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain(
      'Capability index unavailable — network offline. Showing registered surfaces only.',
    );
    expect(screen.queryByRole('navigation', { name: 'Domains' })).toBeNull();
    expect(screen.queryByRole('searchbox', { name: 'Search capabilities' })).toBeNull();

    const retryBtn = screen.getByRole('button', { name: 'Retry loading' });
    fireEvent.click(retryBtn);
    expect(retryMock).toHaveBeenCalledTimes(1);

    // Simulate successful retry returning fixture rows
    mockLoadedFixture();
    rerender(<ExplorerView pushToast={vi.fn()} onNavigate={vi.fn()} />);

    expect(screen.queryByRole('alert')).toBeNull();
    const search = screen.getByRole('searchbox', { name: 'Search capabilities' });
    expect(document.activeElement).toBe(search);
  });

  it('renders empty state when index loads with zero rows', () => {
    mockUseCapabilityIndex.mockReturnValue({
      rows: [],
      entries: [],
      domains: DOMAIN_FIXTURE,
      groups: [],
      coverageAsOf: null,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<ExplorerView pushToast={vi.fn()} />);
    expect(screen.getByRole('heading', { name: 'No capabilities indexed' })).toBeTruthy();
    expect(
      screen.getByText(
        'The capability index is empty. Regenerate it with `vox ci capability-index` and rebuild the app.',
      ),
    ).toBeTruthy();
  });

  it('renders no-results state with Clear filters, exempt-only variant, and Search all domains', () => {
    mockLoadedFixture();
    render(<ExplorerView pushToast={vi.fn()} />);

    // Select Mesh / Populi domain
    const nav = screen.getByRole('navigation', { name: 'Domains' });
    fireEvent.click(within(nav).getByRole('button', { name: /Mesh \/ Populi/i }));
    expect(screen.getByRole('status').textContent).toBe('4 capabilities in Mesh / Populi');

    // Type a query that matches in Compiler (fmt) but not in Mesh / Populi
    const search = screen.getByRole('searchbox', { name: 'Search capabilities' });
    fireEvent.change(search, { target: { value: 'fmt' } });

    expect(screen.getByRole('heading', { name: 'No capabilities match' })).toBeTruthy();
    expect(
      screen.getByText(
        'Nothing in Mesh / Populi matches "fmt" with these filters. Clear a filter, or search all domains.',
      ),
    ).toBeTruthy();

    // Search all domains switches to All domains
    fireEvent.click(screen.getByRole('button', { name: 'Search all domains' }));
    expect(screen.getByText('vox fmt')).toBeTruthy();

    // Exempt-only hidden match offers 'Show exempt groups'
    fireEvent.change(search, { target: { value: 'attention' } });
    expect(
      screen.getByText('1 exempt groups are hidden. Show them to see everything in All domains.'),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Show exempt groups' }));
    expect(screen.getByText('vox attention')).toBeTruthy();

    // Non-matching query offers Clear filters which resets search, chips and show-exempt
    fireEvent.change(search, { target: { value: 'zzz-no-match' } });
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect((search as HTMLInputElement).value).toBe('');
    expect(
      (screen.getByRole('checkbox', { name: /Show exempt groups/i }) as HTMLInputElement).checked,
    ).toBe(false);
  });

  it('applies seed.query when seed.nonce changes and preserves user edits on the same nonce', () => {
    mockLoadedFixture();
    const { rerender } = render(
      <ExplorerView pushToast={vi.fn()} seed={{ query: 'mesh', nonce: 1 }} />,
    );

    const search = screen.getByRole('searchbox', { name: 'Search capabilities' }) as HTMLInputElement;
    expect(search.value).toBe('mesh');

    fireEvent.change(search, { target: { value: 'custom' } });
    expect(search.value).toBe('custom');

    // Re-render with the same nonce does not overwrite user edit
    rerender(<ExplorerView pushToast={vi.fn()} seed={{ query: 'mesh', nonce: 1 }} />);
    expect(search.value).toBe('custom');

    // Re-render with a new nonce applies the new seed query
    rerender(<ExplorerView pushToast={vi.fn()} seed={{ query: 'scientia', nonce: 2 }} />);
    expect(search.value).toBe('scientia');
  });
});
