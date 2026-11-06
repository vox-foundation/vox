import { describe, expect, it } from 'vitest';
import {
  compactChatUpTo,
  editChatMessageInPlace,
  regenerateChatTurn,
  resumeChatStream,
  steerChatQueue,
  unwrapMcpData,
} from '../t3Wave3Rpc';

describe('t3Wave3Rpc envelope unwrapping and Wave 4 chat helpers', () => {
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

  it('throws when Tauri is_error is true or ToolResult success is false', () => {
    expect(() =>
      unwrapMcpData({
        is_error: true,
        result: 'Permission denied',
      }),
    ).toThrow(/Permission denied/);

    expect(() =>
      unwrapMcpData(
        JSON.stringify({
          success: false,
          error: 'Invalid message_id',
        }),
      ),
    ).toThrow(/Invalid message_id/);
  });

  it('provides standalone fallbacks for editChatMessageInPlace, regenerateChatTurn, compactChatUpTo, resumeChatStream, and steerChatQueue', async () => {
    const edited = await editChatMessageInPlace({
      sessionId: 'sess-wave4',
      messageId: 'asst-1',
      newContent: 'Updated in-place content',
    });
    expect(edited.session_id).toBe('sess-wave4');
    expect(edited.edited_message.content).toBe('Updated in-place content');

    const regen = await regenerateChatTurn({
      sessionId: 'sess-wave4',
      fromMessageId: 'asst-1',
      promptOverride: 'Retry with stricter types',
    });
    expect(regen.session_id).toBe('sess-wave4');
    expect(regen.replay_prompt).toBe('Retry with stricter types');

    const compacted = await compactChatUpTo({
      sessionId: 'sess-wave4',
      messageId: 'asst-1',
      direction: 'up_to',
    });
    expect(compacted.session_id).toBe('sess-wave4');

    const resumed = await resumeChatStream({
      sessionId: 'sess-wave4',
      charOffset: 12,
    });
    expect(resumed.char_offset).toBe(12);
    expect(resumed.completed).toBe(true);

    const steered = await steerChatQueue({
      sessionId: 'sess-wave4',
      prompt: 'Focus on zero-copy slices',
      promoteToSteer: true,
    });
    expect(steered.promoted_to_steer).toBe(true);
    expect(steered.steered_intervention).toContain('[MID-TURN STEER INTERVENTION]');
  });
});
