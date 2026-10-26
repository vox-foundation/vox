import React from 'react';
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { ExplainDrawer } from './ExplainDrawer';
import { CAPABILITY_FIXTURE, row } from '../../test-fixtures/capabilityIndexFixture';

const DOMAINS = [{ id: 'compiler', label: 'Compiler' }];
const mcpCheck = CAPABILITY_FIXTURE.find((r) => r.id === 'mcp.vox_check')!;

function setup(over: Partial<React.ComponentProps<typeof ExplainDrawer>> = {}) {
  const props = {
    row: mcpCheck,
    rows: CAPABILITY_FIXTURE,
    domains: DOMAINS,
    catalogPaths: new Set(['check']),
    onClose: vi.fn(),
    onOpenForm: vi.fn(),
    onOpenDoc: vi.fn(),
    pushToast: vi.fn(),
    ...over,
  };
  const utils = render(<ExplainDrawer {...props} />);
  return { ...utils, props };
}

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(navigator, 'clipboard');
});

describe('ExplainDrawer', () => {
  it('renders a dialog named by the row title with the four groups', () => {
    setup();
    expect(screen.getByRole('dialog', { name: 'vox_check' })).toHaveAttribute('aria-modal', 'true');
    for (const h of ['Equivalents', 'Location', 'Status', 'Related']) {
      expect(screen.getByRole('heading', { name: h })).toBeInTheDocument();
    }
    expect(screen.getByText('vox check')).toBeInTheDocument();
  });

  it('closes on Escape and on the backdrop; a null row renders nothing', () => {
    const { props } = setup();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(props.onClose).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: 'Close explain card' }));
    expect(props.onClose).toHaveBeenCalledTimes(2);
    cleanup();
    const { container } = setup({ row: null });
    expect(container).toBeEmptyDOMElement();
  });

  it('Copy MCP name writes the tool name and toasts with cause clipboard', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const { props } = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Copy MCP name' }));
    expect(writeText).toHaveBeenCalledWith('vox_check');
    await waitFor(() =>
      expect(props.pushToast).toHaveBeenCalledWith(expect.objectContaining({ tone: 'ok', title: 'Copied', cause: 'clipboard' })),
    );
  });

  it('Copy MCP name warns when the clipboard API is missing', () => {
    Object.defineProperty(navigator, 'clipboard', { value: undefined, configurable: true });
    const { props } = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Copy MCP name' }));
    expect(props.pushToast).toHaveBeenCalledWith(expect.objectContaining({ tone: 'warn', title: 'Copy failed' }));
  });

  it('Open form passes the CLI path when available, else is disabled with the reason', () => {
    const a = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Open form' }));
    expect(a.props.onOpenForm).toHaveBeenCalledWith(['check']);
    cleanup();
    const b = setup({ catalogPaths: new Set() });
    expect(screen.getByRole('button', { name: 'Open form' })).toBeDisabled();
    expect(screen.getByText(/not in this build's command catalog/)).toBeInTheDocument();
    expect(b.props.onOpenForm).not.toHaveBeenCalled();
  });

  it('Open docs hands the docs-relative path to onOpenDoc, and is absent without a link', () => {
    setup();
    expect(screen.queryByRole('button', { name: 'Open docs' })).toBeNull();
    cleanup();
    const withDocs = row({ id: 'crate.x', kind: 'crate', title: 'x', crate: 'x', docs: 'docs/src/architecture/x.md' });
    const { props } = setup({ row: withDocs, rows: [withDocs] });
    fireEvent.click(screen.getByRole('button', { name: 'Open docs' }));
    expect(props.onOpenDoc).toHaveBeenCalledWith('architecture/x.md');
  });

  it('renders markup in a summary literally (no element created)', () => {
    const evil = row({ id: 'crate.evil', kind: 'crate', title: 'evil', summary: '<b>x</b>' });
    const { baseElement } = setup({ row: evil, rows: [evil] });
    expect(screen.getByText('<b>x</b>')).toBeInTheDocument();
    expect(baseElement.querySelector('b')).toBeNull();
  });
});
