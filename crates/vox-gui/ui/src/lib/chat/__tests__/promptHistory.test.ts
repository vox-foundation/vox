import { describe, expect, it } from 'vitest';
import {
  MAX_PROMPT_HISTORY_ENTRIES,
  PromptHistoryController,
  editAssistantMessageInPlace,
} from '../promptHistory';

describe('PromptHistoryController & editAssistantMessageInPlace (T3-RULES-SEARCH-04)', () => {
  it('caps history at 100 entries and preserves draft across Up/Down navigation', () => {
    const ctrl = new PromptHistoryController();
    for (let i = 1; i <= 110; i += 1) {
      ctrl.pushSubmittedPrompt(`prompt-${i}`);
    }
    expect(ctrl.size()).toBe(MAX_PROMPT_HISTORY_ENTRIES);

    const prev1 = ctrl.navigateUp('my current draft');
    expect(prev1).toBe('prompt-110');
    const prev2 = ctrl.navigateUp(prev1);
    expect(prev2).toBe('prompt-109');
    const back1 = ctrl.navigateDown(prev2);
    expect(back1).toBe('prompt-110');
    const restored = ctrl.navigateDown(back1);
    expect(restored).toBe('my current draft');
  });

  it('edits assistant message markdown in-place without truncating subsequent turns', () => {
    const turns = [
      { id: 'u1', role: 'user' as const, content: 'hi' },
      { id: 'a1', role: 'assistant' as const, content: 'old reply' },
      { id: 'u2', role: 'user' as const, content: 'follow-up' },
    ];
    const updated = editAssistantMessageInPlace(turns, 'a1', 'refined reply', 1700000000000);
    expect(updated).toHaveLength(3);
    expect(updated[1]?.content).toBe('refined reply');
    expect(updated[1]?.editedInPlaceAtMs).toBe(1700000000000);
  });
});
