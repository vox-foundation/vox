import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { VerificationFunnelFooter } from './VerificationFunnelFooter';
import { deriveRiverSnapshot } from './riverMath';

describe('VerificationFunnelFooter', () => {
  beforeEach(() => cleanup());

  it('renders the 4-stage Verification Retention Funnel, Failure Attribution Ledger, and Ludus Crystal yield', () => {
    const snap = deriveRiverSnapshot({
      tasks: [{ id: '1', title: 'Task 1', status: 'completed' }],
      messages: [],
      clutch: 'efficiency',
      risk: 'low',
    });

    render(<VerificationFunnelFooter snapshot={snap} gamifyEnabled />);

    expect(screen.getByText(/VERIFICATION RETENTION FUNNEL/i)).toBeDefined();
    expect(screen.getByText(/CANDIDATE TURNS/i)).toBeDefined();
    expect(screen.getByText(/ADVERSARIAL PASS/i)).toBeDefined();
    expect(screen.getByText(/FAILURE \/ DOUBT ATTRIBUTION/i)).toBeDefined();
    expect(screen.getByText(/\+\d+ ◆/)).toBeDefined();
  });
});
