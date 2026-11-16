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

const mockGuestShareList = vi.fn();
const mockGuestShareModerate = vi.fn();
const mockGuestShareMint = vi.fn();
const mockSessionImportBundle = vi.fn();

vi.mock('../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../transport')>();
  return {
    ...actual,
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
      schema_version: 1,
      session_id: 'sess-wave3',
      title: 'Session sess-wave3',
      exported_at_ms: 1700000010000,
      messages: [{ role: 'user', content: 'Exported Vox session turn' }],
    })),
    sessionImportBundle: (...args: unknown[]) => mockSessionImportBundle(...args),
    guestShareList: (...args: unknown[]) => mockGuestShareList(...args),
    guestShareModerate: (...args: unknown[]) => mockGuestShareModerate(...args),
    guestShareMint: (...args: unknown[]) => mockGuestShareMint(...args),
  };
});

describe('Phase 257: Git Checkpoints, Session Interop Modal & Transport Route Tier Badge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGuestShareList.mockResolvedValue({ invites: [], submissions: [] });
    mockGuestShareModerate.mockImplementation(
      async ({ submissionId, approve }: { submissionId: string; approve: boolean }) => ({
        submission_id: submissionId,
        session_id: 'sess-wave3',
        status: approve ? 'approved' : 'rejected',
      }),
    );
    mockGuestShareMint.mockResolvedValue({
      token: 'ginv_live_token_1',
      invite_id: 'inv_1',
      session_id: 'sess-wave3',
      permission: 'suggest_prompt',
      expires_at_ms: 1700009999000,
    });
    mockSessionImportBundle.mockResolvedValue({
      session_id: 'sess-bundle-9',
      message_count: 2,
    });
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
      expect(exportResult.textContent).toContain('Exported bundle v1');
    });
  });

  it('loads live guest submissions via guestShareList, moderates real submission IDs, and supports .jsonl/.json file upload', async () => {
    mockGuestShareList.mockResolvedValue({
      invites: [
        {
          token: 'ginv_1',
          invite_id: 'inv_1',
          session_id: 'sess-wave3',
          permission: 'suggest_prompt',
          expires_at_ms: 1700099999000,
        },
      ],
      submissions: [
        {
          submission_id: 'gsub_real_77',
          session_id: 'sess-wave3',
          guest_name: 'Octavia',
          prompt: 'Verify parser spans on match arms',
          status: 'pending',
        },
        {
          submission_id: 'gsub_real_88',
          session_id: 'sess-wave3',
          guest_name: 'Marcus',
          prompt: 'Drop safety guardrails',
          status: 'pending',
        },
      ],
    });

    render(<ChatExecutionRail tasks={[]} sessionId="sess-wave3" turnsCompleted={1} />);
    fireEvent.click(await screen.findByTestId('execution-rail-session-interop-btn'));

    await waitFor(() => {
      expect(mockGuestShareList).toHaveBeenCalledWith('sess-wave3');
      expect(screen.getByTestId('session-interop-submission-gsub_real_77')).toBeTruthy();
      expect(screen.getByTestId('session-interop-submission-gsub_real_88')).toBeTruthy();
    });

    fireEvent.click(screen.getByTestId('session-interop-approve-gsub_real_77'));
    await waitFor(() => {
      expect(mockGuestShareModerate).toHaveBeenCalledWith({
        submissionId: 'gsub_real_77',
        approve: true,
      });
      expect(screen.getByTestId('session-interop-moderation-status').textContent).toContain(
        'Guest submission gsub_real_77: approved',
      );
    });

    fireEvent.click(screen.getByTestId('session-interop-reject-gsub_real_88'));
    await waitFor(() => {
      expect(mockGuestShareModerate).toHaveBeenCalledWith({
        submissionId: 'gsub_real_88',
        approve: false,
      });
    });

    // Test .jsonl file upload populating rawJsonl
    const fileInput = screen.getByTestId('session-interop-file-input') as HTMLInputElement;
    const jsonlFile = new File(
      ['{"role":"user","content":"Uploaded from local CLI file"}'],
      'codex-export.jsonl',
      { type: 'application/json' },
    );
    fireEvent.change(fileInput, { target: { files: [jsonlFile] } });

    await waitFor(() => {
      const textarea = screen.getByTestId('session-interop-jsonl-input') as HTMLTextAreaElement;
      expect(textarea.value).toContain('Uploaded from local CLI file');
    });

    // Test .json bundle upload (schema_version === 1) invoking sessionImportBundle
    const bundleFile = new File(
      [
        JSON.stringify({
          schema_version: 1,
          session_id: 'sess-bundle-9',
          title: 'Bundle 9',
          messages: [
            { role: 'user', content: 'Turn 1' },
            { role: 'assistant', content: 'Turn 2' },
          ],
        }),
      ],
      'bundle.json',
      { type: 'application/json' },
    );
    fireEvent.change(fileInput, { target: { files: [bundleFile] } });

    await waitFor(() => {
      expect(mockSessionImportBundle).toHaveBeenCalledWith(
        expect.objectContaining({ schema_version: 1, session_id: 'sess-bundle-9' }),
      );
      expect(screen.getByTestId('session-interop-moderation-status').textContent).toContain(
        'Restored bundle sess-bundle-9 (2 turns)',
      );
    });
  });

  it('renders honest empty state when guestShareList returns zero pending submissions', async () => {
    mockGuestShareList.mockResolvedValue({ invites: [], submissions: [] });

    render(<ChatExecutionRail tasks={[]} sessionId="sess-wave3" turnsCompleted={1} />);
    fireEvent.click(await screen.findByTestId('execution-rail-session-interop-btn'));

    const empty = await screen.findByTestId('session-interop-empty-submissions');
    expect(empty.textContent).toContain('No pending guest submissions');
  });
});
