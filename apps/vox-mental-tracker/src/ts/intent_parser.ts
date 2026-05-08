/**
 * Deterministic NLU for wellness journaling (regex/heuristic).
 * Mirror semantics in tests/fixtures/parser_cases.json.
 * Debug: log raw transcript + chosen intent (plan requirement).
 */

export type ParsedIntent =
  | { kind: "meal_recorded"; payload: { description: string; meal_type?: string }; confidence: number }
  | { kind: "sleep_started"; payload: Record<string, never>; confidence: number }
  | { kind: "exercise_recorded"; payload: { activity: string; duration_minutes: number }; confidence: number }
  | { kind: "mood_recorded"; payload: { mood_score: number }; confidence: number }
  | { kind: "note_recorded"; payload: { body: string }; confidence: number };

export function parseIntent(transcript: string): ParsedIntent {
  const t = transcript.trim().toLowerCase();
  // eslint-disable-next-line no-console
  console.debug("[intent_parser] raw transcript:", transcript);

  const mood = /(?:mood|feeling).*(?:like\s+a\s+)?(\d)(?:\s*\/\s*5)?/.exec(t);
  if (mood) {
    const score = Math.max(1, Math.min(5, parseInt(mood[1]!, 10)));
    return { kind: "mood_recorded", payload: { mood_score: score }, confidence: 0.75 };
  }

  const meal = /(?:ate|eating|had)\s+(.+)/.exec(t);
  if (meal) {
    return {
      kind: "meal_recorded",
      payload: { description: meal[1]!.trim() },
      confidence: 0.65,
    };
  }

  const run = /(?:ran|run|jogged|walked)\s+(?:for\s+)?(\d+)\s*(?:min|minutes)/.exec(t);
  if (run) {
    return {
      kind: "exercise_recorded",
      payload: { activity: "cardio", duration_minutes: parseInt(run[1]!, 10) },
      confidence: 0.7,
    };
  }

  if (/\b(bed|sleep|slept|nap)\b/.test(t)) {
    return { kind: "sleep_started", payload: {}, confidence: 0.55 };
  }

  return { kind: "note_recorded", payload: { body: transcript }, confidence: 0.4 };
}
