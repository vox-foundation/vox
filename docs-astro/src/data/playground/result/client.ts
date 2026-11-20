/** @query `parse_port` → `/api/query/parse_port` (GET) */
export async function parse_port(args: { raw: number }, init?: RequestInit): Promise<unknown> {
  if (isTauri()) return $tauri<unknown>("parse_port", { raw: args.raw }, z.any());
  return $get<unknown>("/api/query/parse_port", z.any(), { raw: args.raw }, init);
}

