// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/react';
import { within } from '@testing-library/react';
import { RiskPopover } from './RiskPopover';

describe('RiskPopover', () => {
  it('renders all three posture buttons when open', () => {
    render(<RiskPopover risk="moderate" onChange={() => {}} open onClose={() => {}} />);
    expect(screen.getByRole('button', { name: /high risk/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /moderate risk/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /low risk/i })).toBeTruthy();
  });

  it('emits the chosen posture', () => {
    const onChange = vi.fn();
    render(<RiskPopover risk="moderate" onChange={onChange} open onClose={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: /low risk/i }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ risk: 'low' }));
  });

  it('renders nothing when closed', () => {
    const { container } = render(
      <RiskPopover risk="moderate" onChange={() => {}} open={false} onClose={() => {}} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('anchors upward from its trigger (bottom-full inset, not static flow)', () => {
    render(<RiskPopover risk="moderate" onChange={() => {}} open onClose={() => {}} />);
    const cls = screen.getByRole('dialog').className;
    expect(cls).toContain('bottom-full');
    expect(cls).toContain('left-0');
    expect(cls).toContain('z-50');
  });

  it('calls onClose on Escape key', () => {
    const onClose = vi.fn();
    render(<RiskPopover risk="moderate" onChange={() => {}} open onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('focuses the selected posture button on open', () => {
    render(<RiskPopover risk="low" onChange={() => {}} open onClose={() => {}} />);
    expect(document.activeElement).toBe(screen.getByRole('button', { name: /low risk/i }));
  });
});

describe('RiskPopover extra controls', () => {
  it('renders children below the postures', () => {
    render(
      <RiskPopover risk="moderate" onChange={() => {}} open onClose={() => {}}>
        <button type="button">Check replies: off</button>
      </RiskPopover>,
    );
    expect(within(screen.getByRole('dialog')).getByRole('button', { name: 'Check replies: off' })).toBeTruthy();
  });

  it('uses the canonical "check replies", never the retired "grounding"', () => {
    for (const risk of ['high', 'moderate', 'low'] as const) {
      const { unmount } = render(<RiskPopover risk={risk} onChange={() => {}} open onClose={() => {}} />);
      expect(screen.getByRole('dialog').textContent).not.toMatch(/grounding/i);
      unmount();
    }
    render(<RiskPopover risk="moderate" onChange={() => {}} open onClose={() => {}} />);
    expect(screen.getByRole('dialog')).toHaveTextContent(/check replies/i);
  });
});

