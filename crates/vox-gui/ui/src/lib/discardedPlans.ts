/** Per-chat discarded plan session ids persisted in shell storage. */

export type DiscardedPlansByChat = Record<string, string[]>;

export function isPlanDiscarded(
  map: DiscardedPlansByChat,
  chatId: string,
  planId: string,
): boolean {
  return map[chatId]?.includes(planId) === true;
}

export function rememberDiscardedPlan(
  map: DiscardedPlansByChat,
  chatId: string,
  planId: string,
): DiscardedPlansByChat {
  const existing = map[chatId] ?? [];
  if (existing.includes(planId)) return map;
  return { ...map, [chatId]: [...existing, planId] };
}

export function forgetDiscardedPlan(
  map: DiscardedPlansByChat,
  chatId: string,
  planId: string,
): DiscardedPlansByChat {
  const existing = map[chatId];
  if (!existing?.includes(planId)) return map;
  const next = existing.filter((id) => id !== planId);
  if (next.length === 0) {
    const { [chatId]: _removed, ...rest } = map;
    return rest;
  }
  return { ...map, [chatId]: next };
}
