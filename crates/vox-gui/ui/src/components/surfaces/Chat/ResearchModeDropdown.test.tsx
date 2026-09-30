// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ResearchModeDropdown, RESEARCH_OPTIONS } from './ResearchModeDropdown';

describe('ResearchModeDropdown', () => {
  it('renders the selected mode trigger correctly for auto', () => {
    render(<ResearchModeDropdown mode="auto" onChange={vi.fn()} />);
    const trigger = screen.getByRole('button', { name: /Research mode: Auto \(Socrates\)/i });
    expect(trigger).toBeDefined();
    expect(trigger.textContent).toContain('Research:');
    expect(trigger.textContent).toContain('Auto');
    expect(trigger.textContent).toContain('Socrates');
  });

  it('renders fast, deep, and off trigger labels', () => {
    const { rerender } = render(<ResearchModeDropdown mode="fast" onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /Research mode: Fast Web/i }).textContent).toContain('Fast');

    rerender(<ResearchModeDropdown mode="deep" onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /Research mode: Deep Research/i }).textContent).toContain('Deep');

    rerender(<ResearchModeDropdown mode="off" onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: /Research mode: Off/i }).textContent).toContain('Off');
  });

  it('opens menu and selects a different mode', () => {
    const onChange = vi.fn();
    render(<ResearchModeDropdown mode="auto" onChange={onChange} />);

    const trigger = screen.getByRole('button', { name: /Research mode: Auto \(Socrates\)/i });
    fireEvent.click(trigger);

    const deepOption = screen.getByRole('button', { name: /Set research mode: Deep Research/i });
    expect(deepOption).toBeDefined();
    fireEvent.click(deepOption);

    expect(onChange).toHaveBeenCalledWith('deep');
  });

  it('contains all 4 expected research options', () => {
    expect(RESEARCH_OPTIONS.map(o => o.id)).toEqual(['auto', 'fast', 'deep', 'off']);
  });
});
