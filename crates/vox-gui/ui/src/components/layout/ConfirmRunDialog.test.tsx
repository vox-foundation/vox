// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { ConfirmRunDialog } from './ConfirmRunDialog';

function setup(over: Partial<React.ComponentProps<typeof ConfirmRunDialog>> = {}) {
  const onCancel = vi.fn();
  const onConfirm = vi.fn();
  render(
    <ConfirmRunDialog
      open
      commandText="vox plugin remove"
      rowId="cli.plugin.remove"
      onCancel={onCancel}
      onConfirm={onConfirm}
      {...over}
    />,
  );
  return { onCancel, onConfirm };
}

describe('ConfirmRunDialog (D-03)', () => {
  it('names the command and the classified row', async () => {
    setup();
    const dlg = await screen.findByRole('dialog', { name: 'Run a destructive command?' });
    expect(dlg.textContent).toContain('vox plugin remove');
    expect(dlg.textContent).toContain('cli.plugin.remove');
  });

  it('focuses Cancel first, so a stray Enter cannot confirm', async () => {
    setup();
    const cancel = await screen.findByRole('button', { name: 'Cancel' });
    await waitFor(() => expect(document.activeElement).toBe(cancel));
  });

  it('Cancel and Escape call onCancel and never onConfirm', async () => {
    const a = setup();
    await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(a.onCancel).toHaveBeenCalledTimes(1);
    expect(a.onConfirm).not.toHaveBeenCalled();
    await userEvent.keyboard('{Escape}');
    expect(a.onCancel).toHaveBeenCalledTimes(2);
    expect(a.onConfirm).not.toHaveBeenCalled();
  });

  it('only Run anyway calls onConfirm', async () => {
    const { onConfirm, onCancel } = setup();
    await userEvent.click(await screen.findByRole('button', { name: 'Run anyway' }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });
});
