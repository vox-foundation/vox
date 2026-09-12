// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import React from 'react';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn((cmd: string) => {
    if (cmd === 'get_active_model') return Promise.resolve('Qwen/Qwen3-8B');
    if (cmd === 'execute_command') {
      return Promise.resolve({ exit_code: 0, stdout: '', stderr: '' });
    }
    return Promise.resolve(null);
  }),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { MensTrainingView } from './MensTrainingView';

const noop = () => {};

describe('MensTrainingView', () => {
  beforeEach(() => {
    invokeMock.mockClear();
  });

  it('asks the CLI for the fit of the model the user actually picked', async () => {
    render(<MensTrainingView pushToast={noop} />);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('execute_command', {
        path: ['mens', 'probe'],
        args: { __argv: ['--detailed', '--model', 'Qwen/Qwen3-8B'] },
      })
    );
  });

  it('falls back to a model-less probe when no active model is set', async () => {
    invokeMock.mockImplementationOnce((cmd: string) =>
      cmd === 'get_active_model' ? Promise.resolve(null) : Promise.resolve(null)
    );
    render(<MensTrainingView pushToast={noop} />);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('execute_command', {
        path: ['mens', 'probe'],
        args: { __argv: ['--detailed'] },
      })
    );
  });
});
