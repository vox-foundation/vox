/**
 * Shared envelope unwrapping, honest MCP invocation, route-racer status, battery ledger,
 * and guest-share helpers for T3 Wave 3/4 RPC (`AXIS-IPC-HONESTY-01`).
 */

import {
  BackendUnavailableError,
  backendAvailable,
  sanitizeErrorForToast,
} from '../lib/backendGuard';
import {
  ConnectionRouteRacer,
  type ConnectionRouteTier,
  type RouteCandidate,
  type StaggeredRaceScheduleEntry,
} from '../lib/connection/routeRacer';
import { voxTransport } from '../transport';

export const BROWSER_PREVIEW_REASON = 'browser_preview_no_desktop_backend' as const;

export interface ActiveTransportRouteStatusDto {
  readonly activeTier: ConnectionRouteTier;
  readonly activeUrl: string;
  readonly schedule: readonly StaggeredRaceScheduleEntry[];
}

export interface BatteryStatusDto {
  readonly session_id: string;
  readonly remaining_bu: number;
  readonly four_hour_spent_bu: number;
  readonly four_hour_limit_bu: number;
  readonly four_hour_pct: number;
  readonly monthly_spent_bu: number;
  readonly monthly_limit_bu: number;
  readonly monthly_pct: number;
  readonly total_cache_read_tokens: number;
  readonly fallback_to_local: boolean;
  readonly preview_reason?: string;
}

export interface GuestInviteSummaryDto {
  readonly token: string;
  readonly invite_id: string;
  readonly session_id: string;
  readonly permission: string;
  readonly expires_at_ms: number;
  readonly revoked?: boolean;
}

export interface GuestSubmissionSummaryDto {
  readonly submission_id: string;
  readonly session_id: string;
  readonly guest_name: string;
  readonly prompt: string;
  readonly status: string;
  readonly submitted_at_ms?: number;
}

const DEFAULT_ROUTE_CANDIDATES: readonly RouteCandidate[] = [
  { tier: 'last_working', url: 'ws://127.0.0.1:4190/ws' },
  { tier: 'localhost_loopback', url: 'ws://localhost:4190/ws' },
  { tier: 'lan_direct', url: 'ws://192.168.1.10:4190/ws' },
  { tier: 'tailscale_MagicDNS', url: 'wss://vox-host.tailnet.ts.net/ws' },
  { tier: 'public_relay', url: 'wss://relay.voxlang.org/ws' },
  { tier: 'manual_fallback', url: 'http://127.0.0.1:4190/rpc' },
];

const sharedRouteRacer = new ConnectionRouteRacer();

export function getActiveTransportRouteStatus(
  nowMs = Date.now(),
  candidates: readonly RouteCandidate[] = DEFAULT_ROUTE_CANDIDATES,
): ActiveTransportRouteStatusDto {
  if (candidates.length === 0) {
    return { activeTier: 'manual_fallback', activeUrl: '', schedule: [] };
  }
  const schedule = sharedRouteRacer.planStaggeredRace(candidates, nowMs);
  const winner = schedule[0]?.candidate ?? candidates[0];
  return { activeTier: winner.tier, activeUrl: winner.url, schedule };
}

export function recordTransportRouteFailure(url: string, nowMs = Date.now()): void {
  sharedRouteRacer.recordRouteFailure(url, nowMs);
}

function extractMcpErrorMessage(errVal: unknown, fallback: string): string {
  if (typeof errVal === 'string' && errVal.trim()) return errVal;
  if (errVal && typeof errVal === 'object') {
    const rec = errVal as Record<string, unknown>;
    if (typeof rec.message === 'string' && rec.message.trim()) return rec.message;
    if (typeof rec.code === 'string' && rec.code.trim()) return rec.code;
  }
  return fallback;
}

export function unwrapMcpData<T>(raw: unknown): T | null {
  if (raw == null) return null;
  if (typeof raw === 'string') {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    let parsed: unknown;
    try {
      parsed = JSON.parse(trimmed);
    } catch {
      return null;
    }
    return unwrapMcpData<T>(parsed);
  }
  if (typeof raw !== 'object') return null;
  const obj = raw as Record<string, unknown>;
  if ('is_error' in obj && typeof obj.is_error === 'boolean') {
    if (obj.is_error) {
      if (obj.result != null && typeof obj.result === 'object') {
        unwrapMcpData(obj.result);
      } else if (typeof obj.result === 'string' && obj.result.trim().startsWith('{')) {
        unwrapMcpData(obj.result);
      }
      throw new Error(
        typeof obj.result === 'string' && obj.result.trim()
          ? obj.result
          : 'MCP tool execution failed',
      );
    }
    return unwrapMcpData<T>(obj.result);
  }
  if (Array.isArray(obj.content) && obj.content.length > 0) {
    const first = obj.content[0] as Record<string, unknown> | undefined;
    if (first && typeof first.text === 'string') {
      return unwrapMcpData<T>(first.text);
    }
  }
  if ('success' in obj && typeof obj.success === 'boolean') {
    if (!obj.success) {
      throw new Error(extractMcpErrorMessage(obj.error, 'Tool failed'));
    }
    if ('data' in obj && obj.data !== undefined) {
      return obj.data as T;
    }
  }
  if ('error' in obj && obj.error && typeof obj.error === 'object') {
    throw new Error(extractMcpErrorMessage(obj.error, 'MCP tool execution failed'));
  }
  if ('data' in obj && obj.data != null) {
    return obj.data as T;
  }
  return raw as T;
}

