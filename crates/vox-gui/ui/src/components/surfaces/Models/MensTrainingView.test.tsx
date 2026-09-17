// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, waitFor, screen, fireEvent } from '@testing-library/react';
import React from 'react';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn((cmd: string) => {
    if (cmd === 'get_active_model') return Promise.resolve('Qwen/Qwen3-8B');
    if (cmd === 'execute_command') {
      return Promise.resolve({ exit_code: 0, stdout: '', stderr: '' });
    }
    if (cmd === 'mens_serve_status') {
      return Promise.resolve({ running: false, port: null, default_port: 11435 });
    }
    if (cmd === 'inference_provider_status') {
      return Promise.resolve([]);
    }
    return Promise.resolve(null);
  }),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { MensTrainingView } from './MensTrainingView';

const noop = () => {};

const loadRunDir = async (dir: string) => {
  const input = await screen.findByLabelText(/run directory/i);
  fireEvent.change(input, { target: { value: dir } });
  fireEvent.blur(input);
  await waitFor(() =>
    expect(invokeMock).toHaveBeenCalledWith('mens_run_reports', { runDir: dir })
  );
};

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

  it('renders a failed gate receipt and names the degraded gate', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_active_model') return Promise.resolve(null);
      if (cmd === 'execute_command') {
        return Promise.resolve({ exit_code: 0, stdout: '', stderr: '' });
      }
      if (cmd === 'mens_serve_status') {
        return Promise.resolve({ running: false, port: null, default_port: 11435 });
      }
      if (cmd === 'inference_provider_status') return Promise.resolve([]);
      if (cmd === 'mens_run_reports') {
        return Promise.resolve({
          eval_local: null,
          gate_receipt: {
            overall_passed: false,
            substantive_gate_count: 2,
            failed_gates: ['throughput'],
            gates: [
              { name: 'throughput', passed: false, message: '18 tok/s < 100 floor' },
              { name: 'pass_at_k', passed: true, message: '0.65 >= 0.60' },
            ],
          },
          collateral_damage: null,
        });
      }
      return Promise.resolve(null);
    });

    render(<MensTrainingView pushToast={noop} />);
    await loadRunDir('/tmp/some-run-dir');

    const status = await screen.findByTestId('gate-receipt-status');
    expect(status.textContent).toMatch(/failed/i);
    expect(status.textContent).toMatch(/throughput/);
  });

  it('does not render a plain green pass when the receipt has zero substantive gates', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_active_model') return Promise.resolve(null);
      if (cmd === 'execute_command') {
        return Promise.resolve({ exit_code: 0, stdout: '', stderr: '' });
      }
      if (cmd === 'mens_serve_status') {
        return Promise.resolve({ running: false, port: null, default_port: 11435 });
      }
      if (cmd === 'inference_provider_status') return Promise.resolve([]);
      if (cmd === 'mens_run_reports') {
        return Promise.resolve({
          eval_local: null,
          gate_receipt: {
            overall_passed: true,
            substantive_gate_count: 0,
            failed_gates: [],
            gates: [
              { name: 'rust_compile_rate', passed: true, message: 'not applicable (no rows)' },
              { name: 'pass_at_k', passed: true, message: 'baseline file missing (skipped)' },
            ],
          },
          collateral_damage: null,
        });
      }
      return Promise.resolve(null);
    });

    render(<MensTrainingView pushToast={noop} />);
    await loadRunDir('/tmp/some-run-dir');

    const status = await screen.findByTestId('gate-receipt-status');
    // The load-bearing negative assertion: a receipt with 0 substantive
    // gates must never carry the plain-pass visual (the emerald "Gate
    // passed" text/class), even though `overall_passed` is true.
    expect(status.className).not.toBe('text-emerald-400');
    expect(status.textContent).not.toBe('Gate passed (0 substantive gates)');
    expect(status.textContent).toMatch(/0 substantive/i);
  });

  it('does not render a plain green pass when the receipt has only 1 substantive gate', async () => {
    // The real backend (assert_serve_preconditions in dispatch.rs) refuses
    // to serve when substantive_gate_count < 2, so a receipt with exactly 1
    // substantive gate is already backend-rejected evidence. The GUI must
    // not show more confidence than the backend has.
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_active_model') return Promise.resolve(null);
      if (cmd === 'execute_command') {
        return Promise.resolve({ exit_code: 0, stdout: '', stderr: '' });
      }
      if (cmd === 'mens_serve_status') {
        return Promise.resolve({ running: false, port: null, default_port: 11435 });
      }
      if (cmd === 'inference_provider_status') return Promise.resolve([]);
      if (cmd === 'mens_run_reports') {
        return Promise.resolve({
          eval_local: null,
          gate_receipt: {
            overall_passed: true,
            substantive_gate_count: 1,
            failed_gates: [],
            gates: [
              { name: 'pass_at_k', passed: true, message: '0.65 >= 0.60' },
              { name: 'rust_compile_rate', passed: true, message: 'not applicable (no rows)' },
            ],
          },
          collateral_damage: null,
        });
      }
      return Promise.resolve(null);
    });

    render(<MensTrainingView pushToast={noop} />);
    await loadRunDir('/tmp/some-run-dir');

    const status = await screen.findByTestId('gate-receipt-status');
    // Same class-name-equality assertion style as the count===0 case above:
    // a substring match here would be vacuous.
    expect(status.className).not.toBe('text-emerald-400');
    expect(status.textContent).not.toBe('Gate passed (1 substantive gates)');
    expect(status.textContent).toMatch(/1 substantive/i);
  });
});
