// @vitest-environment jsdom
import { render, screen, act } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';

vi.mock('../../../transport', () => ({
  activityQuery: vi.fn().mockResolvedValue([]),
  listenActivityAppended: vi.fn().mockResolvedValue(() => {}),
  listenAgentEvents: vi.fn().mockResolvedValue(() => {}),
}));

import { ActivitySurface } from './ActivitySurface';

describe('ActivitySurface filter selects are named (axe select-name)', () => {
  it('names both filter selects distinctly, matching their visible labels', async () => {
    render(<ActivitySurface pushToast={vi.fn()} />);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByRole('combobox', { name: 'Agent' })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Event Type' })).toBeInTheDocument();
    expect(screen.getAllByRole('combobox')).toHaveLength(2);
  });
});
