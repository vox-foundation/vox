import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { BlockStreamView } from './BlockStreamView';
import type { TermBlockDto } from '../../../transport';

describe('RichBlockPreview & BlockCard rusage telemetry', () => {
  beforeEach(() => cleanup());

  it('renders std.frame table cards, std.plot svg charts, and child cpu/rss rusage badges', () => {
    const blocks: Array<TermBlockDto & { cpu_ms?: number; max_rss_kb?: number }> = [
      {
        id: 21,
        kind: 'VoxNative',
        status: 'Ok',
        origin: { kind: 'user' },
        input: 'std.frame.summary()',
        output: [
          {
            stream: 'Stdout',
            text: JSON.stringify({
              __vox_frame__: {
                title: 'Crate Metrics',
                columns: ['crate', 'ms'],
                rows: [['vox-terminal-core', 12], ['vox-term', 4]],
              },
            }),
            ts_ms: 100,
          },
        ],
        exit_code: 0,
        started_ms: 100,
        ended_ms: 112,
        cpu_ms: 9,
        max_rss_kb: 4096,
      },
      {
        id: 22,
        kind: 'VoxNative',
        status: 'Ok',
        origin: { kind: 'user' },
        input: 'std.plot.bars()',
        output: [
          {
            stream: 'Stdout',
            text: JSON.stringify({
              __vox_plot__: {
                title: 'Latency p50',
                points: [
                  { label: 'Ghostty', value: 3 },
                  { label: 'Vox', value: 4 },
                ],
              },
            }),
            ts_ms: 200,
          },
        ],
        exit_code: 0,
        started_ms: 200,
        ended_ms: 215,
      },
    ];

    render(
      <BlockStreamView
        blocks={blocks}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );

    expect(screen.getByTestId('term-std-frame-card')).toBeTruthy();
    expect(screen.getByText('Crate Metrics')).toBeTruthy();
    expect(screen.getByText('vox-terminal-core')).toBeTruthy();

    expect(screen.getByTestId('term-std-plot-card')).toBeTruthy();
    expect(screen.getByText('Latency p50')).toBeTruthy();

    const rusage = screen.getByTestId('term-block-rusage');
    expect(rusage.textContent).toBe('9ms cpu / 4.0MiB rss');
  });
});
