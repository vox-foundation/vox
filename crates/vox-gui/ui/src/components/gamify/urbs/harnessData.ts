// crates/vox-gui/ui/src/components/gamify/urbs/harnessData.ts
import { voxTransport } from '../../../transport';
import { vcsCheckpointList, type VcsCheckpointRecordDto } from '../../../transport/t3Wave3Rpc';
import type { HarnessSnapshot } from './worldRenderer';

async function tryTap<T>(name: string, fn: () => Promise<T>): Promise<T | null> {
  try {
    return await fn();
  } catch (err) {
    console.error(`[urbs] ${name} unavailable:`, err);
    return null;
  }
}

/** One poll of every harness tap. Each failure → null (landmark unlit or calm default). */
export async function fetchHarnessSnapshot(): Promise<HarnessSnapshot> {
  const extendedTransport = voxTransport as typeof voxTransport & {
    mcpServersStatus?: () => Promise<Array<{ name: string; ok: boolean }>>;
    listAttentionItems?: () => Promise<
      Array<{ id: string | number; title?: string; summary?: string; source?: string; target_module?: string }>
    >;
    getContextBudget?: () => Promise<{ used_tokens?: number; max_tokens?: number }>;
  };

  const [ci, vcs, hopper, mcp, checkpoints, attentionRaw, contextBudget] = await Promise.all([
    tryTap('harness_ci_fleet_status', () => voxTransport.harnessCiFleetStatus()),
    tryTap('vcs_town_status', () => voxTransport.vcsTownStatus()),
    tryTap('hopper_list', () => voxTransport.hopperList()),
    typeof extendedTransport.mcpServersStatus === 'function'
      ? tryTap('mcp_servers_status', () => extendedTransport.mcpServersStatus!())
      : Promise.resolve(null),
    tryTap<VcsCheckpointRecordDto[]>('vcs_checkpoint_list', () => vcsCheckpointList('default')),
    typeof extendedTransport.listAttentionItems === 'function'
      ? tryTap('list_attention_items', () => extendedTransport.listAttentionItems!())
      : Promise.resolve(null),
    typeof extendedTransport.getContextBudget === 'function'
      ? tryTap('get_context_budget', () => extendedTransport.getContextBudget!())
      : Promise.resolve(null),
  ]);

  const contextFillPct =
    contextBudget &&
    typeof contextBudget.used_tokens === 'number' &&
    typeof contextBudget.max_tokens === 'number' &&
    contextBudget.max_tokens > 0
      ? Math.min(100, Math.max(5, Math.round((contextBudget.used_tokens / contextBudget.max_tokens) * 100)))
      : null;

  return {
    ci: ci ? { runners: ci.runners.filter((r) => r.online), queued: ci.queued } : null,
    vcs: vcs
      ? {
          branches: vcs.branches.map((b) => ({ name: b.name, isHead: b.is_head, track: b.track })),
          prs: vcs.prs_available ? vcs.prs.map((p) => ({ number: p.number, title: p.title })) : [],
        }
      : null,
    // hopper_list returns inbox PLUS assigned (in-flight); only non-assigned
    // items are honestly "queued" for the PORTVS ship count.
    queueLen: hopper ? hopper.filter((t) => t.state !== 'assigned').length : null,
    mcp: Array.isArray(mcp) ? mcp : null,
    checkpoints: Array.isArray(checkpoints) && checkpoints.length > 0
      ? checkpoints.map((cp, idx) => ({
          ordinal: cp.turn_count || idx + 1,
          refName: cp.ref_name,
          commitOid: cp.commit_oid,
        }))
      : null,
    attentionGates: Array.isArray(attentionRaw)
      ? attentionRaw.map((g) => ({
          id: String(g.id),
          title: g.title ?? g.summary ?? `Gate #${g.id}`,
          district: g.target_module,
        }))
      : null,
    contextFillPct,
  };
}

export const HARNESS_POLL_MS = 20_000;
