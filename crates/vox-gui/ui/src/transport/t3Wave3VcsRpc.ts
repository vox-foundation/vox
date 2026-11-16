/**
 * Strongly-typed VCS, checkpoint, file diff/staging, quarantine, and Muse worktree
 * RPC helpers (`AXIS-IPC-HONESTY-01`).
 */

import { backendAvailable } from '../lib/backendGuard';
import { BROWSER_PREVIEW_REASON, callHonestMcpTool } from './t3Wave3Shared';

export interface VcsCheckpointRecordDto {
  readonly thread_id: string;
  readonly turn_count: number;
  readonly ref_name: string;
  readonly commit_oid: string;
  readonly is_restore_point: boolean;
  readonly preview_reason?: string;
}

export interface CompilerDiagnosticSpanDto {
  readonly file: string;
  readonly line: number;
  readonly column: number;
  readonly severity: 'error' | 'warning' | 'info' | string;
  readonly code?: string;
  readonly message: string;
}

export interface FileDiffAndDiagnosticsDto {
  readonly file_path: string;
  readonly added_lines: number;
  readonly deleted_lines: number;
  readonly unified_diff: string;
  readonly diagnostics: readonly CompilerDiagnosticSpanDto[];
  readonly preview_reason?: string;
}

export interface CheckpointFileDeltaDto {
  readonly file_path: string;
  readonly added_lines: number;
  readonly deleted_lines: number;
}

export interface VcsCheckpointDiffSummaryDto {
  readonly thread_id: string;
  readonly target_turn: number;
  readonly ref_name: string;
  readonly files: readonly CheckpointFileDeltaDto[];
  readonly preview_reason?: string;
}

const previewCheckpointsByThread = new Map<string, VcsCheckpointRecordDto[]>();
const previewQuarantinedPrefixes = new Set<string>();

export function __resetWave3PreviewStateForTests(): void {
  previewCheckpointsByThread.clear();
  previewQuarantinedPrefixes.clear();
}

export async function vcsCheckpointList(threadId: string): Promise<VcsCheckpointRecordDto[]> {
  if (!backendAvailable()) {
    const existing = previewCheckpointsByThread.get(threadId);
    if (existing) return existing;
    const seed: VcsCheckpointRecordDto[] = [
      {
        thread_id: threadId,
        turn_count: 1,
        ref_name: `refs/vox/checkpoints/preview:${threadId}/turn/1`,
        commit_oid: 'preview:turn-1',
        is_restore_point: false,
        preview_reason: BROWSER_PREVIEW_REASON,
      },
    ];
    previewCheckpointsByThread.set(threadId, seed);
    return seed;
  }
  return callHonestMcpTool<
    VcsCheckpointRecordDto[] | { checkpoints?: VcsCheckpointRecordDto[] },
    VcsCheckpointRecordDto[]
  >('vox_checkpoint_list', { thread_id: threadId }, (data) => {
    if (Array.isArray(data)) return data;
    if (data && Array.isArray(data.checkpoints)) return data.checkpoints;
    return null;
  });
}

export async function vcsCheckpointCapture(
  threadId: string,
  turnCount?: number,
  restoreLabel?: string,
): Promise<VcsCheckpointRecordDto> {
  if (!backendAvailable()) {
    const current = await vcsCheckpointList(threadId);
    const nextTurn = turnCount ?? current.length + 1;
    const record: VcsCheckpointRecordDto = {
      thread_id: threadId,
      turn_count: nextTurn,
      ref_name: restoreLabel
        ? `refs/vox/checkpoints/preview:${threadId}/restore-point/${restoreLabel}`
        : `refs/vox/checkpoints/preview:${threadId}/turn/${nextTurn}`,
      commit_oid: `preview:turn-${nextTurn}`,
      is_restore_point: Boolean(restoreLabel),
      preview_reason: BROWSER_PREVIEW_REASON,
    };
    previewCheckpointsByThread.set(threadId, [...current, record]);
    return record;
  }
  const resolvedTurn = turnCount ?? (await vcsCheckpointList(threadId)).length + 1;
  return callHonestMcpTool<VcsCheckpointRecordDto, VcsCheckpointRecordDto>(
    'vox_checkpoint_capture',
    {
      thread_id: threadId,
      turn_count: resolvedTurn,
      restore_label: restoreLabel ?? null,
    },
    (data) => (data?.ref_name ? data : null),
  );
}

