import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { readFileSync } from 'node:fs';
import path from 'node:path';

import {
  ResearchModeDropdown,
  RESEARCH_OPTIONS,
  type ResearchMode,
} from './ResearchModeDropdown';

describe('ResearchModeDropdown', () => {
  it('renders current continuum mode label and Socrates badge for auto', () => {
    render(<ResearchModeDropdown mode="auto" onChange={vi.fn()} />);
    const trigger = screen.getByRole('button', { name: /Research mode: Auto \(Socrates\)/i });
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(trigger).toHaveTextContent('Auto');
    expect(trigger).toHaveTextContent('Socrates');
  });

  it.each<[ResearchMode, string, string]>([
    ['auto', 'Auto (Socrates)', 'Auto'],
    ['fast', 'Fast Web', 'Fast'],
    ['deep', 'Deep Research', 'Deep'],
    ['off', 'Off', 'Off'],
  ])('opens popover and selects %s (%s)', async (targetMode, label, shortLabel) => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const initialMode: ResearchMode = targetMode === 'auto' ? 'off' : 'auto';
    const { unmount } = render(<ResearchModeDropdown mode={initialMode} onChange={onChange} />);

    const trigger = screen.getByTestId('research-mode-trigger');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');

    await user.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');

    const optionBtn = screen.getByRole('button', {
      name: new RegExp(`^Set research mode: ${label.replace(/[()]/g, '\\$&')}$`, 'i'),
    });
    expect(optionBtn).toHaveTextContent(label);
    await user.click(optionBtn);

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(targetMode);
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    unmount();

    const { unmount: unmountSelected } = render(
      <ResearchModeDropdown mode={targetMode} onChange={vi.fn()} />,
    );
    expect(screen.getByTestId('research-mode-trigger')).toHaveTextContent(shortLabel);
    unmountSelected();
  });

  it('closes the open popover when Escape is pressed', async () => {
    const user = userEvent.setup();
    render(<ResearchModeDropdown mode="auto" onChange={vi.fn()} />);

    const trigger = screen.getByTestId('research-mode-trigger');
    await user.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');

    await user.keyboard('{Escape}');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
  });

  it('exposes all four orthogonal continuum modes in RESEARCH_OPTIONS', () => {
    expect(RESEARCH_OPTIONS.map((o) => o.id)).toEqual(['auto', 'fast', 'deep', 'off']);
    expect(RESEARCH_OPTIONS.map((o) => o.continuumTier)).toEqual([
      'Adaptive',
      'QuickWeb',
      'DeepMultiWave',
      'Direct',
    ]);
  });
});

describe('Composer continuum routing wiring guard (CONT-01)', () => {
  it('App.tsx submitFromComposer preserves caller overrides and separates research_mode from research_scope', () => {
    const appSrc = readFileSync(path.resolve(__dirname, '../../../App.tsx'), 'utf8');
    const start = appSrc.indexOf('const submitFromComposer');
    expect(start).toBeGreaterThan(-1);
    const block = appSrc.slice(start, start + 900);
    expect(block).toContain('p.force_research ??');
    expect(block).toContain('research_mode: p.research_mode ?? researchMode');
    expect(block).toContain('research_scope: p.research_scope ?? null');
    expect(block).not.toContain('research_scope: researchMode ===');
  });
});
