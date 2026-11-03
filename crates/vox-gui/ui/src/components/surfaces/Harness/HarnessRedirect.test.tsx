// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, act } from '@testing-library/react';
import React from 'react';

import { HarnessRedirect } from './HarnessRedirect';
import { defaultControl, publishControlState } from '../../../lib/driveConsole';

describe('HarnessRedirect (upgraded to Axis Harness River)', () => {
  beforeEach(() => {
    cleanup();
    publishControlState(defaultControl());
  });

  it('renders the Axis Harness River HUD and retains composer/Loquela reference', () => {
    const { container } = render(<HarnessRedirect onFocusComposer={vi.fn()} />);
    const subtitle = screen.getByText(/Loquela composer/i);
    expect(subtitle.textContent?.toLowerCase()).toMatch(/composer|loquela/);
    expect(screen.getByTestId('harness-waveform-tier-strip')).toBeDefined();
    expect(screen.getByTestId('harness-verification-footer')).toBeDefined();
    expect(container.querySelector('canvas')).not.toBeNull();
  });

  it('calls onFocusComposer when Focus composer action button is clicked', () => {
    const onFocusComposer = vi.fn();
    render(<HarnessRedirect onFocusComposer={onFocusComposer} />);

    fireEvent.click(screen.getByRole('button', { name: 'Focus composer' }));

    expect(onFocusComposer).toHaveBeenCalledTimes(1);
  });

  it('collapses the 8-slice agent braid and failure ledger when condensed=true in narrow dock mode', () => {
    render(<HarnessRedirect condensed />);
    expect(screen.queryByTestId('harness-agent-braid')).toBeNull();
    expect(screen.getByTestId('harness-waveform-tier-strip')).toBeDefined();
    expect(screen.getByTestId('harness-verification-footer')).toBeDefined();
  });

  it('reacts live when Loquela DriveConsole publishes a clutch/risk change', () => {
    render(<HarnessRedirect />);
    expect(screen.getByText(/CLUTCH: EFFICIENCY/i)).toBeDefined();

    act(() => {
      publishControlState({ clutch: 'genius', risk: 'low' });
    });

    expect(screen.getByText(/CLUTCH: GENIUS/i)).toBeDefined();
    expect(screen.getByText(/SOCRATIC RED-TEAM \+ COLD TEST/i)).toBeDefined();
  });
});
