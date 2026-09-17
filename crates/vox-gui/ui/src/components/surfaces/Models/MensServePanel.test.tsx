// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn((cmd: string) => {
    if (cmd === 'mens_serve_status') {
      return Promise.resolve({ running: false, port: null, default_port: 11435 });
    }
    if (cmd === 'mens_serve_start') {
      return Promise.resolve({ running: true, port: 11435, default_port: 11435 });
    }
    if (cmd === 'mens_serve_stop') {
      return Promise.resolve({ running: false, port: null, default_port: 11435 });
    }
    return Promise.resolve(null);
  }),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { MensServePanel } from './MensServePanel';

const noop = () => {};

describe('MensServePanel', () => {
  beforeEach(() => {
    invokeMock.mockClear();
  });

  it('never invokes mens_serve_start on mount — only status is read', async () => {
    render(<MensServePanel pushToast={noop} />);

    // Let any mount-time effects settle.
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('mens_serve_status'));

    // The load-bearing negative assertion: rendering this panel must never,
    // by itself, launch a server — unlike `CommandCardsView`'s cards, which
    // DO run on mount. A `useEffect` that (re)implements CommandCardsView's
    // run-on-mount behavior for `mens_serve_start` would fail this.
    expect(invokeMock).not.toHaveBeenCalledWith('mens_serve_start', expect.anything());
  });

  it('invokes mens_serve_start only after the user clicks Start', async () => {
    render(<MensServePanel pushToast={noop} />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('mens_serve_status'));

    const modelInput = screen.getByLabelText(/model/i);
    fireEvent.change(modelInput, { target: { value: '/tmp/some-run-dir' } });
    fireEvent.click(screen.getByRole('button', { name: /start/i }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('mens_serve_start', {
        model: '/tmp/some-run-dir',
        port: 11435,
      })
    );
  });

  it('invokes mens_serve_stop when the user clicks Stop while running', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'mens_serve_status') {
        return Promise.resolve({ running: true, port: 11435, default_port: 11435 });
      }
      if (cmd === 'mens_serve_stop') {
        return Promise.resolve({ running: false, port: null, default_port: 11435 });
      }
      return Promise.resolve(null);
    });
    render(<MensServePanel pushToast={noop} />);
    await waitFor(() => expect(screen.getByRole('button', { name: /stop/i })).toBeEnabled());

    fireEvent.click(screen.getByRole('button', { name: /stop/i }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('mens_serve_stop'));
  });
});
