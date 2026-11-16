import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { BentoCard, BentoGrid, BentoScrollRegion, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { EmptyState } from '../../ui/EmptyState';
import { DataTable } from '../../ui/DataTable';
import { Button } from '../../ui/Button';
import { Segment } from '../../ui/Segment';
import { Icon } from '../../ui/Icons';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { APPROVALS_POLL_MS } from '../../../config/constants';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { useLabel } from '../../../hooks/useLanguage';
import {
  parsePendingApprovals,
  unwrapMcpEnvelope,
} from '../../../lib/mcpToolResult';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { voxTransport, getPermissionMode, setPermissionMode as setTransportPermissionMode } from '../../../transport';
import type { Toast } from '../../../types/tauri';
import {
  ApprovalsInspectorPane,
  PERMISSION_MODE_OPTIONS,
  type PendingApproval,
  type PermissionMode,
  formatRequestedAt,
} from './ApprovalsInspectorPane';

interface ApprovalsViewProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}

const PERMISSION_MODE_STORAGE_KEY = 'vox.approvals.permission_mode';

function isKnownMode(value: unknown): value is PermissionMode {
  return value === 'ask' || value === 'accept_edits' || value === 'accept_all' || value === 'plan';
}

function resolveAndApplyInitialPermissionMode(): PermissionMode {
  const live = getPermissionMode();
  if (isKnownMode(live)) return live;

  let stored: unknown = null;
  try {
    stored = window.localStorage?.getItem(PERMISSION_MODE_STORAGE_KEY);
  } catch {
    // localStorage unavailable (e.g. embedded surface) — fall back to the safe default.
  }
  const resolved = isKnownMode(stored) ? stored : 'ask';
  setTransportPermissionMode(resolved);
  return resolved;
}

