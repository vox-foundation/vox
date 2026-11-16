import React from 'react';
import { BentoScrollRegion } from '../../ui/BentoGrid';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';

export interface PendingApproval {
  approval_id: string;
  tool: string;
  summary: string;
  requested_at_ms: number;
}

export type PermissionMode = 'ask' | 'accept_edits' | 'accept_all' | 'plan';

export const PERMISSION_MODE_OPTIONS: { id: PermissionMode; label: string; hint: string }[] = [
  { id: 'ask', label: 'Ask', hint: 'Always park dangerous tool calls for approval (default).' },
  { id: 'accept_edits', label: 'Accept Edits', hint: 'Auto-approve reversible file edits; still park destructive actions.' },
  { id: 'accept_all', label: 'Accept All', hint: 'Auto-approve mutating and destructive tool calls.' },
  { id: 'plan', label: 'Plan', hint: 'Read-only / planning mode.' },
];

export function formatRequestedAt(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return '—';
  const deltaSec = Math.round((Date.now() - ms) / 1000);
  let rel: string;
  if (deltaSec < 5) rel = 'just now';
  else if (deltaSec < 60) rel = `${deltaSec}s ago`;
  else if (deltaSec < 3600) rel = `${Math.floor(deltaSec / 60)}m ago`;
  else if (deltaSec < 86400) rel = `${Math.floor(deltaSec / 3600)}h ago`;
  else rel = `${Math.floor(deltaSec / 86400)}d ago`;
  return `${rel} · ${new Date(ms).toLocaleTimeString()}`;
}

export type RiskClassId = 'read_only' | 'writes_files' | 'shell_exec' | 'policy_mutation';

export function classifyApprovalRisk(tool: string): RiskClassId {
  if (/allowlist|policy|secret|permission/i.test(tool)) return 'policy_mutation';
  if (/shell|exec|bash|cmd|dispatch/i.test(tool)) return 'shell_exec';
  if (/patch|write|edit|delete|remove|store/i.test(tool)) return 'writes_files';
  return 'read_only';
}

const RISK_MATRIX: Array<{
  id: RiskClassId;
  badge: string;
  examples: string;
  toneClass: string;
  badgeClass: string;
}> = [
  {
    id: 'read_only',
    badge: 'AUTO',
    examples: 'vox_search_query, vox_skill_list',
    toneClass: 'text-emerald-400',
    badgeClass: 'border-emerald-400/30 bg-emerald-400/10 text-emerald-400',
  },
  {
    id: 'writes_files',
    badge: 'ACCEPT EDITS',
    examples: 'vox_patch, vox_plan_resume',
    toneClass: 'text-text-primary',
    badgeClass: 'border-[#D4AF37]/40 bg-[#78350F]/20 text-text-primary',
  },
  {
    id: 'shell_exec',
    badge: 'ASK GATE',
    examples: 'vox_run_shell, vox_mesh_dispatch',
    toneClass: 'text-amber-400',
    badgeClass: 'border-amber-400/30 bg-amber-400/10 text-amber-400',
  },
  {
    id: 'policy_mutation',
    badge: 'ALWAYS HITL',
    examples: 'vox_add_approval_allowlist_entry',
    toneClass: 'text-rose-400',
    badgeClass: 'border-rose-400/30 bg-rose-400/10 text-rose-400',
  },
];

interface ApprovalsInspectorPaneProps {
  selected: PendingApproval | null;
  permissionMode: PermissionMode;
  resolvingId: string | null;
  alwaysAllowChecked: boolean;
  onToggleAlwaysAllow: (checked: boolean) => void;
  onResolve: (approvalId: string, outcome: 'approved' | 'rejected', tool: string) => void;
}

