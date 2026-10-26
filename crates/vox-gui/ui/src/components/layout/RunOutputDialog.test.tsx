import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { RunOutputDialog } from './RunOutputDialog';
import { MAX_DISPLAY_CHARS } from '../../lib/capabilityRun';

describe('RunOutputDialog bounded display (WR-09)', () => {
  it('renders short output in full with no truncation notice', () => {
    render(<RunOutputDialog run={{ title: 'vox x', out: { exit_code: 0, stdout: 'hello', stderr: '' } }} onClose={vi.fn()} />);
    expect(screen.getByRole('region', { name: 'stdout' }).textContent).toBe('hello');
    expect(screen.getByRole('region', { name: 'stderr' }).textContent).toBe('(empty)');
    expect(screen.queryByText(/truncated/)).toBeNull();
  });

  it('truncates a huge stream, says so, and copies the FULL text', () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const big = 'y'.repeat(MAX_DISPLAY_CHARS * 3);
    render(<RunOutputDialog run={{ title: 'vox x', out: { exit_code: 0, stdout: big, stderr: '' } }} onClose={vi.fn()} />);
    expect(screen.getByRole('region', { name: 'stdout' }).textContent).toHaveLength(MAX_DISPLAY_CHARS);
    expect(screen.getByText(/truncated: showing/).textContent).toContain((MAX_DISPLAY_CHARS * 3).toLocaleString());
    fireEvent.click(screen.getByRole('button', { name: 'Copy full stdout' }));
    expect(writeText).toHaveBeenCalledWith(big);
  });
});
