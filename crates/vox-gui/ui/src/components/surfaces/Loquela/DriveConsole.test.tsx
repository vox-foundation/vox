// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/react';
import { within } from '@testing-library/react';
import { DriveConsole } from './DriveConsole';
import { defaultControl } from '../../../lib/driveConsole';
import { CLUTCH_DETENTS } from '../../../lib/driveConsole';
import { modeLabel } from '../../../lib/turnEvents';

describe('DriveConsole', () => {
  const base = {
    control: defaultControl(),
    onControlChange: vi.fn(),
    spentUsd: 0.42,
    budgetUsd: 1.0,
    burnPerMin: 0.08,
  };

  it('renders all four clutch detents, cost, risk — no model read-out segment', () => {
    render(<DriveConsole {...base} />);
    ['Free', 'Effic.', 'Bal.', 'Genius'].forEach(l =>
      expect(screen.getByRole('radio', { name: new RegExp(l, 'i') })).toBeTruthy()
    );
    expect(screen.getByText(/0\.42/)).toBeTruthy();
    expect(screen.getByText(/Moderate/i)).toBeTruthy();
    // Segment ④ (redundant model read-out) was dropped.
    expect(screen.queryByTitle(/active model/i)).toBeNull();
  });

  it('strip root does not clip the risk popover (no overflow-hidden)', () => {
    const { container } = render(<DriveConsole {...base} />);
    expect((container.firstChild as HTMLElement).className).not.toContain('overflow-hidden');
  });

  it('opens the risk popover anchored above the Risk trigger', () => {
    render(<DriveConsole {...base} />);
    fireEvent.click(screen.getByRole('button', { name: /risk: moderate/i }));
    const dialog = screen.getByRole('dialog', { name: /acceptable risk/i });
    expect(dialog.className).toContain('bottom-full');
    // Anchored to a relative wrapper around the trigger, not the strip root.
    expect((dialog.parentElement as HTMLElement).className).toContain('relative');
  });

  it('closes the risk popover on outside pointerdown', () => {
    render(<DriveConsole {...base} />);
    fireEvent.click(screen.getByRole('button', { name: /risk: moderate/i }));
    expect(screen.getByRole('dialog', { name: /acceptable risk/i })).toBeTruthy();
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole('dialog', { name: /acceptable risk/i })).toBeNull();
  });

  it('keeps the risk popover open on pointerdown inside it', () => {
    render(<DriveConsole {...base} />);
    fireEvent.click(screen.getByRole('button', { name: /risk: moderate/i }));
    fireEvent.pointerDown(screen.getByRole('dialog', { name: /acceptable risk/i }));
    expect(screen.getByRole('dialog', { name: /acceptable risk/i })).toBeTruthy();
  });

  it('clutch detents are radios with aria-checked reflecting selection', () => {
    render(<DriveConsole {...base} control={{ clutch: 'genius', risk: 'moderate' }} />);
    const radios = screen.getAllByRole('radio');
    expect(radios).toHaveLength(4);
    const genius = screen.getByRole('radio', { name: /Genius/i });
    expect(genius.getAttribute('aria-checked')).toBe('true');
    const free = screen.getByRole('radio', { name: /Free/i });
    expect(free.getAttribute('aria-checked')).toBe('false');
  });

  it('emits clutch change', () => {
    const onControlChange = vi.fn();
    render(<DriveConsole {...base} onControlChange={onControlChange} />);
    fireEvent.click(screen.getByRole('radio', { name: /Genius/i }));
    expect(onControlChange).toHaveBeenCalledWith(expect.objectContaining({ clutch: 'genius' }));
  });

  it('shows risk label from control state', () => {
    render(<DriveConsole {...base} control={{ clutch: 'free', risk: 'high' }} />);
    expect(screen.getByText(/High/i)).toBeTruthy();
  });

  it('shows budget bar when budgetUsd > 0', () => {
    const { container } = render(<DriveConsole {...base} />);
    // The bar span exists
    const bar = container.querySelector('[data-testid="drive-console-budget-bar"]');
    expect(bar).toBeTruthy();
  });
});

describe('DriveConsole vocabulary (plan 3a)', () => {
  const base = {
    control: defaultControl(),
    onControlChange: vi.fn(),
    spentUsd: 0.42,
    budgetUsd: 1.0,
  };

  it('takes every mode name from MODE_NAMES (one source with the turn trace)', () => {
    expect(CLUTCH_DETENTS.map((d) => d.label)).toEqual(CLUTCH_DETENTS.map((d) => modeLabel(d.id)));
  });

  it('names every mode in full inside a "Mode" radiogroup', () => {
    render(<DriveConsole {...base} />);
    expect(screen.getAllByRole('radio').map((r) => r.textContent)).toEqual(['Free', 'Efficient', 'Balanced', 'Genius']);
    expect(screen.getByRole('radiogroup', { name: /^Mode/ })).toBeTruthy();
  });

  it("shows the hovered or focused mode's one-line hint, and hides it after", () => {
    render(<DriveConsole {...base} />);
    expect(screen.queryByTestId('drive-mode-hint')).toBeNull();
    const genius = screen.getByRole('radio', { name: 'Genius' });
    fireEvent.mouseEnter(genius);
    expect(screen.getByTestId('drive-mode-hint')).toHaveTextContent('Most intelligent solutions');
    fireEvent.mouseLeave(genius);
    expect(screen.queryByTestId('drive-mode-hint')).toBeNull();
    const efficient = screen.getByRole('radio', { name: 'Efficient' });
    fireEvent.focus(efficient);
    const hint = screen.getByTestId('drive-mode-hint');
    expect(hint).toHaveTextContent('Most out of the tokens you spend');
    expect(efficient).toHaveAttribute('aria-describedby', hint.id);
    fireEvent.blur(efficient);
    expect(screen.queryByTestId('drive-mode-hint')).toBeNull();
  });

  it('labels the risk trigger "Risk: LEVEL"', () => {
    render(<DriveConsole {...base} />);
    expect(screen.getByRole('button', { name: /risk: moderate/i })).toHaveTextContent('Risk: Moderate');
  });

  it('puts extra risk controls (Check replies) inside the Risk popover, not in the strip', () => {
    render(<DriveConsole {...base} riskExtra={<button type="button">Check replies: off</button>} />);
    expect(screen.queryByText('Check replies: off')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /risk: moderate/i }));
    const dialog = screen.getByRole('dialog', { name: /acceptable risk/i });
    expect(within(dialog).getByText('Check replies: off')).toBeTruthy();
  });

  it('Spend shows the cap only when it is positive', () => {
    const { rerender } = render(<DriveConsole {...base} spentUsd={12.34} budgetUsd={50} />);
    expect(screen.getByTestId('drive-console-spend')).toHaveTextContent('Spend$12.34 / $50.00');
    rerender(<DriveConsole {...base} spentUsd={12.34} budgetUsd={0} />);
    expect(screen.getByTestId('drive-console-spend').textContent).toBe('Spend$12.34');
  });
});