export function ApprovalsView({ pushToast, gamifyEnabled = false }: ApprovalsViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [approvals, setApprovals] = useState<PendingApproval[]>([]);
  const [loading, setLoading] = useState(true);
  const [resolving, setResolving] = useState<string | null>(null);
  const [permissionMode, setPermissionModeState] = useState<PermissionMode>(
    resolveAndApplyInitialPermissionMode
  );
  const [alwaysAllow, setAlwaysAllow] = useState<Record<string, boolean>>({});
  const [queueFilter, setQueueFilter] = useState<'all' | 'shell'>('all');
  const [selectedApprovalId, setSelectedApprovalId] = useState<string | null>(null);
  const queueRegionRef = React.useRef<HTMLDivElement | null>(null);

  const setMode = useCallback((mode: PermissionMode) => {
    setPermissionModeState(mode);
    setTransportPermissionMode(mode);
    try {
      window.localStorage?.setItem(PERMISSION_MODE_STORAGE_KEY, mode);
    } catch {
      // best-effort only — an in-memory mode for this session is still correct.
    }
  }, []);

  const refresh = useCallback(async () => {
    try {
      const res = await voxTransport.invokeMcpTool('vox_pending_approvals', {});
      setApprovals(
        parsePendingApprovals({
          tool: 'vox_pending_approvals',
          is_error: !res || !!res.is_error,
          result: res ? res.result : undefined,
        })
      );
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Approvals load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => {
    refresh();
    if (embedded) return;
    const id = setInterval(refresh, APPROVALS_POLL_MS);
    return () => clearInterval(id);
  }, [refresh, embedded]);

  const resolve = useCallback(
    async (approvalId: string, outcome: 'approved' | 'rejected', tool: string) => {
      setResolving(approvalId);
      try {
        if (outcome === 'approved' && alwaysAllow[approvalId]) {
          try {
            const allowRes = await voxTransport.invokeMcpTool('vox_add_approval_allowlist_entry', { tool });
            if (!allowRes || allowRes.is_error) {
              pushToast({ tone: 'warn', title: 'Allowlist not saved', body: tool, cause: 'backend-error' });
            }
          } catch (err) {
            pushToast({ tone: 'warn', title: 'Allowlist not saved', body: sanitizeErrorForToast(err), cause: 'backend-error' });
          }
        }

        const res = await voxTransport.invokeMcpTool('vox_resolve_approval', { approval_id: approvalId, outcome });
        const data = res ? (unwrapMcpEnvelope(res.result) as { resolved?: boolean } | null) : null;
        if (!res || res.is_error || data?.resolved === false) {
          pushToast({ tone: 'warn', title: 'Resolve failed', body: `Could not ${outcome.replace('ed', '')} ${approvalId}`, cause: 'backend-error' });
        } else {
          pushToast({
            tone: outcome === 'approved' ? 'ok' : 'warn',
            title: outcome === 'approved' ? 'Approved' : 'Rejected',
            body: approvalId,
            cause: 'backend-ok',
          });
          void recordGamifyGuiEvent(
            'approval_decision',
            { approval_id: approvalId, outcome },
            { enabled: gamifyEnabled }
          );
        }
        setApprovals((prev) => prev.filter((a) => a.approval_id !== approvalId));
        setAlwaysAllow((prev) => {
          const next = { ...prev };
          // toestub-ignore(vox/ui/layout-and-perf-hazards) -- shallow-clone key removal (not V8 dictionary-mode hot path)
          delete next[approvalId];
          return next;
        });
        await refresh();
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Resolve failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      } finally {
        setResolving(null);
      }
    },
    [pushToast, refresh, gamifyEnabled, alwaysAllow]
  );

  const shellApprovalsCount = approvals.filter((a) => /shell|exec|bash|cmd/i.test(a.tool)).length;
  const displayedApprovals =
    queueFilter === 'shell'
      ? approvals.filter((a) => /shell|exec|bash|cmd/i.test(a.tool))
      : approvals;
  const selectedApproval =
    displayedApprovals.find((a) => a.approval_id === selectedApprovalId) ??
    displayedApprovals[0] ??
    null;

  const columns = [
    {
      key: 'approval_id',
      header: 'Request ID',
      width: 150,
      render: (r: PendingApproval) => {
        const isCurrent = selectedApproval?.approval_id === r.approval_id;
        return (
          <button
            type="button"
            data-testid={`select-approval-${r.approval_id}`}
            onClick={() => setSelectedApprovalId(r.approval_id)}
            className={`rounded px-1.5 py-0.5 font-mono text-left transition-colors ${
              isCurrent
                ? 'bg-brass/15 text-brass'
                : 'text-text-muted hover:bg-overlay-subtle hover:text-text-primary'
            }`}
          >
            #{r.approval_id}
          </button>
        );
      },
    },
    {
      key: 'tool',
      header: 'Tool',
      width: 120,
      render: (r: PendingApproval) => <span className="font-mono text-xs text-brass">{r.tool}</span>
    },
    { key: 'summary', header: 'Action Description' },
    {
      key: 'requested_at',
      header: 'Requested At',
      width: 180,
      render: (r: PendingApproval) => (
        <span className="flex items-center gap-1 font-mono text-[10px] text-text-muted">
          <Icon.clock className="size-3" aria-hidden="true" />
          {formatRequestedAt(r.requested_at_ms)}
        </span>
      )
    },
    {
      key: 'actions',
      header: 'Actions',
      width: 260,
      render: (r: PendingApproval) => {
        const busy = resolving === r.approval_id;
        return (
          <div
            role="group"
            aria-label={`Resolve approval for ${r.tool}`}
            className="flex items-center gap-2"
          >
            <label className="flex items-center gap-1 text-[10px] text-text-muted cursor-pointer select-none">
              <input
                type="checkbox"
                checked={!!alwaysAllow[r.approval_id]}
                onChange={(e) =>
                  setAlwaysAllow((prev) => ({ ...prev, [r.approval_id]: e.target.checked }))
                }
                aria-label={`Always allow ${r.tool} in this repository`}
              />
              Always allow
            </label>
            <Button
              onClick={() => resolve(r.approval_id, 'approved', r.tool)}
              disabled={busy}
              aria-label={`Approve ${r.summary}`}
              variant="outline"
              size="xs"
              className="border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/10"
            >
              <Icon.check className="size-3 mr-1" aria-hidden="true" /> Approve
            </Button>
            <Button
              onClick={() => resolve(r.approval_id, 'rejected', r.tool)}
              disabled={busy}
              aria-label={`Reject ${r.summary}`}
              variant="outline"
              size="xs"
              className="border-rose-500/20 text-rose-400 hover:bg-rose-500/10"
            >
              <Icon.x className="size-3 mr-1" aria-hidden="true" /> Reject
            </Button>
          </div>
        );
      },
    },
  ];

  const modeIndex = Math.max(
    0,
    PERMISSION_MODE_OPTIONS.findIndex((m) => m.id === permissionMode),
  );

  const masterPane = (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <BentoGrid preset="triad" data-testid="approvals-security-deck">
        <BentoCard className="border-[#D4AF37]/35">
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Active Gate Mode</span>
            <RomanWaxSealBadge label={permissionMode.replace(/_/g, ' ')} tone="gold" />
          </div>
          <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
            {PERMISSION_MODE_OPTIONS.find((m) => m.id === permissionMode)?.hint}
          </p>
        </BentoCard>

        <BentoCard className="border-[#D4AF37]/35">
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Authenticated Wire</span>
            <RomanWaxSealBadge label="DispatchRequest" tone="emerald" />
          </div>
          <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
            Permission mode rides on the top-level authenticated IPC envelope rather than untrusted tool JSON arguments.
          </p>
        </BentoCard>

        <BentoCard className="border-[#D4AF37]/35">
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Allowlist Guard</span>
            <RomanWaxSealBadge label={`${approvals.length} queued`} tone={approvals.length > 0 ? 'crimson' : 'emerald'} />
          </div>
          <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
            Adding a tool to the repository allowlist (Always allow) requires explicit HITL confirmation.
          </p>
        </BentoCard>
      </BentoGrid>

      <BentoScrollRegion ariaLabel="Pending approval queue" className="flex-1 pr-1">
        <div ref={queueRegionRef} aria-live="polite">
          <DataTable
            rows={displayedApprovals}
            columns={columns}
            getRowId={(r) => r.approval_id}
            loading={loading}
            density="compact"
            emptyState={
              <EmptyState
                icon={<Icon.check className="size-8 text-emerald-300" />}
                title="No pending approvals"
                description="Dangerous tool invocations will park here for a human to review."
              />
            }
          />
        </div>
      </BentoScrollRegion>
    </div>
  );

  return (
    <div className="flex h-full min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4">
      <RomanSurfaceHeader
        domainTag="REVIEW · APPROVALS"
        title="Pre-Execution Approval Ledger & Security Gate"
        subtitle="Inspect parked tool executions, permission policies, and capability gates."
        waxSealBadge={approvals.length > 0 ? 'GATE PENDING' : 'GATE SECURED'}
        icon={<Icon.shield className="size-5 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'PENDING',
            valueText: `${approvals.length}`,
            ratio:
              approvals.length === 0
                ? 0
                : queueFilter === 'shell'
                  ? shellApprovalsCount / approvals.length
                  : displayedApprovals.length / approvals.length,
            tone: approvals.length > 0 ? 'crimson' : 'emerald',
            purpose: 'Parked actions requiring explicit human approval',
            actionHint:
              queueFilter === 'shell'
                ? 'Click to show all parked approvals'
                : 'Click to filter queue to shell/exec approvals',
            active: queueFilter === 'shell',
            onClick: () => {
              setQueueFilter((prev) => (prev === 'shell' ? 'all' : 'shell'));
              queueRegionRef.current?.scrollIntoView?.({ block: 'nearest' });
            },
          },
          {
            label: 'MODE',
            valueText: permissionMode.toUpperCase().slice(0, 6),
            ratio: loading ? 0 : (modeIndex + 1) / PERMISSION_MODE_OPTIONS.length,
            tone: 'gold',
            purpose: 'Active permission gate security posture',
            actionHint: 'Click to cycle active permission gate mode',
            active: permissionMode !== 'ask',
            onClick: () => {
              const next =
                PERMISSION_MODE_OPTIONS[(modeIndex + 1) % PERMISSION_MODE_OPTIONS.length]!;
              setMode(next.id);
            },
          },
        ]}
        actions={
          <div className="flex items-center gap-3">
            <span className="sr-only">{useLabel('appr-pending')}</span>
            <div role="group" aria-label="Permission mode" className="flex items-center gap-2">
              <span className="text-[10px] uppercase tracking-[0.15em] text-[#D4AF37]">Mode</span>
              <Segment
                value={permissionMode}
                onChange={(id) => setMode(id as PermissionMode)}
                options={PERMISSION_MODE_OPTIONS}
                size="xs"
              />
            </div>
            <Button variant="ghost" size="xs" onClick={refresh} aria-label="Refresh approvals">
              <Icon.refresh className="size-4 text-[#D6CEBF]" />
            </Button>
          </div>
        }
      />

      <MasterDetailWorkbench
        storageKey="vox.layout.approvals.v4"
        detailTitle="Approval Policy & Payload"
        detailDescription="Selected approval command payload, capability risk posture, and quick-resolve controls."
        hasActiveDetail={selectedApprovalId !== null}
        defaultMasterPct={60}
        master={masterPane}
        detail={
          <ApprovalsInspectorPane
            selected={selectedApproval}
            permissionMode={permissionMode}
            resolvingId={resolving}
            alwaysAllowChecked={selectedApproval ? !!alwaysAllow[selectedApproval.approval_id] : false}
            onToggleAlwaysAllow={(checked) => {
              if (!selectedApproval) return;
              setAlwaysAllow((prev) => ({ ...prev, [selectedApproval.approval_id]: checked }));
            }}
            onResolve={(id, outcome, tool) => void resolve(id, outcome, tool)}
          />
        }
      />
    </div>
  );
}
