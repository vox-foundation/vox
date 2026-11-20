/** @query `get_tasks` → `/api/query/get_tasks` (GET) */
export async function get_tasks(init?: RequestInit): Promise<number> {
  if (isTauri()) return $tauri<number>("get_tasks", {}, z.number());
  return $get<number>("/api/query/get_tasks", z.number(), undefined, init);
}

