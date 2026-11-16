/**
 * Strongly-typed Consensus, Context handoff, Session interop, and Wave 4 Chat
 * turn/stream control RPC helpers (`AXIS-IPC-HONESTY-01`).
 */

import type { ConsensusLegViewDto } from '../components/surfaces/Chat/ConsensusGridView';
import { backendAvailable } from '../lib/backendGuard';
import { BROWSER_PREVIEW_REASON, callHonestMcpTool } from './t3Wave3Shared';

export interface ConsensusRunRequestDto {
  readonly prompt: string;
  readonly models?: string[];
}

export interface ConsensusRunResponseDto {
  readonly prompt: string;
  readonly models: string[];
  readonly legs: ConsensusLegViewDto[];
  readonly synthesis?: string;
  readonly totalElapsedMs: number;
  readonly preview_reason?: string;
}

export async function chatConsensusRun(
  req: ConsensusRunRequestDto,
): Promise<ConsensusRunResponseDto> {
  const models =
    req.models && req.models.length >= 2
      ? req.models.slice(0, 4)
      : ['anthropic/claude-sonnet', 'openai/gpt', 'mens/qwen3-8b'];
  if (!backendAvailable()) {
    return {
      prompt: req.prompt,
      models,
      legs: models.map((modelId) => ({
        modelId,
        status: 'completed',
        content: `[Browser preview] Desktop backend offline for \`${modelId}\`.`,
      })),
      totalElapsedMs: 0,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<
    {
      prompt?: string;
      models?: string[];
      synthesis?: string;
      synthesized_consensus?: string;
      rich_results?: Array<{
        model_id: string;
        status: string;
        content: string;
        reasoning?: string;
        response_time_ms?: number;
        quality?: { quality_score: number };
      }>;
      total_elapsed_ms?: number;
    },
    ConsensusRunResponseDto
  >('vox_consensus_run', { prompt: req.prompt, models }, (data) => {
    if (!data || !Array.isArray(data.rich_results)) return null;
    return {
      prompt: data.prompt ?? req.prompt,
      models: Array.isArray(data.models) ? data.models : models,
      synthesis: data.synthesis ?? data.synthesized_consensus,
      totalElapsedMs: data.total_elapsed_ms ?? 0,
      legs: data.rich_results.map((r) => ({
        modelId: r.model_id,
        status: (r.status as ConsensusLegViewDto['status']) ?? 'completed',
        content: r.content,
        reasoning: r.reasoning,
        responseTimeMs: r.response_time_ms,
        qualityScore: r.quality?.quality_score ?? 0,
      })),
    };
  });
}

export async function chatContextHandoff(args: {
  sourceThreadId: string;
  sourceProvider: string;
  targetProvider: string;
  turns: Array<{ role: string; content: string; is_plan_or_code?: boolean }>;
}): Promise<{ formatted_prompt: string; truncated: boolean; preview_reason?: string }> {
  if (!backendAvailable()) {
    const body = `<context_handoff>\n${args.turns.map((t) => `[${t.role}]: ${t.content}`).join('\n')}\n</context_handoff>`;
    return { formatted_prompt: body, truncated: false, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<{ formatted_prompt: string; truncated: boolean }, { formatted_prompt: string; truncated: boolean }>(
    'vox_context_handoff',
    {
      source_thread_id: args.sourceThreadId,
      source_provider: args.sourceProvider,
      target_provider: args.targetProvider,
      turns: args.turns.map((t) => ({ role: t.role, content: t.content, is_plan_or_code: Boolean(t.is_plan_or_code) })),
    },
    (data) => (data && typeof data.formatted_prompt === 'string' ? data : null),
  );
}

export interface ExternalSessionMessageDto {
  readonly role: string;
  readonly content: string;
}

export interface ImportedExternalSessionDto {
  readonly source_harness: 'claude' | 'codex' | 'cursor' | string;
  readonly external_session_id: string;
  readonly title: string;
  readonly messages: ExternalSessionMessageDto[];
  readonly preview_reason?: string;
}

export interface VoxSessionExportBundleV1Dto {
  readonly schema_version: number;
  readonly session_id: string;
  readonly title: string;
  readonly messages: ExternalSessionMessageDto[];
  readonly preview_reason?: string;
}

export interface DiscoveredExternalSessionDto {
  readonly source_harness: string;
  readonly external_session_id: string;
  readonly file_path: string;
  readonly size_bytes: number;
}

export interface EditedChatMessageDto {
  readonly id: string;
  readonly role: string;
  readonly content: string;
  readonly edited_at?: number;
}

type RawExternalItem = Partial<DiscoveredExternalSessionDto> & { session_id?: string };

export async function sessionListExternal(
  sourceHarness?: 'claude' | 'codex' | 'cursor',
): Promise<DiscoveredExternalSessionDto[]> {
  if (!backendAvailable()) {
    const h = sourceHarness ?? 'claude';
    return [{
      source_harness: h,
      external_session_id: `preview:${h}-session-1`,
      file_path: `~/.${h}/projects/preview-session-1.jsonl`,
      size_bytes: 0,
    }];
  }
  return callHonestMcpTool<RawExternalItem[] | { sessions?: RawExternalItem[] }, DiscoveredExternalSessionDto[]>(
    'vox_session_list_external',
    { source_harness: sourceHarness },
    (data) => {
      const list = Array.isArray(data) ? data : Array.isArray(data?.sessions) ? data.sessions : null;
      if (!list) return null;
      return list.map((it) => ({
        source_harness: it.source_harness ?? sourceHarness ?? 'claude',
        external_session_id: it.external_session_id ?? it.session_id ?? 'ext-session',
        file_path: it.file_path ?? '',
        size_bytes: typeof it.size_bytes === 'number' ? it.size_bytes : 0,
      }));
    },
  );
}

export async function sessionImportExternal(args: {
  sourceHarness: 'claude' | 'codex' | 'cursor';
  externalSessionId: string;
  rawJsonl?: string;
  cursorRootBlobHex?: string;
}): Promise<ImportedExternalSessionDto> {
  if (!backendAvailable()) {
    const lines = (args.rawJsonl || '{"role":"user","content":"Browser preview prompt"}')
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean);
    const messages: ExternalSessionMessageDto[] = lines.map((line) => {
      try {
        const parsed = JSON.parse(line) as { role?: string; content?: string };
        return { role: parsed.role ?? 'user', content: parsed.content ?? line };
      } catch {
        return { role: 'user', content: line };
      }
    });
    return {
      source_harness: args.sourceHarness,
      external_session_id: args.externalSessionId,
      title: messages[0]?.content.slice(0, 48) ?? `preview:${args.externalSessionId}`,
      messages,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<ImportedExternalSessionDto, ImportedExternalSessionDto>(
    'vox_session_import_external',
    {
      source_harness: args.sourceHarness,
      external_session_id: args.externalSessionId,
      raw_jsonl: args.rawJsonl,
      cursor_root_blob_hex: args.cursorRootBlobHex,
    },
    (data) => (data && Array.isArray(data.messages) ? data : null),
  );
}

export async function sessionExportBundle(args: {
  sessionId: string;
  title: string;
  messages: ExternalSessionMessageDto[];
}): Promise<VoxSessionExportBundleV1Dto> {
  if (!backendAvailable()) {
    return {
      schema_version: 1,
      session_id: args.sessionId,
      title: args.title,
      messages: args.messages,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<VoxSessionExportBundleV1Dto, VoxSessionExportBundleV1Dto>(
    'vox_session_export_bundle',
    { session_id: args.sessionId, title: args.title, messages: args.messages },
    (data) => (data?.schema_version === 1 ? data : null),
  );
}

export async function sessionImportBundle(
  bundle: VoxSessionExportBundleV1Dto | string,
): Promise<{
  ok?: boolean;
  error?: string;
  imported: boolean;
  session_id: string;
  title: string;
  message_count: number;
  preview_reason?: string;
}> {
  if (!backendAvailable()) {
    let parsed: Partial<VoxSessionExportBundleV1Dto> | null = null;
    try {
      parsed =
        typeof bundle === 'string'
          ? (JSON.parse(bundle) as Partial<VoxSessionExportBundleV1Dto>)
          : bundle;
    } catch {
      return {
        ok: false,
        error: 'Invalid session bundle JSON',
        imported: false,
        session_id: '',
        title: '',
        message_count: 0,
        preview_reason: BROWSER_PREVIEW_REASON,
      };
    }
    if (
      !parsed ||
      typeof parsed !== 'object' ||
      parsed.schema_version !== 1 ||
      !Array.isArray(parsed.messages)
    ) {
      return {
        ok: false,
        error: 'Unsupported session bundle schema (expected schema_version 1 and messages array)',
        imported: false,
        session_id: '',
        title: '',
        message_count: 0,
        preview_reason: BROWSER_PREVIEW_REASON,
      };
    }
    return {
      ok: true,
      imported: true,
      session_id: parsed.session_id ?? 'preview:imported-session',
      title: parsed.title ?? '',
      message_count: parsed.messages.length,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  const bundleJson = typeof bundle === 'string' ? bundle : JSON.stringify(bundle);
  return callHonestMcpTool<
    { imported?: boolean; session_id?: string; title?: string; message_count?: number; messages?: unknown[] },
    { ok: boolean; imported: boolean; session_id: string; title: string; message_count: number }
  >('vox_session_import_bundle', { bundle_json: bundleJson }, (data) => {
    if (!data?.session_id || data.imported === false) return null;
    return {
      ok: true,
      imported: data.imported ?? true,
      session_id: data.session_id,
      title: data.title ?? '',
      message_count:
        typeof data.message_count === 'number'
          ? data.message_count
          : Array.isArray(data.messages)
            ? data.messages.length
            : 0,
    };
  });
}

export async function editChatMessageInPlace(args: {
  sessionId: string;
  messageId: string;
  newContent: string;
}): Promise<{ session_id: string; edited_message: EditedChatMessageDto; preview_reason?: string }> {
  if (!backendAvailable()) {
    return {
      session_id: args.sessionId,
      edited_message: { id: args.messageId, role: 'assistant', content: args.newContent, edited_at: Math.floor(Date.now() / 1000) },
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<
    { session_id: string; edited_message: EditedChatMessageDto },
    { session_id: string; edited_message: EditedChatMessageDto }
  >(
    'vox_chat_edit_message_in_place',
    { session_id: args.sessionId, message_id: args.messageId, new_content: args.newContent },
    (data) => (data?.edited_message ? data : null),
  );
}

export async function regenerateChatTurn(args: {
  sessionId: string;
  fromMessageId?: string;
  promptOverride?: string;
  execute?: boolean;
}): Promise<{ session_id: string; replay_prompt: string; removed_turns: number; preview_reason?: string }> {
  if (!backendAvailable()) {
    return { session_id: args.sessionId, replay_prompt: args.promptOverride ?? '', removed_turns: 1, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<
    { session_id: string; replay_prompt: string; removed_turns: number },
    { session_id: string; replay_prompt: string; removed_turns: number }
  >(
    'vox_chat_regenerate',
    { session_id: args.sessionId, from_message_id: args.fromMessageId, prompt_override: args.promptOverride, execute: args.execute ?? false },
    (data) => (data?.session_id ? data : null),
  );
}

export async function compactChatUpTo(args: {
  sessionId: string;
  messageId?: string;
  index?: number;
  direction?: 'up_to' | 'from';
  summary?: string;
}): Promise<{ session_id: string; compact_message_index: number; preserved_thought_signatures: string[]; preview_reason?: string }> {
  if (!backendAvailable()) {
    return { session_id: args.sessionId, compact_message_index: 0, preserved_thought_signatures: [], preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<
    { session_id: string; compact_message_index: number; preserved_thought_signatures: string[] },
    { session_id: string; compact_message_index: number; preserved_thought_signatures: string[] }
  >(
    'vox_chat_compact_up_to',
    { session_id: args.sessionId, message_id: args.messageId, index: args.index, direction: args.direction ?? 'up_to', summary: args.summary },
    (data) => (data?.session_id ? data : null),
  );
}

interface StreamResumeOut {
  session_id: string;
  catch_up_delta: string;
  char_offset: number;
  next_char_offset: number;
  last_seq?: number;
  chunks?: unknown[];
  cancelled?: boolean;
  completed: boolean;
  preview_reason?: string;
}

export async function resumeChatStream(args: {
  sessionId: string;
  charOffset?: number;
  afterSeq?: number;
}): Promise<StreamResumeOut> {
  if (!backendAvailable()) {
    const off = args.charOffset ?? 0;
    return {
      session_id: args.sessionId,
      catch_up_delta: '',
      char_offset: off,
      next_char_offset: off,
      last_seq: args.afterSeq ?? 0,
      chunks: [],
      cancelled: false,
      completed: true,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<StreamResumeOut, StreamResumeOut>(
    'vox_chat_stream_resume',
    { session_id: args.sessionId, char_offset: args.charOffset ?? 0, after_seq: args.afterSeq },
    (data) => (data?.session_id ? data : null),
  );
}

export async function cancelChatStream(sessionId: string): Promise<{
  session_id: string;
  cancelled: boolean;
  preview_reason?: string;
}> {
  if (!backendAvailable()) {
    return { session_id: sessionId, cancelled: true, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<{ session_id: string; cancelled: boolean }, { session_id: string; cancelled: boolean }>(
    'vox_chat_stream_cancel',
    { session_id: sessionId },
    (data) => (data?.session_id ? data : null),
  );
}

interface QueueSteerOut {
  session_id: string;
  entry_id: string;
  promoted_to_steer: boolean;
  steered_intervention?: string | null;
  preview_reason?: string;
}

export async function steerChatQueue(args: {
  sessionId: string;
  prompt: string;
  entryId?: string;
  promoteToSteer?: boolean;
}): Promise<QueueSteerOut> {
  if (!backendAvailable()) {
    return {
      session_id: args.sessionId,
      entry_id: args.entryId ?? `preview:q:${args.sessionId}`,
      promoted_to_steer: args.promoteToSteer ?? true,
      steered_intervention: `[MID-TURN STEER INTERVENTION]: ${args.prompt}`,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<QueueSteerOut, QueueSteerOut>(
    'vox_chat_queue_steer',
    { session_id: args.sessionId, prompt: args.prompt, entry_id: args.entryId, promote_to_steer: args.promoteToSteer ?? true },
    (data) => (data?.session_id ? data : null),
  );
}
