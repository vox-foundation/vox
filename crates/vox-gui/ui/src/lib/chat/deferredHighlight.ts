/**
 * Deferred syntax highlighting & diff rendering threshold policy (`T3-CLIENT-BRANCH-02`).
 */

export const DEFERRED_HIGHLIGHT_CHAR_THRESHOLD = 20_000;
export const DEFERRED_INLINE_CODE_CHAR_THRESHOLD = 2_000;
export const DEFERRED_HIGHLIGHT_IDLE_DELAY_MS = 300;

export interface HighlightBudgetDecision {
  readonly deferToIdle: boolean;
  readonly idleDelayMs: number;
  readonly renderPlainPreFirst: boolean;
}

export function evaluateHighlightBudget(
  codeLengthChars: number,
  isStreaming: boolean,
  isBlockCode: boolean = true,
): HighlightBudgetDecision {
  const threshold = isBlockCode
    ? DEFERRED_HIGHLIGHT_CHAR_THRESHOLD
    : DEFERRED_INLINE_CODE_CHAR_THRESHOLD;
  if (isStreaming || codeLengthChars > threshold) {
    return {
      deferToIdle: true,
      idleDelayMs: DEFERRED_HIGHLIGHT_IDLE_DELAY_MS,
      renderPlainPreFirst: true,
    };
  }
  return {
    deferToIdle: false,
    idleDelayMs: 0,
    renderPlainPreFirst: false,
  };
}