export function ApprovalsInspectorPane({
  selected,
  permissionMode,
  resolvingId,
  alwaysAllowChecked,
  onToggleAlwaysAllow,
  onResolve,
}: ApprovalsInspectorPaneProps) {
  const activeRisk = selected ? classifyApprovalRisk(selected.tool) : null;
  const busy = selected ? resolvingId === selected.approval_id : false;

  return (
    <BentoScrollRegion
      ariaLabel="Approval policy and payload inspector"
      data-testid="approvals-inspector-pane"
      className="flex-1 pr-1"
    >
      <div className="flex flex-col gap-3">
        {selected ? (
          <div className="rounded-xl border border-[#D4AF37]/40 bg-bg-surface p-3.5">
            <div className="flex items-center justify-between gap-2">
              <span className="font-display text-xs tracking-widest text-brass uppercase">
                {`Request #${selected.approval_id}`}
              </span>
              <RomanWaxSealBadge
                label={activeRisk ?? 'ask_gate'}
                tone={activeRisk === 'policy_mutation' || activeRisk === 'shell_exec' ? 'crimson' : 'gold'}
              />
            </div>

            <div className="mt-2.5 space-y-1.5 font-mono text-[11px] text-text-secondary">
              <div>{`Tool: ${selected.tool}`}</div>
              <div>{`Queued: ${formatRequestedAt(selected.requested_at_ms)}`}</div>
            </div>

            <div className="mt-2.5 rounded-lg border border-border-subtle bg-bg-base/80 p-2.5 font-mono text-[11px] text-text-primary break-all">
              {`Payload: ${selected.summary}`}
            </div>

            <div className="mt-3 flex flex-col gap-2.5 border-t border-border-subtle pt-3">
              <label className="flex items-center gap-2 text-[11px] text-text-secondary cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={alwaysAllowChecked}
                  onChange={(e) => onToggleAlwaysAllow(e.target.checked)}
                  aria-label={`Persist repository allowlist rule for ${selected.tool}`}
                />
                <span>Persist tool in repository allowlist (requires HITL confirmation)</span>
              </label>

              <div className="flex flex-wrap items-center gap-2">
                <Button
                  type="button"
                  data-testid="inspector-approve-btn"
                  aria-label="Confirm parked tool execution"
                  disabled={busy}
                  onClick={() => onResolve(selected.approval_id, 'approved', selected.tool)}
                  variant="outline"
                  size="xs"
                  className="border-emerald-500/30 text-emerald-400 hover:bg-emerald-500/10"
                >
                  <Icon.check className="size-3 mr-1" aria-hidden="true" />
                  Grant Execution
                </Button>
                <Button
                  type="button"
                  data-testid="inspector-reject-btn"
                  aria-label="Deny parked tool execution"
                  disabled={busy}
                  onClick={() => onResolve(selected.approval_id, 'rejected', selected.tool)}
                  variant="outline"
                  size="xs"
                  className="border-rose-500/30 text-rose-400 hover:bg-rose-500/10"
                >
                  <Icon.x className="size-3 mr-1" aria-hidden="true" />
                  Deny Execution
                </Button>
              </div>
            </div>
          </div>
        ) : (
          <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-4">
            <div className="flex items-center justify-between gap-2">
              <span className="font-display text-xs tracking-widest text-emerald-300 uppercase">
                Gate Secured · No Parked Payloads
              </span>
              <RomanWaxSealBadge label={permissionMode.replace(/_/g, ' ')} tone="emerald" />
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Select any parked request from the ledger when one arrives to inspect its command arguments, capability tier, and repository allowlist posture.
            </p>
          </div>
        )}

        <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
          <div className="mb-2.5 flex flex-wrap items-center justify-between gap-2">
            <span className="font-display text-[10px] tracking-widest text-brass uppercase">
              Tool Capability Risk Classification Matrix
            </span>
            <span className="rounded border border-[#D4AF37]/25 bg-bg-elevated px-2 py-0.5 font-mono text-[10px] text-text-secondary">
              contracts/terminal/exec-policy.v1.yaml
            </span>
          </div>
          <div className="grid grid-cols-1 gap-2 text-[11px] @xl/surface:grid-cols-2">
            {RISK_MATRIX.map((tier) => {
              const highlighted = activeRisk === tier.id;
              return (
                <div
                  key={tier.id}
                  className={`rounded-lg border p-2.5 transition-colors ${
                    highlighted
                      ? 'border-brass bg-brass/10'
                      : 'border-[#D4AF37]/25 bg-bg-elevated'
                  }`}
                >
                  <div className="flex items-center justify-between gap-1">
                    <span className={`font-mono text-[10px] ${tier.toneClass}`}>{tier.id}</span>
                    <span className={`rounded border px-1.5 py-0.5 font-mono text-[9px] ${tier.badgeClass}`}>
                      {tier.badge}
                    </span>
                  </div>
                  <div className="mt-1 font-mono text-[10px] text-text-secondary">{tier.examples}</div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </BentoScrollRegion>
  );
}
