import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetBackendAvailabilityForTests } from '../../lib/backendGuard';
import { voxTransport } from '../../transport';
import {
  __resetWave3PreviewStateForTests,
  BROWSER_PREVIEW_REASON,
  batteryStatus,
  chatConsensusRun,
  compactChatUpTo,
  editChatMessageInPlace,
  getActiveTransportRouteStatus,
  guestShareList,
  guestShareMint,
  guestShareSubmit,
  regenerateChatTurn,
   resumeChatStream,
  revertWorkspaceFile,
  sessionImportBundle,
  sessionListExternal,
  stageWorkspaceFile,
  steerChatQueue,
  unwrapMcpData,
  vcsCheckpointCapture,
  vcsCheckpointList,
  vcsMusePoolAdopt,
} from '../t3Wave3Rpc';

describe('t3Wave3Rpc envelope unwrapping, live IPC honesty, and explicit browser preview', () => {
  beforeEach(() => {
    (globalThis as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    __resetWave3PreviewStateForTests();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    (globalThis as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    __resetWave3PreviewStateForTests();
  });

  it('recursively unwraps Tauri { is_error, result } and JSON string ToolResult envelopes', () => {
    const tauriEnvelope = {
      is_error: false,
      result: JSON.stringify({
        success: true,
        data: {
          session_id: 's-1',
          compact_message_index: 2,
          preserved_thought_signatures: ['sig-01'],
        },
      }),
    };
    const unwrapped = unwrapMcpData<{
      session_id: string;
      compact_message_index: number;
      preserved_thought_signatures: string[];
    }>(tauriEnvelope);
    expect(unwrapped).toEqual({
      session_id: 's-1',
      compact_message_index: 2,
      preserved_thought_signatures: ['sig-01'],
    });
  });

  it('recursively unwraps standard MCP { content: [{ type: "text", text }] } envelopes', () => {
    const mcpContentEnvelope = {
      content: [
        {
          type: 'text',
          text: JSON.stringify({
            success: true,
            data: { file_path: 'src/lib.rs', staged: true },
          }),
        },
      ],
    };
    const unwrapped = unwrapMcpData<{ file_path: string; staged: boolean }>(mcpContentEnvelope);
    expect(unwrapped).toEqual({ file_path: 'src/lib.rs', staged: true });
  });

  it('throws clean human-readable messages on is_error object envelopes and profile-denial envelopes', () => {
    expect(() =>
      unwrapMcpData({
        is_error: true,
        result: 'Permission denied',
      }),
    ).toThrow('Permission denied');

    expect(() =>
      unwrapMcpData({
        is_error: true,
        result: { success: false, error: 'write ttl: permission denied' },
      }),
    ).toThrow('write ttl: permission denied');

    expect(() =>
      unwrapMcpData({
        is_error: true,
        result: { error: { code: 'profile_denied', message: 'Tool blocked by active profile' } },
      }),
    ).toThrow('Tool blocked by active profile');

    expect(() =>
      unwrapMcpData({
        is_error: true,
        result: { unexpected: true },
      }),
    ).toThrow('MCP tool execution failed');

    expect(() =>
      unwrapMcpData({
        error: { code: 'profile_denied', message: 'Denied by daemon policy' },
      }),
    ).toThrow('Denied by daemon policy');

    expect(() =>
      unwrapMcpData(
        JSON.stringify({
          success: false,
          error: 'Invalid message_id',
        }),
      ),
    ).toThrow(/Invalid message_id/);
  });

  it('guards empty route candidates and rejects staged:false / reverted:false payloads', async () => {
    expect(getActiveTransportRouteStatus(1_000, [])).toEqual({
      activeTier: 'manual_fallback',
      activeUrl: '',
      schedule: [],
    });

    const spy = vi.spyOn(voxTransport, 'invokeMcpTool');
    spy.mockResolvedValueOnce({
      is_error: false,
      result: { success: true, data: { file_path: 'src/lib.rs', staged: false } },
    });
    await expect(stageWorkspaceFile('src/lib.rs')).rejects.toThrow(/malformed payload/);

    spy.mockResolvedValueOnce({
      is_error: false,
      result: { success: true, data: { file_path: 'src/lib.rs', reverted: false } },
    });
    await expect(revertWorkspaceFile('src/lib.rs')).rejects.toThrow(/malformed payload/);
  });

  it('rejects with sanitized errors when backendAvailable() is true and invokeMcpTool rejects or returns is_error', async () => {
    const spy = vi
      .spyOn(voxTransport, 'invokeMcpTool')
      .mockRejectedValue(new Error('failed __TAURI_INTERNALS__ leak'));

    const leakProbes = [
      () => guestShareMint({ sessionId: 'sess-1' }),
      () => vcsCheckpointCapture('thread-1', 2),
      () => batteryStatus('sess-1'),
      () => chatConsensusRun({ prompt: 'Verify IDs' }),
      () =>
        editChatMessageInPlace({
          sessionId: 'sess-1',
          messageId: 'm-1',
          newContent: 'Updated',
        }),
    ];

    for (const call of leakProbes) {
      let caught: Error | undefined;
      try {
        await call();
      } catch (err) {
        caught = err as Error;
      }
      expect(caught).toBeInstanceOf(Error);
      expect(caught?.message).not.toContain('__TAURI_INTERNALS__');
      expect(caught?.message).toBe('An unexpected error occurred.');
    }

    spy.mockResolvedValue({ is_error: true, result: 'daemon offline' });

    for (const call of leakProbes) {
      await expect(call()).rejects.toThrow(/daemon offline/);
    }
  });

  it('maps live Rust envelopes for guestShareList, guestShareMint, vcsMusePoolAdopt, and bare-array sessionListExternal', async () => {
    const spy = vi.spyOn(voxTransport, 'invokeMcpTool');

    spy.mockResolvedValueOnce({
      is_error: false,
      result: JSON.stringify({
        success: true,
        data: {
          invites: [
            {
              invite_id: 'inv-42',
              signature_hex: 'deadbeef',
              thread_id: 'sess-42',
              scope: 'READ_ONLY',
              expires_at_ms: 1790000000000,
              revoked: false,
            },
          ],
          submissions: [
            {
              submission_id: 'sub-9',
              session_id: 'sess-42',
              guest_name: 'Alice',
              prompt: 'Check edge case',
              status: 'pending',
              submitted_at_ms: 1789999990000,
            },
          ],
        },
      }),
    });
    const listRes = await guestShareList('sess-42');
    expect(listRes).toEqual({
      invites: [
        {
          token: 'inv-42:deadbeef',
          invite_id: 'inv-42',
          session_id: 'sess-42',
          permission: 'read_only',
          expires_at_ms: 1790000000000,
          revoked: false,
        },
      ],
      submissions: [
        {
          submission_id: 'sub-9',
          session_id: 'sess-42',
          guest_name: 'Alice',
          prompt: 'Check edge case',
          status: 'pending',
          submitted_at_ms: 1789999990000,
        },
      ],
    });

    spy.mockResolvedValueOnce({
      is_error: false,
      result: JSON.stringify({
        success: true,
        data: {
          invite_id: 'inv-77',
          signature_hex: 'cafebabe',
          thread_id: 'sess-77',
          scope: 'SUGGEST_PROMPT',
          expires_at_ms: 1791000000000,
        },
      }),
    });
    const minted = await guestShareMint({ sessionId: 'sess-77' });
    expect(minted).toEqual({
      token: 'inv-77:cafebabe',
      session_id: 'sess-77',
      permission: 'suggest_prompt',
      expires_at_ms: 1791000000000,
    });

    spy.mockResolvedValueOnce({
      is_error: false,
      result: JSON.stringify({
        success: true,
        data: {
          winner: 'variant-b',
        },
      }),
    });
    const adopted = await vcsMusePoolAdopt('run-9', 'variant-b');
    expect(adopted).toEqual({
      run_id: 'run-9',
      adopted_variant_id: 'variant-b',
      adopted_branch_name: 'vox/muse/run-9/variant-b',
      pruned_variant_ids: [],
    });

    spy.mockResolvedValueOnce({
      is_error: false,
      result: JSON.stringify({
        success: true,
        data: [
          {
            source_harness: 'claude',
            external_session_id: 'claude-ext-1',
            file_path: '~/.claude/projects/ext-1.jsonl',
            size_bytes: 4096,
          },
        ],
      }),
    });
    const externals = await sessionListExternal('claude');
    expect(externals).toEqual([
      {
        source_harness: 'claude',
        external_session_id: 'claude-ext-1',
        file_path: '~/.claude/projects/ext-1.jsonl',
        size_bytes: 4096,
      },
    ]);
  });

  it('returns explicit preview_reason payloads only when backendAvailable() is false', async () => {
    delete (globalThis as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetBackendAvailabilityForTests();

    const minted = await guestShareMint({ sessionId: 'sess-preview' });
    expect(minted.token).toBe('preview:sess-preview');
    expect(minted.token).not.toContain('_demo');
    expect(minted.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const sub = await guestShareSubmit({ token: minted.token, prompt: 'Hello' });
    expect(sub.submission_id).toBe('preview:gsub-1');
    expect(sub.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const bat = await batteryStatus('sess-preview');
    expect(bat.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const edited = await editChatMessageInPlace({
      sessionId: 'sess-wave4',
      messageId: 'asst-1',
      newContent: 'Updated in-place content',
    });
    expect(edited.session_id).toBe('sess-wave4');
    expect(edited.edited_message.content).toBe('Updated in-place content');
    expect(edited.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const regen = await regenerateChatTurn({
      sessionId: 'sess-wave4',
      fromMessageId: 'asst-1',
      promptOverride: 'Retry with stricter types',
    });
    expect(regen.replay_prompt).toBe('Retry with stricter types');
    expect(regen.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const compacted = await compactChatUpTo({
      sessionId: 'sess-wave4',
      messageId: 'asst-1',
      direction: 'up_to',
    });
    expect(compacted.session_id).toBe('sess-wave4');
    expect(compacted.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const resumed = await resumeChatStream({
      sessionId: 'sess-wave4',
      charOffset: 12,
    });
    expect(resumed.char_offset).toBe(12);
    expect(resumed.completed).toBe(true);
    expect(resumed.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const steered = await steerChatQueue({
      sessionId: 'sess-wave4',
      prompt: 'Focus on zero-copy slices',
      promoteToSteer: true,
    });
    expect(steered.promoted_to_steer).toBe(true);
    expect(steered.preview_reason).toBe(BROWSER_PREVIEW_REASON);

    const consensus = await chatConsensusRun({ prompt: 'Compare default legs' });
    expect(consensus.models).toEqual(['anthropic/claude-sonnet', 'openai/gpt', 'mens/qwen3-8b']);

    const badJsonBundle = await sessionImportBundle('{not-valid-json');
    expect(badJsonBundle.ok).toBe(false);
    expect(badJsonBundle.imported).toBe(false);
    expect(badJsonBundle.error).toMatch(/Invalid session bundle JSON/);

    const badSchemaBundle = await sessionImportBundle(JSON.stringify({ schema_version: 2 }));
    expect(badSchemaBundle.ok).toBe(false);
    expect(badSchemaBundle.imported).toBe(false);

    await vcsCheckpointCapture('thread-reset', 2, 'before-reset');
    expect((await vcsCheckpointList('thread-reset')).length).toBe(2);
    __resetWave3PreviewStateForTests();
    expect((await vcsCheckpointList('thread-reset')).length).toBe(1);
  });
});
