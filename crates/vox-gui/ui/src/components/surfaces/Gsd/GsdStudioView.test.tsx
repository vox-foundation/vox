import React from 'react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { GsdStudioView } from './GsdStudioView';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('GsdStudioView', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'gsd_get_status') {
        return {
          health: { healthy: true },
          snapshot: {
            active_milestone: {
              milestone_id: 'v7.0',
              title: 'GSD Deep Suite Integration',
              status: 'active',
              start_phase: 407,
              end_phase: 412,
            },
            phases: [
              {
                milestone_id: 'v7.0',
                phase_number: 407,
                title: 'GSD Base Skills',
                status: 'completed',
                wave_count: 2,
              },
              {
                milestone_id: 'v7.0',
                phase_number: 411,
                title: 'GSD Axis GUI',
                status: 'planned',
                wave_count: 1,
                plan_session_id: 'gsd-411',
              },
            ],
            requirements: [],
            documents: [
              {
                doc_id: 'v7.0:.planning/STATE.md',
                milestone_id: 'v7.0',
                kind: 'state',
                rel_path: '.planning/STATE.md',
                cas_hash: 'deadbeef12345678',
                byte_len: 42,
                updated_at_ms: 1700000000000,
              },
            ],
            next_recommended_skill: '/gsd-execute-phase 411',
          },
        };
      }
      if (cmd === 'gsd_execute_phase') {
        return { milestone_id: 'v7.0', phase: 411, nodes: [] };
      }
      if (cmd === 'gsd_get_document') {
        return {
          document: { rel_path: '.planning/STATE.md' },
          content: '# State\nCurrent Phase: 411',
        };
      }
      return {};
    });
  });

  it('renders milestone v7.0, executes phase waves, and previews CAS docs', async () => {
    const pushToast = vi.fn();
    render(<GsdStudioView pushToast={pushToast} />);

    expect(await screen.findByText(/v7\.0 · GSD Deep Suite Integration/)).toBeTruthy();
    expect(screen.getByTestId('gsd-phase-card-407')).toBeTruthy();
    expect(screen.getByTestId('gsd-phase-card-411')).toBeTruthy();

    fireEvent.click(screen.getAllByRole('button', { name: 'Execute Phase' })[1]!);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'gsd_execute_phase',
        expect.objectContaining({ phase: 411 }),
      );
    });

    fireEvent.click(screen.getByTestId('gsd-doc-item-.planning/STATE.md'));
    expect(await screen.findByTestId('gsd-doc-preview')).toHaveTextContent('Current Phase: 411');
  });
});
