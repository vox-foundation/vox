/**
 * Terminal-style composer prompt history (`100` entries), per-message generation
 * posture inspector snapshot, and in-place assistant markdown editing (`T3-RULES-SEARCH-04`).
 */

export const MAX_PROMPT_HISTORY_ENTRIES = 100;

export class PromptHistoryController {
  private readonly entries: string[] = [];
  private cursor: number = -1;
  private draftBeforeNavigation: string = '';

  public pushSubmittedPrompt(prompt: string): void {
    const trimmed = prompt.trim();
    if (!trimmed) {
      return;
    }
    if (this.entries[this.entries.length - 1] !== trimmed) {
      this.entries.push(trimmed);
      if (this.entries.length > MAX_PROMPT_HISTORY_ENTRIES) {
        this.entries.shift();
      }
    }
    this.cursor = -1;
    this.draftBeforeNavigation = '';
  }

  public navigateUp(currentComposerValue: string): string {
    if (this.entries.length === 0) {
      return currentComposerValue;
    }
    if (this.cursor === -1) {
      this.draftBeforeNavigation = currentComposerValue;
      this.cursor = this.entries.length - 1;
    } else if (this.cursor > 0) {
      this.cursor -= 1;
    }
    return this.entries[this.cursor] ?? currentComposerValue;
  }

  public navigateDown(currentComposerValue: string): string {
    if (this.cursor === -1) {
      return currentComposerValue;
    }
    if (this.cursor < this.entries.length - 1) {
      this.cursor += 1;
      return this.entries[this.cursor] ?? currentComposerValue;
    }
    this.cursor = -1;
    return this.draftBeforeNavigation;
  }

  public size(): number {
    return this.entries.length;
  }
}

export interface GenerationPostureSnapshot {
  readonly messageId: string;
  readonly modelId: string;
  readonly reasoningEffort?: 'low' | 'medium' | 'high';
  readonly temperature?: number;
  readonly topP?: number;
  readonly activeRules: readonly string[];
  readonly activeMcpServers: readonly string[];
  readonly costUsd?: number;
}

export interface EditableChatMessage {
  readonly id: string;
  readonly role: 'user' | 'assistant' | 'system';
  readonly content: string;
  readonly editedInPlaceAtMs?: number;
}

export function editAssistantMessageInPlace(
  messages: readonly EditableChatMessage[],
  targetMessageId: string,
  updatedContent: string,
  nowMs: number,
): readonly EditableChatMessage[] {
  return messages.map((msg) => {
    if (msg.id === targetMessageId && msg.role === 'assistant') {
      return {
        ...msg,
        content: updatedContent,
        editedInPlaceAtMs: nowMs,
      };
    }
    return msg;
  });
}
