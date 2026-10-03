import React from 'react';

/**
 * Opt-in toggle for the post-reply grounding/hallucination check (see
 * `hooks/useGroundingCheck.ts`). Minimal by design — this is a low-stakes,
 * opt-in composer setting, not a major UI feature. State is lifted to the
 * caller (App.tsx), which persists it via `useGroundingCheck(sessionId)` and
 * threads the value into the chat submit payload as `grounding_check_enabled`
 * (mirrors how the composer's model-tier pick lifts to `chatModelOverride`).
 */
export function GroundingCheckToggle({
  enabled,
  onToggle,
}: {
  enabled: boolean;
  onToggle: (next: boolean) => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={enabled}
      aria-label={`Check replies ${enabled ? 'on' : 'off'}`}
      onClick={() => onToggle(!enabled)}
      title="When on, each reply gets a background check for unsupported claims; it never blocks the reply"
      className={`rounded-lg border px-2 py-1 font-mono text-[11px] ${
        enabled
          ? 'border-brass/40 text-brass'
          : 'border-border-subtle text-text-muted hover:text-brass'
      }`}
    >
      Check replies: {enabled ? 'on' : 'off'}
    </button>
  );
}
