// @vitest-environment jsdom
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { EmpiricalVerificationBadge } from './EmpiricalVerificationBadge';

describe('EmpiricalVerificationBadge', () => {
  it('renders verified pill with stability score', () => {
    render(<EmpiricalVerificationBadge sessionId={42} stabilityScore={0.92} />);
    expect(screen.getByText(/Verified by Deep Research/i)).toBeDefined();
    expect(screen.getByText(/\(S = 0.92\)/)).toBeDefined();
  });

  it('opens drawer on click and closes on backdrop or close button', () => {
    render(<EmpiricalVerificationBadge sessionId={42} stabilityScore={0.92} />);
    const btn = screen.getByRole('button', { name: /Verified by Deep Research/i });
    fireEvent.click(btn);

    expect(screen.getByText(/Empirical Verification Ledger/i)).toBeDefined();
    expect(screen.getByText(/Research Session #42 · Stability S = 0.92/i)).toBeDefined();

    const closeBtn = screen.getByRole('button', { name: /^close$/i });
    fireEvent.click(closeBtn);
    expect(screen.queryByText(/Empirical Verification Ledger/i)).toBeNull();
  });

  it('triggers onNavigateToResearch when navigate button clicked', () => {
    const onNavigate = vi.fn();
    render(
      <EmpiricalVerificationBadge
        sessionId={42}
        stabilityScore={0.92}
        onNavigateToResearch={onNavigate}
      />
    );

    const pillBtn = screen.getByRole('button', { name: /Verified by Deep Research/i });
    fireEvent.click(pillBtn);

    const navBtn = screen.getByRole('button', { name: /Open Full Research Session #42/i });
    fireEvent.click(navBtn);

    expect(onNavigate).toHaveBeenCalledWith(42);
    expect(screen.queryByText(/Empirical Verification Ledger/i)).toBeNull();
  });

  it('enforces type="button" on all buttons', () => {
    render(
      <EmpiricalVerificationBadge
        sessionId={42}
        stabilityScore={0.92}
        onNavigateToResearch={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /Verified by Deep Research/i }));

    const buttons = screen.getAllByRole('button');
    for (const b of buttons) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('closes drawer on backdrop click', () => {
    render(<EmpiricalVerificationBadge sessionId={42} stabilityScore={0.92} />);
    fireEvent.click(screen.getByRole('button', { name: /Verified by Deep Research/i }));
    expect(screen.getByText(/Empirical Verification Ledger/i)).toBeDefined();

    const backdrop = screen.getByRole('button', { name: /Close backdrop/i });
    fireEvent.click(backdrop);
    expect(screen.queryByText(/Empirical Verification Ledger/i)).toBeNull();
  });

  it('closes drawer on Escape key press', () => {
    render(<EmpiricalVerificationBadge sessionId={42} stabilityScore={0.92} />);
    fireEvent.click(screen.getByRole('button', { name: /Verified by Deep Research/i }));
    expect(screen.getByText(/Empirical Verification Ledger/i)).toBeDefined();

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByText(/Empirical Verification Ledger/i)).toBeNull();
  });

  it('applies amber styling when stability score is below 0.85', () => {
    render(<EmpiricalVerificationBadge sessionId={12} stabilityScore={0.74} />);
    const pill = screen.getByRole('button', { name: /Verified by Deep Research/i });
    expect(pill.className).toContain('text-amber-300');
  });
});

