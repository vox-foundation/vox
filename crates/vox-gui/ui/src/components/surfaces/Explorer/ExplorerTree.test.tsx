import React, { useState } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import { ExplorerTree } from './ExplorerTree';
import { applyFilters, buildTree, flatten } from './explorerModel';
import {
  CAPABILITY_FIXTURE,
  DOMAIN_FIXTURE,
  GROUP_FIXTURE,
  row,
  group,
} from '../../../test-fixtures/capabilityIndexFixture';
import type { CommandCatalogEntry } from '../../../types/catalog';

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
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function StatefulHarness({
  domain = 'mesh',
  showExempt = false,
  initialExpanded = [],
  commandCatalog,
  onNavigate,
  onOpenForm,
  onExplain,
  onOpenDoc,
}: {
  domain?: string | null;
  showExempt?: boolean;
  initialExpanded?: string[];
  commandCatalog?: readonly CommandCatalogEntry[];
  onNavigate?: (v: string) => void;
  onOpenForm?: (p: string[]) => void;
  onExplain?: (id: string) => void;
  onOpenDoc?: (d: string) => void;
}) {
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(initialExpanded));
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);
  const filtered = applyFilters(tree, { domain, showExempt });
  const { rows } = flatten(filtered, expanded);

  return (
    <div>
      <input type="checkbox" aria-label="Show exempt groups" />
      <ExplorerTree
        rows={rows}
        activeKey={activeKey}
        onActivate={setActiveKey}
        onToggle={(key) => {
          setExpanded((prev) => {
            const next = new Set(prev);
            if (next.has(key)) next.delete(key);
            else next.add(key);
            return next;
          });
        }}
        commandCatalog={commandCatalog}
        onNavigate={onNavigate}
        onOpenForm={onOpenForm}
        onExplain={onExplain}
        onOpenDoc={onOpenDoc}
      />
    </div>
  );
}

