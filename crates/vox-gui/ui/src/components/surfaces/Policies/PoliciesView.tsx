import { useEffect, useMemo, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { cn } from '../../../lib/cn';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { Icon } from '../../ui/Icons';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { useLocalStorage } from '../../../hooks/useLocalStorage';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { buildGroupTree, needsAttention, overallWorst, statusForRow } from './policyTree';
import { invokeCmd, policySetEnabled, policyEdit } from '../../../transport';
import type { PolicyRow, PolicyDetail, PolicyStatus, BranchInfo, RunStatus } from './types';
import type { Toast } from '../../../types/tauri';

const STATUS_DOT: Record<RunStatus, string> = {
  fail: 'bg-red-500',
  warn: 'bg-amber-400',
  pass: 'bg-emerald-400',
  not_run: 'bg-text-muted',
};
function StatusGlyph({ status }: { status: RunStatus }) {
  switch (status) {
    case 'fail':
      return <Icon.circleDot className="size-2.5 shrink-0" aria-hidden="true" />;
    case 'warn':
      return <Icon.trendUp className="size-2.5 shrink-0" aria-hidden="true" />;
    case 'pass':
      return <Icon.check className="size-2.5 shrink-0" aria-hidden="true" />;
    case 'not_run':
      return <span aria-hidden="true">—</span>;
  }
}

function StatusCount({ counts }: { counts: Record<RunStatus, number> }) {
  return (
    <span className="flex items-center gap-1.5 font-mono text-[10px]">
      {(['fail', 'warn', 'pass', 'not_run'] as RunStatus[])
        .filter(s => counts[s] > 0)
        .map(s => (
          <span key={s} className={`flex items-center gap-0.5 ${s === 'fail' ? 'text-red-400' : s === 'warn' ? 'text-amber-300' : s === 'pass' ? 'text-emerald-300' : 'text-text-muted'}`}>
            <StatusGlyph status={s} />
            {counts[s]}
          </span>
        ))}
    </span>
  );
}

export function PoliciesView({
  pushToast,
  gamifyEnabled = false,
}: {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}) {
  const [rows, setRows] = useState<PolicyRow[]>([]);
  const [status, setStatus] = useState<PolicyStatus[]>([]);
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [selectedBranches, setSelectedBranches] = useState<string[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<PolicyDetail | null>(null);
  const [railCollapsed, setRailCollapsed] = useLocalStorage<boolean>('vox_policy_rail_collapsed', false);
  const [collapsedGroups, setCollapsedGroups] = useLocalStorage<Record<string, boolean>>('vox_policy_groups', {});
  const [editOpen, setEditOpen] = useState(false);
  const [editTitle, setEditTitle] = useState('');
  const [editDesc, setEditDesc] = useState('');
  const [ruleFilter, setRuleFilter] = useState<'all' | 'attention' | 'blocking'>('all');

  // Load catalog + branches once.
  useEffect(() => {
    invokeCmd<PolicyRow[]>('policy_list', { domain: null, group: null })
      .then(r => {
        const list = Array.isArray(r) ? r : [];
        setRows(list);
        if (list.length) setSelectedId(prev => prev ?? list[0].id);
      })
      .catch(err => pushToast({ tone: 'error', title: 'Policy catalog failed', body: sanitizeErrorForToast(err), cause: 'backend-error' }));
    invokeCmd<BranchInfo[]>('list_branches')
      .then(b => { setBranches(b); setSelectedBranches(b.filter(x => x.isCurrent).map(x => x.branch)); })
      .catch(() => setBranches([]));
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load policy catalog once on mount
  }, []);

  // Reload status when the branch selection changes.
  useEffect(() => {
    if (selectedBranches.length === 0) { setStatus([]); return; }
    invokeCmd<PolicyStatus[]>('policy_status', { branches: selectedBranches })
      .then(setStatus)
      .catch(() => setStatus([]));
  }, [selectedBranches]);

  // Load detail when the selected rule changes.
  useEffect(() => {
    if (!selectedId) { setDetail(null); return; }
    recordGamifyGuiEvent(
      'policy_rule_viewed',
      { rule_id: selectedId },
      { enabled: gamifyEnabled },
    );
    invokeCmd<PolicyDetail>('policy_show', { id: selectedId })
      .then(setDetail)
      .catch(err => pushToast({ tone: 'error', title: 'Detail failed', body: sanitizeErrorForToast(err), cause: 'backend-error' }));
  // eslint-disable-next-line react-hooks/exhaustive-deps -- re-fetch rule detail only when selectedId changes
  }, [selectedId]);

  const attention = useMemo(() => needsAttention(rows, status, selectedBranches), [rows, status, selectedBranches]);
  const filteredRows = useMemo(
    () =>
      ruleFilter === 'attention'
        ? attention
        : ruleFilter === 'blocking'
          ? rows.filter((r) => r.blocking)
          : rows,
    [rows, attention, ruleFilter],
  );
  const tree = useMemo(
    () => buildGroupTree(filteredRows, status, selectedBranches),
    [filteredRows, status, selectedBranches],
  );
  const worst = useMemo(() => overallWorst(rows, status, selectedBranches), [rows, status, selectedBranches]);
  const enabledCount = useMemo(() => rows.filter((r) => r.enabled).length, [rows]);
  const blockingCount = useMemo(() => rows.filter((r) => r.blocking).length, [rows]);
  const passingCount = useMemo(
    () => rows.filter((r) => statusForRow(r.id, status, selectedBranches) === 'pass').length,
    [rows, status, selectedBranches],
  );

  const refresh = () => {
    if (!selectedId) return;
    invokeCmd<PolicyDetail>('policy_show', { id: selectedId })
      .then(setDetail)
      .catch(err => pushToast({ tone: 'error', title: 'Detail failed', body: sanitizeErrorForToast(err), cause: 'backend-error' }));
  };

  const toggleBranch = (b: string) =>
    setSelectedBranches(prev => prev.includes(b) ? prev.filter(x => x !== b) : [...prev, b]);
  const toggleGroup = (g: string) =>
    setCollapsedGroups(prev => ({ ...prev, [g]: !prev[g] }));

  return (
    <div className="@container/surface flex h-full min-h-0 min-w-0 flex-col gap-3 p-3 @2xl/surface:p-4">
      <RomanSurfaceHeader
        domainTag="REVIEW · POLICIES"
        title="Governance Policy Ledger & Architectural Rules"
        subtitle="Audit compliance rules, structural invariants, and automated gate enforcement across branches."
        waxSealBadge={worst === 'fail' ? 'POLICY VIOLATIONS' : worst === 'warn' ? 'POLICY WARNINGS' : 'POLICY COMPLIANT'}
        icon={<Icon.gavel className="size-5 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'POLICIES',
            valueText: `${rows.length}`,
            ratio: rows.length > 0 ? enabledCount / rows.length : 0,
            tone: 'gold',
            purpose: 'Enabled governance policy rules out of total registered catalog',
            actionHint: 'Click to show all policy rules in rail',
            active: ruleFilter === 'all' && rows.length > 0,
            onClick: () => {
              setRuleFilter('all');
              setRailCollapsed(false);
            },
          },
          {
            label: 'ATTENTION',
            valueText: `${attention.length}`,
            ratio: rows.length > 0 ? attention.length / rows.length : 0,
            tone: attention.length > 0 ? 'crimson' : 'emerald',
            purpose: 'Policies failing or needing immediate attention',
            actionHint:
              ruleFilter === 'attention'
                ? 'Click to show all policy rules'
                : 'Click to filter policy tree to rules needing attention',
            active: ruleFilter === 'attention',
            onClick: () => {
              setRailCollapsed(false);
              setRuleFilter((f) => (f === 'attention' ? 'all' : 'attention'));
              if (attention[0]) setSelectedId(attention[0].id);
            },
          },
          {
            label: 'STATUS',
            valueText: worst.toUpperCase(),
            ratio:
              rows.length === 0
                ? 0
                : status.length > 0
                  ? passingCount / rows.length
                  : blockingCount / rows.length,
            tone: worst === 'pass' ? 'emerald' : worst === 'warn' ? 'gold' : 'crimson',
            purpose: 'Overall governance policy evaluation status and CI-blocking share',
            actionHint:
              ruleFilter === 'blocking'
                ? 'Click to show all policy rules'
                : 'Click to filter policy tree to CI-blocking rules',
            active: ruleFilter === 'blocking',
            onClick: () => {
              setRailCollapsed(false);
              setRuleFilter((f) => (f === 'blocking' ? 'all' : 'blocking'));
            },
          },
        ]}
      />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 @3xl/surface:flex-row @3xl/surface:gap-4">
      {/* SECONDARY: group rail (collapsible, stacks in <768px / 320px docks) */}
      <nav
        aria-label="Policy tree"
        className={cn(
          'min-w-0 transition-[width] duration-200 @3xl/surface:shrink-0',
          railCollapsed ? 'w-full @3xl/surface:w-14' : 'w-full @3xl/surface:w-[300px]',
        )}
      >
        <BentoCard className="h-full gap-3 overflow-hidden p-3">
          <div className="flex items-center justify-between gap-2">
            {!railCollapsed && (
              <span className="font-display text-[10px] uppercase tracking-[0.28em] text-text-muted">
                Policies <span className={`ml-1 inline-block size-1.5 rounded-full align-middle ${STATUS_DOT[worst]}`} />
              </span>
            )}
            <button type="button" onClick={() => setRailCollapsed(c => !c)}
              aria-label={railCollapsed ? 'Expand policy rail' : 'Collapse policy rail'}
              aria-expanded={!railCollapsed}
              title={railCollapsed ? 'Expand' : 'Collapse'}
              className="flex size-6 shrink-0 items-center justify-center rounded-md border border-border-subtle text-text-muted hover:bg-overlay-subtle hover:text-text-primary">
              <Icon.chevL aria-hidden="true" className={`size-3 transition-transform ${railCollapsed ? 'rotate-180' : ''}`} />
            </button>
          </div>

          {!railCollapsed && (
            <>
              {/* Multi-branch selector (worktrees) */}
              <div className="flex flex-wrap gap-1">
                {branches.map(b => (
                  <button key={b.branch} type="button" onClick={() => toggleBranch(b.branch)}
                    aria-pressed={selectedBranches.includes(b.branch)}
                    aria-label={`Branch: ${b.branch}`}
                    className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 font-mono text-[9px] border ${selectedBranches.includes(b.branch) ? 'border-brass/40 bg-brass/10 text-brass' : 'border-border-subtle text-text-muted hover:text-text-secondary'}`}>
                    <span>{b.branch}</span>
                    {b.isCurrent && <Icon.waxSeal className="size-2.5 shrink-0" aria-hidden="true" />}
                  </button>
                ))}
                {branches.length === 0 && <span className="font-mono text-[9px] text-text-muted">no git worktrees</span>}
              </div>

              {/* Needs attention — shrinks to an all-clear strip */}
              <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-2">
                {attention.length === 0 ? (
                  <div className="flex items-center gap-1.5 font-mono text-[10px] text-emerald-300/80">
                    <Icon.check aria-hidden="true" className="size-3" /> all clear
                  </div>
                ) : (
                  <div className="flex flex-col gap-0.5">
                    <span className="inline-flex items-center gap-1 font-display text-[9px] uppercase tracking-[0.2em] text-red-300/90">
                      <Icon.alert className="size-3 shrink-0" aria-hidden="true" />
                      <span>Needs attention ({attention.length})</span>
                    </span>
                    {attention.map(r => (
                      <button key={r.id} type="button" onClick={() => setSelectedId(r.id)}
                        className="text-left font-mono text-[10px] text-text-secondary hover:text-red-200 truncate">{r.id}</button>
                    ))}
                  </div>
                )}
              </div>

              {/* Group tree with status-colored counts */}
              <BentoScrollRegion
                ariaLabel="Policy rule groups"
                className="flex max-h-56 flex-1 flex-col gap-1 @3xl/surface:max-h-none"
              >
                {tree.map(node => {
                  const open = !collapsedGroups[node.group];
                  return (
                    <div key={node.group} className="flex flex-col">
                      <button type="button" onClick={() => toggleGroup(node.group)}
                        aria-expanded={open}
                        aria-label={`${node.group} group`}
                        className="flex items-center justify-between gap-2 rounded-md px-1.5 py-1 hover:bg-overlay-subtle">
                        <span className="flex items-center gap-1.5 min-w-0">
                          <span aria-hidden="true" className={`size-1.5 rounded-full shrink-0 ${STATUS_DOT[node.worst]}`} />
                          <span className="font-display text-[10px] text-text-secondary truncate">{node.group}</span>
                        </span>
                        <span className="flex items-center gap-1.5 shrink-0">
                          <StatusCount counts={node.counts} />
                          <Icon.chevronDown aria-hidden="true" className={`size-3 text-text-muted transition-transform ${open ? '' : '-rotate-90'}`} />
                        </span>
                      </button>
                      {open && node.rows.map(r => {
                        const s = statusForRow(r.id, status, selectedBranches);
                        return (
                          <button key={r.id} type="button" onClick={() => setSelectedId(r.id)}
                            aria-pressed={selectedId === r.id}
                            className={`flex items-center gap-1.5 rounded-md pl-5 pr-1.5 py-1 text-left ${selectedId === r.id ? 'bg-overlay-subtle text-text-primary' : 'text-text-muted hover:bg-overlay-subtle hover:text-text-secondary'}`}>
                            <span aria-hidden="true" className={`size-1 rounded-full shrink-0 ${STATUS_DOT[s]}`} />
                            <span className="font-mono text-[10px] truncate">{r.title}</span>
                          </button>
                        );
                      })}
                    </div>
                  );
                })}
              </BentoScrollRegion>
            </>
          )}
        </BentoCard>
      </nav>

      {/* PRIMARY: rule detail + contents (largest pane) */}
      <section aria-label="Policy detail" className="flex min-h-0 min-w-0 flex-1 flex-col">
        <BentoCard className="h-full overflow-hidden p-0">
          <BentoScrollRegion
            ariaLabel="Selected rule inspector"
            className="flex h-full flex-col gap-4 p-4 @2xl/surface:p-5"
          >
          {!detail ? (
            <div className="m-auto font-mono text-xs text-text-muted">select a policy</div>
          ) : (
            <>
              <header className="flex flex-wrap items-start justify-between gap-3 border-b border-border-subtle pb-3">
                <div className="min-w-0">
                  <div className="font-mono text-sm text-text-primary truncate">{detail.id}</div>
                  <div className="font-display text-[11px] text-text-muted">{detail.title}</div>
                </div>
                <div className="flex flex-wrap items-center gap-2 shrink-0">
                  <button type="button"
                    disabled={detail.protected}
                    title={detail.protected ? 'Protected policies cannot be edited' : 'Edit title and description'}
                    onClick={() => { setEditTitle(detail.title); setEditDesc(detail.description); setEditOpen(o => !o); }}
                    className={`flex items-center gap-1 rounded-md border border-border-subtle px-2 py-1 font-mono text-[10px] ${detail.protected ? 'text-text-muted opacity-50 cursor-not-allowed' : 'text-text-secondary hover:text-text-primary hover:bg-overlay-subtle'}`}>
                    <Icon.penQuill className="size-3 shrink-0" aria-hidden="true" />
                    <span>Edit</span>
                  </button>
                  <button type="button"
                    disabled={detail.protected}
                    title={detail.protected ? 'Protected policies cannot be toggled' : (detail.enabled ? 'Disable this policy' : 'Enable this policy')}
                    onClick={() => policySetEnabled(detail.id, !detail.enabled).then(refresh)}
                    className={`flex items-center gap-1 rounded-md border border-border-subtle px-2 py-1 font-mono text-[10px] ${detail.protected ? 'text-text-muted opacity-50 cursor-not-allowed' : 'text-text-secondary hover:text-text-primary hover:bg-overlay-subtle'}`}>
                    <Icon.power className="size-3 shrink-0" aria-hidden="true" />
                    <span>{detail.enabled ? 'Disable' : 'Enable'}</span>
                  </button>
                </div>
              </header>

              {editOpen && !detail.protected && (
                <section aria-label="Edit policy" className="flex flex-col gap-2 rounded-lg border border-border-subtle bg-overlay-subtle p-3">
                  <div className="font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">Edit title / description</div>
                  <input
                    type="text"
                    aria-label="Policy title"
                    value={editTitle}
                    onChange={e => setEditTitle(e.target.value)}
                    className="rounded-sm border border-border-subtle bg-black/30 px-2 py-1 font-mono text-xs text-text-primary focus:outline-hidden focus:ring-1 focus:ring-brass/40"
                  />
                  <textarea
                    aria-label="Policy description"
                    value={editDesc}
                    onChange={e => setEditDesc(e.target.value)}
                    rows={3}
                    className="rounded-sm border border-border-subtle bg-black/30 px-2 py-1 font-mono text-xs text-text-secondary focus:outline-hidden focus:ring-1 focus:ring-brass/40 resize-y"
                  />
                  <div className="flex gap-2">
                    <button type="button"
                      onClick={() => policyEdit(detail.id, editTitle, editDesc).then(() => { setEditOpen(false); refresh(); })}
                      className="rounded-md border border-brass/40 bg-brass/10 px-2 py-1 font-mono text-[10px] text-brass hover:bg-brass/20">
                      Save
                    </button>
                    <button type="button"
                      onClick={() => setEditOpen(false)}
                      className="rounded-md border border-border-subtle px-2 py-1 font-mono text-[10px] text-text-muted hover:text-text-primary">
                      Cancel
                    </button>
                  </div>
                </section>
              )}

              <section className="flex flex-col gap-1.5">
                <div className="font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">What it does</div>
                <p className="text-xs text-text-secondary leading-relaxed">{detail.description}</p>
                <div className="flex flex-wrap gap-3 pt-1 font-mono text-[10px] text-text-muted">
                  <span>domain: <span className="text-text-secondary">{detail.domain}</span></span>
                  <span>severity: <span className="text-text-secondary">{detail.severity ?? '—'}</span></span>
                  <span>{detail.blocking ? 'blocking' : 'non-blocking'}</span>
                  {detail.protected && <span className="text-amber-300/80">protected</span>}
                  <span>runs on: <span className="text-text-secondary">{detail.runsOn.join(', ') || '—'}</span></span>
                  <span>origin: <span className="text-text-secondary">{detail.origin}</span></span>
                </div>
              </section>

              <section className="flex flex-col gap-1.5">
                <div className="font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">Contents (edit target)</div>
                <div className="rounded-lg border border-border-subtle bg-black/30 p-3 font-mono text-[11px] text-text-secondary">
                  <div className="text-text-muted">kind: <span className="text-text-secondary">{detail.sourceKind}</span></div>
                  <div className="text-text-muted">source: <span className="text-text-secondary break-all">{detail.sourceRef}</span></div>
                  {detail.sourceDetail && (
                    <pre className="mt-2 whitespace-pre-wrap break-all text-emerald-200/80">{detail.sourceDetail}</pre>
                  )}
                </div>
                {detail.docs && (
                  <button
                    type="button"
                    onClick={() => {
                      void invokeCmd('open_locator', { locator: { kind: 'file', value: detail.docs } }).catch(() => {});
                    }}
                    className="inline-flex w-fit items-center gap-1 rounded border border-brass/30 bg-brass/10 px-2 py-0.5 font-mono text-[10px] text-brass hover:bg-brass/20"
                  >
                    <span>{detail.docs}</span>
                    <Icon.externalLink className="size-3 shrink-0" aria-hidden="true" />
                  </button>
                )}
              </section>

              <section className="flex flex-col gap-1.5">
                <div className="font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">Last run (per branch)</div>
                <div className="flex flex-wrap gap-2">
                  {selectedBranches.length === 0 && <span className="font-mono text-[10px] text-text-muted">no branch selected</span>}
                  {selectedBranches.map(b => {
                    const s = statusForRow(detail.id, status, [b]);
                    return (
                      <span key={b} className="flex items-center gap-1.5 rounded-full border border-border-subtle px-2 py-0.5 font-mono text-[10px]">
                        <span className={`size-1.5 rounded-full ${STATUS_DOT[s]}`} />
                        <span className="text-text-secondary">{b}</span>
                        <span className="text-text-muted">{s}</span>
                      </span>
                    );
                  })}
                </div>
              </section>

              <BentoGrid preset="triad" className="gap-2.5">
                <div className="min-w-0 rounded-lg border border-[#D4AF37]/30 bg-bg-surface p-2.5">
                  <div className="font-mono text-[9px] uppercase tracking-wider text-[#D4AF37]">CLI Invocation</div>
                  <div className="mt-1 font-mono text-[11px] text-[#F3E5AB]">vox audit code</div>
                  <div className="mt-0.5 truncate font-mono text-[10px] text-[#D6CEBF]">--rule {detail.id}</div>
                </div>
                <div className="min-w-0 rounded-lg border border-[#D4AF37]/30 bg-bg-surface p-2.5">
                  <div className="font-mono text-[9px] uppercase tracking-wider text-[#D4AF37]">Severity Tier</div>
                  <div className="mt-1 font-mono text-[11px] uppercase text-[#F3E5AB]">{detail.severity ?? 'info'}</div>
                  <div className="mt-0.5 font-mono text-[10px] text-[#D6CEBF]">
                    {detail.blocking ? 'Blocks CI merge' : 'Advisory warning'}
                  </div>
                </div>
                <div className="min-w-0 rounded-lg border border-[#D4AF37]/30 bg-bg-surface p-2.5">
                  <div className="font-mono text-[9px] uppercase tracking-wider text-[#D4AF37]">Detector Ref</div>
                  <div className="mt-1 truncate font-mono text-[11px] text-emerald-300">{detail.sourceRef}</div>
                  <div className="mt-0.5 font-mono text-[10px] text-[#D6CEBF]">{detail.sourceKind}</div>
                </div>
              </BentoGrid>

              <section
                data-testid="policy-enforcement-deck"
                className="mt-auto grid grid-cols-1 gap-3 border-t border-[#D4AF37]/25 pt-4 @xl/surface:grid-cols-2"
              >
                <div className="min-w-0 rounded-lg border border-[#D4AF37]/35 bg-bg-surface p-3">
                  <div className="flex flex-wrap items-center justify-between gap-1.5">
                    <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Enforcement Gate</span>
                    <RomanWaxSealBadge
                      label={detail.blocking ? 'Blocking' : 'Advisory'}
                      tone={detail.blocking ? 'crimson' : 'gold'}
                    />
                  </div>
                  <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
                    Evaluated by <code className="text-[#F3E5AB]">vox audit code</code> and <code className="text-[#F3E5AB]">vox ci pre-push</code> across selected worktree branches.
                  </p>
                </div>
                <div className="min-w-0 rounded-lg border border-[#D4AF37]/35 bg-bg-surface p-3">
                  <div className="flex flex-wrap items-center justify-between gap-1.5">
                    <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Rule Provenance</span>
                    <RomanWaxSealBadge label={detail.origin} tone="emerald" />
                  </div>
                  <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
                    Emits structured JSONL findings with <code className="text-[#F3E5AB]">schema_version</code> and rule discriminators for effort routing.
                  </p>
                </div>
              </section>
            </>
          )}
          </BentoScrollRegion>
        </BentoCard>
      </section>
      </div>
    </div>
  );
}
