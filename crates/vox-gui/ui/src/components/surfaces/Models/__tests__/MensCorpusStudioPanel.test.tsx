import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { MensCorpusStudioPanel } from '../MensCorpusStudioPanel';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('MensCorpusStudioPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: unknown) => {
      const a = (args ?? {}) as Record<string, unknown>;
      if (cmd === 'mens_corpus_studio_snapshot' || cmd === 'mens_corpus_rebuild_mix') {
        return {
          active_config_path: (a.configPath as string) ?? 'mens/config/mix.yaml',
          available_configs: [
            'mens/config/mix.yaml',
            'mens/config/mix-spoke-vox-lang.yaml',
          ],
          config_fingerprint: '9f84c2a1e70d3b14',
          mix_fresh: true,
          output_path: 'target/dogfood/train_mixed.jsonl',
          output_exists: true,
          output_records: 1286,
          supervised_ratio_pct: 74.2,
          system_prompt_tokens: 21,
          sources: [
            {
              path: 'mens/data/mix_sources/vox_reasoning_traces.jsonl',
              label: 'vox_reasoning_traces.jsonl',
              spoke: 'vox-lang',
              weight: 4.0,
              required: false,
              exists: true,
              record_count: 120,
              estimated_weighted_rows: 480,
              share_pct: 60.0,
              fingerprint: '120:84210:1794152000:a81c04f2',
              status_badge: 'AUTO-SYNTH · FRESH',
              auto_synthesizable: true,
              has_think_traces: true,
              ast_verified: true,
              min_val_jaccard_max: 0.25,
            },
            {
              path: 'mens/data/mix_sources/golden_examples.jsonl',
              label: 'golden_examples.jsonl',
              spoke: 'vox-lang',
              weight: 3.0,
              required: false,
              exists: true,
              record_count: 164,
              estimated_weighted_rows: 320,
              share_pct: 40.0,
              fingerprint: '164:91400:1794151000:7b29d1e0',
              status_badge: 'FRESH',
              auto_synthesizable: false,
              has_think_traces: true,
              ast_verified: true,
              min_val_jaccard_max: 0.25,
            },
          ],
          ladder_checkpoints: [
            {
              size: '0.6B',
              label: 'Qwen3-0.6B',
              status: 'GATED & SYNCED',
              vox_think_done: 128,
              vox_think_total: 128,
              vox_think_acc_pct: 59.38,
              vox_think_delta_pct: 2.34,
              gsm8k_think_done: 128,
              gsm8k_think_total: 128,
              gsm8k_think_acc_pct: 58.59,
              mcnemar_p_value: 0.289,
              gate_passed: true,
            },
          ],
          leakage_overlaps: 0,
        };
      }
      if (cmd === 'mens_corpus_read_jsonl') {
        return {
          source_path: (a.path as string) ?? 'mens/data/mix_sources/vox_reasoning_traces.jsonl',
          total_records: 120,
          total_matching: 1,
          offset: 0,
          limit: 30,
          records: [
            {
              line_number: 1,
              prompt: 'Trace clamp_score bounds in Vox.',
              response:
                '<think>\n1. Check lower bound.\n</think>\n```vox\nfn clamp_score(val: int) to int { ret val }\n```',
              think_block: '1. Check lower bound.',
              answer_block: '```vox\nfn clamp_score(val: int) to int { ret val }\n```',
              category: 'vox_reasoning',
              task_family: 'literal_debug',
              difficulty: 4,
              has_think: true,
              think_steps: 1,
              ast_valid: true,
              ast_diagnostic: null,
            },
          ],
        };
      }
      if (cmd === 'mens_corpus_save_jsonl_row') {
        return {
          saved_line: 1,
          source_path: 'mens/data/mix_sources/vox_reasoning_traces.jsonl',
          ast_valid: true,
          ast_diagnostic: null,
          mix_rebuilt: true,
          snapshot: null,
        };
      }
      return null;
    });
  });

  it('renders Romanesque three-arch studio and dispatches inline JSONL save + auto-sync', async () => {
    const pushToast = vi.fn();
    render(<MensCorpusStudioPanel pushToast={pushToast} />);

    await waitFor(() => {
      expect(screen.getByTestId('mens-corpus-studio-panel')).toBeTruthy();
      expect(screen.getByTestId('mens-arch-mix-sources')).toBeTruthy();
      expect(screen.getByTestId('mens-arch-jsonl-inspector')).toBeTruthy();
      expect(screen.getByTestId('mens-arch-ladder-checkpoints')).toBeTruthy();
    });

    expect(screen.getByTestId('mens-corpus-freshness-badge').textContent).toContain(
      'AUTO-MATERIALIZED · FRESH',
    );

    const promptInput = screen.getByTestId('mens-editor-prompt-input') as HTMLTextAreaElement;
    fireEvent.change(promptInput, {
      target: { value: 'Updated Vox reasoning prompt for clamp_score.' },
    });

    const saveBtn = screen.getByTestId('mens-save-row-sync-btn');
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        'mens_corpus_save_jsonl_row',
        expect.objectContaining({
          request: expect.objectContaining({
            source_path: 'mens/data/mix_sources/vox_reasoning_traces.jsonl',
            prompt: 'Updated Vox reasoning prompt for clamp_score.',
            auto_sync: true,
          }),
        }),
      );
      expect(pushToast).toHaveBeenCalled();
    });
  });
});
