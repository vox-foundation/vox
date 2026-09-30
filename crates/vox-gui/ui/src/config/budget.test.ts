import { describe, it, expect } from 'vitest';
import { formatSpend } from './budget';

describe('formatSpend', () => {
  it('shows a positive cap', () => {
    expect(formatSpend(12.34, 50)).toBe('$12.34 / $50.00');
  });

  it('never renders a zero, negative or missing cap', () => {
    expect(formatSpend(12.34, 0)).toBe('$12.34');
    expect(formatSpend(12.34, -1)).toBe('$12.34');
    expect(formatSpend(12.34, null)).toBe('$12.34');
  });
});
