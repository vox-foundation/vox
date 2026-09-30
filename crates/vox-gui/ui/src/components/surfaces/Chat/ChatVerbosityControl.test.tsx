// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ChatVerbosityControl } from './ChatVerbosityControl';

describe('ChatVerbosityControl', () => {
  it('offers the three levels and marks the current one', () => {
    render(<ChatVerbosityControl value="normal" onChange={() => {}} />);
    expect(screen.getAllByRole('radio').map((r) => r.textContent)).toEqual(['Quiet', 'Normal', 'Verbose']);
    expect(screen.getByRole('radio', { name: 'Normal' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'Verbose' })).toHaveAttribute('aria-checked', 'false');
  });

  it('reports the chosen level', () => {
    const onChange = vi.fn();
    render(<ChatVerbosityControl value="normal" onChange={onChange} />);
    fireEvent.click(screen.getByRole('radio', { name: 'Quiet' }));
    expect(onChange).toHaveBeenCalledWith('quiet');
  });

  it('checks nothing for a stale stored value', () => {
    render(<ChatVerbosityControl value={'loud' as 'normal'} onChange={() => {}} />);
    expect(screen.getAllByRole('radio').filter((r) => r.getAttribute('aria-checked') === 'true')).toHaveLength(0);
  });
});
