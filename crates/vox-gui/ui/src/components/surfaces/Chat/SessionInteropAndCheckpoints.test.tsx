import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ChatExecutionRail } from './ChatExecutionRail';

vi.mock('../../../hooks/useLanguage', () => ({
  useLabel: (key: string) => key,
}));

vi.mock('../../../hooks/useMetricSeries', () => ({
  useMetricSeries: () => [],
}));

vi.mock('../../../transport', () => ({
  getContextBudget: vi.fn(async () => ({
    used_tokens: 1200,
    max_context_tokens: 128000,
    threshold_tokens: 96000,
    strategy: 'balanced',
  })),
  setContextBudgetStrategy: vi.fn(async () => undefined),
  clearSessionToolResults: vi.fn(async () => undefined),
  getSessionContextPins: vi.fn(async () => []),
  setSessionContextPins: vi.fn(async () => []),
  getSessionChatPosture: vi.fn(async () => null),
  hopperSubmit: vi.fn(async () => ({ ok: true })),
  hopperRename: vi.fn(async () => ({ ok: true })),
  hopperCancel: vi.fn(async () => ({ ok: true })),
  hopperMarkDone: vi.fn(async () => ({ ok: true })),
  hopperReprioritize: vi.fn(async () => ({ ok: true })),
  doubtOrchestratorTask: vi.fn(async () => ({ ok: true })),
  forceOrchestratorVerify: vi.fn(async () => ({ ok: true })),
  getActiveTransportRouteStatus: vi.fn(() => ({
    activeTier: 'local_ipc',
    endpointUrl: 'ipc://vox-tauri',
    latencyMs: 1,
    active: true,
  })),
  vcsCheckpointList: vi.fn(async () => [
    {
      thread_id: 'sess-wave3',
      turn_count: 1,
      ref_name: 'refs/vox/checkpoints/sess-wave3/turn/1',
      tree_oid: 'a'.repeat(40),
      commit_oid: 'b'.repeat(40),
      created_at_ms: 1700000000000,
    },
  ]),
  vcsCheckpointCapture: vi.fn(async () => ({
    thread_id: 'sess-wave3',
    turn_count: 2,
    ref_name: 'refs/vox/checkpoints/sess-wave3/turn/2',
    tree_oid: 'c'.repeat(40),
    commit_oid: 'd'.repeat(40),
    created_at_ms: 1700000005000,
  })),
  vcsCheckpointRestore: vi.fn(async () => ({
    restored: true,
    target_turn: 1,
    ref_name: 'refs/vox/checkpoints/sess-wave3/turn/1',
  })),
  sessionImportExternal: vi.fn(async () => ({
    source_harness: 'claude',
    external_session_id: 'claude-session-1',
    title: 'Claude Code Refactor Session',
    messages: [
      { role: 'user', content: 'Hello from Claude' },
      { role: 'assistant', content: 'Imported into Vox' },
    ],
  })),
  sessionExportBundle: vi.fn(async () => ({
    schema_version: 'vox.session.export.v1',
    session_id: 'sess-wave3',
    title: 'Session sess-wave3',
    exported_at_ms: 1700000010000,
    messages: [{ role: 'user', content: 'Exported Vox session turn' }],
  })),
}));

describe('Phase 257: Git Checkpoints, Session Interop Modal & Transport Route Tier Badge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders Git Checkpoints timeline, captures a new checkpoint, and restores an existing turn ref', async () => {
    render(<ChatExecutionRail tasks={[]} sessionId="sess-wave3" turnsCompleted={1} />);

    await waitFor(() => {
      expect(screen.getByTestId('chat-git-checkpoints-panel')).toBeTruthy();
      expect(screen.getByTestId('execution-rail-route-tier').textContent).toContain('local_ipc');
    });

    const restoreBtn = await screen.findByTestId('chat-checkpoint-restore-1');
    fireEvent.click(restoreBtn);

    await waitFor(() => {
      const banner = screen.getByTestId('chat-checkpoint-status-banner');
      expect(banner.textContent).toContain('Restored refs/vox/checkpoints/sess-wave3/turn/1');
    });

    fireEvent.click(screen.getByTestId('chat-checkpoint-capture-btn'));
    await waitFor(() => {
      const banner = screen.getByTestId('chat-checkpoint-status-banner');
      expect(banner.textContent).toContain('Captured refs/vox/checkpoints/sess-wave3/turn/2');
    });
  });

  it('opens the SessionInteropModal and imports an external CLI transcript + exports a portable JSON bundle', async () => {
    render(<ChatExecutionRail tasks={[]} sessionId="sess-wave3" turnsCompleted={1} />);

    const interopBtn = await screen.findByTestId('execution-rail-session-interop-btn');
    fireEvent.click(interopBtn);

    const modal = await screen.findByTestId('session-interop-modal');
    expect(modal).toBeTruthy();

    fireEvent.change(screen.getByTestId('session-interop-jsonl-input'), {
      target: { value: '{"role":"user","content":"Hello from Claude"}' },
    });
    fireEvent.click(screen.getByTestId('session-interop-import-btn'));

    await waitFor(() => {
      const importResult = screen.getByTestId('session-interop-import-result');
      expect(importResult.textContent).toContain('Imported 2 turns from claude');
    });

    fireEvent.click(screen.getByTestId('session-interop-export-btn'));
    await waitFor(() => {
      const exportResult = screen.getByTestId('session-interop-export-result');
      expect(exportResult.textContent).toContain('Exported bundle vvox.session.export.v1');
    });
  });
});