export async function vcsCheckpointRestore(
  threadId: string,
  targetTurn: number,
): Promise<{
  thread_id: string;
  restored_turn: number;
  ref_name: string;
  preview_reason?: string;
}> {
  if (!backendAvailable()) {
    const current = await vcsCheckpointList(threadId);
    previewCheckpointsByThread.set(
      threadId,
      current.filter((c) => c.turn_count <= targetTurn),
    );
    return {
      thread_id: threadId,
      restored_turn: targetTurn,
      ref_name: `refs/vox/checkpoints/preview:${threadId}/turn/${targetTurn}`,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  const current = await vcsCheckpointList(threadId);
  const maxTurn = current.reduce((acc, c) => Math.max(acc, c.turn_count), targetTurn);
  return callHonestMcpTool<
    { thread_id: string; restored_turn: number; ref_name: string },
    { thread_id: string; restored_turn: number; ref_name: string }
  >(
    'vox_checkpoint_restore',
    {
      thread_id: threadId,
      target_turn: targetTurn,
      max_known_turn: maxTurn,
    },
    (data) => (data?.ref_name ? data : null),
  );
}

export async function vcsCheckpointDiffSummary(
  threadId: string,
  targetTurn: number,
  fallbackFiles: readonly CheckpointFileDeltaDto[] = [],
): Promise<VcsCheckpointDiffSummaryDto> {
  if (!backendAvailable()) {
    return {
      thread_id: threadId,
      target_turn: targetTurn,
      ref_name: `refs/vox/checkpoints/preview:${threadId}/turn/${targetTurn}`,
      files: fallbackFiles,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<VcsCheckpointDiffSummaryDto, VcsCheckpointDiffSummaryDto>(
    'vox_vcs_checkpoint_diff_summary',
    {
      thread_id: threadId,
      target_turn: targetTurn,
    },
    (data) => {
      if (!data || !Array.isArray(data.files)) return null;
      return {
        thread_id: data.thread_id ?? threadId,
        target_turn: data.target_turn ?? targetTurn,
        ref_name: data.ref_name ?? `refs/vox/checkpoints/${threadId}/turn/${targetTurn}`,
        files: data.files,
      };
    },
  );
}

export async function fetchFileDiffAndDiagnostics(
  filePath: string,
  fallbackAdded = 0,
  fallbackDeleted = 0,
  fallbackDiagnostics: readonly CompilerDiagnosticSpanDto[] = [],
): Promise<FileDiffAndDiagnosticsDto> {
  if (!backendAvailable()) {
    return {
      file_path: filePath,
      added_lines: fallbackAdded,
      deleted_lines: fallbackDeleted,
      unified_diff: `--- a/${filePath}\n+++ b/${filePath}\n@@ -0,0 +0,0 @@`,
      diagnostics: fallbackDiagnostics,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<FileDiffAndDiagnosticsDto, FileDiffAndDiagnosticsDto>(
    'vox_file_diff_and_diagnostics',
    { file_path: filePath },
    (data) => {
      if (!data || typeof data.file_path !== 'string' || typeof data.unified_diff !== 'string') {
        return null;
      }
      return {
        ...data,
        diagnostics: Array.isArray(data.diagnostics) ? data.diagnostics : fallbackDiagnostics,
      };
    },
  );
}

export async function stageWorkspaceFile(
  filePath: string,
): Promise<{ file_path: string; staged: boolean; preview_reason?: string }> {
  if (!backendAvailable()) {
    return { file_path: filePath, staged: true, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<
    { file_path: string; staged: boolean },
    { file_path: string; staged: boolean }
  >('vox_stage_file', { file_path: filePath }, (data) =>
    data?.file_path && data.staged === true ? data : null,
  );
}

export async function revertWorkspaceFile(
  filePath: string,
): Promise<{ file_path: string; reverted: boolean; preview_reason?: string }> {
  if (!backendAvailable()) {
    return { file_path: filePath, reverted: true, preview_reason: BROWSER_PREVIEW_REASON };
  }
  return callHonestMcpTool<
    { file_path: string; reverted: boolean },
    { file_path: string; reverted: boolean }
  >('vox_revert_file', { file_path: filePath }, (data) =>
    data?.file_path && data.reverted === true ? data : null,
  );
}

export async function setDistrictQuarantine(
  pathPrefix: string,
  quarantined: boolean,
): Promise<{
  path_prefix: string;
  quarantined: boolean;
  quarantined_paths: string[];
  preview_reason?: string;
}> {
  const clean = pathPrefix.trim().replace(/\\/g, '/').replace(/^\/+|\/+$/g, '');
  if (!backendAvailable()) {
    if (quarantined) {
      previewQuarantinedPrefixes.add(clean);
    } else {
      previewQuarantinedPrefixes.delete(clean);
    }
    return {
      path_prefix: clean,
      quarantined,
      quarantined_paths: Array.from(previewQuarantinedPrefixes),
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<
    { path_prefix: string; quarantined: boolean; quarantined_paths: string[] },
    { path_prefix: string; quarantined: boolean; quarantined_paths: string[] }
  >('vox_quarantine_set', { path_prefix: clean, quarantined }, (data) =>
    data && Array.isArray(data.quarantined_paths) ? data : null,
  );
}

export async function listDistrictQuarantines(): Promise<string[]> {
  if (!backendAvailable()) {
    return Array.from(previewQuarantinedPrefixes);
  }
  return callHonestMcpTool<string[] | { quarantined_paths?: string[] }, string[]>(
    'vox_quarantine_list',
    {},
    (data) => {
      if (Array.isArray(data)) return data;
      if (data && Array.isArray(data.quarantined_paths)) return data.quarantined_paths;
      return null;
    },
  );
}

export async function worktreeSpawnVariant(
  runId: string,
  modelIds: string[],
): Promise<{ run_id: string; variant_count: number; preview_reason?: string }> {
  if (!backendAvailable()) {
    return {
      run_id: runId,
      variant_count: modelIds.length,
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<
    { run_id?: string; variant_count?: number; variants?: unknown[] },
    { run_id: string; variant_count: number }
  >('vox_muse_pool_spawn', { run_id: runId, model_ids: modelIds }, (data) => {
    if (!data || (typeof data.variant_count !== 'number' && !Array.isArray(data.variants))) {
      return null;
    }
    return {
      run_id: data.run_id ?? runId,
      variant_count:
        typeof data.variant_count === 'number' ? data.variant_count : data.variants!.length,
    };
  });
}

export async function vcsMusePoolAdopt(
  runId: string,
  winningVariantId: string,
): Promise<{
  run_id: string;
  adopted_variant_id: string;
  adopted_branch_name: string;
  pruned_variant_ids: string[];
  preview_reason?: string;
}> {
  if (!backendAvailable()) {
    return {
      run_id: runId,
      adopted_variant_id: winningVariantId,
      adopted_branch_name: `vox/muse/preview:${runId}/${winningVariantId}`,
      pruned_variant_ids: [],
      preview_reason: BROWSER_PREVIEW_REASON,
    };
  }
  return callHonestMcpTool<
    {
      run_id?: string;
      adopted_variant_id?: string;
      adopted_branch_name?: string;
      pruned_variant_ids?: string[];
      winner?:
        | string
        | {
            variant_id?: string;
            branch_name?: string;
            model_id?: string;
            variant_index?: number;
          };
    },
    {
      run_id: string;
      adopted_variant_id: string;
      adopted_branch_name: string;
      pruned_variant_ids: string[];
    }
  >(
    'vox_muse_pool_adopt',
    { run_id: runId, winning_variant_id: winningVariantId },
    (data) => {
      if (!data) return null;
      const winnerObj =
        data.winner && typeof data.winner === 'object' ? data.winner : undefined;
      const winnerId =
        typeof data.winner === 'string'
          ? data.winner
          : winnerObj?.variant_id ??
            winnerObj?.model_id ??
            (winnerObj?.variant_index != null ? `variant-${winnerObj.variant_index}` : undefined);
      const adoptedId = data.adopted_variant_id ?? winnerId;
      if (!adoptedId) return null;
      return {
        run_id: data.run_id ?? runId,
        adopted_variant_id: adoptedId,
        adopted_branch_name:
          data.adopted_branch_name ??
          winnerObj?.branch_name ??
          `vox/muse/${runId}/${adoptedId}`,
        pruned_variant_ids: Array.isArray(data.pruned_variant_ids) ? data.pruned_variant_ids : [],
      };
    },
  );
}
