/**
 * Parse envelopes returned by `invoke_mcp_tool` (B5).
 * Daemon tools wrap payloads as `{ success, data, error?, remediation? }`.
 *
 * All parsers here accept a null/undefined envelope (invokeMcpTool's
 * Promise<{...}> signature is not truthful about non-nullability — it's a
 * thin passthrough with no runtime check, F-02) and return a null-safe
 * result INSTEAD OF throwing a raw TypeError. That result is per-function,
 * not uniform: list-shaped parsers fall back to an empty list/null (there is
 * a sensible "nothing here" value), while parseGraphifyStatus throws an
 * honest Error (a search-index status has no meaningful empty state, so a
 * throw preserving its existing is_error-throws contract is the right null
 * response too). Callers still own UX-level fallback/empty-state rendering —
 * this module only guarantees the parse layer never leaks a raw TypeError.
 */

export interface McpInvokeResult {
  tool: string;
  is_error: boolean;
  result: unknown;
}

export interface PendingApprovalRow {
  approval_id: string;
  tool: string;
  summary: string;
  requested_at_ms: number;
  floor_category?: string;
  session_id?: string;
  command?: string;
  control_mode?: string;
  credential_prompt?: boolean;
}

/** Unwrap a ToolResult `.data` field when present. */
export function unwrapMcpEnvelope(result: unknown): unknown {
  if (result && typeof result === 'object' && 'data' in result) {
    return (result as { data: unknown }).data;
  }
  return result;
}

export function parsePendingApprovals(
  invokeResult: McpInvokeResult | null | undefined,
): PendingApprovalRow[] {
  if (!invokeResult) return [];
  const data = unwrapMcpEnvelope(invokeResult.result) as { approvals?: unknown[] } | null;
  const list = data?.approvals;
  if (!Array.isArray(list)) return [];
  return list
    .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === 'object')
    .map((r) => {
      const row: PendingApprovalRow = {
        approval_id: typeof r.approval_id === 'string' ? r.approval_id : '',
        tool: typeof r.tool === 'string' ? r.tool : '',
        summary: typeof r.summary === 'string' ? r.summary : '',
        requested_at_ms: typeof r.requested_at_ms === 'number' ? r.requested_at_ms : 0,
      };
      if (typeof r.floor_category === 'string') row.floor_category = r.floor_category;
      if (typeof r.session_id === 'string') row.session_id = r.session_id;
      if (typeof r.command === 'string') row.command = r.command;
      if (typeof r.control_mode === 'string') row.control_mode = r.control_mode;
      if (typeof r.credential_prompt === 'boolean') row.credential_prompt = r.credential_prompt;
      return row;
    });
}

export interface GraphifyStatusPayload {
  default_corpus_id: string;
  corpora: unknown[];
  /** Effective TTL after env > contract precedence. Optional: a backend that
   *  omits it must still parse, and the panel hides the TTL editor then. */
  ttl_days?: number;
  ttl_days_contract?: number;
  ttl_days_env_forced?: boolean;
  ttl_contract_path?: string;
}

/** Parse the `vox_search_status` envelope into the panel's status shape. */
export function parseGraphifyStatus(
  invokeResult: McpInvokeResult | null | undefined,
): GraphifyStatusPayload {
  if (!invokeResult) {
    throw new Error('vox_search_status: no response from backend');
  }
  if (invokeResult.is_error) {
    throw new Error('vox_search_status reported an error');
  }
  const data = unwrapMcpEnvelope(invokeResult.result) as Partial<GraphifyStatusPayload> | null;
  // Deliberate pick, not a spread: every field is type-checked here so a
  // malformed payload cannot reach the panel. Each TTL key is forwarded only
  // when it has the right type, so an older backend that omits them yields
  // `undefined` and the panel's presence guard hides the editor.
  return {
    default_corpus_id: data?.default_corpus_id ?? '',
    corpora: Array.isArray(data?.corpora) ? data.corpora : [],
    ttl_days: typeof data?.ttl_days === 'number' ? data.ttl_days : undefined,
    ttl_days_contract:
      typeof data?.ttl_days_contract === 'number' ? data.ttl_days_contract : undefined,
    ttl_days_env_forced:
      typeof data?.ttl_days_env_forced === 'boolean' ? data.ttl_days_env_forced : undefined,
    ttl_contract_path:
      typeof data?.ttl_contract_path === 'string' ? data.ttl_contract_path : undefined,
  };
}

/** Extract string payload from `vox_git_diff` and similar text tools. */
export function parseMcpToolText(invokeResult: McpInvokeResult | null | undefined): string | null {
  if (!invokeResult) return null;
  const inner = invokeResult.result;
  if (typeof inner === 'string') return inner;
  if (inner && typeof inner === 'object') {
    const envelope = inner as Record<string, unknown>;
    if (envelope.success === true) {
      const data = envelope.data;
      if (typeof data === 'string') return data;
    }
    if (typeof envelope.error === 'string') return null;
  }
  return null;
}