export function toSanitizedRpcError(err: unknown): Error {
  const cleaned =
    err instanceof BackendUnavailableError
      ? sanitizeErrorForToast(err)
      : sanitizeErrorForToast(err instanceof Error ? err.message : err);
  return new Error(cleaned);
}

export async function callHonestMcpTool<TRaw, TOut>(
  tool: string,
  args: Record<string, unknown>,
  mapAndValidate: (data: TRaw | null) => TOut | null | undefined,
): Promise<TOut> {
  try {
    const raw = await voxTransport.invokeMcpTool(tool, args);
    const data = unwrapMcpData<TRaw>(raw);
    const mapped = mapAndValidate(data);
    if (mapped === null || mapped === undefined) {
      throw new Error(`${tool} returned malformed payload`);
    }
    return mapped;
  } catch (err) {
    throw toSanitizedRpcError(err);
  }
}

export async function batteryStatus(sessionId = 'default'): Promise<BatteryStatusDto> {
  if (!backendAvailable()) {
    return {
      session_id: sessionId,
      remaining_bu: 0,
      four_hour_spent_bu: 0,
      four_hour_limit_bu: 500,
      four_hour_pct: 0,
      monthly_spent_bu: 0,
      monthly_limit_bu: 5000,
      monthly_pct: 0,
      total_cache_read_tokens: 0,
      fallback_to_local: true,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<Partial<BatteryStatusDto>, BatteryStatusDto>(
    'vox_battery_status',
    { session_id: sessionId },
    (data) => {
      if (!data || typeof data.remaining_bu !== 'number') return null;
      return {
        session_id: data.session_id ?? sessionId,
        remaining_bu: data.remaining_bu,
        four_hour_spent_bu: data.four_hour_spent_bu ?? 0,
        four_hour_limit_bu: data.four_hour_limit_bu ?? 500,
        four_hour_pct: data.four_hour_pct ?? 0,
        monthly_spent_bu: data.monthly_spent_bu ?? 0,
        monthly_limit_bu: data.monthly_limit_bu ?? 5000,
        monthly_pct: data.monthly_pct ?? 0,
        total_cache_read_tokens: data.total_cache_read_tokens ?? 0,
        fallback_to_local: Boolean(data.fallback_to_local),
      };
    },
  );
}

type RawGuestInvite = Partial<GuestInviteSummaryDto> & {
  signature_hex?: string;
  thread_id?: string;
  scope?: string;
};
type RawGuestSub = Partial<GuestSubmissionSummaryDto> & {
  message_id?: string;
  thread_id?: string;
  guest_display_name?: string;
};

export async function guestShareList(sessionId?: string): Promise<{
  invites: GuestInviteSummaryDto[];
  submissions: GuestSubmissionSummaryDto[];
  preview_reason?: string;
}> {
  if (!backendAvailable()) {
    return { invites: [], submissions: [], preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<
    { invites?: RawGuestInvite[]; submissions?: RawGuestSub[] },
    { invites: GuestInviteSummaryDto[]; submissions: GuestSubmissionSummaryDto[] }
  >('vox_guest_share_list', { session_id: sessionId }, (data) => {
    if (!data || !Array.isArray(data.invites) || !Array.isArray(data.submissions)) return null;
    return {
      invites: data.invites.map((inv) => {
        const id = inv.invite_id ?? 'invite';
        return {
          token: inv.token ?? (inv.signature_hex ? `${id}:${inv.signature_hex}` : id),
          invite_id: id,
          session_id: inv.session_id ?? inv.thread_id ?? sessionId ?? 'default',
          permission: inv.permission ?? (typeof inv.scope === 'string' ? inv.scope.toLowerCase() : 'suggest_prompt'),
          expires_at_ms: inv.expires_at_ms ?? 0,
          ...(inv.revoked !== undefined ? { revoked: inv.revoked } : {}),
        };
      }),
      submissions: data.submissions.map((sub) => ({
        submission_id: sub.submission_id ?? sub.message_id ?? 'sub',
        session_id: sub.session_id ?? sub.thread_id ?? sessionId ?? 'default',
        guest_name: sub.guest_name ?? sub.guest_display_name ?? 'Guest',
        prompt: sub.prompt ?? '',
        status: sub.status ?? 'pending',
        ...(sub.submitted_at_ms !== undefined ? { submitted_at_ms: sub.submitted_at_ms } : {}),
      })),
    };
  });
}

export async function guestShareMint(args: {
  sessionId: string;
  permission?: 'read_only' | 'suggest_prompt';
  ttlMs?: number;
}): Promise<{ token: string; session_id: string; permission: string; expires_at_ms: number; preview_reason?: string }> {
  const perm = args.permission ?? 'suggest_prompt';
  const ttl = args.ttlMs ?? 3_600_000;
  if (!backendAvailable()) {
    return {
      token: `preview:${args.sessionId}`,
      session_id: args.sessionId,
      permission: perm,
      expires_at_ms: Date.now() + ttl,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<RawGuestInvite, { token: string; session_id: string; permission: string; expires_at_ms: number }>(
    'vox_guest_share_mint',
    { session_id: args.sessionId, permission: perm, ttl_ms: ttl },
    (data) => {
      const token = data?.token ?? (data?.invite_id && data?.signature_hex ? `${data.invite_id}:${data.signature_hex}` : null);
      if (!token) return null;
      return {
        token,
        session_id: data?.session_id ?? data?.thread_id ?? args.sessionId,
        permission: data?.permission ?? (typeof data?.scope === 'string' ? data.scope.toLowerCase() : perm),
        expires_at_ms: data?.expires_at_ms ?? Date.now() + ttl,
      };
    },
  );
}

export async function guestShareRedeem(token: string): Promise<{
  session_id: string;
  permission: string;
  expires_at_ms: number;
  valid: boolean;
  preview_reason?: string;
}> {
  if (!backendAvailable()) {
    return { session_id: 'default', permission: 'suggest_prompt', expires_at_ms: Date.now() + 3_600_000, valid: true, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<
    { session_id?: string; thread_id?: string; permission?: string; expires_at_ms?: number; valid?: boolean; redeemed?: boolean },
    { session_id: string; permission: string; expires_at_ms: number; valid: boolean }
  >('vox_guest_share_redeem', { token }, (data) => {
    const sid = data?.session_id ?? data?.thread_id;
    if (!sid) return null;
    return {
      session_id: sid,
      permission: data?.permission ?? 'suggest_prompt',
      expires_at_ms: data?.expires_at_ms ?? Date.now() + 3_600_000,
      valid: typeof data?.valid === 'boolean' ? data.valid : Boolean(data?.redeemed),
    };
  });
}

export async function guestShareSubmit(args: {
  token: string;
  guestName?: string;
  prompt: string;
}): Promise<{ submission_id: string; session_id: string; guest_name: string; prompt: string; status: string; preview_reason?: string }> {
  const guestName = args.guestName ?? 'Guest Reviewer';
  if (!backendAvailable()) {
    return { submission_id: 'preview:gsub-1', session_id: 'default', guest_name: guestName, prompt: args.prompt, status: 'pending', preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<RawGuestSub, { submission_id: string; session_id: string; guest_name: string; prompt: string; status: string }>(
    'vox_guest_share_submit',
    { token: args.token, guest_name: guestName, prompt: args.prompt },
    (data) => {
      const subId = data?.submission_id ?? data?.message_id;
      if (!subId) return null;
      return {
        submission_id: subId,
        session_id: data?.session_id ?? data?.thread_id ?? 'default',
        guest_name: data?.guest_name ?? data?.guest_display_name ?? guestName,
        prompt: data?.prompt ?? args.prompt,
        status: data?.status ?? 'pending',
      };
    },
  );
}

export async function guestShareModerate(args: {
  submissionId: string;
  approve: boolean;
}): Promise<{ submission_id: string; session_id: string; status: string; preview_reason?: string }> {
  const fallbackStatus = args.approve ? 'approved' : 'rejected';
  if (!backendAvailable()) {
    return { submission_id: args.submissionId, session_id: 'default', status: fallbackStatus, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<RawGuestSub, { submission_id: string; session_id: string; status: string }>(
    'vox_guest_share_moderate',
    { submission_id: args.submissionId, approve: args.approve },
    (data) => {
      const subId = data?.submission_id ?? data?.message_id;
      if (!subId) return null;
      return {
        submission_id: subId,
        session_id: data?.session_id ?? data?.thread_id ?? 'default',
        status: data?.status ?? fallbackStatus,
      };
    },
  );
}
