// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TaskDiffDialog } from './TaskDiffDialog';

const DIFF = 'diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-old\n+new\n';

describe('TaskDiffDialog', () => {
  it('shows the pending diff verbatim in a named dialog', () => {
    render(<TaskDiffDialog open loading={false} text={DIFF} onClose={vi.fn()} />);
    expect(screen.getByRole('dialog', { name: 'Pending diff' })).toBeInTheDocument();
    expect(screen.getByTestId('task-diff-text').textContent).toBe(DIFF);
  });

  it('says it is loading while the diff is fetched, without an empty-diff claim', () => {
    render(<TaskDiffDialog open loading text="" onClose={vi.fn()} />);
    expect(screen.getByText('Loading the pending diff…')).toBeInTheDocument();
    expect(screen.queryByText('No pending changes.')).toBeNull();
  });

  it('says so when there is nothing pending', () => {
    render(<TaskDiffDialog open loading={false} text="   " onClose={vi.fn()} />);
    expect(screen.getByText('No pending changes.')).toBeInTheDocument();
    expect(screen.queryByTestId('task-diff-text')).toBeNull();
  });

  it('closes on Escape', () => {
    const onClose = vi.fn();
    render(<TaskDiffDialog open loading={false} text={DIFF} onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('renders nothing when closed', () => {
    render(<TaskDiffDialog open={false} loading={false} text={DIFF} onClose={vi.fn()} />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