describe('ExplorerTree', () => {
  it('renders treegrid semantics with header row and correct row attributes', () => {
    render(<StatefulHarness domain="mesh" />);
    const grid = screen.getByRole('treegrid', { name: 'Capabilities' });
    expect(grid.getAttribute('aria-colcount')).toBe('3');
    expect(Number(grid.getAttribute('aria-rowcount'))).toBeGreaterThan(1);

    const headers = screen.getAllByRole('columnheader');
    expect(headers.map((h) => h.textContent)).toEqual(['Capability', 'Status', 'Actions']);

    const rows = screen.getAllByRole('row');
    expect(rows[0].getAttribute('aria-rowindex')).toBe('1');
    const firstDataRow = rows[1];
    expect(firstDataRow.getAttribute('aria-level')).toBe('1');
    expect(firstDataRow.getAttribute('aria-posinset')).toBe('1');
    expect(firstDataRow.getAttribute('aria-rowindex')).toBe('2');
    expect(firstDataRow.getAttribute('aria-expanded')).toBe('false');
  });

  it('toggles parent rows on click and marks leaf rows active without expanding', () => {
    render(<StatefulHarness domain="mesh" />);
    const meshRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox mesh'))!;
    expect(meshRow.getAttribute('aria-expanded')).toBe('false');

    fireEvent.click(meshRow);
    const expandedMeshRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox mesh') && !r.textContent?.includes('vox mesh status'))!;
    expect(expandedMeshRow.getAttribute('aria-expanded')).toBe('true');

    const statusLeaf = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox mesh status'))!;
    expect(statusLeaf.getAttribute('aria-expanded')).toBeNull();
    fireEvent.click(statusLeaf);
    expect(statusLeaf.getAttribute('tabindex')).toBe('0');
  });

  it('virtualizes a 1,000-leaf fixture under 150 DOM rows and retains focused row via rangeExtractor', () => {
    const bigVerbs = Array.from({ length: 1000 }, (_, i) =>
      row({
        id: `cli.big.v${i}`,
        kind: 'cli',
        title: `Verb ${i}`,
        domain: 'mesh',
        cli: `vox big v${i}`,
        catalogForm: true,
      }),
    );
    const bigGroups = [
      group({
        name: 'big',
        domain: 'mesh',
        status: 'generic_form',
        verbs: 1000,
        compiled: 1000,
      }),
    ];
    const tree = buildTree(DOMAIN_FIXTURE, bigGroups, bigVerbs);
    const filtered = applyFilters(tree, { domain: 'mesh' });
    const { rows } = flatten(filtered, new Set(['group:big']));

    function BigWrapper() {
      const [activeKey, setActiveKey] = useState<string | null>(rows[0].key);
      return (
        <ExplorerTree
          rows={rows}
          activeKey={activeKey}
          onActivate={setActiveKey}
          onToggle={() => {}}
        />
      );
    }

    render(<BigWrapper />);
    const grid = screen.getByRole('treegrid', { name: 'Capabilities' });
    expect(Number(grid.getAttribute('aria-rowcount'))).toBeGreaterThan(1000);

    const renderedRows = screen.getAllByRole('row');
    expect(renderedRows.length).toBeLessThan(150);

    // Focus retention: press End then ArrowUp x3 while virtual window is at top
    const firstDataRow = renderedRows[1];
    firstDataRow.focus();
    fireEvent.keyDown(firstDataRow, { key: 'End' });

    const lastRow = screen
      .getAllByRole('row')
      .find((r) => r.getAttribute('aria-rowindex') === String(rows.length + 1))!;
    expect(lastRow).toBeTruthy();
    expect(document.activeElement).toBe(lastRow);

    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });

    const expectedRowIndex = String(rows.length + 1 - 3);
    expect(document.activeElement?.getAttribute('role')).toBe('row');
    expect(document.activeElement?.getAttribute('aria-rowindex')).toBe(expectedRowIndex);
    expect(document.activeElement?.isConnected).toBe(true);
  });

  it('renders exemption strip and overdue tag when system time is past reviewBy', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-12-31T12:00:00'));

    const { unmount } = render(
      <StatefulHarness
        domain="orchestrator"
        showExempt={true}
        initialExpanded={['group:attention']}
      />,
    );
    expect(screen.getByText('Exempt')).toBeTruthy();
    expect(screen.queryByText('Review overdue')).toBeNull();
    expect(screen.getByText('vox-orchestrator')).toBeTruthy();
    expect(
      screen.getByText('Feature-gated behind dei; scheduled for Inbox consolidation.'),
    ).toBeTruthy();
    expect(screen.getByText('2026-12-31')).toBeTruthy();
    unmount();

    vi.setSystemTime(new Date('2027-01-02T12:00:00'));
    render(
      <StatefulHarness
        domain="orchestrator"
        showExempt={true}
        initialExpanded={['group:attention']}
      />,
    );
    expect(screen.getByText('Review overdue')).toBeTruthy();
    expect(screen.getByText('2026-12-31, overdue')).toBeTruthy();
  });

  it('wires row actions per D-10a and respects decideCliActivation without toggling parent row', () => {
    const onNavigate = vi.fn();
    const onOpenForm = vi.fn();
    const onExplain = vi.fn();
    const onOpenDoc = vi.fn();

    const catalog: CommandCatalogEntry[] = [
      {
        path: ['mesh', 'status'],
        command: 'vox mesh status',
        about: 'Status',
        aliases: [],
        has_subcommands: false,
        compiled_in: true,
        source_group: 'mesh',
        feature_gate: null,
        tier: 'recommended',
        arguments: [],
      },
      {
        path: ['mesh', 'gpu'],
        command: 'vox mesh gpu',
        about: 'GPU',
        aliases: [],
        has_subcommands: false,
        compiled_in: false,
        source_group: 'mesh',
        feature_gate: 'gpu',
        tier: 'recommended',
        arguments: [],
      },
    ];

    render(
      <StatefulHarness
        domain={null}
        initialExpanded={['group:mesh', 'mcp-only:mesh']}
        commandCatalog={catalog}
        onNavigate={onNavigate}
        onOpenForm={onOpenForm}
        onExplain={onExplain}
        onOpenDoc={onOpenDoc}
      />,
    );

    const meshGroupRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox mesh') && r.textContent?.includes('2 verbs'))!;

    // Dedicated group offers Open surface, Explain capability, Open docs, and crate button
    fireEvent.click(within(meshGroupRow).getByRole('button', { name: 'Open surface' }));
    expect(onNavigate).toHaveBeenCalledWith('mesh');
    expect(meshGroupRow.getAttribute('aria-expanded')).toBe('true');

    fireEvent.click(
      within(meshGroupRow).getByRole('button', { name: 'Explain capability vox mesh' }),
    );
    expect(onExplain).toHaveBeenCalledWith('surface.mesh');

    fireEvent.click(within(meshGroupRow).getByRole('button', { name: /Open docs/i }));
    expect(onOpenDoc).toHaveBeenCalledWith('architecture/where-things-live.md');

    fireEvent.click(within(meshGroupRow).getByRole('button', { name: /vox-populi/i }));
    expect(onExplain).toHaveBeenCalledWith('crate.vox-populi');

    // Generic group offers Open form([name])
    const fmtGroupRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox fmt'))!;
    fireEvent.click(within(fmtGroupRow).getByRole('button', { name: 'Open form' }));
    expect(onOpenForm).toHaveBeenCalledWith(['fmt']);

    // Read-only compiled verb (decide -> run) still renders Open form and calls onOpenForm
    const statusVerbRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox mesh status'))!;
    fireEvent.click(within(statusVerbRow).getByRole('button', { name: 'Open form' }));
    expect(onOpenForm).toHaveBeenCalledWith(['mesh', 'status']);

    // Uncompiled verb renders Open form with aria-disabled="true" and aria-describedby reason
    const gpuVerbRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox mesh gpu'))!;
    const disabledFormBtn = within(gpuVerbRow).getByRole('button', { name: 'Open form' });
    expect(disabledFormBtn.getAttribute('aria-disabled')).toBe('true');
    const descId = disabledFormBtn.getAttribute('aria-describedby')!;
    expect(document.getElementById(descId)?.textContent).toBe('Not compiled into this build');
    onOpenForm.mockClear();
    fireEvent.click(disabledFormBtn);
    expect(onOpenForm).not.toHaveBeenCalled();

    // MCP-only tool row offers only Explain capability and Open docs
    const mcpLeafRow = screen
      .getAllByRole('row')
      .find((r) => r.textContent?.includes('vox_orphan_probe'))!;
    expect(within(mcpLeafRow).queryByRole('button', { name: /Open (form|surface)/i })).toBeNull();
    expect(
      within(mcpLeafRow).getByRole('button', { name: 'Explain capability vox_orphan_probe' }),
    ).toBeTruthy();
  });

  it('implements the UI-SPEC treegrid keyboard sequence and action navigation', () => {
    const onOpenForm = vi.fn();
    render(
      <StatefulHarness domain={null} showExempt={true} onOpenForm={onOpenForm} />,
    );

    const dataRows = () => screen.getAllByRole('row').slice(1);
    const firstRow = dataRows()[0];
    firstRow.focus();
    expect(document.activeElement).toBe(firstRow);

    // Down to a collapsed expandable group row (vox mesh under Mesh / Populi)
    const meshIdx = dataRows().findIndex((r) => r.textContent?.includes('vox mesh'));
    for (let i = 0; i < meshIdx; i++) {
      fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    }
    const meshRow = document.activeElement as HTMLElement;
    expect(meshRow.textContent).toContain('vox mesh');
    expect(meshRow.getAttribute('aria-expanded')).toBe('false');

    // Right expands collapsed group
    fireEvent.keyDown(meshRow, { key: 'ArrowRight' });
    const expandedMeshRow = document.activeElement as HTMLElement;
    expect(expandedMeshRow.getAttribute('aria-expanded')).toBe('true');

    // Right moves to first action ('Open surface')
    fireEvent.keyDown(expandedMeshRow, { key: 'ArrowRight' });
    expect(document.activeElement?.textContent).toBe('Open surface');

    // Right -> 'Explain capability' -> 'Open docs' -> stays on 'Open docs'
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
    expect(document.activeElement?.textContent).toBe('Explain capability');
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
    expect(document.activeElement?.textContent).toContain('Open docs');
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
    expect(document.activeElement?.textContent).toContain('Open docs');

    // Left x2 -> back to first action, Left -> back to row (still expanded), Left -> collapsed
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
    expect(document.activeElement?.textContent).toBe('Explain capability');
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
    expect(document.activeElement?.textContent).toBe('Open surface');
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
    expect(document.activeElement?.getAttribute('role')).toBe('row');
    expect(document.activeElement?.getAttribute('aria-expanded')).toBe('true');

    // Escape on an action also returns focus to the row
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
    expect(document.activeElement?.textContent).toBe('Open surface');
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(document.activeElement?.getAttribute('role')).toBe('row');

    // Left collapses the expanded group row
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
    expect(document.activeElement?.getAttribute('aria-expanded')).toBe('false');

    // End -> last visible row; Home -> first row; PageDown/PageUp clamp cleanly
    fireEvent.keyDown(document.activeElement!, { key: 'End' });
    const allRowsNow = dataRows();
    expect(document.activeElement).toBe(allRowsNow[allRowsNow.length - 1]);

    fireEvent.keyDown(document.activeElement!, { key: 'Home' });
    expect(document.activeElement).toBe(allRowsNow[0]);

    fireEvent.keyDown(document.activeElement!, { key: 'PageDown' });
    expect(document.activeElement).toBe(allRowsNow[allRowsNow.length - 1]);
    fireEvent.keyDown(document.activeElement!, { key: 'PageUp' });
    expect(document.activeElement).toBe(allRowsNow[0]);
  });
});
