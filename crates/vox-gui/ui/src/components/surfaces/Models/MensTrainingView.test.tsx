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

  it('loads and renders the 5-spoke before/after scorecard when Load 5-Spoke Benchmark is clicked', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_active_model') return Promise.resolve('Qwen/Qwen3-8B');
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
          gate_receipt: null,
          collateral_damage: null,
          spoke_scorecard: {
            schema_version: 'mens_before_after_comparison_v1',
            total_cases: 25,
            before_overall_pass_at_1: 0.24,
            mid_overall_pass_at_1: 0.72,
            overall_pass_at_1: 1.0,
            overall_pass_at_k: 1.0,
            overall_delta_pass_at_1: 0.76,
            monotonic_improvement_verified: true,
            promotion_gate_passed: true,
            promoted_model_id: 'mens/bench25-cycle-2',
            promoted_adapter_uri: 'trusty://mens-adapter/abc',
            spokes: [
              {
                spoke: 'vox_lang',
                total_cases: 5,
                before_pass_at_1: 0.4,
                mid_pass_at_1: 0.8,
                pass_at_1: 1.0,
                pass_at_k: 1.0,
                delta_pass_at_1: 0.6,
                syntax_validity_rate: 1.0,
                semantic_validity_rate: 1.0,
                execution_success_rate: 1.0,
                curriculum_weight: 0.2,
              },
              {
                spoke: 'agent_harness',
                total_cases: 5,
                before_pass_at_1: 0.2,
                mid_pass_at_1: 0.6,
                pass_at_1: 1.0,
                pass_at_k: 1.0,
                delta_pass_at_1: 0.8,
                syntax_validity_rate: 1.0,
                semantic_validity_rate: 1.0,
                execution_success_rate: 1.0,
                curriculum_weight: 0.2,
              },
            ],
          },
          organic_scorecard: {
            schema_version: 'mens_organic_search_spoke_scorecard_v1',
            total_cases: 24,
            overall_search_recall_at_5: 1.0,
            overall_mrr: 1.0,
            overall_unbiased_search_recall_at_5: 1.0,
            overall_unbiased_mrr: 0.98,
            overall_grounding_ratio: 1.0,
            overall_distractor_resilience_rate: 1.0,
            overall_compilation_pass_rate: 1.0,
            overall_end_to_end_pass_at_1: 1.0,
            spokes: [
              {
                spoke: 'vox_lang',
                total_cases: 3,
                search_recall_at_5: 1.0,
                mean_reciprocal_rank: 1.0,
                unbiased_search_recall_at_5: 1.0,
                unbiased_mrr: 1.0,
                mean_grounding_ratio: 1.0,
                distractor_resilience_rate: 1.0,
                syntax_validity_rate: 1.0,
                semantic_validity_rate: 1.0,
                compilation_pass_rate: 1.0,
                end_to_end_pass_at_1: 1.0,
              },
              {
                spoke: 'speech_to_code',
                total_cases: 3,
                search_recall_at_5: 1.0,
                mean_reciprocal_rank: 1.0,
                unbiased_search_recall_at_5: 1.0,
                unbiased_mrr: 0.95,
                mean_grounding_ratio: 1.0,
                distractor_resilience_rate: 1.0,
                syntax_validity_rate: 1.0,
                semantic_validity_rate: 1.0,
                compilation_pass_rate: 1.0,
                end_to_end_pass_at_1: 1.0,
              },
            ],
          },
        });
      }
      return Promise.resolve(null);
    });

    render(<MensTrainingView pushToast={noop} />);
    const btn = await screen.findByRole('button', { name: /load 5-spoke benchmark/i });
    fireEvent.click(btn);

    const card = await screen.findByTestId('mens-spoke-scorecard');
    expect(card.textContent).toMatch(/24% → 72% → 100% \(\+76%\)/);
    expect(card.textContent).toMatch(/vox_lang/);
    expect(card.textContent).toMatch(/agent_harness/);

    const organicCard = await screen.findByTestId('mens-organic-scorecard');
    expect(organicCard.textContent).toMatch(/Organic Search → Compilation/i);
    expect(organicCard.textContent).toMatch(/Recall@5 100% \(Unbiased 100%\)/i);
    expect(organicCard.textContent).toMatch(/Distractor 100%/i);
    expect(organicCard.textContent).toMatch(/24 organic cases/i);
    expect(organicCard.textContent).toMatch(/speech_to_code/i);
  });

  it('surfaces missing Clavis HF_TOKEN banner with direct Settings deep-link and allows hiding contribution bar', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_active_model') return Promise.resolve('Qwen/Qwen3-8B');
      if (cmd === 'list_secret_status') {
        return Promise.resolve([{ canonicalEnv: 'HF_TOKEN', isPresent: false }]);
      }
      if (cmd === 'execute_command') {
        return Promise.resolve({ exit_code: 0, stdout: '', stderr: '' });
      }
      if (cmd === 'mens_serve_status') {
        return Promise.resolve({ running: false, port: null, default_port: 11435 });
      }
      if (cmd === 'inference_provider_status') return Promise.resolve([]);
      return Promise.resolve(null);
    });

    render(<MensTrainingView pushToast={noop} />);
    const banner = await screen.findByTestId('mens-hf-token-missing-banner');
    expect(banner.textContent).toMatch(/HF_TOKEN/i);

    const jumpBtn = await screen.findByTestId('mens-jump-hf-token-settings-btn');
    fireEvent.click(jumpBtn);
    expect(window.location.hash).toBe('#view=settings');

    const hideBtn = await screen.findByTestId('mens-toggle-hub-contrib-btn');
    fireEvent.click(hideBtn);
    expect(screen.queryByTestId('mens-hub-contribution-bar')).toBeNull();
  });

  it('allows defining and adding an additional custom spoke column to the ladder matrix', async () => {
    render(<MensTrainingView pushToast={noop} />);
    const openBtn = await screen.findByTestId('mens-add-spoke-btn');
    fireEvent.click(openBtn);

    const idInput = await screen.findByLabelText(/New Spoke ID/i);
    fireEvent.change(idInput, { target: { value: 'graphql_api' } });

    const confirmBtn = await screen.findByTestId('mens-confirm-add-spoke-btn');
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(screen.getAllByText(/graphql_api/i).length).toBeGreaterThan(0);
    });
    expect(invokeMock).toHaveBeenCalledWith(
      'mens_save_custom_spoke',
      expect.objectContaining({
        runDir: 'workspace',
        spoke: expect.objectContaining({ id: 'graphql_api' }),
      }),
    );
  });

  it('auto-syncs gated train to HF Hub when Auto-Sync Gated is ON and HF_TOKEN is present', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_active_model') return Promise.resolve('Qwen/Qwen3-8B');
      if (cmd === 'list_secret_status') {
        return Promise.resolve([{ canonicalEnv: 'HF_TOKEN', isPresent: true }]);
      }
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
            substantive_gate_count: 3,
            failed_gates: [],
            gates: [],
          },
          collateral_damage: { status: 'pass', failed_on: null },
        });
      }
      return Promise.resolve(null);
    });

    render(<MensTrainingView pushToast={noop} />);
    await loadRunDir('/tmp/gated-run');

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('execute_command', {
        path: ['mens', 'hub', 'upload'],
        args: {
          __argv: ['--repo', 'vox/mens-qwen3-adapters', '--model-dir', '/tmp/gated-run'],
        },
      }),
    );
  });

  it('wires Inspector hardware presets and selected-cell quick actions (Train Cell, Eval Gate, Serve, Rollback) to real backend commands', async () => {
    render(<MensTrainingView pushToast={noop} />);
    const presetCluster = await screen.findByTestId('mens-hw-preset-cluster');
    const btn24g = presetCluster.querySelector('button:nth-child(2)') as HTMLButtonElement;
    fireEvent.click(btn24g);

    expect(invokeMock).toHaveBeenCalledWith(
      'mens_save_crucible_config',
      expect.objectContaining({
        runDir: 'workspace',
        config: expect.objectContaining({ lora_rank: 32, lora_alpha: 64, seq_len: 2048 }),
      }),
    );

    const trainBtn = await screen.findByTestId('mens-cell-train-now-btn');
    fireEvent.click(trainBtn);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('execute_command', {
        path: ['mens', 'train'],
        args: {
          __argv: [
            '--model',
            'Qwen/Qwen3-8B',
            '--domain',
            'vox_lang',
            '--rank',
            '32',
            '--alpha',
            '64',
            '--seq-len',
            '2048',
          ],
        },
      }),
    );

    const evalBtn = await screen.findByTestId('mens-cell-eval-gate-btn');
    fireEvent.click(evalBtn);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('execute_command', {
        path: ['mens', 'eval-local'],
        args: {
          __argv: [
            '--model',
            'mens/runs/8B_vox_lang',
            '--output',
            'mens/runs/8B_vox_lang/eval_local_report.json',
          ],
        },
      }),
    );

    const rollbackBtn = await screen.findByTestId('mens-cell-rollback-btn');
    fireEvent.click(rollbackBtn);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('execute_command', {
        path: ['mens', 'hub', 'download'],
        args: {
          __argv: [
            '--repo',
            'Qwen/Qwen3-8B',
            '--output-dir',
            'mens/runs/rollback_8B_vox_lang',
          ],
        },
      }),
    );
  });
});
