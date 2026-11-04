import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ExplorerRail } from './ExplorerRail';
import type { DomainCountEntry, StatusMixCounts } from './explorerModel';

const ALL_MIX: StatusMixCounts = {
  dedicated: 1,
  generic_form: 1,
  cli_only: 1,
  mcp_only: 2,
  total: 5,
};

const DOMAINS: DomainCountEntry[] = [
  {
    id: 'compiler',
    label: 'Compiler',
    mix: { dedicated: 0, generic_form: 1, cli_only: 0, mcp_only: 0, total: 1 },
  },
  {
    id: 'orchestrator',
    label: 'Orchestrator & agents',
    mix: { dedicated: 0, generic_form: 0, cli_only: 0, mcp_only: 0, total: 0 },
  },
  {
    id: 'mesh',
    label: 'Mesh / Populi',
    mix: { dedicated: 1, generic_form: 0, cli_only: 1, mcp_only: 2, total: 4 },
  },
];

describe('ExplorerRail', () => {
  it('renders All domains first, then domains in order with counts, mix bars, and sr-only summary', () => {
    const onSelect = vi.fn();
    render(
      <ExplorerRail
        domains={DOMAINS}
        allMix={ALL_MIX}
        selected={null}
        onSelect={onSelect}
      />,
    );

    const nav = screen.getByRole('navigation', { name: 'Domains' });
    const buttons = nav.querySelectorAll('button');
    expect(buttons).toHaveLength(4);
    expect(buttons[0].getAttribute('aria-current')).toBe('true');
    expect(buttons[0].textContent).toContain('All domains, 5 capabilities:');
    expect(buttons[1].textContent).toContain('Compiler, 1 capability:');
    expect(buttons[2].textContent).toContain('Orchestrator & agents, 0 capabilities:');
    expect(buttons[2].className).toContain('text-text-muted');
  });

  it('supports roving tabindex keyboard navigation without changing selection until Enter/Space', () => {
    const onSelect = vi.fn();
    render(
      <ExplorerRail
        domains={DOMAINS}
        allMix={ALL_MIX}
        selected={null}
        onSelect={onSelect}
      />,
    );

    const buttons = screen.getByRole('navigation', { name: 'Domains' }).querySelectorAll('button');
    expect(buttons[0].tabIndex).toBe(0);
    expect(buttons[1].tabIndex).toBe(-1);

    buttons[0].focus();
    fireEvent.keyDown(buttons[0], { key: 'ArrowDown' });
    expect(document.activeElement).toBe(buttons[1]);
    expect(onSelect).not.toHaveBeenCalled();

    fireEvent.keyDown(buttons[1], { key: 'End' });
    expect(document.activeElement).toBe(buttons[3]);

    fireEvent.keyDown(buttons[3], { key: 'Home' });
    expect(document.activeElement).toBe(buttons[0]);

    fireEvent.click(buttons[2]);
    expect(onSelect).toHaveBeenCalledWith('orchestrator');
  });

  it('replaces the rail with a native <select aria-label="Domain"> when narrow is true', () => {
    const onSelect = vi.fn();
    render(
      <ExplorerRail
        domains={DOMAINS}
        allMix={ALL_MIX}
        selected="mesh"
        onSelect={onSelect}
        narrow={true}
      />,
    );

    const select = screen.getByRole('combobox', { name: 'Domain' }) as HTMLSelectElement;
    expect(select.value).toBe('mesh');
    fireEvent.change(select, { target: { value: '' } });
    expect(onSelect).toHaveBeenCalledWith(null);
  });
});
